use super::{
    Anchor, CellPicture, CellRef, Color, EngineError, Image, Json, MAX_OBJECT_OFFSET, ObjKind, ObjRef, PROTECTED, Result, Session, Shape, ShapeKind,
    Sparkline, SparklineKind, anchor_mode_name, anchor_mode_param, bad, bool_param, cell_param, edit, f64_param, json, locked_cell_error, ok,
    size_cap, str_param, u32_param,
};

#[cfg(test)]
pub(super) fn compiled() -> bool {
    true
}

pub(super) fn insert_sparkline(s: &mut Session, p: &Json) -> Result<Json> {
    let src = str_param(p, "range").ok_or_else(|| bad("insert.sparkline", "missing `range`"))?.to_string();
    let loc = cell_param(s, p, "location")?.ok_or_else(|| bad("insert.sparkline", "missing `location`"))?;
    let kind = match str_param(p, "type") {
        Some("column") => SparklineKind::Column,
        Some("winLoss") => SparklineKind::WinLoss,
        _ => SparklineKind::Line,
    };
    let markers = bool_param(p, "markers").unwrap_or(false);
    let sheet = s.doc()?.wb.active_sheet();
    edit(s, |cx| {
        let sh = cx.sheet_mut(sheet)?;
        sh.sparklines.retain(|x| x.cell != loc);
        sh.sparklines.push(Sparkline { cell: loc, source: src.clone(), kind, color: Color::Theme(4, 0), markers });
        Ok(Json::Null)
    })
}

pub(super) fn insert_picture(s: &mut Session, p: &Json) -> Result<Json> {
    let placement = str_param(p, "placement").unwrap_or("overCells");
    if !matches!(placement, "cell" | "overCells") {
        return Err(bad("insert.picture", "placement must be cell or overCells"));
    }
    let data = if let Some(b) = str_param(p, "base64") {
        crate::io::base64_decode(b).ok_or_else(|| bad("insert.picture", "invalid base64"))?
    } else if let Some(path) = str_param(p, "path") {
        crate::io::read_file(path)?
    } else {
        s.ui_requests.push(crate::UiRequest::Dialog("insertPicture".into(), json!({"placement": placement})));
        return ok();
    };
    if placement == "cell" {
        let mime = validate_cell_picture(&data)?;
        let d = s.doc()?;
        let sheet = d.wb.active_sheet();
        let sh = d.wb.active().ok_or(EngineError::NoDocument)?;
        let at = match str_param(p, "at") {
            Some(a) => CellRef::parse(a).ok_or_else(|| bad("insert.picture", "invalid cell address"))?,
            None => d.selection.active,
        };
        let at = sh.merge_at(at).map(|r| r.start).unwrap_or(at);
        let style = sh.style_id(at);
        if sh.is_protected() && d.wb.styles.get(style).protection.locked && locked_cell_error(sh, at, &d.unlocked_ranges).is_some() {
            return Err(EngineError::Other("The cell you're trying to change is on a protected sheet.".into()));
        }
        let picture = CellPicture { data, mime: mime.into(), alt: str_param(p, "alt").unwrap_or("").into() };
        return edit(s, |cx| {
            cx.sheet_mut(sheet)?.set_picture(at, std::sync::Arc::new(picture));
            cx.touch(sheet, at);
            Ok(json!({"cell": at.a1(), "placement": "cell"}))
        });
    }
    if data.len() > 64 * 1024 * 1024 {
        return Err(bad("insert.picture", "image larger than 64 MB"));
    }
    let mime = str_param(p, "mime")
        .map(str::to_string)
        .unwrap_or_else(|| if data.starts_with(&[0x89, b'P', b'N', b'G']) { "image/png".into() } else { "image/jpeg".into() });
    let (w, h) = crate::io::image_size(&data).unwrap_or((320, 240));
    let d = s.doc()?;
    // Sheet protection locks drawing objects, as for a text box's text.
    if d.wb.active().is_some_and(|sh| sh.is_protected()) {
        return Err(EngineError::Other(PROTECTED.into()));
    }
    let id = d.wb.next_object_id();
    let at = cell_param(s, p, "at")?.unwrap_or(d.selection.active);
    let scale = (640.0 / w.max(1) as f32).min(1.0);
    let img = Image {
        id,
        anchor: Anchor {
            cell: at,
            dx: 0.0,
            dy: 0.0,
            width: f64_param(p, "width").map(|v| v as f32).unwrap_or(w as f32 * scale),
            height: f64_param(p, "height").map(|v| v as f32).unwrap_or(h as f32 * scale),
            // Excel inserts pictures as "Move but don't size with cells" (`editAs="oneCell"`).
            mode: anchor_mode_param(p).unwrap_or(gridcraft_model::AnchorMode::MoveOnly),
        },
        data: data.into(),
        mime,
        alt: str_param(p, "alt").unwrap_or("").into(),
    };
    let sheet = d.wb.active_sheet();
    edit(s, |cx| {
        cx.sheet_mut(sheet)?.images.push(img);
        Ok(json!({"image": id}))
    })
}

