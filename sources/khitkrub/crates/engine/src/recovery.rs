//! Desktop crash recovery. Each checkpoint is one private, atomically replaced file containing
//! bounded metadata and a macro-capable XLSM snapshot. A restored workbook has no save path.
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{DocState, EngineError, Result, Session, Trust};

const MAGIC: &[u8; 8] = b"KRUBREC1";
const MAX_META: usize = 64 * 1024;
const MAX_FILE: u64 = 256 * 1024 * 1024;
const MAX_ENTRIES: usize = 1024;
const EXT: &str = "krubrecovery";
static LOCKS: Mutex<Vec<(PathBuf, u64, File)>> = Mutex::new(Vec::new());
static WRITES: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Metadata {
    title: String,
    saved: u64,
    source_dir: Option<PathBuf>,
    source_path: Option<PathBuf>,
}

/// A process-unique identifier, kept within JSON's exact integer range.
fn owner_id() -> u64 {
    static OWNER: OnceLock<u64> = OnceLock::new();
    *OWNER.get_or_init(|| {
        use std::hash::{BuildHasher, Hasher};
        let mut hash = std::collections::hash_map::RandomState::new().build_hasher();
        hash.write_u32(std::process::id());
        if let Some(now) = crate::core::date::unix_now_ms() {
            hash.write_u64(now as u64);
        }
        (hash.finish() & ((1u64 << 53) - 1)).max(1)
    })
}

fn key(owner: u64, uid: u64) -> String {
    format!("{owner}-{uid}")
}
fn parse_key(id: &str) -> Option<(u64, u64)> {
    let (a, b) = id.split_once('-')?;
    if a.is_empty() || b.is_empty() || !a.bytes().all(|x| x.is_ascii_digit()) || !b.bytes().all(|x| x.is_ascii_digit()) {
        return None;
    }
    Some((a.parse().ok()?, b.parse().ok()?))
}
fn path(dir: &Path, id: &str) -> Option<PathBuf> {
    parse_key(id).map(|_| dir.join(format!("{id}.{EXT}")))
}
fn lock_path(dir: &Path, owner: u64) -> PathBuf {
    dir.join(format!("owner-{owner}.lock"))
}
fn io_error(e: std::io::Error) -> EngineError {
    EngineError::Other(format!("Recovery: {e}"))
}

fn ensure_dir(dir: &Path) -> Result<()> {
    let mut b = std::fs::DirBuilder::new();
    b.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        b.mode(0o700);
    }
    b.create(dir).map_err(io_error)
}

/// Claim an owner for the life of this process; another live process cannot claim it.
fn claim(dir: &Path, owner: u64) -> Result<()> {
    let mut locks = LOCKS.lock().map_err(|_| EngineError::Other("Recovery lock is unavailable".into()))?;
    if locks.iter().any(|(d, o, _)| d == dir && *o == owner) {
        return Ok(());
    }
    ensure_dir(dir)?;
    let mut opts = OpenOptions::new();
    opts.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let f = opts.open(lock_path(dir, owner)).map_err(io_error)?;
    f.try_lock().map_err(|e| EngineError::Other(format!("This recovery copy is in use: {e}")))?;
    locks.push((dir.to_path_buf(), owner, f));
    Ok(())
}

fn available(dir: &Path, owner: u64) -> bool {
    if owner == owner_id() {
        return false;
    }
    if LOCKS.lock().is_ok_and(|held| held.iter().any(|(d, o, _)| d == dir && *o == owner)) {
        return true;
    }
    let Ok(f) = File::open(lock_path(dir, owner)) else { return true };
    f.try_lock().is_ok()
}

