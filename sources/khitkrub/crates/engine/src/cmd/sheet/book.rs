//! Copy or move a sheet into a new workbook or another open one (Excel's "Move or Copy › To book").

use super::{Arc, EngineError, Json, Result, Session, Sheet, bad, bool_param, json, target_sheet};
use crate::cmd::commit;
use gridcraft_formula::{Expr, SheetSel};
use gridcraft_model::{DefinedName, Workbook};

/// Whether `f` reads a sheet other than `own`: such a formula has nothing to point at in another workbook.
fn reads_other_sheet(own: &str, f: &gridcraft_model::Formula) -> bool {
    let Some(e) = f.expr() else { return false };
    let mut hit = false;
    e.walk(&mut |x| {
        if let Expr::Ref(r) = x {
            match &r.sheet {
                SheetSel::Named(n) if !n.eq_ignore_ascii_case(own) => hit = true,
                SheetSel::Span(..) => hit = true,
                _ => {}
            }
        }
    });
    hit
}

/// The destination: `"new"`, an open workbook's position, or its title.
fn destination(s: &Session, p: &Json) -> Result<Option<usize>> {
    let book = p.get("book").ok_or_else(|| bad("sheet.toBook", "missing `book` (\"new\", a position or a title)"))?;
    if book.as_str() == Some("new") {
        return Ok(None);
    }
    let found = match book {
        Json::Number(n) => n.as_u64().map(|v| v as usize).filter(|v| *v < s.docs.len()),
        Json::String(t) => s.docs.iter().position(|d| d.title == *t),
        _ => None,
    };
    found.map(Some).ok_or_else(|| bad("sheet.toBook", "no such open workbook"))
}

/// Copies (or, with `copy: false`, moves) a sheet to a new workbook or another open one.
/// Formulas that read other sheets of the source keep their values but lose the formula, since the
/// sheets they point at are not in the destination. Returns `{book, sheet, frozen}`.
pub(super) fn sheet_to_book(s: &mut Session, p: &Json) -> Result<Json> {
    let i = target_sheet(s, p)?;
    let copy = bool_param(p, "copy").unwrap_or(true);
    let dest = destination(s, p)?;
    let here = s.active;
    if dest == Some(here) {
        return Err(bad("sheet.toBook", "choose a different workbook; Move or Copy handles the same one"));
    }
    let src_doc = s.doc()?;
    if !copy && src_doc.wb.protected_structure {
        return Err(EngineError::Other("The workbook is protected and cannot be changed.".into()));
    }
    if let Some(d) = dest.and_then(|d| s.docs.get(d))
        && d.wb.protected_structure
    {
        return Err(EngineError::Other("The destination workbook is protected and cannot be changed.".into()));
    }
    let src_wb = Arc::clone(&src_doc.wb);
    let Some(src) = src_wb.sheets.get(i) else { return Err(bad("sheet.toBook", "no such sheet")) };
    let own = src.name.clone();
    let mut sh: Sheet = (**src).clone();
    sh.uid = gridcraft_model::SheetUid::fresh();
    sh.preserved = None;
    let scoped: Vec<DefinedName> = src_wb.names.iter().filter(|n| n.scope == Some(i)).cloned().collect();
    // Formulas that read other sheets become their values.
    let foreign: Vec<_> =
        sh.cells.iter().filter(|(_, c)| c.formula.as_deref().is_some_and(|f| reads_other_sheet(&own, f))).map(|(at, _)| at).collect();
    let frozen = foreign.len();
    for at in foreign {
        if let Some(cell) = sh.cells.get_mut(at) {
            cell.formula = None;
        }
    }
    if !copy && src_wb.sheets.len() <= 1 {
        return Err(EngineError::Other("A workbook must contain at least one worksheet.".into()));
    }
    if !copy {
        super::tabs::delete_sheet(s, &json!({"sheet": i}))?;
    }
    let title;
    match dest {
        None => {
            let mut wb = Workbook::new();
            wb.styles = src_wb.styles.clone();
            wb.theme = src_wb.theme.clone();
            wb.cell_styles = src_wb.cell_styles.clone();
            wb.date_system = src_wb.date_system;
            wb.calc = src_wb.calc.clone();
            wb.custom_lists = src_wb.custom_lists.clone();
            wb.props.author = s.prefs.user_name.clone();
            wb.sheets = vec![Arc::new(sh)];
            wb.names = scoped.into_iter().map(|n| DefinedName { scope: Some(0), ..n }).collect();
            wb.set_active_sheet(0);
            let n = (1..).find(|n| !s.docs.iter().any(|d| d.title == format!("Book{n}") && d.path.is_none())).unwrap_or(1);
            title = format!("Book{n}");
            s.add_document(crate::DocState::new(wb, None, title.clone(), crate::Trust::Local));
        }
        Some(di) => {
            let d = s.docs.get_mut(di).ok_or_else(|| bad("sheet.toBook", "no such open workbook"))?;
            title = d.title.clone();
            commit(d, |cx| {
                // Cells carry style numbers of the source workbook: put each style in this one's table.
                let ids: Vec<_> = sh.cells.iter().map(|(at, c)| (at, c.style)).collect();
                for (at, id) in ids {
                    let style = src_wb.styles.get(id).clone();
                    let mapped = cx.wb.styles.intern(style);
                    if let Some(cell) = sh.cells.get_mut(at) {
                        cell.style = mapped;
                    }
                }
                let base = sh.name.clone();
                sh.name = unique_name(&cx.wb, &base);
                sh.tables.clear(); // table names must be unique in a workbook
                let count = sh.charts.len() + sh.images.len() + sh.shapes.len() + sh.slicers.len() + sh.controls.len();
                let ids = cx.wb.free_object_ids(count);
                if ids.len() < count {
                    return Err(bad("sheet.toBook", "no free object ids are left for the copied objects"));
                }
                let mut ids = ids.into_iter();
                let mut fresh = || ids.next().unwrap_or(u32::MAX);
                sh.charts.iter_mut().for_each(|o| o.id = fresh());
                sh.images.iter_mut().for_each(|o| o.id = fresh());
                sh.shapes.iter_mut().for_each(|o| o.id = fresh());
                sh.slicers.iter_mut().for_each(|o| o.id = fresh());
                sh.controls.iter_mut().for_each(|o| o.id = fresh());
                let at = cx.wb.sheets.len();
                cx.wb.names.extend(scoped.into_iter().map(|n| DefinedName { scope: Some(at), ..n }));
                cx.wb.sheets.push(Arc::new(sh));
                cx.wb.set_active_sheet(at);
                cx.structural = true;
                Ok(())
            })?;
            if let Some(w) = s.docs.get(di).and_then(|d| s.doc_window(d.uid)) {
                s.activate_window(w);
            }
        }
    }
    Ok(json!({"book": title, "frozen": frozen}))
}