/// Validate before mutating the workbook; header dimensions alone do not prove the pixels decode.
fn validate_cell_picture(data: &[u8]) -> Result<&'static str> {
    if data.len() > 16 * 1024 * 1024 {
        return Err(bad("insert.picture", "cell pictures must be at most 16 MiB"));
    }
    let format = image::guess_format(data).map_err(|_| bad("insert.picture", "choose a valid PNG or JPEG picture"))?;
    let mime = match format {
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::Jpeg => "image/jpeg",
        _ => return Err(bad("insert.picture", "cell pictures support PNG and JPEG")),
    };
    let (width, height) = image::ImageReader::with_format(std::io::Cursor::new(data), format)
        .into_dimensions()
        .map_err(|_| bad("insert.picture", "invalid image dimensions"))?;
    if width == 0 || height == 0 || width > 8192 || height > 8192 || u64::from(width) * u64::from(height) > 16_000_000 {
        return Err(bad("insert.picture", "cell pictures are limited to 8192 pixels per side and 16 million pixels"));
    }
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(data), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    reader.decode().map_err(|_| bad("insert.picture", "picture cannot be decoded within the 64 MiB limit"))?;
    Ok(mime)
}

pub(super) fn insert_shape(s: &mut Session, p: &Json) -> Result<Json> {
    let kind = match str_param(p, "kind").unwrap_or("rectangle") {
        "roundedRectangle" => ShapeKind::RoundedRectangle,
        "ellipse" | "oval" => ShapeKind::Ellipse,
        "triangle" => ShapeKind::Triangle,
        "line" => ShapeKind::Line,
        "arrow" => ShapeKind::Arrow,
        "textBox" => ShapeKind::TextBox,
        // Any standard DrawingML shape by its name (`star24`, `flowChartDecision`: the shared
        // Insert › Shapes menu, owner 2026-11-19); a name it doesn't know is a rectangle.
        other => ShapeKind::from_preset(other).unwrap_or(ShapeKind::Rectangle),
    };
    // Standard shapes other than KhitKrub's own go in square, as the menu draws them; lines flat.
    let preset = matches!(kind, ShapeKind::Preset(_));
    let open = matches!(kind, ShapeKind::Preset(id) if id.is_open());
    let d = s.doc()?;
    let id = d.wb.next_object_id();
    let at = cell_param(s, p, "at")?.unwrap_or(d.selection.active);
    let fill = crate::cmd::format::color_param(p.get("fill")).unwrap_or(if kind == ShapeKind::TextBox {
        Color::rgb(255, 255, 255)
    } else {
        Color::Theme(4, 0)
    });
    let line = crate::cmd::format::color_param(p.get("line")).unwrap_or(if kind == ShapeKind::TextBox {
        Color::rgb(0x80, 0x80, 0x80)
    } else {
        Color::Theme(4, -250)
    });
    let shape = Shape {
        id,
        kind,
        anchor: Anchor {
            cell: at,
            dx: 0.0,
            dy: 0.0,
            width: f64_param(p, "width").unwrap_or(if preset { 96.0 } else { 144.0 }) as f32,
            height: f64_param(p, "height").unwrap_or(if kind == ShapeKind::Line || open { 0.0 } else { 96.0 }) as f32,
            mode: anchor_mode_param(p).unwrap_or_default(),
        },
        fill,
        line,
        text: str_param(p, "text").unwrap_or("").into(),
        format: Default::default(),
        math: Default::default(),
    };
    let sheet = d.wb.active_sheet();
    edit(s, |cx| {
        cx.sheet_mut(sheet)?.shapes.push(shape);
        Ok(json!({"shape": id}))
    })
}

