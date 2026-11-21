use super::arrange::arrange_group;
use super::insert::chart_menu_types;
use super::{Icon, SlideApp, Ui, big, big_button, json};

pub(super) fn chart_design(app: &mut SlideApp, ui: &mut Ui) {
    crate::widgets::group_named(ui, "Chart Type and Data", |ui| {
        let r = big_button(ui, Icon::ChangeChartType, tl!("Change\nChart Type"), true);
        egui::Popup::menu(&r).show(|ui| {
            for k in chart_menu_types() {
                let Some(k) = k else {
                    ui.separator();
                    continue;
                };
                let current = app.session.active().and_then(|d| {
                    d.selected_shapes().into_iter().find_map(|s| match &s.kind {
                        deckcraft_model::ShapeKind::Chart(c) => Some(c.kind),
                        _ => None,
                    })
                });
                let icon = krub_icons::chart_icons::id_for(&format!("{k:?}"), "");
                if krub_icons::chart_icons::item(ui, icon, tl!(k.label()), current == Some(k), 190.0).clicked() {
                    // Replace the chart's type keeping its data.
                    if let Some(id) = app.session.active().and_then(|d| d.selection.shapes.first().copied()) {
                        let _ = app.run("chart.type", json!({"id": id.0, "type": format!("{k:?}")}));
                    }
                }
            }
        });
        big(app, ui, Icon::EditData, tl!("Edit\nData"), "app.dialog", json!({"id": "chartData"}));
    });
    arrange_group(app, ui);
}
