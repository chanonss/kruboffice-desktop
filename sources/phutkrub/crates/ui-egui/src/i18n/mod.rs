//! Interface languages. Strings in the code stay English and are the lookup keys; a catalog per
//! language (`*.tsv`, format in `th.tsv`) maps them to display text when the UI is drawn. Command
//! ids, presentation text, file names, the control channel, the CLI and MCP never see translated
//! text, so scripts and agents are unaffected. A string without a translation shows in English,
//! so coverage can grow one entry at a time.
//!
//! The `language` setting is `auto` by default: the UI follows the operating system's language
//! (`LANG`/`LC_*`, or `PHUTKRUB_LANGUAGE`) when there is a catalog for it, and English otherwise.
//! The web shell opens a first visit in Thai whatever the browser's language; the setting is
//! remembered in the browser. This is the system KhianKrub, WordCraft, PdfCraft and PhotoCraft
//! use, so the catalog format and habits carry over between the Crafting Apps.
//!
//! # Looking strings up
//! - [`tl!`](crate::tl) / [`t`]: a string in the current language. [`tr`]: in a given one.
//! - [`tc`] / [`trc`]: worded for a context (`template`).
//! - [`fmt`]: fill `{name}` placeholders after a lookup; translators may reorder them. [`fmtp`]: the
//!   same for positional `{}` placeholders (filled in order, so translations keep the English order).
//!
//! # Translating
//! Entries are written from the plain meaning of the English label in ordinary Thai computing
//! vocabulary (clean room): never from Microsoft PowerPoint's or another product's localisation.

mod catalog;

use std::cell::Cell;
use std::sync::OnceLock;

use catalog::Catalog;

/// The `language` setting that follows the system.
pub const AUTO: &str = "auto";

/// One supported interface language.
pub struct LangInfo {
    /// BCP 47 code, lowercase (`th`). Also the `language` setting's value.
    pub code: &'static str,
    /// The language's name in itself, shown in the Options dropdown.
    pub name: &'static str,
    /// Catalog file contents (empty for the built-in English).
    pub source: &'static str,
    catalog: OnceLock<Catalog>,
}

/// The registry: English, the source language of every string (so it needs no catalog), and Thai.
pub static LANGUAGES: [LangInfo; 2] = [
    LangInfo { code: "en", name: "English", source: "", catalog: OnceLock::new() },
    // Thai; `th`, `th-TH` and `th_TH.UTF-8` resolve here.
    LangInfo { code: "th", name: "ไทย", source: include_str!("th.tsv"), catalog: OnceLock::new() },
];

impl LangInfo {
    fn catalog(&self) -> &Catalog {
        self.catalog.get_or_init(|| {
            let (catalog, errors) = Catalog::parse(self.source);
            for error in errors {
                log::warn!("{} interface catalog: {error}; that entry shows in English", self.code);
            }
            catalog
        })
    }
}

/// A language the UI can be shown in (a handle into [`LANGUAGES`]).
#[derive(Clone, Copy)]
pub struct Lang(&'static LangInfo);

impl std::fmt::Debug for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lang({})", self.0.code)
    }
}

impl PartialEq for Lang {
    fn eq(&self, other: &Self) -> bool {
        self.0.code == other.0.code
    }
}

impl Eq for Lang {}

impl Lang {
    pub const EN: Lang = Lang(&LANGUAGES[0]);

    pub fn code(self) -> &'static str {
        self.0.code
    }

    pub fn name(self) -> &'static str {
        self.0.name
    }

    /// Is this the Thai interface (it needs a Thai face and Thai word breaking)?
    pub fn is_thai(self) -> bool {
        self.0.code == "th"
    }

    /// A language by its exact code (any case).
    pub fn from_code(code: &str) -> Option<Lang> {
        LANGUAGES.iter().find(|l| l.code.eq_ignore_ascii_case(code)).map(Lang)
    }

    /// Resolve the `language` setting: a language code, or [`AUTO`] (and anything unknown, e.g. a
    /// code from a newer version) to follow the system.
    pub fn from_pref(pref: &str) -> Lang {
        Lang::from_code(pref).unwrap_or_else(system_lang)
    }

    /// Every registered language.
    pub fn all() -> impl Iterator<Item = Lang> {
        LANGUAGES.iter().map(Lang)
    }

    fn catalog(self) -> &'static Catalog {
        self.0.catalog()
    }
}