/// A text box holds at most as many characters as a cell.
const SHAPE_TEXT_MAX_CHARS: usize = 32_767;

pub(super) fn shape_set_text(s: &mut Session, p: &Json) -> Result<Json> {
    let id =
        p.get("id").and_then(Json::as_u64).and_then(|id| u32::try_from(id).ok()).ok_or_else(|| bad("shape.setText", "`id` must be a text box id"))?;
    let text = str_param(p, "text").ok_or_else(|| bad("shape.setText", "`text` must be a string"))?;
    if text.chars().nth(SHAPE_TEXT_MAX_CHARS).is_some() {
        return Err(bad("shape.setText", "`text` is longer than 32,767 characters"));
    }
    let d = s.doc()?;
    let sheet = d.wb.active_sheet();
    let sh = d.wb.active().ok_or_else(|| bad("shape.setText", "no such text box"))?;
    let shape = sh.shapes.iter().find(|shape| shape.id == id).ok_or_else(|| bad("shape.setText", "no such text box"))?;
    if !matches!(shape.kind, ShapeKind::TextBox | ShapeKind::WordArt) {
        return Err(bad("shape.setText", "the shape is not a text box"));
    }
    if shape.kind == ShapeKind::WordArt && text.is_empty() {
        return Err(bad("shape.setText", "WordArt text must not be empty"));
    }
    if shape.kind == ShapeKind::WordArt && text.chars().nth(crate::cmd::wordart::MAX_CHARS).is_some() {
        return Err(bad("shape.setText", format!("`text` is longer than {} characters", crate::cmd::wordart::MAX_CHARS)));
    }
    // Sheet protection locks drawing objects (there is no "edit objects" allowance yet).
    if sh.is_protected() {
        return Err(EngineError::Other(PROTECTED.into()));
    }
    if shape.text == text {
        return ok();
    }
    edit(s, |cx| {
        let shape = cx.sheet_mut(sheet)?.shapes.iter_mut().find(|shape| shape.id == id).ok_or_else(|| bad("shape.setText", "no such text box"))?;
        shape.text = text.into();
        Ok(Json::Null)
    })
}

/// The floating object a `{kind, id}` command names on the active sheet. Unknown kinds and
/// objects are errors, so a stale id never succeeds silently.
fn object_param(s: &Session, p: &Json, cmd: &str) -> Result<(usize, ObjRef)> {
    let id = u32_param(p, "id").ok_or_else(|| bad(cmd, "missing `id`"))?;
    let kind =
        str_param(p, "kind").and_then(ObjKind::parse).ok_or_else(|| bad(cmd, "kind must be chart, image, shape, slicer, equation or control"))?;
    let d = s.doc()?;
    let sheet = d.wb.active_sheet();
    let o = ObjRef::new(kind, id);
    if !d.wb.sheet(sheet).is_some_and(|sh| sh.has_object(o)) {
        return Err(bad(cmd, "no such object"));
    }
    // A locked form control on a protected sheet stays as it is.
    crate::cmd::controls::check_objects_unlocked(s, sheet, kind, &[id])?;
    Ok((sheet, o))
}

