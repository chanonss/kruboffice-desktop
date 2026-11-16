//! The object (shape, text box) pane, the same in every KrubOffice app (owner, 2026-11-16; after
//! LibreOffice's object Properties sidebar): folding sections Area, Line, Effect, and Position
//! and Size, in that order, with the same rows and the same names.
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
    fn button(&mut self, ui: &mut Ui, icon: &str, label: &str, name: &str) -> bool;
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
pub const NAMES: [&str; 44] = [
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

/// A section: its name as a heading that folds it, open at first.
fn section(ui: &mut Ui, title: String, add: impl FnOnce(&mut Ui)) {
    egui::CollapsingHeader::new(egui::RichText::new(title).strong()).id_salt("krub_shape_section").default_open(true).show(ui, add);
    ui.add_space(2.0);
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

/// The shared sections, in order. Returns this frame's edits.
pub fn sections(ui: &mut Ui, v: &Values, caps: Caps, tr: &dyn Fn(&str) -> String, host: &mut dyn Host) -> Vec<Edit> {
    let mut edits = Vec::new();
    if caps.area && !v.open {
        ui.push_id("krub_area", |ui| section(ui, tr("Area"), |ui| area(ui, v, tr, host, &mut edits)));
    }
    if caps.line {
        ui.push_id("krub_line", |ui| section(ui, tr("Line"), |ui| line(ui, v, caps, tr, host, &mut edits)));
    }
    if caps.effects {
        ui.push_id("krub_effect", |ui| section(ui, tr("Effect"), |ui| effect(ui, v, tr, host, &mut edits)));
    }
    ui.push_id("krub_position", |ui| section(ui, tr("Position and Size"), |ui| position_size(ui, v, caps, tr, host, &mut edits)));
    edits
}

fn area(ui: &mut Ui, v: &Values, tr: &dyn Fn(&str) -> String, host: &mut dyn Host, edits: &mut Vec<Edit>) {
    egui::Grid::new("krub_shape_area").num_columns(2).spacing(vec2(8.0, 6.0)).show(ui, |ui| {
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
    egui::Grid::new("krub_shape_line").num_columns(2).spacing(vec2(8.0, 6.0)).show(ui, |ui| {
        let style_label = ui.label(tr("Line style:")).id;
        let shown = if v.has_line { tr(v.dash.name()) } else { tr("None") };
        // A fixed list of six that never changes while open, so egui's combo box is fine here
        // (the apps' own wrappers can't be reached from this shared crate).
        #[allow(clippy::disallowed_types)]
        let combo = egui::ComboBox::from_id_salt("krub_shape_line_style").width(150.0).selected_text(shown).show_ui(ui, |ui| {
            if ui.selectable_label(!v.has_line, tr("None")).clicked() && v.has_line {
                edits.push(Edit::LineNone);
            }
            for d in Dash::ALL {
                if ui.selectable_label(v.has_line && v.dash == d, tr(d.name())).clicked() {
                    edits.push(Edit::LineDash(d));
                }
            }
        });
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

fn effect(ui: &mut Ui, v: &Values, tr: &dyn Fn(&str) -> String, host: &mut dyn Host, edits: &mut Vec<Edit>) {
    host.shadow(ui);
    ui.add_space(4.0);
    ui.label(egui::RichText::new(tr("Glow")).strong());
    egui::Grid::new("krub_shape_glow").num_columns(2).spacing(vec2(8.0, 6.0)).show(ui, |ui| {
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
    ui.add_space(4.0);
    ui.label(egui::RichText::new(tr("Soft Edges")).strong());
    egui::Grid::new("krub_shape_soft").num_columns(2).spacing(vec2(8.0, 6.0)).show(ui, |ui| {
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

fn position_size(ui: &mut Ui, v: &Values, caps: Caps, tr: &dyn Fn(&str) -> String, host: &mut dyn Host, edits: &mut Vec<Edit>) {
    egui::Grid::new("krub_shape_size").num_columns(2).spacing(vec2(8.0, 6.0)).show(ui, |ui| {
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
    });
    if caps.flip {
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Flip:"));
            if host.button(ui, "flipVertical", &tr("Vertically"), &tr("Vertically")) {
                edits.push(Edit::FlipVertical);
            }
            if host.button(ui, "flipHorizontal", &tr("Horizontally"), &tr("Horizontally")) {
                edits.push(Edit::FlipHorizontal);
            }
        });
    }
    if caps.align {
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Align:"));
            let all = [
                ("alignLeft", "Left", "Align Objects Left", "left"),
                ("alignCenter", "Center", "Align Objects Center", "center"),
                ("alignRight", "Right", "Align Objects Right", "right"),
                ("alignTop", "Top", "Align Objects Top", "top"),
                ("alignMiddle", "Middle", "Align Objects Middle", "middle"),
                ("alignBottom", "Bottom", "Align Objects Bottom", "bottom"),
            ];
            let n = if caps.align_vertical { 6 } else { 3 };
            for (icon, label, name, edge) in all.into_iter().take(n) {
                if host.button(ui, icon, &tr(label), &tr(name)) {
                    edits.push(Edit::Align(edge));
                }
            }
        });
    }
    if caps.arrange {
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Arrange:"));
            for (icon, label, op) in [
                ("bringToFront", "Bring to Front", "bringToFront"),
                ("bringForward", "Bring Forward", "bringForward"),
                ("sendBackward", "Send Backward", "sendBackward"),
                ("sendToBack", "Send to Back", "sendToBack"),
            ] {
                if host.button(ui, icon, &tr(label), &tr(label)) {
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
        for name in ["Area", "Line", "Effect", "Position and Size", "Line style:", "Keep ratio", "Arrange:"] {
            assert!(NAMES.contains(&name), "{name}");
        }
        assert_eq!(ICONS.len(), 12);
    }
}