/// The canonical `language` setting for user input: [`AUTO`] or a registered code (any case).
/// `None` for anything else, so callers keep the current setting.
pub fn normalize_pref(pref: &str) -> Option<&'static str> {
    if pref.eq_ignore_ascii_case(AUTO) {
        return Some(AUTO);
    }
    Lang::from_code(pref).map(Lang::code)
}

/// The registered language for a locale tag such as `th_TH.UTF-8` or `th-TH`; `None` when there
/// is no catalog for it. `C`/`POSIX` mean English.
pub fn lang_from_tag(tag: &str) -> Option<Lang> {
    let base = tag.split(['.', '@']).next().unwrap_or("").replace('_', "-").to_ascii_lowercase();
    let primary = base.split('-').find(|p| !p.is_empty())?;
    if matches!(primary, "c" | "posix") {
        return Some(Lang::EN);
    }
    Lang::from_code(&base).or_else(|| Lang::from_code(primary))
}

/// The system language (looked up once). English when it can't be determined or isn't supported.
pub fn system_lang() -> Lang {
    // Tests drive the UI by its English labels whatever the machine's language is.
    if cfg!(test) {
        return Lang::EN;
    }
    static SYSTEM: OnceLock<Lang> = OnceLock::new();
    *SYSTEM.get_or_init(detect_system_lang)
}

fn detect_system_lang() -> Lang {
    let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
    ["PHUTKRUB_LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"].iter().find_map(|name| var(name)).and_then(|v| lang_from_tag(&v)).unwrap_or(Lang::EN)
}

thread_local! {
    // Per thread: parallel test harnesses must not change each other's language.
    static CURRENT: Cell<Lang> = const { Cell::new(Lang::EN) };
}

/// Set the language the UI is drawn in (the app does this every frame from its setting).
pub fn set_current(lang: Lang) {
    CURRENT.set(lang);
}

/// The language the UI is drawn in.
pub fn current() -> Lang {
    CURRENT.get()
}

/// Does `lang` have a catalog entry for this string? (English never does: it is the source.)
pub fn has(lang: Lang, s: &str) -> bool {
    lang.catalog().plain(s).is_some()
}

/// `s` in the current language ([`tr`] with [`current`]).
pub fn t(s: &str) -> &str {
    tr(current(), s)
}

/// `s` in the current language as worded for `context` (one of `catalog::CONTEXTS`), falling
/// back to the plain translation and then to English.
pub fn tc<'a>(context: &str, s: &'a str) -> &'a str {
    trc(current(), context, s)
}

/// [`tc`] in a given language.
pub fn trc<'a>(lang: Lang, context: &str, s: &'a str) -> &'a str {
    #[cfg(test)]
    record::note(context, s);
    lang.catalog().in_context(context, s).unwrap_or(s)
}

/// `s` in `lang`; strings without a translation come back unchanged.
pub fn tr(lang: Lang, s: &str) -> &str {
    #[cfg(test)]
    record::note("", s);
    lang.catalog().plain(s).unwrap_or(s)
}

/// Fill `{name}` placeholders in one pass. Unknown placeholders stay as written, and inserted
/// values are never read as templates again, so a file name containing `{n}` stays intact.
pub fn fmt(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some((before, after_open)) = rest.split_once('{') {
        out.push_str(before);
        let Some((name, after_close)) = after_open.split_once('}') else {
            out.push('{');
            out.push_str(after_open);
            return out;
        };
        match args.iter().find(|(key, _)| *key == name) {
            Some((_, value)) => out.push_str(value),
            None => {
                out.push('{');
                out.push_str(name);
                out.push('}');
            }
        }
        rest = after_close;
    }
    out.push_str(rest);
    out
}

/// Fill positional `{}` placeholders in order, for strings that were `format!` templates (values
/// are never read as templates again). Extra `{}` stay as written; extra values are ignored.
pub fn fmtp(template: &str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut values = args.iter();
    let mut rest = template;
    while let Some((before, after)) = rest.split_once("{}") {
        out.push_str(before);
        match values.next() {
            Some(v) => out.push_str(&v.to_string()),
            None => out.push_str("{}"),
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
pub(crate) mod record;
#[cfg(test)]
mod tests;
