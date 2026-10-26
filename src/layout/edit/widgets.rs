//! Small egui widgets shared by the sidebar and the Properties panel.

use bevy_egui::egui;

/// Returns true when the number was edited. The value is already clamped.
pub(super) fn mm_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) -> bool {
    ui.horizontal(|ui| {
        ui.add_sized([92.0, 18.0], egui::Label::new(label));
        ui.add(
            egui::DragValue::new(value)
                .speed(5.0)
                .range(range)
                .suffix(" mm"),
        )
    })
    .inner
    .changed()
}
