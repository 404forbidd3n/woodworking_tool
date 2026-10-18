//! Shared look for every window.
//!
//! The layout window and the cut planner both call [`apply_theme`]. The 3D
//! room is not recolored here; only the panels, toolbar, and controls are.

use bevy_egui::egui;

/// Primary text on the dark panels.
pub fn ink() -> egui::Color32 {
    egui::Color32::from_rgb(242, 242, 247)
}

/// Secondary text, still readable on graphite.
pub fn muted() -> egui::Color32 {
    egui::Color32::from_rgb(174, 174, 178)
}

pub fn paper() -> egui::Color32 {
    egui::Color32::from_rgb(44, 44, 46)
}

pub fn chip() -> egui::Color32 {
    egui::Color32::from_rgb(58, 58, 60)
}

fn chrome() -> egui::Color32 {
    egui::Color32::from_rgb(28, 28, 30)
}

fn line() -> egui::Color32 {
    egui::Color32::from_rgb(72, 72, 74)
}

fn accent() -> egui::Color32 {
    egui::Color32::from_rgb(10, 132, 255)
}

pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = paper();
    visuals.window_fill = paper();
    visuals.extreme_bg_color = chrome();
    visuals.faint_bg_color = chip();
    visuals.code_bg_color = chip();
    visuals.weak_text_color = Some(muted());
    visuals.collapsing_header_frame = true;
    visuals.indent_has_left_vline = false;
    visuals.slider_trailing_fill = true;
    visuals.selection.bg_fill = accent();
    visuals.selection.stroke = egui::Stroke::new(1.0, ink());
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, ink());
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, line());
    visuals.widgets.noninteractive.weak_bg_fill = paper();
    visuals.widgets.inactive.weak_bg_fill = chip();
    visuals.widgets.inactive.bg_fill = chip();
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, ink());
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, line());
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(72, 72, 74);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(72, 72, 74);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ink());
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, accent());
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.active.weak_bg_fill = accent();
    visuals.widgets.active.bg_fill = accent();
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, accent());
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.open.weak_bg_fill = chip();
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, ink());
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, line());
    visuals.widgets.open.corner_radius = egui::CornerRadius::same(6);
    // Both themes, so a system appearance setting cannot put dark text on these panels.
    ctx.set_visuals_of(egui::Theme::Light, visuals.clone());
    ctx.set_visuals_of(egui::Theme::Dark, visuals);
    ctx.set_theme(egui::Theme::Dark);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.interact_size.y = 28.0;
        style.spacing.indent = 16.0;
    });
}

pub fn toolbar_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(chrome())
        .inner_margin(egui::Margin::symmetric(14, 8))
        .stroke(egui::Stroke::new(1.0, line()))
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
    egui::CollapsingHeader::new(egui::RichText::new(title).strong().size(15.0).color(ink()))
        .default_open(open)
        .show_background(true)
        .show(ui, |ui| {
            ui.add_space(2.0);
            add_contents(ui);
        });
}
