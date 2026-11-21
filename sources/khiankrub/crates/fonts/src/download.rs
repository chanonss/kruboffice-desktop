//! Fonts a Thai document may ask for, and where to get them: when one isn't installed, the app
//! can say where to download it (or that it comes only with Windows) instead of only naming the
//! substitute. Ported by hand from WordCraft 8331f2e6 (#6826), with KhianKrub's own list: the
//! Thai national fonts SIPA gives away, and Microsoft's Thai fonts that come with Windows.
//!
//! Only a publisher's own page is listed. The app links to the page; it never downloads, bundles
//! or redistributes the fonts, and following the publisher's licence terms is the user's part.
//!
//! The list is `font_downloads.json` beside this file, built into the app: adding a font is a
//! new entry there.

/// The list: `{"downloads": [{"family", "publisher", "url"?, "windows"?, "restricted"?, "note"?}]}`.
const LIST: &str = include_str!("font_downloads.json");

/// Most entries read from the list.
const MAX_DOWNLOADS: usize = 1024;

/// Where a font family can be had.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontDownload {
    /// The family the entry names. It covers the family itself and every name that continues
    /// it after a space ("TH Sarabun New" covers "TH Sarabun New Bold").
    pub name: String,
    /// Who publishes the font.
    pub publisher: String,
    /// The publisher's download page; `None` for a font that comes only with Windows.
    pub url: Option<String>,
    /// The font comes with Windows and is not offered on its own.
    pub windows: bool,
    /// The publisher limits who may use the font or what for: worth saying beside the link.
    pub restricted: bool,
}

/// The entries of `list` that name a family, a publisher and either an `https` page or
/// `"windows": true`; others are left out, and a list that isn't JSON has none.
fn parse(list: &str) -> Vec<FontDownload> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(list) else {
        log::warn!("font downloads: the list isn't JSON");
        return Vec::new();
    };
    let entries = json.get("downloads").and_then(|d| d.as_array()).map(Vec::as_slice).unwrap_or_default();
    entries
        .iter()
        .take(MAX_DOWNLOADS)
        .filter_map(|e| {
            let field = |k: &str| e.get(k).and_then(|v| v.as_str()).map(str::trim).filter(|v| !v.is_empty());
            let flag = |k: &str| e.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
            let (name, publisher) = (field("family")?, field("publisher")?);
            let url = match field("url") {
                Some(u) if u.starts_with("https://") => Some(u.to_string()),
                Some(_) => return None,
                None if flag("windows") => None,
                None => return None,
            };
            Some(FontDownload { name: name.into(), publisher: publisher.into(), url, windows: flag("windows"), restricted: flag("restricted") })
        })
        .collect()
}

/// The built-in list, read once.
fn downloads() -> &'static [FontDownload] {
    static DOWNLOADS: std::sync::OnceLock<Vec<FontDownload>> = std::sync::OnceLock::new();
    DOWNLOADS.get_or_init(|| parse(LIST))
}

/// Where `family` can be had, if the list knows. The name is matched whatever its case, with a
/// charset suffix ("Angsana New CE") or alternates ("TH SarabunPSK;Tahoma") as a document may
/// write it.
pub fn download_for(family: &str) -> Option<&'static FontDownload> {
    family.split(';').map(str::trim).filter(|f| !f.is_empty()).find_map(|f| {
        let f = crate::word::charset_base_family(f).trim();
        downloads().iter().find(|d| {
            // Compared as bytes: `get` on a `str` would refuse a split inside a character.
            let (name, f) = (d.name.as_bytes(), f.as_bytes());
            f.get(..name.len()).is_some_and(|head| head.eq_ignore_ascii_case(name)) && matches!(f.get(name.len()), None | Some(b' '))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_thai_national_fonts_point_at_sipas_pages() {
        let new = download_for("TH Sarabun New").unwrap();
        assert_eq!(new.publisher, "SIPA");
        assert_eq!(new.url.as_deref(), Some("https://www.f0nt.com/release/th-sarabun-new/"));
        for family in ["th sarabun new", "TH Sarabun New Bold", " TH Sarabun New ", "TH Sarabun New;Tahoma"] {
            assert_eq!(download_for(family), Some(new), "{family}");
        }
        for family in ["TH SarabunPSK", "TH Sarabun PSK", "TH Niramit AS", "TH K2D July8", "TH Chakra Petch", "TH Fah kwang", "TH KoHo"] {
            let d = download_for(family).unwrap_or_else(|| panic!("{family}"));
            assert_eq!(d.url.as_deref(), Some("https://www.f0nt.com/release/13-free-fonts-from-sipa/"), "{family}");
            assert!(!d.windows && !d.restricted, "{family}");
        }
    }

    #[test]
    fn windows_thai_fonts_say_they_come_with_windows() {
        for family in ["Angsana New", "AngsanaUPC", "Cordia New", "Browallia New", "Leelawadee UI", "EucrosiaUPC"] {
            let d = download_for(family).unwrap_or_else(|| panic!("{family}"));
            assert!(d.windows && d.url.is_none(), "{family}");
        }
    }

    #[test]
    fn other_fonts_have_none() {
        for family in ["Calibri", "Tahoma", "TH Sarabun", "TH Sarabun Newer", "Sarabun", "", ";", "ไทย", "Aptosé"] {
            assert_eq!(download_for(family), None, "{family}");
        }
    }

    #[test]
    fn every_entry_is_read_and_every_page_is_https() {
        let entries = serde_json::from_str::<serde_json::Value>(LIST).unwrap()["downloads"].as_array().unwrap().len();
        assert_eq!(downloads().len(), entries, "no entry is left out for a missing field");
        let hosts = ["https://www.f0nt.com/release/", "https://www.microsoft.com/"];
        for d in downloads() {
            match &d.url {
                Some(u) => assert!(hosts.iter().any(|h| u.starts_with(h)), "{u}"),
                None => assert!(d.windows, "{}", d.name),
            }
        }
    }

    #[test]
    fn a_broken_list_has_no_downloads() {
        assert!(parse("not json").is_empty());
        assert!(parse("{}").is_empty());
        assert!(parse(r#"{"downloads": 7}"#).is_empty());
        let list = r#"{"downloads": [
            {"family": "A", "publisher": "P", "url": "http://plain.example"},
            {"family": "B", "publisher": "P"},
            {"family": "", "publisher": "P", "url": "https://x.example"},
            {"publisher": "P", "url": "https://x.example"},
            {"family": 3, "publisher": "P", "url": "https://x.example"},
            "junk",
            {"family": " Kept ", "publisher": "P", "url": "https://x.example"},
            {"family": "Win", "publisher": "P", "windows": true}
        ]}"#;
        let read = parse(list);
        assert_eq!(read.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(), ["Kept", "Win"]);
        assert!(!read[0].restricted && !read[0].windows);
        assert!(read[1].windows && read[1].url.is_none());
    }
}