/// The file extension of a picture's bytes: what Save as Picture writes.
pub fn picture_format(bytes: &[u8]) -> &'static str {
    match bytes.get(..4) {
        Some([0x89, b'P', b'N', b'G']) => "png",
        Some([0xFF, 0xD8, ..]) => "jpg",
        Some([b'G', b'I', b'F', _]) => "gif",
        Some([b'R', b'I', b'F', b'F']) => "webp",
        Some([b'B', b'M', ..]) => "bmp",
        Some([0xD7, 0xCD, 0xC6, 0x9A]) => "wmf",
        Some([0x01, 0x00, 0x00, 0x00]) if bytes.get(40..44) == Some(b" EMF") => "emf",
        _ if bytes.starts_with(b"<svg") || bytes.starts_with(b"<?xml") => "svg",
        _ => "png",
    }
}

/// Picture `id` on the active sheet: its bytes and a file name in their own format,
/// "Picture.png" (owner, 2026-11-20).
pub fn picture_file(s: &Session, id: u32) -> Option<(String, std::sync::Arc<Vec<u8>>)> {
    let d = s.doc().ok()?;
    let im = d.wb.sheet(d.wb.active_sheet())?.images.iter().find(|im| im.id == id)?;
    Some((format!("Picture.{}", picture_format(&im.data)), im.data.clone()))
}

/// Save as Picture: picture `id`'s bytes to `path`, in their own format.
pub(super) fn save_picture(s: &mut Session, p: &Json) -> Result<Json> {
    let id = u32_param(p, "id").ok_or_else(|| bad("picture.save", "missing `id`"))?;
    let path = str_param(p, "path").filter(|x| !x.trim().is_empty()).ok_or_else(|| bad("picture.save", "missing `path`"))?;
    let (_, bytes) = picture_file(s, id).ok_or_else(|| bad("picture.save", "no such picture"))?;
    let format = picture_format(&bytes);
    let has_ext = std::path::Path::new(path).extension().is_some_and(|e| !e.is_empty());
    let path = if has_ext { path.to_string() } else { format!("{path}.{format}") };
    crate::io::write_file(&path, &bytes)?;
    Ok(json!({"path": path, "format": format, "bytes": bytes.len()}))
}

pub(super) fn delete_object(s: &mut Session, p: &Json) -> Result<Json> {
    let (sheet, o) = object_param(s, p, "object.delete")?;
    edit(s, |cx| {
        cx.sheet_mut(sheet)?.remove_object(o);
        Ok(json!({"deleted": o}))
    })
}

pub(super) fn move_object(s: &mut Session, p: &Json) -> Result<Json> {
    let (sheet, o) = object_param(s, p, "object.move")?;
    let at = cell_param(s, p, "at")?;
    edit(s, |cx| {
        let Some(a) = cx.sheet_mut(sheet)?.object_anchor_mut(o) else { return Err(bad("object.move", "no such object")) };
        if let Some(at) = at {
            a.cell = at;
        }
        if let Some(v) = f64_param(p, "dx") {
            a.dx = v.clamp(-MAX_OBJECT_OFFSET, MAX_OBJECT_OFFSET) as f32;
        }
        if let Some(v) = f64_param(p, "dy") {
            a.dy = v.clamp(-MAX_OBJECT_OFFSET, MAX_OBJECT_OFFSET) as f32;
        }
        if let Some(v) = f64_param(p, "width") {
            a.width = v.clamp(1.0, size_cap(a.width)) as f32;
        }
        if let Some(v) = f64_param(p, "height") {
            a.height = v.clamp(0.0, size_cap(a.height)) as f32;
        }
        Ok(Json::Null)
    })
}

/// Sets an object's anchor mode (Excel's "Move and size with cells" / "Move but don't size with
/// cells" / "Don't move or size with cells"), so it follows — or ignores — its rows and columns
/// when the sheet is sorted or edited structurally.
pub(super) fn set_anchor_mode(s: &mut Session, p: &Json) -> Result<Json> {
    let (sheet, o) = object_param(s, p, "object.setAnchorMode")?;
    let mode = anchor_mode_param(p).ok_or_else(|| bad("object.setAnchorMode", "mode must be moveAndSize, moveOnly or absolute"))?;
    edit(s, |cx| {
        let Some(a) = cx.sheet_mut(sheet)?.object_anchor_mut(o) else { return Err(bad("object.setAnchorMode", "no such object")) };
        a.mode = mode;
        Ok(json!({"mode": anchor_mode_name(mode)}))
    })
}

