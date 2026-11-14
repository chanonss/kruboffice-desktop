//! Desktop recovery runs the workbook encoding and disk writes off the UI thread.
use super::SheetApp;
use std::sync::mpsc::{Receiver, TryRecvError};

const INTERVAL_MS: f64 = 30_000.0;

#[derive(Default)]
pub(crate) struct Runtime {
    last: f64,
    running: Option<Receiver<Result<usize, String>>>,
}

impl SheetApp {
    pub(super) fn tick_recovery(&mut self, ctx: &egui::Context) {
        let Some(dir) = self.session.recovery_dir.clone() else { return };
        if let Some(rx) = &self.recovery_rt.running {
            match rx.try_recv() {
                Ok(result) => {
                    self.recovery_rt.running = None;
                    if let Err(e) = result {
                        self.recovery_warning(&e);
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    self.recovery_rt.running = None;
                    self.recovery_warning("Recovery checkpoint worker ended unexpectedly");
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(500));
                    return;
                }
            }
        }
        let now = crate::clock::monotonic_ms();
        if now - self.recovery_rt.last < INTERVAL_MS {
            let remaining = (INTERVAL_MS - (now - self.recovery_rt.last)).clamp(1_000.0, INTERVAL_MS) as u64;
            ctx.request_repaint_after(std::time::Duration::from_millis(remaining));
            return;
        }
        // Capture committed workbook state without committing an in-progress cell or object
        // edit. Other completed edits still get checkpoints during a long editing session.
        self.recovery_rt.last = now;
        gridcraft_engine::recovery::clean(&self.session);
        let batch = gridcraft_engine::recovery::capture(&self.session);
        if batch.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(INTERVAL_MS as u64));
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        match std::thread::Builder::new().name("khitkrub-recovery".into()).spawn(move || {
            let result = gridcraft_engine::recovery::write_batch(batch, &dir).map_err(|e| e.to_string());
            let _ = tx.send(result);
        }) {
            Ok(_) => {
                self.recovery_rt.running = Some(rx);
                ctx.request_repaint_after(std::time::Duration::from_millis(500));
            }
            Err(e) => self.recovery_warning(&e.to_string()),
        }
    }
    fn recovery_warning(&mut self, error: &str) {
        log::warn!("Recovery checkpoint: {error}");
        if self.notice.is_none() {
            let label = self.ui.language.tr("Could not save a recovery copy");
            self.notice = Some(crate::Notice { text: format!("{label}: {error}"), kind: crate::NoticeKind::Warning, action: None });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn timer_writes_dirty_workbooks_on_a_worker() {
        let dir = std::env::temp_dir().join(format!("khitkrub-ui-recovery-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut session = gridcraft_engine::Session::new();
        session.recovery_dir = Some(dir.clone());
        session.new_workbook();
        session.execute("cell.set", json!({"cell":"A1", "input":"draft"})).unwrap();
        let mut app = SheetApp::new(session, Default::default());
        app.recovery_rt.last = -INTERVAL_MS;
        let ctx = egui::Context::default();
        app.tick_recovery(&ctx);
        assert!(app.recovery_rt.running.is_some());
        for _ in 0..500 {
            app.tick_recovery(&ctx);
            if app.recovery_rt.running.is_none() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(app.recovery_rt.running.is_none(), "the worker completed");
        assert!(dir.read_dir().unwrap().flatten().any(|e| e.path().extension().is_some_and(|e| e == "krubrecovery")));
        let _ = std::fs::remove_dir_all(dir);
    }
}
