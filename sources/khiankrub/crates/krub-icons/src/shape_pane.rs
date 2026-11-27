//! The object (shape, text box) pane, the same in every KrubOffice app (owner, 2026-11-16; after
//! LibreOffice's object Properties sidebar). Owner, 2026-11-18: three sub-panels chosen with a
//! segmented control ([`crate::pane_ui`]): Fill and Line (Area, Line), Effects (Shadow, Glow,
//! Soft Edges) and Size and Position (Size, Arrange), each section under a shaded heading that
//! folds it, with compact rows whose boxes line up.
//!
//! An app passes the selected object's current [`Values`], the rows it can store ([`Caps`]: a
//! row it can't store is left out, never shown as a button that does nothing), a translator for
//! the English names, and a [`Host`] that draws what looks different in each app (colour boxes,
//! the shadow presets, labeled buttons). [`sections`] returns the [`Edit`]s the person made this
//! frame; the app runs them as its own commands. Every change applies at once (no OK button).
//! The apps run different egui versions, so this uses only what they share.

use egui::{Ui, vec2};

/// What the app can store for the selected object; rows it can't are left out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Caps {
    /// Area: the fill colour and its transparency.
    pub area: bool,
    /// Line: style, thickness and colour.
    pub line: bool,
    /// Line › Transparency.
    pub line_transparency: bool,
    /// Line › Arrow styles (lines and connectors).
    pub arrows: bool,
    /// Effect: shadow, glow and soft edges.
    pub effects: bool,
    /// Position and Size › Rotation.
    pub rotation: bool,
    /// Position and Size › Flip.
    pub flip: bool,
    /// Position and Size › Align: left, centre, right.
    pub align: bool,
    /// Position and Size › Align also offers top, middle, bottom.
    pub align_vertical: bool,
    /// Position and Size › Arrange: the stacking order.
    pub arrange: bool,
}

impl Caps {
    /// Every row.
    pub const ALL: Caps = Caps {
        area: true,
        line: true,
        line_transparency: true,
        arrows: true,
        effects: true,
        rotation: true,
        flip: true,
        align: true,
        align_vertical: true,
        arrange: true,
    };
}

/// An outline's dash, in the order Line style lists them after None.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Dash {
    #[default]
    Solid,
    Dotted,
    Dashed,
    LongDash,
    DashDot,
}

impl Dash {
    /// Every dash, in the list's order.
    pub const ALL: [Dash; 5] = [Dash::Solid, Dash::Dotted, Dash::Dashed, Dash::LongDash, Dash::DashDot];

    /// The English name the apps translate.
    pub fn name(self) -> &'static str {
        match self {
            Dash::Solid => "Solid",
            Dash::Dotted => "Dotted",
            Dash::Dashed => "Dashed",
            Dash::LongDash => "Long dash",
            Dash::DashDot => "Dash dot",
        }
    }

    /// How it is drawn in the Line style list.
    pub fn look(self) -> crate::line_style::Look {
        use crate::line_style::Look;
        match self {
            Dash::Solid => Look::Solid,
            Dash::Dotted => Look::Dotted,
            Dash::Dashed => Look::Dashed,
            Dash::LongDash => Look::LongDash,
            Dash::DashDot => Look::DashDot,
        }
    }
}

/// The selected object's current values. Sizes are in the app's display unit (`size_suffix`);
/// thicknesses and radii in points; transparencies and angles in percent and degrees.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Values {
    /// A line or connector: no Area, and arrows.
    pub open: bool,
    pub has_fill: bool,
    pub fill_transparency: f32,
    pub has_line: bool,
    pub dash: Dash,
    pub thickness: f32,
    pub line_transparency: f32,
    pub arrow_start: bool,
    pub arrow_end: bool,
    pub has_glow: bool,
    pub glow_radius: f32,
    pub glow_transparency: f32,
    pub soft_edge: f32,
    pub width: f32,
    pub height: f32,
    /// The display unit after a size: " in", " cm"…
    pub size_suffix: String,
    /// The largest width or height the app stores, in the display unit.
    pub size_max: f32,
    pub rotation: f32,
    /// Keep ratio, the app's remembered choice.
    pub keep_ratio: bool,
    /// A rounded rectangle's corner radius, in the display unit; `None` for any other shape (no
    /// Corner radius row).
    pub corner: Option<f32>,
    /// The largest corner radius (half the shorter side, round ends), in the display unit.
    pub corner_max: f32,
}