fn read_entry(file: &Path, with_content: bool) -> Result<(Metadata, Vec<u8>)> {
    let stat = std::fs::symlink_metadata(file).map_err(io_error)?;
    if !stat.is_file() || stat.len() > MAX_FILE {
        return Err(EngineError::Other("Invalid recovery file".into()));
    }
    let mut f = File::open(file).map_err(io_error)?;
    let mut header = [0u8; 12];
    f.read_exact(&mut header).map_err(io_error)?;
    if header.get(..8) != Some(MAGIC.as_slice()) {
        return Err(EngineError::Other("Invalid recovery file".into()));
    }
    let len = u32::from_le_bytes([header[8], header[9], header[10], header[11]]) as usize;
    if len > MAX_META || (len as u64).saturating_add(12) > stat.len() {
        return Err(EngineError::Other("Invalid recovery metadata".into()));
    }
    let mut data = vec![0; len];
    f.read_exact(&mut data).map_err(io_error)?;
    let meta = serde_json::from_slice(&data).map_err(|e| EngineError::Other(format!("Invalid recovery metadata: {e}")))?;
    let mut content = Vec::new();
    if with_content {
        let limit = MAX_FILE.saturating_sub(12 + len as u64);
        f.take(limit.saturating_add(1)).read_to_end(&mut content).map_err(io_error)?;
        if content.len() as u64 > limit {
            return Err(EngineError::Other("Recovery file grew too large".into()));
        }
    }
    Ok((meta, content))
}

fn atomic_write(file: &Path, meta: &Metadata, workbook: &[u8]) -> Result<()> {
    let data = serde_json::to_vec(meta).map_err(|e| EngineError::Other(format!("Recovery: {e}")))?;
    if data.len() > MAX_META || (workbook.len() as u64).saturating_add(data.len() as u64).saturating_add(12) > MAX_FILE {
        return Err(EngineError::Other("Workbook is too large for recovery".into()));
    }
    static NEXT_TMP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let counter = NEXT_TMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = file.with_extension(format!("{EXT}.{}-{counter}.tmp", owner_id()));
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(&tmp).map_err(io_error)?;
    let result = f
        .write_all(MAGIC)
        .and_then(|()| f.write_all(&(data.len() as u32).to_le_bytes()))
        .and_then(|()| f.write_all(&data))
        .and_then(|()| f.write_all(workbook))
        .and_then(|()| f.sync_all())
        .and_then(|()| std::fs::rename(&tmp, file));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result.map_err(io_error)
}

/// Immutable, cheaply captured workbook state for a background checkpoint.
pub struct Batch(Vec<Snapshot>);
struct Snapshot {
    id: String,
    wb: Arc<gridcraft_model::Workbook>,
    meta: Metadata,
    epoch: Arc<std::sync::atomic::AtomicU64>,
    generation: u64,
}

/// Capture the workbook state on the UI thread; no files are read or written here.
pub fn capture(s: &Session) -> Batch {
    use std::sync::atomic::Ordering;
    let saved = crate::core::date::unix_now_ms().map_or(0, |now| (now / 1000.0) as u64);
    Batch(
        s.documents()
            .iter()
            .filter(|d| d.is_dirty() && d.recovery_allowed)
            .map(|d| Snapshot {
                id: d.recovery_id.clone().unwrap_or_else(|| key(owner_id(), d.uid)),
                wb: d.wb.clone(),
                meta: Metadata { title: d.display_title(), saved, source_dir: d.wb.source_dir.clone(), source_path: d.wb.source_path.clone() },
                epoch: d.recovery_epoch.clone(),
                generation: d.recovery_epoch.load(Ordering::Acquire),
            })
            .collect(),
    )
}
impl Batch {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Encode and atomically publish a captured batch. Saving or closing a workbook invalidates its
/// generation under the same lock, so a slow checkpoint cannot resurrect a discarded copy.
pub fn write_batch(batch: Batch, dir: &Path) -> Result<usize> {
    use std::sync::atomic::Ordering;
    if batch.is_empty() {
        return Ok(0);
    }
    ensure_dir(dir)?;
    claim(dir, owner_id())?;
    let mut written = 0;
    let mut first_error = None;
    for doc in batch.0 {
        let Some(file) = path(dir, &doc.id) else { continue };
        let bytes = match crate::io::save_bytes(&doc.wb, "recovery.xlsm") {
            Ok(bytes) => bytes,
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
                continue;
            }
        };
        let _guard = WRITES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if doc.epoch.load(Ordering::Acquire) != doc.generation {
            continue;
        }
        match atomic_write(&file, &doc.meta, &bytes) {
            Ok(()) => written += 1,
            Err(e) if first_error.is_none() => first_error = Some(e),
            Err(_) => {}
        }
    }
    match first_error {
        Some(e) => Err(e),
        None => Ok(written),
    }
}

