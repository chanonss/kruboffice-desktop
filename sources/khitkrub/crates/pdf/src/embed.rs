//! Embedded TrueType faces for scripts the standard 14 fonts lack (Thai first of all).
//!
//! The host registers font files it already has: named families ([`set_font_family`]), used for
//! all text drawn in that family, and fallback faces ([`set_fallback_faces`]), used for characters
//! WinAnsiEncoding cannot encode in any other text. Characters taken from a face are written as
//! its glyph ids, the face embedded whole as a `CIDFontType2` with `Identity-H` and a `ToUnicode` CMap (so the text stays
//! searchable and copyable). Without a face, or for characters no face maps, the predefined
//! Adobe-GB1 font stays the fallback; that font has no Thai, so viewers drew Thai with the
//! Chinese glyphs that happened to share the code (bug fixed by this module).
//!
//! There is no shaping: each character is its cmap glyph at its advance. Thai combining marks
//! have zero advance in Thai fonts and sit over the preceding letter, which reads correctly; marks
//! stacked on an upper vowel are not lowered (the font's GPOS is not applied).

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::{Arc, PoisonError, RwLock};

/// Largest font file accepted for embedding.
const MAX_FONT_BYTES: usize = 32 << 20;

/// One parsed face: the file and what the writer needs from it.
pub struct EmbeddedFace {
    bytes: Arc<[u8]>,
    /// The font's shaping tables, read once (GSUB/GPOS: Thai marks placed by the font's rules).
    shaper: harfrust::ShaperData,
    /// Advance of every glyph id, in 1/1000 em (the PDF `W` widths).
    advances: Vec<u16>,
    units_per_em: f32,
    name: String,
    /// Character → (glyph id, advance in 1/1000 em).
    glyphs: HashMap<char, (u16, u16)>,
    ascent: i32,
    descent: i32,
    cap_height: i32,
    bbox: [i32; 4],
}

impl EmbeddedFace {
    /// Parses a TrueType (`glyf`) font. CFF-flavoured OpenType, collections and unreadable data
    /// give `None`: they cannot be embedded as `FontFile2`.
    pub fn parse(bytes: Arc<[u8]>) -> Option<EmbeddedFace> {
        use skrifa::MetadataProvider as _;
        use skrifa::instance::{LocationRef, Size};
        use skrifa::raw::TableProvider as _;
        use skrifa::string::StringId;
        if bytes.is_empty() || bytes.len() > MAX_FONT_BYTES {
            return None;
        }
        let font = skrifa::FontRef::new(&bytes).ok()?;
        font.glyf().ok()?;
        let metrics = font.metrics(Size::unscaled(), LocationRef::default());
        let upem = f32::from(metrics.units_per_em.max(1));
        let em = |v: f32| -> i32 { (v * 1000.0 / upem).round() as i32 };
        let advances = font.glyph_metrics(Size::unscaled(), LocationRef::default());
        let advances: Vec<u16> = (0..metrics.glyph_count)
            .map(|g| {
                advances
                    .advance_width(skrifa::GlyphId::new(u32::from(g)))
                    .map_or(0, |w| u16::try_from(em(w).clamp(0, i32::from(u16::MAX))).unwrap_or(0))
            })
            .collect();
        let advance_of = |gid: u16| advances.get(usize::from(gid)).copied().unwrap_or(0);
        let mut glyphs = HashMap::new();
        for (code, gid) in font.charmap().mappings() {
            let Some(c) = char::from_u32(code) else { continue };
            let Ok(gid16) = u16::try_from(gid.to_u32()) else { continue };
            if gid16 == 0 || c.is_control() {
                continue;
            }
            glyphs.insert(c, (gid16, advance_of(gid16)));
        }
        if glyphs.is_empty() {
            return None;
        }
        let raw_name = font.localized_strings(StringId::POSTSCRIPT_NAME).english_or_first().map(|s| s.to_string()).unwrap_or_default();
        let mut name: String = raw_name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(60).collect();
        if name.is_empty() {
            name = "EmbeddedFont".into();
        }
        let bbox = metrics.bounds.map_or([0, -250, 1000, 1000], |b| [em(b.x_min), em(b.y_min), em(b.x_max), em(b.y_max)]);
        let ascent = em(metrics.ascent);
        let shaper = harfrust::ShaperData::new(&font);
        Some(EmbeddedFace {
            shaper,
            advances,
            units_per_em: upem,
            bytes,
            name,
            glyphs,
            ascent,
            descent: em(metrics.descent),
            cap_height: metrics.cap_height.map_or(ascent, em),
            bbox,
        })
    }