/// A change the person made, for the app to run as its own command.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    FillTransparency(f32),
    /// Line style › None.
    LineNone,
    LineDash(Dash),
    Thickness(f32),
    LineTransparency(f32),
    ArrowStart(bool),
    ArrowEnd(bool),
    /// 0 removes the glow.
    GlowRadius(f32),
    GlowTransparency(f32),
    /// 0 removes the soft edges.
    SoftEdge(f32),
    /// In the display unit; with Keep ratio the app scales the other side.
    Width(f32),
    Height(f32),
    KeepRatio(bool),
    Rotation(f32),
    /// A rounded rectangle's corner radius, in the display unit (0: square corners).
    CornerRadius(f32),
    FlipVertical,
    FlipHorizontal,
    /// left, center, right, top, middle or bottom.
    Align(&'static str),
    /// bringToFront, bringForward, sendBackward or sendToBack.
    Order(&'static str),
}

/// A colour box the app draws (its own palette).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Fill,
    Line,
    Glow,
}

/// What each app draws its own way.
pub trait Host {
    /// A colour box for `slot`, which runs the app's command when a colour is picked.
    fn color(&mut self, ui: &mut Ui, slot: Slot, enabled: bool);
    /// The Shadow button and its menu (No Shadow, then the app's presets).
    fn shadow(&mut self, ui: &mut Ui);
    /// A button with `icon` (an id from [`ICONS`]) and the visible `label`, announced as `name`;
    /// `true` when clicked.
    fn button(&mut self, ui: &mut Ui, icon: &str, label: &str, name: &str, width: f32) -> bool;
    /// A number box; `Some(v)` when the person changed it. The app's own box, so typing in it
    /// works as everywhere else in the app; [`drag_number`] otherwise.
    fn number(&mut self, ui: &mut Ui, n: Number) -> Option<f32> {
        drag_number(ui, n)
    }
}

/// The icon ids [`Host::button`] is asked for.
pub const ICONS: [&str; 12] = [
    "flipVertical",
    "flipHorizontal",
    "alignLeft",
    "alignCenter",
    "alignRight",
    "alignTop",
    "alignMiddle",
    "alignBottom",
    "bringToFront",
    "bringForward",
    "sendBackward",
    "sendToBack",
];

/// The English names the module shows, for the apps' translation tables.
pub const NAMES: [&str; 51] = [
    "Fill and Line",
    "Effects",
    "Size and Position",
    "Shadow",
    "Size",
    "Arrange",
    "Area",
    "Line",
    "Effect",
    "Position and Size",
    "Fill:",
    "Transparency:",
    "Line style:",
    "None",
    "Solid",
    "Dotted",
    "Dashed",
    "Long dash",
    "Dash dot",
    "Thickness:",
    "Color:",
    "Arrow styles:",
    "Line start",
    "Line end",
    "Glow",
    "Radius:",
    "Soft Edges",
    "Width:",
    "Height:",
    "Keep ratio",
    "Rotation:",
    "Corner radius:",
    "Flip:",
    "Vertically",
    "Horizontally",
    "Align:",
    "Left",
    "Center",
    "Right",
    "Top",
    "Middle",
    "Bottom",
    "Align Objects Left",
    "Align Objects Center",
    "Align Objects Right",
    "Align Objects Top",
    "Align Objects Middle",
    "Align Objects Bottom",
    "Arrange:",
    "Bring to Front",
    "Send to Back",
];

/// The label shown beside a rounded rectangle's corner handle while it is dragged: the row's
/// name and `number`, the app's own text for the Corner radius box ("0.85 cm"), so the two read
/// the same.
pub fn corner_readout_text(tr: &dyn Fn(&str) -> String, number: &str) -> String {
    format!("{} {number}", tr("Corner radius"))
}

