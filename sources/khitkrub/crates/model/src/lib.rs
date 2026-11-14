//! The GridCraft document model: workbooks, sheets, a copy-on-write cell store, interned
//! styles, defined names, tables, conditional formats, validation, comments, charts and print
//! settings. Pure data; calculation lives in `gridcraft-calc`.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
#![forbid(unsafe_code)]

mod b64;
pub mod cell;
mod cellmap;
pub mod features;
pub mod macros;
pub mod password;
pub mod preserved;
pub mod python;
pub mod sheet;
pub mod store;
pub mod style;
pub mod workbook;

pub use cell::{Cell, CellPicture, Formula};
pub use features::*;
pub use macros::*;
pub use password::PasswordHash;
pub use preserved::{Preserved, PreservedPart, PreservedRel, PreservedSheet, PreservedXml};
pub use python::{PyCode, PyOutput};
pub use sheet::{DEFAULT_COL_WIDTH, DEFAULT_ROW_HEIGHT, LineIndex, Sheet, SheetUid, SheetViewMode, Visibility, points_to_chars};
pub use store::CellStore;
pub use style::{
    Alignment, BorderLine, BorderStyle, Borders, Color, Fill, Font, HAlign, NumFmt, PatternType, Style, StyleId, StyleTable, Theme, Underline,
    VAlign, VertAlign,
};
pub use workbook::{
    CalcMode, CalcSettings, DefinedName, DocProps, EQUATION_BUDGET_MESSAGE, MAX_EQUATION_LAYOUT_BYTES, PhoneticProps, PhoneticRun, PhoneticString,
    ShowObjects, Workbook, WorkbookView,
};

/// Font used for new workbooks. Only a name: renderers substitute an available open font when it
/// isn't installed. KhitKrub's default is Google Sarabun (owner, 2026-11-13): most workbooks mix Thai
/// and English, and Sarabun draws both from one family. The web ships it; elsewhere it is used when
/// installed, and the interface's fallback faces draw the Thai otherwise.
pub const DEFAULT_FONT: &str = "Sarabun";
/// The theme font a spreadsheet file means when it names none (no theme part, or a theme font
/// without a typeface): the usual Office body font. New workbooks use [`DEFAULT_FONT`] instead;
/// reading a file must not put KhitKrub's choice into someone else's workbook.
pub const FILE_THEME_FONT: &str = "Calibri";
pub const DEFAULT_FONT_SIZE: f32 = 11.0;