/// Explicit synchronous checkpoint, also usable by the CLI and tests.
pub fn checkpoint(s: &Session, dir: &Path) -> Result<usize> {
    write_batch(capture(s), dir)
}

/// Recoverable copies, excluding workbooks still open or owned by another live process.
pub fn list(s: &Session, dir: &Path) -> Result<Vec<Value>> {
    let mut result = Vec::new();
    let rd = match std::fs::read_dir(dir) {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(result),
        Err(e) => return Err(io_error(e)),
    };
    for entry in rd.take(MAX_ENTRIES).flatten() {
        let file = entry.path();
        if file.extension().and_then(|s| s.to_str()) != Some(EXT) {
            continue;
        }
        let Some(id) = file.file_stem().and_then(|s| s.to_str()) else { continue };
        let Some((owner, _)) = parse_key(id) else { continue };
        if !available(dir, owner) || s.documents().iter().any(|d| d.recovery_id.as_deref() == Some(id)) {
            continue;
        }
        if let Ok((m, _)) = read_entry(&file, false) {
            result.push(json!({"id":id, "title":m.title, "saved":m.saved}));
        }
    }
    result.sort_by(|a, b| b["saved"].as_u64().cmp(&a["saved"].as_u64()));
    Ok(result)
}

/// Restore as an unsaved, untrusted copy. The snapshot remains until the copy is saved or closed.
pub fn restore(s: &mut Session, dir: &Path, id: &str) -> Result<usize> {
    let (owner, _) = parse_key(id).ok_or_else(|| EngineError::Other("Invalid recovery ID".into()))?;
    if owner == owner_id() || s.documents().iter().any(|d| d.recovery_id.as_deref() == Some(id)) {
        return Err(EngineError::Other("This recovery copy is already open".into()));
    }
    claim(dir, owner)?;
    let file = path(dir, id).ok_or_else(|| EngineError::Other("Invalid recovery ID".into()))?;
    let (meta, bytes) = read_entry(&file, true)?;
    let (mut wb, _) = crate::io::open_bytes("recovery.xlsm", &bytes)?;
    wb.source_dir = meta.source_dir;
    wb.source_path = meta.source_path;
    let stem = Path::new(&meta.title).file_stem().and_then(|s| s.to_str()).filter(|s| !s.is_empty()).unwrap_or("Book");
    let ext = if wb.vba_project.is_some() { "xlsm" } else { "xlsx" };
    let title = format!("{stem} (Recovered).{ext}");
    let mut doc = DocState::from_saved(wb, None, title, Trust::FromFile);
    doc.saved = Arc::new((*doc.wb).clone());
    doc.recovery_id = Some(id.to_string());
    Ok(s.add_document(doc))
}

/// Discard a specific unopened recovery copy, never one owned by a live instance.
pub fn discard(s: &Session, dir: &Path, id: &str) -> Result<()> {
    let (owner, _) = parse_key(id).ok_or_else(|| EngineError::Other("Invalid recovery ID".into()))?;
    if owner == owner_id() || s.documents().iter().any(|d| d.recovery_id.as_deref() == Some(id)) {
        return Err(EngineError::Other("This recovery copy is open".into()));
    }
    claim(dir, owner)?;
    let file = path(dir, id).ok_or_else(|| EngineError::Other("Invalid recovery ID".into()))?;
    std::fs::remove_file(file).map_err(io_error)
}

/// Remove only this workbook's snapshot, after a successful save or confirmed close.
pub(crate) fn remove(s: &Session, doc: &DocState) {
    let Some(dir) = &s.recovery_dir else { return };
    let _guard = WRITES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    doc.recovery_epoch.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    let id = doc.recovery_id.clone().unwrap_or_else(|| key(owner_id(), doc.uid));
    if let Some(file) = path(dir, &id) {
        let _ = std::fs::remove_file(file);
    }
}