/// Draw `text` as a small dark label beside the corner handle at `at` (screen points): above and
/// to the right of it, below it where there is no room above, kept inside the painter's clip.
pub fn paint_corner_readout(painter: &egui::Painter, at: egui::Pos2, text: String) {
    let white = egui::Color32::WHITE;
    let galley = painter.layout_no_wrap(text, egui::FontId::proportional(12.0), white);
    let (pad, gap) = (5.0, 10.0);
    let size = galley.size() + vec2(pad * 2.0, pad);
    let clip = painter.clip_rect();
    let mut min = egui::pos2(at.x + gap, at.y - gap - size.y);
    if min.y < clip.min.y {
        min.y = at.y + gap;
    }
    min.x = min.x.min(clip.max.x - size.x).max(clip.min.x);
    let rect = egui::Rect::from_min_size(min, size);
    painter.rect_filled(rect, egui::CornerRadius::same(4), egui::Color32::from_rgba_unmultiplied(28, 28, 32, 232));
    painter.galley(rect.min + vec2(pad, pad / 2.0), galley, white);
}

/// A section under a shaded heading that folds it ([`crate::pane_ui::section`]); `key` keeps
/// its folding apart from the other sections'.
fn section(ui: &mut Ui, key: &str, title: String, open: bool, add: impl FnOnce(&mut Ui)) {
    crate::pane_ui::section(ui, crate::pane_ui::id(("krub_shape_section", key)), &title, open, add);
}

/// A grid of label and box rows: the labels in one column of a fixed width, so the boxes of
/// every section line up.
fn rows(id: &str) -> egui::Grid {
    egui::Grid::new(id).num_columns(2).spacing(vec2(8.0, crate::pane_ui::ROW_GAP)).min_col_width(crate::pane_ui::LABEL_W)
}

/// The sub-panels, in the segmented control's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    FillLine,
    Effects,
    SizePosition,
}

impl Tab {
    /// The tabs an app with `caps` shows (Effects only when it stores effects).
    pub fn shown(caps: Caps) -> Vec<Tab> {
        let mut v = vec![Tab::FillLine];
        if caps.effects {
            v.push(Tab::Effects);
        }
        v.push(Tab::SizePosition);
        v
    }

    fn icon(self) -> &'static str {
        match self {
            Tab::FillLine => "shape-fill",
            Tab::Effects => "effects",
            Tab::SizePosition => "size",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Tab::FillLine => "Fill and Line",
            Tab::Effects => "Effects",
            Tab::SizePosition => "Size and Position",
        }
    }
}

/// One number box of the pane, for [`Host::number`].
pub struct Number<'a> {
    /// A key unique to the box ("fill_transparency", "width"…).
    pub key: &'static str,
    /// The row's label (its response id), which names the box for assistive technology.
    pub label: egui::Id,
    /// The row's translated name without the colon.
    pub name: &'a str,
    pub value: f32,
    pub range: std::ops::RangeInclusive<f32>,
    pub speed: f32,
    pub suffix: &'a str,
    pub enabled: bool,
}

/// egui's drag value as the number box, for an app without one of its own.
pub fn drag_number(ui: &mut Ui, n: Number) -> Option<f32> {
    let mut v = n.value;
    let r =
        ui.add_enabled(n.enabled, egui::DragValue::new(&mut v).range(n.range).speed(n.speed).max_decimals(2).suffix(n.suffix)).labelled_by(n.label);
    r.changed().then_some(v)
}

/// Where the chosen sub-panel is kept for a pane showing `tabs`.
fn tab_id(tabs: &[Tab]) -> egui::Id {
    crate::pane_ui::id(("krub_shape_tab", tabs.len()))
}

/// The sub-panel the pane shows for `caps`, so an app can put its own sections under it.
pub fn current_tab(ctx: &egui::Context, caps: Caps) -> Tab {
    let tabs = Tab::shown(caps);
    let i = ctx.data_mut(|d| *d.get_persisted_mut_or(tab_id(&tabs), 0usize));
    tabs.get(i).copied().unwrap_or(Tab::FillLine)
}