    /// Advance of glyph `gid` in 1/1000 em (what the PDF's `W` array says).
    pub fn advance(&self, gid: u16) -> u16 {
        self.advances.get(usize::from(gid)).copied().unwrap_or(0)
    }

    /// Shapes `text` with the font's own rules, left to right (glyph substitution and mark
    /// positioning: Thai tone marks lowered when there is no upper vowel, tails dropped before a
    /// lower vowel). Values are in 1/1000 em. `None` when a glyph id does not fit 16 bits.
    pub fn shape(&self, text: &str) -> Option<Vec<ShapedGlyph>> {
        let font = skrifa::FontRef::new(&self.bytes).ok()?;
        let shaper = self.shaper.shaper(&font).build();
        let mut buffer = harfrust::UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.set_direction(harfrust::Direction::LeftToRight);
        buffer.guess_segment_properties();
        let out = shaper.shape(buffer, harfrust::ShapeOptions::new());
        let em = |v: i32| -> i32 { (v as f32 * 1000.0 / self.units_per_em).round() as i32 };
        let infos = out.glyph_infos();
        let positions = out.glyph_positions();
        let mut glyphs = Vec::with_capacity(infos.len());
        for (i, (info, pos)) in infos.iter().zip(positions).enumerate() {
            let gid = u16::try_from(info.glyph_id).ok()?;
            let start = info.cluster as usize;
            let end = infos.iter().map(|g| g.cluster as usize).filter(|c| *c > start).min().unwrap_or(text.len());
            let cluster_text = text.get(start..end).unwrap_or_default();
            // Which glyph of its cluster this is, and how many the cluster has.
            let first = infos.iter().position(|g| g.cluster == info.cluster).unwrap_or(i);
            let count = infos.iter().filter(|g| g.cluster == info.cluster).count();
            // Copy and search: one character per glyph when the counts match (Thai letters and
            // their marks), else the whole cluster on its first glyph.
            let chars = if cluster_text.chars().count() == count {
                cluster_text.chars().nth(i - first).map(String::from).unwrap_or_default()
            } else if i == first {
                cluster_text.to_string()
            } else {
                String::new()
            };
            glyphs.push(ShapedGlyph { gid, text: chars, x_advance: em(pos.x_advance), x_offset: em(pos.x_offset), y_offset: em(pos.y_offset) });
        }
        Some(glyphs)
    }

    /// Glyph id and advance (1/1000 em) of `c`, when this face has it.
    pub fn glyph(&self, c: char) -> Option<(u16, u16)> {
        self.glyphs.get(&c).copied()
    }
}

/// One glyph of a shaped run, in 1/1000 em.
pub struct ShapedGlyph {
    pub gid: u16,
    /// The characters this glyph stands for (empty for the further glyphs of a cluster).
    pub text: String,
    pub x_advance: i32,
    pub x_offset: i32,
    pub y_offset: i32,
}

/// Style index of a face in a family: regular, bold, italic, bold italic.
fn style_index(bold: bool, italic: bool) -> usize {
    usize::from(bold) | (usize::from(italic) << 1)
}

/// The key a family name is matched by: lower case, spaces, `-` and `_` dropped, so
/// `TH Sarabun New` and `THSarabunNew` are the same family.
fn family_key(name: &str) -> String {
    name.chars().filter(|c| !matches!(c, ' ' | '-' | '_')).flat_map(char::to_lowercase).collect()
}

