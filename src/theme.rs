//! Shared look for every window.
//!
//! The layout window and the cut planner both call [`apply_theme`], so a button
//! or a list in one place matches the same control in the other.

use bevy_egui::egui;

pub fn ink() -> egui::Color32 {
    egui::Color32::from_rgb(62, 48, 38)
}

pub fn paper() -> egui::Color32 {
    egui::Color32::from_rgb(246, 241, 232)
}

pub fn chip() -> egui::Color32 {
    egui::Color32::from_rgb(232, 220, 202)
}

fn line() -> egui::Color32 {
    egui::Color32::from_rgb(196, 176, 150)
}

fn copper() -> egui::Color32 {
    egui::Color32::from_rgb(158, 90, 42)
}

pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = paper();
    visuals.window_fill = paper();
    visuals.extreme_bg_color = egui::Color32::from_rgb(255, 252, 247);
    visuals.faint_bg_color = chip();
    visuals.code_bg_color = chip();
    visuals.collapsing_header_frame = true;
    visuals.indent_has_left_vline = false;
    visuals.slider_trailing_fill = true;
    visuals.selection.bg_fill = egui::Color32::from_rgb(214, 176, 132);
    visuals.selection.stroke = egui::Stroke::new(1.0, ink());
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, ink());
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, line());
    visuals.widgets.noninteractive.weak_bg_fill = paper();
    visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(255, 250, 244);
    visuals.widgets.inactive.bg_fill = chip();
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, ink());
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, line());
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(255, 244, 230);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(236, 210, 176);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ink());
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, copper());
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(232, 196, 154);
    visuals.widgets.active.bg_fill = copper();
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, copper());
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.open.weak_bg_fill = chip();
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, ink());
    visuals.widgets.open.corner_radius = egui::CornerRadius::same(6);
    // Both themes, so a system dark-mode setting does not put light text on this paper.
    ctx.set_visuals_of(egui::Theme::Light, visuals.clone());
    ctx.set_visuals_of(egui::Theme::Dark, visuals);
    ctx.set_theme(egui::Theme::Light);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.interact_size.y = 28.0;
        style.spacing.indent = 16.0;
    });
}

pub fn toolbar_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(74, 56, 42))
        .inner_margin(egui::Margin::symmetric(14, 8))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(48, 36, 28)))
}

pub fn sidebar_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(paper())
        .inner_margin(egui::Margin::symmetric(10, 8))
        .stroke(egui::Stroke::new(1.0, line()))
}

/// A sidebar section that opens and closes. `open` is only the first-run state.
pub fn dropdown(
    ui: &mut egui::Ui,
    title: &str,
    open: bool,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    ui.add_space(6.0);
    egui::CollapsingHeader::new(
        egui::RichText::new(title)
            .strong()
            .size(15.0)
            .color(egui::Color32::BLACK),
    )
    .default_open(open)
    .show_background(true)
    .show(ui, |ui| {
        ui.add_space(2.0);
        add_contents(ui);
    });
}