/// The segmented control and the chosen sub-panel's sections. Returns this frame's edits.
pub fn sections(ui: &mut Ui, v: &Values, caps: Caps, tr: &dyn Fn(&str) -> String, host: &mut dyn Host) -> Vec<Edit> {
    let mut edits = Vec::new();
    let tabs = Tab::shown(caps);
    let segments: Vec<crate::pane_ui::Segment> = tabs.iter().map(|t| crate::pane_ui::Segment { icon: t.icon(), label: tr(t.name()) }).collect();
    let picked = crate::pane_ui::segmented(ui, tab_id(&tabs), &segments, 0);
    match tabs.get(picked).copied().unwrap_or(Tab::FillLine) {
        Tab::FillLine => {
            if caps.area && !v.open {
                ui.push_id("krub_area", |ui| section(ui, "area", tr("Area"), true, |ui| area(ui, v, tr, host, &mut edits)));
            }
            if caps.line {
                ui.push_id("krub_line", |ui| section(ui, "line", tr("Line"), true, |ui| line(ui, v, caps, tr, host, &mut edits)));
            }
        }
        Tab::Effects => {
            ui.push_id("krub_shadow", |ui| section(ui, "shadow", tr("Shadow"), true, |ui| host.shadow(ui)));
            ui.push_id("krub_glow", |ui| section(ui, "glow", tr("Glow"), true, |ui| glow(ui, v, tr, host, &mut edits)));
            ui.push_id("krub_soft", |ui| section(ui, "soft", tr("Soft Edges"), true, |ui| soft_edges(ui, v, tr, host, &mut edits)));
        }
        Tab::SizePosition => {
            ui.push_id("krub_size", |ui| section(ui, "size", tr("Size"), true, |ui| size(ui, v, caps, tr, host, &mut edits)));
            if caps.flip || caps.align || caps.arrange {
                ui.push_id("krub_arrange", |ui| section(ui, "arrange", tr("Arrange"), true, |ui| arrange(ui, caps, tr, host, &mut edits)));
            }
        }
    }
    edits
}

fn area(ui: &mut Ui, v: &Values, tr: &dyn Fn(&str) -> String, host: &mut dyn Host, edits: &mut Vec<Edit>) {
    rows("krub_shape_area").show(ui, |ui| {
        ui.label(tr("Fill:"));
        host.color(ui, Slot::Fill, true);
        ui.end_row();
        let label = ui.label(tr("Transparency:")).id;
        let name = tr("Transparency");
        if let Some(t) = host.number(
            ui,
            Number {
                key: "fill_transparency",
                label,
                name: &name,
                value: v.fill_transparency,
                range: 0.0..=100.0,
                speed: 1.0,
                suffix: "%",
                enabled: v.has_fill,
            },
        ) {
            edits.push(Edit::FillTransparency(t));
        }
        ui.end_row();
    });
}

fn line(ui: &mut Ui, v: &Values, caps: Caps, tr: &dyn Fn(&str) -> String, host: &mut dyn Host, edits: &mut Vec<Edit>) {
    rows("krub_shape_line").show(ui, |ui| {
        let style_label = ui.label(tr("Line style:")).id;
        // Each style drawn as a line (owner, 2026-11-19); None (no outline) by name.
        let names: Vec<String> = Dash::ALL.iter().map(|d| tr(d.name())).collect();
        let none = tr("None");
        let mut items = vec![crate::line_style::Item::new(None, None, none.as_str())];
        items.extend(Dash::ALL.iter().zip(&names).map(|(d, n)| crate::line_style::Item::new(Some(*d), Some(d.look()), n.as_str())));
        let current = if v.has_line { Some(v.dash) } else { None };
        let (resp, picked) = crate::line_style::combo(ui, crate::pane_ui::id("krub_shape_line_style"), 150.0, Some(current), &items);
        match picked {
            Some(None) if v.has_line => edits.push(Edit::LineNone),
            Some(Some(d)) => edits.push(Edit::LineDash(d)),
            _ => {}
        }
        let combo = egui::InnerResponse::new((), resp);
        let _ = combo.response.labelled_by(style_label);
        ui.end_row();
        let label = ui.label(tr("Thickness:")).id;
        let name = tr("Thickness");
        if let Some(t) = host.number(
            ui,
            Number { key: "thickness", label, name: &name, value: v.thickness, range: 0.25..=72.0, speed: 0.25, suffix: " pt", enabled: v.has_line },
        ) {
            edits.push(Edit::Thickness(t));
        }
        ui.end_row();
        ui.label(tr("Color:"));
        host.color(ui, Slot::Line, true);
        ui.end_row();
        if caps.line_transparency {
            let label = ui.label(tr("Transparency:")).id;
            let name = tr("Transparency");
            if let Some(t) = host.number(
                ui,
                Number {
                    key: "line_transparency",
                    label,
                    name: &name,
                    value: v.line_transparency,
                    range: 0.0..=100.0,
                    speed: 1.0,
                    suffix: "%",
                    enabled: v.has_line,
                },
            ) {
                edits.push(Edit::LineTransparency(t));
            }
            ui.end_row();
        }
        if caps.arrows && v.open {
            ui.label(tr("Arrow styles:"));
            ui.horizontal(|ui| {
                let (mut a, mut b) = (v.arrow_start, v.arrow_end);
                if ui.checkbox(&mut a, tr("Line start")).changed() {
                    edits.push(Edit::ArrowStart(a));
                }
                if ui.checkbox(&mut b, tr("Line end")).changed() {
                    edits.push(Edit::ArrowEnd(b));
                }
            });
            ui.end_row();
        }
    });
}