/// The faces a document can embed: the fallback (for characters the standard fonts lack) and
/// named families (for text set in that family, every character the face has).
#[derive(Clone, Default)]
pub struct FaceSet {
    faces: Vec<Arc<EmbeddedFace>>,
    /// Indexes into `faces` of the fallback regular and bold faces.
    fallback: [Option<usize>; 2],
    /// Family key → indexes of its regular, bold, italic and bold italic faces.
    families: Vec<(String, [Option<usize>; 4])>,
}

impl FaceSet {
    pub fn len(&self) -> usize {
        self.faces.len()
    }

    pub fn face(&self, i: usize) -> Option<&Arc<EmbeddedFace>> {
        self.faces.get(i)
    }

    /// The fallback face for `bold` text: the bold one when registered, else the regular one.
    pub fn fallback(&self, bold: bool) -> Option<usize> {
        let [regular, b] = self.fallback;
        if bold { b.or(regular) } else { regular.or(b) }
    }

    /// The face of the named family closest to the style: the exact one, else the same weight
    /// upright, else the regular, else any.
    pub fn family(&self, name: &str, bold: bool, italic: bool) -> Option<usize> {
        let key = family_key(name);
        let (_, faces) = self.families.iter().find(|(k, _)| *k == key)?;
        let get = |i: usize| faces.get(i).copied().flatten();
        get(style_index(bold, italic)).or_else(|| get(style_index(bold, false))).or_else(|| get(0)).or_else(|| faces.iter().flatten().next().copied())
    }

    /// Adds `face`, reusing an equal one already present (the same file registered twice).
    fn push(&mut self, face: Arc<EmbeddedFace>) -> usize {
        if let Some(i) = self.faces.iter().position(|f| Arc::ptr_eq(&f.bytes, &face.bytes) || f.bytes[..] == face.bytes[..]) {
            return i;
        }
        self.faces.push(face);
        self.faces.len() - 1
    }

    /// A set with these fallback faces (regular, bold) and no families.
    pub fn with_fallback(regular: Option<Arc<[u8]>>, bold: Option<Arc<[u8]>>) -> FaceSet {
        let mut set = FaceSet::default();
        set.set_fallback(regular, bold);
        set
    }

    fn set_fallback(&mut self, regular: Option<Arc<[u8]>>, bold: Option<Arc<[u8]>>) -> bool {
        let mut parse = |b: Option<Arc<[u8]>>| b.and_then(EmbeddedFace::parse).map(|f| self.push(Arc::new(f)));
        self.fallback = [parse(regular), parse(bold)];
        self.fallback.iter().any(Option::is_some)
    }

    /// Registers a family's faces (regular, bold, italic, bold italic), replacing it.
    pub fn set_family(&mut self, name: &str, faces: [Option<Arc<[u8]>>; 4]) -> bool {
        let key = family_key(name);
        self.families.retain(|(k, _)| *k != key);
        let idx = faces.map(|b| b.and_then(EmbeddedFace::parse).map(|f| self.push(Arc::new(f))));
        let any = idx.iter().any(Option::is_some);
        if any && !key.is_empty() {
            self.families.push((key, idx));
        }
        any
    }

    /// Drops faces no fallback or family refers to any more.
    fn prune(&mut self) {
        let mut keep: Vec<usize> = self.fallback.iter().chain(self.families.iter().flat_map(|(_, f)| f.iter())).flatten().copied().collect();
        keep.sort_unstable();
        keep.dedup();
        let map = |i: Option<usize>| i.and_then(|i| keep.iter().position(|k| *k == i));
        self.fallback = self.fallback.map(map);
        for (_, f) in &mut self.families {
            *f = f.map(map);
        }
        self.faces = keep.iter().filter_map(|i| self.faces.get(*i).cloned()).collect();
    }
}

static FACES: RwLock<Option<Arc<FaceSet>>> = RwLock::new(None);

fn update(change: impl FnOnce(&mut FaceSet) -> bool) -> bool {
    let mut guard = FACES.write().unwrap_or_else(PoisonError::into_inner);
    let mut set = guard.as_deref().cloned().unwrap_or_default();
    let ok = change(&mut set);
    set.prune();
    *guard = Some(Arc::new(set));
    ok
}