/// Drop snapshots for open workbooks that are no longer dirty (e.g. after Undo).
pub fn clean(s: &Session) {
    for d in s.documents().iter().filter(|d| !d.is_dirty()) {
        remove(s, d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp() -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!("khitkrub-recovery-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn saved_copy(dir: &Path) -> (String, Session) {
        let mut s = Session::new();
        s.recovery_dir = Some(dir.to_path_buf());
        s.new_workbook();
        s.execute("cell.set", json!({"cell":"A1", "input":"before crash"})).unwrap();
        assert_eq!(checkpoint(&s, dir).unwrap(), 1);
        let uid = s.doc().unwrap().uid;
        let other = owner_id() ^ (1 << 40);
        let new_id = key(other, uid);
        std::fs::rename(path(dir, &key(owner_id(), uid)).unwrap(), path(dir, &new_id).unwrap()).unwrap();
        (new_id, s)
    }

    #[test]
    fn restore_keeps_a_copy_dirty_and_original_file_unchanged() {
        let dir = temp();
        let original = dir.join("original.xlsx");
        std::fs::write(&original, b"original unchanged").unwrap();
        let (id, _) = saved_copy(&dir);
        let mut s = Session::new();
        s.recovery_dir = Some(dir.clone());
        assert_eq!(list(&s, &dir).unwrap().len(), 1);
        let i = restore(&mut s, &dir, &id).unwrap();
        let d = s.documents().get(i).unwrap();
        assert!(d.is_dirty());
        assert!(d.path.is_none());
        assert_eq!(d.trust, Trust::FromFile);
        assert!(d.display_title().contains("(Recovered)"));
        assert!(path(&dir, &id).unwrap().exists(), "a second crash can recover the same copy");
        let cell = s.execute("cell.get", json!({"cell":"A1"})).unwrap();
        assert!(cell.to_string().contains("before crash"), "{cell}");
        assert_eq!(std::fs::read(&original).unwrap(), b"original unchanged");
        assert!(list(&s, &dir).unwrap().is_empty(), "an open copy is not offered twice");
        let target = dir.join("saved.xlsx");
        s.execute("file.save", json!({"path":target.to_string_lossy()})).unwrap();
        assert!(!path(&dir, &id).unwrap().exists(), "a successful save removes its checkpoint");
        assert_eq!(std::fs::read(&original).unwrap(), b"original unchanged");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn recovery_keeps_vba_and_suggests_a_macro_enabled_copy() {
        let dir = temp();
        let mut s = Session::new();
        s.recovery_dir = Some(dir.clone());
        s.new_workbook();
        s.execute("cell.set", json!({"cell":"A1", "input":"work"})).unwrap();
        let d = s.doc_mut().unwrap();
        Arc::make_mut(&mut d.wb).vba_project = Some(Arc::new(vec![1, 2, 3, 4]));
        let uid = d.uid;
        checkpoint(&s, &dir).unwrap();
        let foreign = key(owner_id() ^ (1 << 40), uid);
        std::fs::rename(path(&dir, &key(owner_id(), uid)).unwrap(), path(&dir, &foreign).unwrap()).unwrap();
        let mut restored = Session::new();
        restored.recovery_dir = Some(dir.clone());
        restore(&mut restored, &dir, &foreign).unwrap();
        let doc = restored.doc().unwrap();
        assert_eq!(doc.wb.vba_project.as_deref().map(Vec::as_slice), Some([1, 2, 3, 4].as_slice()));
        assert!(doc.display_title().ends_with(".xlsm"));
        assert_eq!(doc.trust, Trust::FromFile);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn pending_checkpoint_cannot_reappear_after_save_or_discard() {
        for close in [false, true] {
            let dir = temp();
            let mut s = Session::new();
            s.recovery_dir = Some(dir.clone());
            s.new_workbook();
            s.execute("cell.set", json!({"cell":"A1", "input":"work"})).unwrap();
            let batch = capture(&s);
            let id = key(owner_id(), s.doc().unwrap().uid);
            if close {
                s.execute("file.close", json!({"force":true})).unwrap();
            } else {
                s.execute("file.save", json!({"path":dir.join("saved.xlsx").to_string_lossy()})).unwrap();
            }
            assert_eq!(write_batch(batch, &dir).unwrap(), 0);
            assert!(!path(&dir, &id).unwrap().exists());
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn undoing_back_to_saved_removes_and_invalidates_the_checkpoint() {
        let dir = temp();
        let mut s = Session::new();
        s.recovery_dir = Some(dir.clone());
        s.new_workbook();
        s.execute("cell.set", json!({"cell":"A1", "input":"edit"})).unwrap();
        checkpoint(&s, &dir).unwrap();
        let pending = capture(&s);
        let id = key(owner_id(), s.doc().unwrap().uid);
        assert!(path(&dir, &id).unwrap().exists());
        s.execute("edit.undo", json!({})).unwrap();
        assert!(!s.doc().unwrap().is_dirty());
        assert!(!path(&dir, &id).unwrap().exists());
        assert_eq!(write_batch(pending, &dir).unwrap(), 0);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn other_live_process_copies_are_not_offered_or_overwritten() {
        let dir = temp();
        let (id, _) = saved_copy(&dir);
        let (owner, _) = parse_key(&id).unwrap();
        let lock = File::create(lock_path(&dir, owner)).unwrap();
        lock.try_lock().unwrap();
        let mut s = Session::new();
        s.recovery_dir = Some(dir.clone());
        assert!(list(&s, &dir).unwrap().is_empty());
        assert!(restore(&mut s, &dir, &id).is_err());
        assert!(discard(&s, &dir, &id).is_err());
        assert!(path(&dir, &id).unwrap().exists());
        lock.unlock().unwrap();
        assert_eq!(list(&s, &dir).unwrap().len(), 1);
        restore(&mut s, &dir, &id).unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn discard_only_removes_the_requested_inactive_copy() {
        let dir = temp();
        let (id, _) = saved_copy(&dir);
        let (other, _) = saved_copy(&dir);
        let s = Session::new();
        assert!(discard(&s, &dir, "../outside").is_err());
        discard(&s, &dir, &id).unwrap();
        assert!(!path(&dir, &id).unwrap().exists());
        assert!(path(&dir, &other).unwrap().exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn malformed_files_and_untrusted_ids_are_refused() {
        let dir = temp();
        let id = key(owner_id() ^ (1 << 40), 1);
        std::fs::write(path(&dir, &id).unwrap(), b"bad").unwrap();
        let mut s = Session::new();
        s.recovery_dir = Some(dir.clone());
        assert!(list(&s, &dir).unwrap().is_empty());
        assert!(restore(&mut s, &dir, "../other").is_err());
        assert!(restore(&mut s, &dir, &id).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn encrypted_source_is_never_written_to_recovery() {
        let dir = temp();
        let mut source = Session::new();
        source.new_workbook();
        source.execute("cell.set", json!({"cell":"A1", "input":"private"})).unwrap();
        let plain = crate::io::save_bytes(&source.doc().unwrap().wb, "source.xlsx").unwrap();
        let encrypted = gridcraft_xlsx::testing::encrypt_workbook(&plain, "test-password");
        let mut s = Session::new();
        s.recovery_dir = Some(dir.clone());
        s.execute("file.open", json!({"name":"secret.xlsx", "base64":crate::io::base64_encode(&encrypted), "password":"test-password"})).unwrap();
        assert!(!s.doc().unwrap().recovery_allowed);
        s.execute("cell.set", json!({"cell":"A2", "input":"private edit"})).unwrap();
        assert_eq!(checkpoint(&s, &dir).unwrap(), 0);
        assert!(!dir.read_dir().unwrap().flatten().any(|e| e.path().extension().is_some_and(|e| e == EXT)));
        s.execute("file.save", json!({"path":dir.join("unencrypted.xlsx").to_string_lossy()})).unwrap();
        assert!(s.doc().unwrap().recovery_allowed);
        s.execute("cell.set", json!({"cell":"A2", "input":"new edit"})).unwrap();
        assert_eq!(checkpoint(&s, &dir).unwrap(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}
