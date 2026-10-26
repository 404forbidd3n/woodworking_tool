//! The bar across the top of the window: app name, open file, the two
//! planner buttons in the middle, and Undo / Redo / Load / Save on the right.

use bevy::prelude::*;
use bevy_egui::egui;

use super::files::{begin_dialog, perform_history, save_layout};
use crate::builder::{self, ComponentWindow};
use crate::cutting::{self, CutJob, CutWindow};
use crate::layout::history::History;
use crate::layout::model::{resolve_layout_path, Editor, Furniture, Room, Selection};
use crate::layout::project::{DialogRequest, NativeDialog};
use crate::theme;

/// The top bar. See the top of this file.
pub(super) fn draw_toolbar(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    room: &mut Room,
    editor: &mut Editor,
    selection: &mut Selection,
    pieces: &Query<(Entity, &mut Furniture, &mut Transform)>,
    cut_job: &mut CutJob,
    cut_window: &mut CutWindow,
    component_window: &mut ComponentWindow,
    history: &mut History,
    dialogs: &mut NativeDialog,
) {
    let cream = theme::ink();
    let muted = theme::muted();
    let error = editor.status.starts_with("Could") || editor.status.starts_with("No file");
    ui.horizontal(|ui| {
        ui.set_min_height(48.0);
        let origin = ui.cursor().min;
        let full_width = ui.available_width();
        ui.set_min_width(full_width);
        ui.label(
            egui::RichText::new("Woodworking")
                .strong()
                .size(16.0)
                .color(theme::heading()),
        );
        ui.add_space(6.0);
        egui::Frame::new()
            .fill(theme::chip())
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(10, 4))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(layout_label(&editor.path))
                        .strong()
                        .color(cream),
                );
            });
        let planner = |label: &str| {
            egui::Button::new(egui::RichText::new(label).size(16.0).strong())
                .min_size(egui::vec2(210.0, 40.0))
        };
        let gap = 12.0;
        let button_size = egui::vec2(210.0, 40.0);
        let pair = button_size.x * 2.0 + gap;
        let x = origin.x + (full_width - pair) * 0.5;
        let y = origin.y + 4.0;
        if ui
            .put(
                egui::Rect::from_min_size(egui::pos2(x, y), button_size),
                planner("Cut planner"),
            )
            .clicked()
        {
            cutting::request_open(cut_window);
        }
        if ui
            .put(
                egui::Rect::from_min_size(egui::pos2(x + button_size.x + gap, y), button_size),
                planner("Component planner"),
            )
            .clicked()
        {
            builder::request_open(component_window);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(history.can_redo(), egui::Button::new("Redo"))
                .clicked()
            {
                perform_history(true, history, commands, room, editor, selection, pieces);
            }
            if ui
                .add_enabled(history.can_undo(), egui::Button::new("Undo"))
                .clicked()
            {
                perform_history(false, history, commands, room, editor, selection, pieces);
            }
            if ui.button("Load").clicked() {
                begin_dialog(dialogs, editor, DialogRequest::Open);
            }
            if ui.button("Save").clicked() {
                save_layout(editor, room, pieces, cut_job);
            }
            if ui.button("Save As").clicked() {
                begin_dialog(dialogs, editor, DialogRequest::Save);
            }
            if !editor.status.is_empty() {
                let color = if error {
                    egui::Color32::from_rgb(255, 186, 150)
                } else {
                    muted
                };
                ui.label(egui::RichText::new(editor.status.as_str()).color(color));
            }
        });
    });
}

/// The name shown in the toolbar. A file inside the project folder is shown
/// without the long folder path.
pub(super) fn layout_label(path: &str) -> String {
    let resolved = resolve_layout_path(path);
    resolved
        .strip_prefix(crate::layout::model::project_dir())
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| resolved.display().to_string())
}
