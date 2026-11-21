//! Dragging a formula's references on the sheet while the formula is edited, as in Excel: each
//! reference is outlined in its colour with a square on every corner. Dragging a corner resizes
//! the reference (the opposite corner stays), dragging its border moves it, and the formula's
//! text follows as the pointer moves: `=SUM(B2:B5)` becomes `=SUM(B2:B8)`. Absolute parts (`$`)
//! and a sheet prefix are kept. Works for every function, since it acts on the references.

use super::geo::Geo;
use super::{CellRef, CursorIcon, Drag, Pos2, RangeRef, Sheet, SheetApp, Workbook};

/// How far from a corner square a press still grabs it, and how wide the grabbable border is
/// (screen points).
const CORNER_REACH: f32 = 5.0;
const BORDER_REACH: f32 = 3.0;

/// What a press on a reference grabbed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefGrab {
    /// A corner: the reference runs from `fixed` (the opposite corner) to the pointer.
    Corner { fixed: CellRef },
    /// The border: the reference moves with the pointer, from the cell grabbed.
    Border { grab: CellRef },
}

/// The edited formula's reference under `p` on this sheet, and what a press there grabs: its
/// byte span in the text, the range and the grab. Only plain cell or block references (with or
/// without `$` and a sheet), not whole rows or columns.
pub(super) fn grab_at(app: &SheetApp, geo: &Geo, sh: &Sheet, wb: &Workbook, p: Pos2) -> Option<((usize, usize), RangeRef, RefGrab)> {
    let ed = app.editor.as_ref().filter(|ed| ed.is_formula())?;
    if app.session.active().is_none_or(|d| d.uid != ed.doc) || !geo.cells.contains(p) {
        return None;
    }
    let origin = wb.sheet(ed.sheet).map_or(sh.name.as_str(), |o| o.name.as_str());
    let mut best: Option<((usize, usize), RangeRef, RefGrab)> = None;
    for (a, b, r, sheet, _) in crate::editor::formula_refs(&ed.text) {
        if !crate::editor::ref_is_on_sheet(sheet.as_deref(), origin, &sh.name) || !draggable(ed.text.get(a..b).unwrap_or("")) {
            continue;
        }
        let rr = geo.range_rect(sh, r);
        let corners = [
            (rr.left_top(), r.end),
            (rr.right_top(), CellRef::new(r.end.row, r.start.col)),
            (rr.left_bottom(), CellRef::new(r.start.row, r.end.col)),
            (rr.right_bottom(), r.start),
        ];
        // On a right-to-left sheet the columns run the other way.
        let corners = if geo.rtl {
            corners.map(|(q, fixed)| (q, CellRef::new(fixed.row, if fixed.col == r.start.col { r.end.col } else { r.start.col })))
        } else {
            corners
        };
        if let Some((_, fixed)) = corners.into_iter().find(|(q, _)| q.distance(p) <= CORNER_REACH) {
            return Some(((a, b), r, RefGrab::Corner { fixed }));
        }
        if best.is_none() && rr.expand(BORDER_REACH).contains(p) && !rr.shrink(BORDER_REACH).contains(p) {
            best = Some(((a, b), r, RefGrab::Border { grab: geo.cell_at(sh, p) }));
        }
    }
    best
}

/// The pointer shape over a reference's corner or border while a formula is edited.
pub(super) fn cursor(app: &SheetApp, geo: &Geo, sh: &Sheet, wb: &Workbook, p: Pos2) -> Option<CursorIcon> {
    let (_, r, grab) = grab_at(app, geo, sh, wb, p)?;
    Some(match grab {
        RefGrab::Border { .. } => CursorIcon::Move,
        // The corner's own diagonal: top-left and bottom-right share one, the others the other.
        RefGrab::Corner { fixed } => {
            let down_right = (fixed == r.start) || (fixed == r.end);
            if down_right != geo.rtl { CursorIcon::ResizeNwSe } else { CursorIcon::ResizeNeSw }
        }
    })
}

/// A reference's text the drag may rewrite: cells with a row and a column (`B2`, `$B$2:C9`,
/// `Sheet2!B2`), not whole rows or columns (`A:A`, `1:3`) or names.
fn draggable(text: &str) -> bool {
    let body = text.rsplit_once('!').map_or(text, |(_, b)| b);
    !body.is_empty() && body.split(':').all(|part| part.chars().any(|c| c.is_ascii_digit()) && part.chars().any(|c| c.is_ascii_alphabetic()))
}

/// The cell under the pointer during the drag moves the reference: returns its new range.
pub(super) fn dragged_range(orig: RangeRef, grab: RefGrab, at: CellRef) -> RangeRef {
    match grab {
        RefGrab::Corner { fixed } => RangeRef::new(fixed, at),
        RefGrab::Border { grab } => {
            let (dr, dc) = (at.row as i64 - grab.row as i64, at.col as i64 - grab.col as i64);
            // Keep the block's size at the sheet's edges.
            let dr = dr.clamp(-(orig.start.row as i64), (super::MAX_ROWS - 1) as i64 - orig.end.row as i64);
            let dc = dc.clamp(-(orig.start.col as i64), (super::MAX_COLS - 1) as i64 - orig.end.col as i64);
            RangeRef::new(orig.start.offset_clamped(dr, dc), orig.end.offset_clamped(dr, dc))
        }
    }
}