/// The shape `id` on the active sheet for command `cmd`, refusing a protected sheet.
fn editable_shape(s: &Session, cmd: &str, p: &Json) -> Result<(usize, u32)> {
    let id = p.get("id").and_then(Json::as_u64).and_then(|id| u32::try_from(id).ok()).ok_or_else(|| bad(cmd, "`id` must be a shape id"))?;
    let d = s.doc()?;
    let sheet = d.wb.active_sheet();
    let sh = d.wb.active().ok_or_else(|| bad(cmd, "no such shape"))?;
    if !sh.shapes.iter().any(|shape| shape.id == id) {
        return Err(bad(cmd, "no such shape"));
    }
    // Sheet protection locks drawing objects (there is no "edit objects" allowance yet).
    if sh.is_protected() {
        return Err(EngineError::Other(PROTECTED.into()));
    }
    Ok((sheet, id))
}

/// A finite number in `range` from `p[key]`, or an error naming it.
fn ranged(cmd: &str, p: &Json, key: &str, range: std::ops::RangeInclusive<f64>) -> Result<Option<f32>> {
    match p.get(key) {
        None => Ok(None),
        Some(v) => match v.as_f64() {
            Some(x) if x.is_finite() && range.contains(&x) => Ok(Some(x as f32)),
            _ => Err(bad(cmd, format!("`{key}` must be a number from {} to {}", range.start(), range.end()))),
        },
    }
}

