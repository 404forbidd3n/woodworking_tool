//! Controls for the block being built. The 3D view is the rest of the window.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

use crate::layout::model::CustomRole;
use crate::theme;

use super::{BuilderChrome, ComponentPlan, ComponentPlannerCamera, CustomLibrary};

pub fn draw(
    mut contexts: Query<&mut EguiContext, With<ComponentPlannerCamera>>,
    mut plan: ResMut<ComponentPlan>,
    mut library: ResMut<CustomLibrary>,
    mut chrome: ResMut<BuilderChrome>,
) {
    let Ok(mut context) = contexts.single_mut() else {
        return;
    };
    let ctx = context.get_mut();
    theme::apply_theme(ctx);

    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "component_viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    let toolbar = egui::Panel::top("component_toolbar")
        .frame(theme::toolbar_frame())
        .show(&mut viewport_ui, |ui| {
            draw_toolbar(ui, &mut plan, &mut library)
        });
    chrome.toolbar = toolbar.response.rect.height();

    let side_max = (ctx.viewport_rect().width() * 0.42).clamp(320.0, 460.0);
    let side = egui::Panel::left("component_parts")
        .resizable(true)
        .default_size(360.0)
        .max_size(side_max)
        .frame(theme::sidebar_frame())
        .show(&mut viewport_ui, |ui| draw_sidebar(ui, &mut plan, &library));
    chrome.sidebar = side.response.rect.right();
}

fn draw_toolbar(ui: &mut egui::Ui, plan: &mut ComponentPlan, library: &mut CustomLibrary) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Component planner")
                .strong()
                .size(16.0)
                .color(theme::heading()),
        );
        if ui.button("Save type").clicked() {
            library.save_plan(plan);
        }
        if ui.button("New block").clicked() {
            plan.name = "New block".into();
            plan.boards.clear();
            plan.selected = None;
            plan.mesh_dirty = true;
            plan.status = "Started an empty block.".into();
        }
        if !plan.status.is_empty() {
            ui.label(egui::RichText::new(plan.status.as_str()).color(theme::muted()));
        }
    });
}

fn draw_sidebar(ui: &mut egui::Ui, plan: &mut ComponentPlan, library: &CustomLibrary) {
    egui::ScrollArea::vertical()
        .id_salt("component_sidebar")
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(
                    "Drag a board to move it. Shift-drag raises it. Arrow keys nudge it. Yaw turns it, and pitch or roll tilts it for a roof. R adds 90° of yaw. Drag empty space to look around.",
                )
                .color(theme::muted()),
            );
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Name").color(theme::muted()));
            ui.text_edit_singleline(&mut plan.name);
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Add a board")
                    .strong()
                    .color(theme::heading()),
            );
            ui.horizontal_wrapped(|ui| {
                for role in CustomRole::ALL {
                    if ui.button(role.label()).clicked() {
                        let mut board = role.default_board();
                        let count = plan.boards.iter().filter(|saved| saved.role == role).count();
                        board.x_mm += count as f32 * 40.0;
                        plan.boards.push(board);
                        plan.selected = Some(plan.boards.len() - 1);
                        plan.mesh_dirty = true;
                    }
                }
            });
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Saved types")
                    .strong()
                    .color(theme::heading()),
            );
            if library.types.is_empty() {
                ui.label(
                    egui::RichText::new("Nothing saved yet.")
                        .color(theme::muted()),
                );
            } else {
                let mut load = None;
                for block in &library.types {
                    if ui
                        .selectable_label(plan.name == block.name, block.name.as_str())
                        .clicked()
                    {
                        load = Some(block.clone());
                    }
                }
                if let Some(block) = load {
                    plan.load_type(&block);
                }
            }

            let Some(index) = plan.selected.filter(|index| *index < plan.boards.len()) else {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Select a board, or add one.")
                        .color(theme::muted()),
                );
                return;
            };
            ui.separator();
            ui.label(
                egui::RichText::new(plan.boards[index].role.label())
                    .strong()
                    .color(theme::heading()),
            );
            ui.horizontal_wrapped(|ui| {
                for role in CustomRole::ALL {
                    if ui
                        .selectable_label(plan.boards[index].role == role, role.label())
                        .clicked()
                        && plan.boards[index].role != role
                    {
                        plan.boards[index].role = role;
                        plan.mesh_dirty = true;
                    }
                }
            });
            let board = &mut plan.boards[index];
            if deg_field(ui, "Yaw", &mut board.yaw_deg)
                | deg_field(ui, "Pitch", &mut board.pitch_deg)
                | deg_field(ui, "Roll", &mut board.roll_deg)
                | mm_field(ui, "Width", &mut board.width_mm)
                | mm_field(ui, "Height", &mut board.height_mm)
                | mm_field(ui, "Depth", &mut board.depth_mm)
                | mm_field(ui, "X", &mut board.x_mm)
                | mm_field(ui, "Y", &mut board.y_mm)
                | mm_field(ui, "Z", &mut board.z_mm)
            {
                board.rest_on_floor();
                plan.mesh_dirty = true;
            }
            if ui.button("Remove board").clicked() {
                plan.boards.remove(index);
                plan.selected = None;
                plan.mesh_dirty = true;
            }
        });
}

fn deg_field(ui: &mut egui::Ui, label: &str, value: &mut f32) -> bool {
    ui.horizontal(|ui| {
        ui.add_sized([52.0, 18.0], egui::Label::new(label));
        ui.add(
            egui::DragValue::new(value)
                .range(-180.0..=180.0)
                .speed(0.5)
                .suffix("°"),
        )
        .changed()
    })
    .inner
}

fn mm_field(ui: &mut egui::Ui, label: &str, value: &mut f32) -> bool {
    ui.horizontal(|ui| {
        ui.add_sized([52.0, 18.0], egui::Label::new(label));
        ui.add(
            egui::DragValue::new(value)
                .range(-10000.0..=4000.0)
                .speed(1.0)
                .suffix(" mm"),
        )
        .changed()
    })
    .inner
}
