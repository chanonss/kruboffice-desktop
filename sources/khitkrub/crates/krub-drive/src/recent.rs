//! The Google Drive files this browser opened or saved, newest first (owner, 2026-11-17): Home
//! lists them so a Drive file is reopened from Drive (and saved back to it) rather than from a
//! draft that no longer knows where it came from. Each entry also names the draft that holds
//! the file's unsaved changes in this browser, if there is one; Home shows that draft on the
//! file's row instead of as a draft of its own.
//!
//! Only the file's Drive ID, name, content version and when it was last used are kept, in the
//! browser's local storage. No file contents.

use serde_json::{Value, json};

/// Most files the list keeps; the oldest go first.
pub const MAX_RECENT: usize = 20;

/// One Drive file in the list.
#[derive(Clone, Debug, PartialEq)]
pub struct RecentFile {
    pub id: String,
    pub name: String,
    /// The content version it had when it was last opened or saved here: a draft reopened later
    /// is saved back only if the file is still at this version.
    pub version: String,
    /// When it was last opened or saved, milliseconds since 1970.
    pub at: f64,
    /// The draft (by ID) holding unsaved changes to it in this browser.
    pub draft: Option<String>,
}

/// A row of Home's Google Drive list, its time written for the interface language.
#[derive(Clone, Debug, PartialEq)]
pub struct RecentDriveFile {
    pub id: String,
    pub name: String,
    pub when: String,
    /// The draft with this file's unsaved changes, if any.
    pub draft: Option<String>,
}

/// The list kept in storage as JSON; anything malformed is left out.
pub fn parse(text: &str) -> Vec<RecentFile> {
    let Ok(Value::Array(items)) = serde_json::from_str::<Value>(text) else { return Vec::new() };
    let mut out: Vec<RecentFile> = items
        .iter()
        .filter_map(|v| {
            let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
            let file = RecentFile {
                id: s("id").filter(|id| crate::valid_file_id(id))?,
                name: s("name").filter(|n| !n.trim().is_empty() && n.len() <= 255)?,
                version: s("version").filter(|v| !v.is_empty() && v.len() <= 32 && v.bytes().all(|b| b.is_ascii_digit()))?,
                at: v.get("at").and_then(Value::as_f64).filter(|t| t.is_finite())?,
                draft: s("draft").filter(|d| !d.is_empty() && d.len() <= 128),
            };
            Some(file)
        })
        .collect();
    out.truncate(MAX_RECENT);
    out
}

/// The list as JSON for storage.
pub fn to_json(list: &[RecentFile]) -> String {
    let items: Vec<Value> = list
        .iter()
        .map(|f| {
            let mut v = json!({"id": f.id, "name": f.name, "version": f.version, "at": f.at});
            if let Some(d) = &f.draft {
                v["draft"] = json!(d);
            }
            v
        })
        .collect();
    Value::Array(items).to_string()
}

/// File `id` was opened or saved at `at` with this name and version: it moves to the top (its
/// draft, if any, stays with it); the list keeps at most [`MAX_RECENT`].
pub fn upsert(list: &mut Vec<RecentFile>, id: &str, name: &str, version: &str, at: f64) {
    let draft = list.iter().position(|f| f.id == id).and_then(|i| list.remove(i).draft);
    list.insert(0, RecentFile { id: id.into(), name: name.into(), version: version.into(), at, draft });
    list.truncate(MAX_RECENT);
}

/// File `id`'s unsaved changes are now in draft `draft` (`None`: there are none). Whether the
/// list changed.
pub fn set_draft(list: &mut [RecentFile], id: &str, draft: Option<&str>) -> bool {
    match list.iter_mut().find(|f| f.id == id) {
        Some(f) if f.draft.as_deref() != draft => {
            f.draft = draft.map(str::to_string);
            true
        }
        _ => false,
    }
}

/// Take file `id` off the list. Whether it was on it.
pub fn forget(list: &mut Vec<RecentFile>, id: &str) -> bool {
    let before = list.len();
    list.retain(|f| f.id != id);
    list.len() != before
}

/// The file whose unsaved changes draft `draft` holds.
pub fn by_draft<'a>(list: &'a [RecentFile], draft: &str) -> Option<&'a RecentFile> {
    list.iter().find(|f| f.draft.as_deref() == Some(draft))
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "1aaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "1bbbbbbbbbbbbbbbbbbbbbbbbb";

    #[test]
    fn files_move_to_the_top_keep_their_draft_and_round_trip() {
        let mut list = Vec::new();
        upsert(&mut list, A, "Report.docx", "3", 10.0);
        upsert(&mut list, B, "Plan.docx", "7", 20.0);
        assert!(set_draft(&mut list, A, Some("tab-1")));
        assert!(!set_draft(&mut list, A, Some("tab-1")), "unchanged");
        upsert(&mut list, A, "Report.docx", "4", 30.0);
        assert_eq!(list[0].id, A);
        assert_eq!((list[0].version.as_str(), list[0].draft.as_deref()), ("4", Some("tab-1")), "the draft stays with the file");
        assert_eq!(by_draft(&list, "tab-1").map(|f| f.id.as_str()), Some(A));
        assert_eq!(parse(&to_json(&list)), list);
        assert!(forget(&mut list, B) && !forget(&mut list, B));
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn the_list_is_bounded_and_ignores_what_it_cannot_read() {
        let mut list = Vec::new();
        for i in 0..(MAX_RECENT + 5) {
            upsert(&mut list, &format!("1{:0>25}", i), "x.docx", "1", i as f64);
        }
        assert_eq!(list.len(), MAX_RECENT);
        assert!(parse("not json").is_empty());
        let bad = r#"[{"id": "../x", "name": "a", "version": "1", "at": 1}, {"id": "1aaaaaaaaaaaaaaaaaaaaaaaaa", "name": "a", "version": "v1", "at": 1}, {"id": "1aaaaaaaaaaaaaaaaaaaaaaaaa", "name": "ok.docx", "version": "2", "at": 5}]"#;
        let read = parse(bad);
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].name, "ok.docx");
    }
}