fn glow(ui: &mut Ui, v: &Values, tr: &dyn Fn(&str) -> String, host: &mut dyn Host, edits: &mut Vec<Edit>) {
    rows("krub_shape_glow").show(ui, |ui| {
        let label = ui.label(tr("Radius:")).id;
        let name = tr("Radius");
        if let Some(r) = host.number(
            ui,
            Number {
                key: "glow_radius",
                label,
                name: &name,
                value: if v.has_glow { v.glow_radius } else { 0.0 },
                range: 0.0..=150.0,
                speed: 0.5,
                suffix: " pt",
                enabled: true,
            },
        ) {
            edits.push(Edit::GlowRadius(r));
        }
        ui.end_row();
        ui.label(tr("Color:"));
        host.color(ui, Slot::Glow, v.has_glow);
        ui.end_row();
        let label = ui.label(tr("Transparency:")).id;
        let name = tr("Transparency");
        if let Some(t) = host.number(
            ui,
            Number {
                key: "glow_transparency",
                label,
                name: &name,
                value: v.glow_transparency,
                range: 0.0..=100.0,
                speed: 1.0,
                suffix: "%",
                enabled: v.has_glow,
            },
        ) {
            edits.push(Edit::GlowTransparency(t));
        }
        ui.end_row();
    });
}

fn soft_edges(ui: &mut Ui, v: &Values, tr: &dyn Fn(&str) -> String, host: &mut dyn Host, edits: &mut Vec<Edit>) {
    rows("krub_shape_soft").show(ui, |ui| {
        let label = ui.label(tr("Radius:")).id;
        let name = tr("Radius");
        if let Some(r) = host.number(
            ui,
            Number { key: "soft_edge", label, name: &name, value: v.soft_edge, range: 0.0..=100.0, speed: 0.5, suffix: " pt", enabled: true },
        ) {
            edits.push(Edit::SoftEdge(r));
        }
        ui.end_row();
    });
}

fn size(ui: &mut Ui, v: &Values, caps: Caps, tr: &dyn Fn(&str) -> String, host: &mut dyn Host, edits: &mut Vec<Edit>) {
    rows("krub_shape_size").show(ui, |ui| {
        let max = v.size_max.max(0.1);
        let label = ui.label(tr("Width:")).id;
        let name = tr("Width");
        if let Some(w) = host.number(
            ui,
            Number { key: "width", label, name: &name, value: v.width, range: 0.01..=max, speed: 0.01, suffix: &v.size_suffix, enabled: true },
        ) {
            edits.push(Edit::Width(w));
        }
        ui.end_row();
        let label = ui.label(tr("Height:")).id;
        let name = tr("Height");
        if let Some(h) = host.number(
            ui,
            Number { key: "height", label, name: &name, value: v.height, range: 0.01..=max, speed: 0.01, suffix: &v.size_suffix, enabled: true },
        ) {
            edits.push(Edit::Height(h));
        }
        ui.end_row();
        ui.label("");
        let mut keep = v.keep_ratio;
        if ui.checkbox(&mut keep, tr("Keep ratio")).changed() {
            edits.push(Edit::KeepRatio(keep));
        }
        ui.end_row();
        if caps.rotation {
            let label = ui.label(tr("Rotation:")).id;
            let name = tr("Rotation");
            if let Some(r) = host.number(
                ui,
                Number { key: "rotation", label, name: &name, value: v.rotation, range: -360.0..=360.0, speed: 1.0, suffix: "°", enabled: true },
            ) {
                edits.push(Edit::Rotation(r));
            }
            ui.end_row();
        }
        if let Some(c) = v.corner {
            let label = ui.label(tr("Corner radius:")).id;
            let name = tr("Corner radius");
            let max = v.corner_max.max(0.0);
            if let Some(r) = host.number(
                ui,
                Number {
                    key: "corner",
                    label,
                    name: &name,
                    value: c.clamp(0.0, max),
                    range: 0.0..=max,
                    speed: 0.01,
                    suffix: &v.size_suffix,
                    enabled: true,
                },
            ) {
                edits.push(Edit::CornerRadius(r));
            }
            ui.end_row();
        }
    });
}