/// Shape pane: the fill, outline, arrows, rotation and flips of a shape, in one undo step.
pub(super) fn shape_format(s: &mut Session, p: &Json) -> Result<Json> {
    use gridcraft_model::LineDash;
    const C: &str = "shape.format";
    let (sheet, id) = editable_shape(s, C, p)?;
    // Read and check every parameter before changing anything.
    let fill = match p.get("fill") {
        None => None,
        Some(Json::Null) => Some(None),
        Some(v) => Some(Some(crate::cmd::format::color_param(Some(v)).ok_or_else(|| bad(C, "`fill` must be a colour or null"))?)),
    };
    let line = match p.get("line") {
        None => None,
        Some(Json::Null) => Some(None),
        Some(v) => Some(Some(crate::cmd::format::color_param(Some(v)).ok_or_else(|| bad(C, "`line` must be a colour or null"))?)),
    };
    let fill_t = ranged(C, p, "fillTransparency", 0.0..=100.0)?;
    let line_t = ranged(C, p, "lineTransparency", 0.0..=100.0)?;
    let width = ranged(C, p, "lineWidth", 0.25..=72.0)?;
    let rotation = ranged(C, p, "rotation", -3600.0..=3600.0)?;
    let dash = match p.get("lineDash").map(|v| v.as_str()) {
        None => None,
        Some(Some("solid")) => Some(LineDash::Solid),
        Some(Some("sysDot" | "dot")) => Some(LineDash::SysDot),
        Some(Some("dash")) => Some(LineDash::Dash),
        Some(Some("lgDash")) => Some(LineDash::LgDash),
        Some(Some("dashDot")) => Some(LineDash::DashDot),
        _ => return Err(bad(C, "`lineDash` must be solid, sysDot, dash, lgDash or dashDot")),
    };
    let flag = |key: &str| -> Result<Option<bool>> {
        match p.get(key) {
            None => Ok(None),
            Some(v) => v.as_bool().map(Some).ok_or_else(|| bad(C, format!("`{key}` must be true or false"))),
        }
    };
    let (arrow_start, arrow_end, flip_h, flip_v) = (flag("arrowStart")?, flag("arrowEnd")?, flag("flipH")?, flag("flipV")?);
    let shadow = match p.get("shadow") {
        None => None,
        Some(Json::Null) => Some(None),
        Some(v @ Json::Object(_)) => {
            let d = gridcraft_model::ShapeShadow::default();
            Some(Some(gridcraft_model::ShapeShadow {
                dist: ranged(C, v, "dist", 0.0..=200.0)?.unwrap_or(d.dist),
                dir: ranged(C, v, "dir", -3600.0..=3600.0)?.map_or(d.dir, |a| a.rem_euclid(360.0)),
                blur: ranged(C, v, "blur", 0.0..=100.0)?.unwrap_or(d.blur),
                transparency: ranged(C, v, "transparency", 0.0..=100.0)?.unwrap_or(d.transparency),
            }))
        }
        Some(_) => return Err(bad(C, "`shadow` must be null or {dist, dir, blur, transparency}")),
    };
    let glow_radius = ranged(C, p, "glowRadius", 0.0..=150.0)?;
    let glow_t = ranged(C, p, "glowTransparency", 0.0..=100.0)?;
    let soft_edge = ranged(C, p, "softEdge", 0.0..=100.0)?;
    let glow_color = match p.get("glowColor") {
        None => None,
        Some(v) => Some(crate::cmd::format::color_param(Some(v)).ok_or_else(|| bad(C, "`glowColor` must be a colour"))?),
    };
    edit(s, |cx| {
        let shape = cx.sheet_mut(sheet)?.shapes.iter_mut().find(|shape| shape.id == id).ok_or_else(|| bad(C, "no such shape"))?;
        let f = &mut shape.format;
        match fill {
            Some(Some(c)) => {
                shape.fill = c;
                f.no_fill = false;
            }
            Some(None) => f.no_fill = true,
            None => {}
        }
        match line {
            Some(Some(c)) => {
                shape.line = c;
                f.no_line = false;
            }
            Some(None) => f.no_line = true,
            None => {}
        }
        if let Some(t) = fill_t {
            f.fill_transparency = t;
        }
        if let Some(t) = line_t {
            f.line_transparency = t;
        }
        if let Some(w) = width {
            f.line_width = Some(w);
        }
        if let Some(d) = dash {
            f.line_dash = d;
            // Choosing a dash shows the outline again.
            f.no_line = false;
        }
        if let Some(r) = rotation {
            f.rotation = r.rem_euclid(360.0);
        }
        f.arrow_start = arrow_start.unwrap_or(f.arrow_start);
        f.arrow_end = arrow_end.unwrap_or(f.arrow_end);
        f.flip_h = flip_h.unwrap_or(f.flip_h);
        f.flip_v = flip_v.unwrap_or(f.flip_v);
        if let Some(sh) = shadow {
            f.shadow = sh;
        }
        if let Some(c) = glow_color {
            f.glow_color = c;
            // Choosing a colour for no glow turns on a small one.
            if f.glow_radius <= 0.0 && glow_radius.is_none() {
                f.glow_radius = 5.0;
            }
        }
        if let Some(r) = glow_radius {
            f.glow_radius = r;
        }
        if let Some(t) = glow_t {
            f.glow_transparency = t;
        }
        if let Some(r) = soft_edge {
            f.soft_edge = r;
        }
        Ok(Json::Null)
    })
}

/// Shape pane › Arrange: move a shape up or down the sheet's stacking order of shapes.
pub(super) fn object_order(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "object.order";
    if p.get("kind").and_then(Json::as_str).is_some_and(|k| k != "shape") {
        return Err(bad(C, "only shapes can be reordered"));
    }
    let (sheet, id) = editable_shape(s, C, p)?;
    let op = p.get("op").and_then(Json::as_str).unwrap_or("");
    if !matches!(op, "bringToFront" | "bringForward" | "sendBackward" | "sendToBack") {
        return Err(bad(C, "`op` must be bringToFront, bringForward, sendBackward or sendToBack"));
    }
    edit(s, |cx| {
        let shapes = &mut cx.sheet_mut(sheet)?.shapes;
        let i = shapes.iter().position(|shape| shape.id == id).ok_or_else(|| bad(C, "no such shape"))?;
        let last = shapes.len().saturating_sub(1);
        let to = match op {
            "bringToFront" => last,
            "bringForward" => (i + 1).min(last),
            "sendBackward" => i.saturating_sub(1),
            _ => 0,
        };
        let shape = shapes.remove(i);
        shapes.insert(to.min(shapes.len()), shape);
        Ok(Json::Null)
    })
}