/// `Name`, or `Name (2)`, `Name (3)`… fitting Excel's 31 characters.
fn unique_name(wb: &Workbook, base: &str) -> String {
    if wb.sheet_index(base).is_none() {
        return base.to_string();
    }
    (2..)
        .map(|k| {
            let suffix = format!(" ({k})");
            let keep = 31usize.saturating_sub(suffix.chars().count());
            format!("{}{suffix}", base.chars().take(keep).collect::<String>().trim_end())
        })
        .find(|nm| wb.sheet_index(nm).is_none())
        .unwrap_or_else(|| base.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_goes_to_a_new_workbook_with_its_styles_and_keeps_the_source() {
        let mut s = Session::new();
        s.new_workbook();
        s.execute("cell.set", json!({"cell": "A1", "input": "7"})).unwrap();
        s.execute("home.bold", json!({"range": "A1"})).unwrap();
        s.execute("cell.set", json!({"cell": "B1", "input": "=A1*2"})).unwrap();
        let r = s.execute("sheet.toBook", json!({"sheet": 0, "book": "new"})).unwrap();
        assert_eq!(r["frozen"], 0);
        assert_eq!(s.docs.len(), 2);
        let d = s.doc().unwrap();
        let sh = d.wb.sheet(0).unwrap();
        assert_eq!(d.wb.sheets.len(), 1);
        assert!(d.wb.styles.get(sh.style_id(gridcraft_core::CellRef::new(0, 0))).font.bold, "the bold style came along");
        assert!(sh.cells.get(gridcraft_core::CellRef::new(0, 1)).is_some_and(|c| c.formula.is_some()), "own-sheet formulas stay formulas");
        assert_eq!(s.docs[0].wb.sheets.len(), 1, "the source still has its sheet");
    }

    #[test]
    fn a_move_leaves_the_source_and_cross_sheet_formulas_become_values() {
        let mut s = Session::new();
        s.new_workbook();
        s.execute("home.insertSheet", json!({"name": "Two"})).unwrap();
        s.execute("sheet.activate", json!({"sheet": 0})).unwrap();
        s.execute("cell.set", json!({"cell": "A1", "input": "5"})).unwrap();
        s.execute("sheet.activate", json!({"sheet": 1})).unwrap();
        s.execute("cell.set", json!({"cell": "A1", "input": "=Sheet1!A1+1"})).unwrap();
        let r = s.execute("sheet.toBook", json!({"sheet": 1, "book": "new", "copy": false})).unwrap();
        assert_eq!(r["frozen"], 1);
        assert_eq!(s.docs[0].wb.sheets.len(), 1, "moved out of the source");
        let sh = s.doc().unwrap().wb.sheet(0).unwrap().clone();
        let c = sh.cells.get(gridcraft_core::CellRef::new(0, 0)).unwrap();
        assert!(c.formula.is_none());
    }

    #[test]
    fn a_copy_joins_another_open_workbook_under_a_free_name() {
        let mut s = Session::new();
        s.new_workbook();
        s.execute("cell.set", json!({"cell": "A1", "input": "x"})).unwrap();
        s.new_workbook();
        s.execute("sheet.activate", json!({"sheet": 0})).unwrap();
        let first_title = s.docs[0].title.clone();
        s.execute("sheet.toBook", json!({"sheet": 0, "book": first_title})).unwrap();
        // The active workbook is the second one, so the first is the destination.
        assert_eq!(s.docs[0].wb.sheets.len(), 2);
        assert_eq!(s.docs[0].wb.sheets[1].name, "Sheet1 (2)");
        assert!(s.execute("sheet.toBook", json!({"sheet": 0, "book": s.doc().unwrap().title.clone()})).is_err(), "not into itself");
    }
}