/// The width of one of `cols` equal columns across `ui`, `gap` apart.
fn col_w(ui: &Ui, cols: u8, gap: f32) -> f32 {
    let n = f32::from(cols.max(1));
    ((ui.available_width() - gap * (n - 1.0)) / n).floor().max(1.0)
}

/// Flip, Align and the stacking order, each its label over an even grid of buttons (owner,
/// 2026-11-22: pane buttons in even grids).
fn arrange(ui: &mut Ui, caps: Caps, tr: &dyn Fn(&str) -> String, host: &mut dyn Host, edits: &mut Vec<Edit>) {
    const GAP: f32 = 6.0;
    let grid = |ui: &mut Ui, add: &mut dyn FnMut(&mut Ui)| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(GAP, GAP);
            add(ui);
        });
    };
    let name = |ui: &mut Ui, s: String| {
        ui.label(egui::RichText::new(s).small().weak());
    };
    if caps.flip {
        name(ui, tr("Flip:"));
        let w = col_w(ui, 2, GAP);
        grid(ui, &mut |ui| {
            if host.button(ui, "flipVertical", &tr("Vertically"), &tr("Vertically"), w) {
                edits.push(Edit::FlipVertical);
            }
            if host.button(ui, "flipHorizontal", &tr("Horizontally"), &tr("Horizontally"), w) {
                edits.push(Edit::FlipHorizontal);
            }
        });
    }
    if caps.align {
        name(ui, tr("Align:"));
        let all = [
            ("alignLeft", "Left", "Align Objects Left", "left"),
            ("alignCenter", "Center", "Align Objects Center", "center"),
            ("alignRight", "Right", "Align Objects Right", "right"),
            ("alignTop", "Top", "Align Objects Top", "top"),
            ("alignMiddle", "Middle", "Align Objects Middle", "middle"),
            ("alignBottom", "Bottom", "Align Objects Bottom", "bottom"),
        ];
        let n = if caps.align_vertical { 6 } else { 3 };
        let w = col_w(ui, 3, GAP);
        grid(ui, &mut |ui| {
            for (icon, label, name, edge) in all.into_iter().take(n) {
                if host.button(ui, icon, &tr(label), &tr(name), w) {
                    edits.push(Edit::Align(edge));
                }
            }
        });
    }
    if caps.arrange {
        name(ui, tr("Arrange:"));
        let w = col_w(ui, 2, GAP);
        grid(ui, &mut |ui| {
            for (icon, label, op) in [
                ("bringToFront", "Bring to Front", "bringToFront"),
                ("bringForward", "Bring Forward", "bringForward"),
                ("sendBackward", "Send Backward", "sendBackward"),
                ("sendToBack", "Send to Back", "sendToBack"),
            ] {
                if host.button(ui, icon, &tr(label), &tr(label), w) {
                    edits.push(Edit::Order(op));
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashes_and_names_are_listed_in_order() {
        assert_eq!(Dash::ALL.map(Dash::name), ["Solid", "Dotted", "Dashed", "Long dash", "Dash dot"]);
        for name in
            ["Area", "Line", "Effect", "Position and Size", "Line style:", "Keep ratio", "Arrange:", "Fill and Line", "Effects", "Size and Position"]
        {
            assert!(NAMES.contains(&name), "{name}");
        }
        assert_eq!(ICONS.len(), 12);
    }

    #[test]
    fn effects_get_a_sub_panel_only_where_they_are_stored() {
        assert_eq!(Tab::shown(Caps::ALL), [Tab::FillLine, Tab::Effects, Tab::SizePosition]);
        assert_eq!(Tab::shown(Caps { effects: false, ..Caps::ALL }), [Tab::FillLine, Tab::SizePosition]);
        for t in Tab::shown(Caps::ALL) {
            assert!(NAMES.contains(&t.name()), "{:?}", t);
        }
    }
}