/// `original` (a reference as written) pointing at `to`: its sheet prefix and the `$` of each
/// end are kept. A single cell is written as one cell.
pub fn rewrite(original: &str, to: RangeRef) -> String {
    let (prefix, body) = match original.rfind('!') {
        Some(i) => original.split_at(i + 1),
        None => ("", original),
    };
    let parts: Vec<&str> = body.split(':').collect();
    let flags = |s: &str| {
        let col_abs = s.starts_with('$');
        let row_abs = s.trim_start_matches('$').contains('$');
        (col_abs, row_abs)
    };
    let first = flags(parts.first().copied().unwrap_or(""));
    let last = flags(parts.last().copied().unwrap_or(""));
    let cell = |c: CellRef, (ca, ra): (bool, bool)| {
        format!("{}{}{}{}", if ca { "$" } else { "" }, gridcraft_engine::core::col_to_letters(c.col), if ra { "$" } else { "" }, c.row + 1)
    };
    if to.is_single() { format!("{prefix}{}", cell(to.start, first)) } else { format!("{prefix}{}:{}", cell(to.start, first), cell(to.end, last)) }
}

/// Start dragging the reference under `p`, if the press is on one; true when it is.
pub(super) fn press(app: &mut SheetApp, geo: &Geo, sh: &Sheet, wb: &Workbook, p: Pos2) -> bool {
    let Some((span, orig, grab)) = grab_at(app, geo, sh, wb, p) else { return false };
    app.grid.drag = Drag::FormulaRef { span, orig, grab };
    true
}

/// The pointer is over `at`: rewrite the dragged reference in the formula.
pub(super) fn drag_to(app: &mut SheetApp, at: CellRef) {
    let Drag::FormulaRef { span: (a, b), orig, grab } = app.grid.drag else { return };
    let Some(ed) = app.editor.as_mut() else {
        app.grid.drag = Drag::None;
        return;
    };
    let (Some(head), Some(old), Some(tail)) = (ed.text.get(..a), ed.text.get(a..b), ed.text.get(b..)) else { return };
    let new = rewrite(old, dragged_range(orig, grab, at));
    if new == old {
        return;
    }
    // The caret keeps its place in the text: after the reference, it moves by the change.
    let caret_byte = ed.text.char_indices().nth(ed.caret).map_or(ed.text.len(), |(i, _)| i);
    let text = format!("{head}{new}{tail}");
    let caret_byte = if caret_byte >= b { caret_byte + new.len() - old.len() } else { caret_byte.min(a) };
    ed.caret = text.get(..caret_byte).map_or(0, |s| s.chars().count());
    ed.anchor = ed.caret;
    ed.text = text;
    ed.point = None;
    ed.sync_caret = true;
    ed.request_focus = true;
    app.grid.drag = Drag::FormulaRef { span: (a, a + new.len()), orig, grab };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(s: &str) -> RangeRef {
        RangeRef::parse(s).unwrap()
    }

    #[test]
    fn rewriting_keeps_dollars_and_the_sheet() {
        assert_eq!(rewrite("B2:B5", r("B2:B8")), "B2:B8");
        assert_eq!(rewrite("$B$2:$B$5", r("B2:C8")), "$B$2:$C$8");
        assert_eq!(rewrite("B$2:$B5", r("B2:D9")), "B$2:$D9");
        assert_eq!(rewrite("Sheet2!B2", r("B2:C3")), "Sheet2!B2:C3");
        assert_eq!(rewrite("'My Data'!$A$1:A4", r("A1:A2")), "'My Data'!$A$1:A2");
        assert_eq!(rewrite("A1:A4", r("C3")), "C3", "one cell is written as one cell");
    }

    #[test]
    fn corners_resize_and_the_border_moves() {
        let orig = r("B2:B5");
        assert_eq!(dragged_range(orig, RefGrab::Corner { fixed: r("B2").start }, r("C9").start), r("B2:C9"));
        // Past the fixed corner, the range flips round it.
        assert_eq!(dragged_range(orig, RefGrab::Corner { fixed: r("B5").start }, r("A1").start), r("A1:B5"));
        assert_eq!(dragged_range(orig, RefGrab::Border { grab: r("B3").start }, r("D4").start), r("D3:D6"));
        // At the sheet's top-left edge the block keeps its size.
        assert_eq!(dragged_range(orig, RefGrab::Border { grab: r("B5").start }, r("A1").start), r("A1:A4"));
    }

    #[test]
    fn whole_rows_columns_and_names_are_not_dragged() {
        for t in ["A:A", "1:3", "$A:$C", "Sales", ""] {
            assert!(!draggable(t), "{t}");
        }
        for t in ["B2", "$B$2:C9", "Sheet2!B2", "'My Data'!A1:B2"] {
            assert!(draggable(t), "{t}");
        }
    }
}