/// Registers the faces new documents embed for characters the standard fonts lack (Thai and
/// any other script the file covers) in text whose own family is not registered, replacing
/// earlier ones. Files that cannot be embedded are ignored. Returns whether any face was taken.
pub fn set_fallback_faces(regular: Option<Arc<[u8]>>, bold: Option<Arc<[u8]>>) -> bool {
    update(|set| set.set_fallback(regular, bold))
}

/// Registers a font family by name (regular, bold, italic, bold italic; any may be missing):
/// text drawn with that family name ([`crate::Page::text_in`]) is set in it, every character it
/// has. All `None` removes the family. Returns whether any face was taken.
pub fn set_font_family(name: &str, faces: [Option<Arc<[u8]>>; 4]) -> bool {
    update(|set| set.set_family(name, faces))
}

/// Whether a fallback face is registered.
pub fn has_fallback_faces() -> bool {
    FACES.read().unwrap_or_else(PoisonError::into_inner).as_ref().is_some_and(|s| s.fallback.iter().any(Option::is_some))
}

/// Whether the named family is registered.
pub fn has_font_family(name: &str) -> bool {
    current().family(name, false, false).is_some()
}

pub(crate) fn current() -> Arc<FaceSet> {
    FACES.read().unwrap_or_else(PoisonError::into_inner).clone().unwrap_or_default()
}

/// The PDF objects of one embedded face: `[Type0, CIDFontType2, FontDescriptor, FontFile2,
/// ToUnicode]` with ids `id..id + 5`. `used` maps glyph id → (character, advance).
pub(crate) fn objects(face: &EmbeddedFace, id: u32, used: &std::collections::BTreeMap<u16, (String, u16)>) -> Vec<(u32, Vec<u8>)> {
    let (cid, desc, file, to_unicode) = (id + 1, id + 2, id + 3, id + 4);
    let name = &face.name;
    let mut widths = String::new();
    for (gid, (_, w)) in used {
        let _ = write!(widths, "{gid} [{w}] ");
    }
    let [x0, y0, x1, y1] = face.bbox;
    let mut font_file = format!("<< /Length {} /Length1 {} >>\nstream\n", face.bytes.len(), face.bytes.len()).into_bytes();
    font_file.extend_from_slice(&face.bytes);
    font_file.extend_from_slice(b"\nendstream");
    let mut cmap = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    let entries: Vec<_> = used.iter().collect();
    let entries: Vec<_> = entries.into_iter().filter(|(_, (t, _))| !t.is_empty()).collect();
    for chunk in entries.chunks(100) {
        let _ = writeln!(cmap, "{} beginbfchar", chunk.len());
        for (gid, (text, _)) in chunk {
            let hex: String = text.encode_utf16().take(64).map(|u| format!("{u:04X}")).collect();
            let _ = writeln!(cmap, "<{gid:04X}> <{hex}>");
        }
        cmap.push_str("endbfchar\n");
    }
    cmap.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend");
    let mut to_unicode_obj = format!("<< /Length {} >>\nstream\n", cmap.len()).into_bytes();
    to_unicode_obj.extend_from_slice(cmap.as_bytes());
    to_unicode_obj.extend_from_slice(b"\nendstream");
    vec![
        (id, format!("<< /Type /Font /Subtype /Type0 /BaseFont /{name} /Encoding /Identity-H /DescendantFonts [{cid} 0 R] /ToUnicode {to_unicode} 0 R >>").into_bytes()),
        (
            cid,
            format!(
                "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{name} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {desc} 0 R /CIDToGIDMap /Identity /DW 1000 /W [{widths}] >>"
            )
            .into_bytes(),
        ),
        (
            desc,
            format!(
                "<< /Type /FontDescriptor /FontName /{name} /Flags 4 /FontBBox [{x0} {y0} {x1} {y1}] /ItalicAngle 0 /Ascent {} /Descent {} /CapHeight {} /StemV 80 /FontFile2 {file} 0 R >>",
                face.ascent, face.descent, face.cap_height
            )
            .into_bytes(),
        ),
        (file, font_file),
        (to_unicode, to_unicode_obj),
    ]
}
