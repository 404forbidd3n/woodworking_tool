//! Sketch window for a custom furniture block.
//!
//! The left side lists the boards. The right side draws a front view from
//! those boards. The drawing is a picture of the idea, not a built cabinet.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

use crate::theme;

use super::{BlockSketch, BoardRole, ComponentPlan, ComponentPlannerCamera, SketchBoard};

pub fn draw(
    mut contexts: Query<&mut EguiContext, With<ComponentPlannerCamera>>,
    mut plan: ResMut<ComponentPlan>,
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
    egui::Panel::top("component_toolbar")
        .frame(theme::toolbar_frame())
        .show(&mut viewport_ui, |ui| draw_toolbar(ui));

    let side_max = (ctx.viewport_rect().width() * 0.42).clamp(300.0, 460.0);
    egui::Panel::left("component_parts")
        .resizable(true)
        .default_size(340.0)
        .max_size(side_max)
        .frame(theme::sidebar_frame())
        .show(&mut viewport_ui, |ui| draw_sidebar(ui, &mut plan));

    egui::CentralPanel::default()
        .frame(theme::sidebar_frame())
        .show(&mut viewport_ui, |ui| {
            let block = plan.selected_mut().clone();
            draw_canvas(ui, &block);
        });
}

fn draw_toolbar(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Component planner")
                .strong()
                .size(16.0)
                .color(theme::heading()),
        );
        ui.label(
            egui::RichText::new(
                "Sketch only. These blocks are not saved and are not placed in the room yet.",
            )
            .color(theme::muted()),
        );
    });
}

fn draw_sidebar(ui: &mut egui::Ui, plan: &mut ComponentPlan) {
    egui::ScrollArea::vertical()
        .id_salt("component_sidebar")
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("Blocks")
                    .strong()
                    .color(theme::heading()),
            );
            let mut select = None;
            for (index, block) in plan.blocks.iter().enumerate() {
                if ui
                    .selectable_label(plan.selected == index, block.name.as_str())
                    .clicked()
                {
                    select = Some(index);
                }
            }
            if let Some(index) = select {
                plan.selected = index;
            }
            if ui.button("New block").clicked() {
                plan.add_block();
            }
            ui.separator();

            let selected = plan.selected.min(plan.blocks.len().saturating_sub(1));
            plan.selected = selected;
            let Some(block) = plan.blocks.get_mut(selected) else {
                return;
            };
            ui.label(egui::RichText::new("Name").color(theme::muted()));
            ui.text_edit_singleline(&mut block.name);
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Add a board")
                    .strong()
                    .color(theme::heading()),
            );
            ui.horizontal_wrapped(|ui| {
                for role in [
                    BoardRole::Wall,
                    BoardRole::Door,
                    BoardRole::Bottom,
                    BoardRole::Top,
                    BoardRole::Back,
                ] {
                    if ui
                        .button(format!("Add {}", role.label().to_lowercase()))
                        .clicked()
                    {
                        block.boards.push(SketchBoard::new(role));
                    }
                }
            });
            ui.add_space(8.0);
            let mut remove = None;
            for (index, board) in block.boards.iter_mut().enumerate() {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(board.role.label())
                            .strong()
                            .color(theme::heading()),
                    );
                    if ui.button("Remove").clicked() {
                        remove = Some(index);
                    }
                });
                mm_field(ui, "Width", &mut board.width_mm);
                mm_field(ui, "Height", &mut board.height_mm);
            }
            if let Some(index) = remove {
                block.boards.remove(index);
            }
            if block.boards.is_empty() {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Add a wall, a door, or a bottom to start this block.")
                        .color(theme::muted()),
                );
            }
        });
}

fn mm_field(ui: &mut egui::Ui, label: &str, value: &mut f32) {
    ui.horizontal(|ui| {
        ui.add_sized([64.0, 18.0], egui::Label::new(label));
        ui.add(
            egui::DragValue::new(value)
                .range(1.0..=4000.0)
                .suffix(" mm"),
        );
    });
}

fn draw_canvas(ui: &mut egui::Ui, block: &BlockSketch) {
    ui.label(
        egui::RichText::new(block.name.as_str())
            .strong()
            .size(18.0)
            .color(theme::heading()),
    );
    ui.label(
        egui::RichText::new("Front sketch. Walls stand at the sides, bottoms and tops close the box, and doors fill the opening.")
            .color(theme::muted()),
    );
    let available = ui.available_rect_before_wrap();
    let (rect, _) = ui.allocate_exact_size(available.size(), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(24, 24, 26));
    if block.boards.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Empty block",
            egui::FontId::proportional(18.0),
            theme::muted(),
        );
        return;
    }
    paint_block(&painter, rect.shrink(36.0), block);
}

fn paint_block(painter: &egui::Painter, bounds: egui::Rect, block: &BlockSketch) {
    let walls: Vec<_> = block
        .boards
        .iter()
        .filter(|board| board.role == BoardRole::Wall)
        .collect();
    let doors: Vec<_> = block
        .boards
        .iter()
        .filter(|board| board.role == BoardRole::Door)
        .collect();
    let bottoms: Vec<_> = block
        .boards
        .iter()
        .filter(|board| board.role == BoardRole::Bottom)
        .collect();
    let tops: Vec<_> = block
        .boards
        .iter()
        .filter(|board| board.role == BoardRole::Top)
        .collect();
    let backs: Vec<_> = block
        .boards
        .iter()
        .filter(|board| board.role == BoardRole::Back)
        .collect();

    let span = doors
        .iter()
        .chain(bottoms.iter())
        .chain(tops.iter())
        .chain(backs.iter())
        .map(|board| board.width_mm)
        .fold(560.0_f32, f32::max);
    let rise = walls
        .iter()
        .chain(doors.iter())
        .chain(backs.iter())
        .map(|board| board.height_mm)
        .fold(720.0_f32, f32::max);
    let aspect = (span / rise).clamp(0.45, 1.8);
    let mut width = bounds.width().min(bounds.height() * aspect);
    let mut height = width / aspect;
    if height > bounds.height() {
        height = bounds.height();
        width = height * aspect;
    }
    let outer = egui::Rect::from_center_size(bounds.center(), egui::vec2(width, height));
    let wood = egui::Color32::from_rgb(196, 154, 96);
    let wood_dark = egui::Color32::from_rgb(132, 88, 46);
    let door_fill = egui::Color32::from_rgb(232, 214, 184);
    let back_fill = egui::Color32::from_rgb(72, 64, 52);

    if !backs.is_empty() {
        painter.rect_filled(outer.shrink(10.0), 2.0, back_fill);
        painter.text(
            outer.center(),
            egui::Align2::CENTER_CENTER,
            format!("Back × {}", backs.len()),
            egui::FontId::proportional(14.0),
            egui::Color32::from_rgb(230, 220, 200),
        );
    }

    let rail = (height * 0.06).clamp(10.0, 28.0);
    let stile = (width * 0.07).clamp(12.0, 36.0);
    if !bottoms.is_empty() {
        let strip = egui::Rect::from_min_max(
            egui::pos2(outer.left(), outer.bottom() - rail),
            outer.right_bottom(),
        );
        painter.rect_filled(strip, 0.0, wood);
        label_board(painter, strip, "Bottom", &bottoms);
    }
    if !tops.is_empty() {
        let strip = egui::Rect::from_min_max(
            outer.left_top(),
            egui::pos2(outer.right(), outer.top() + rail),
        );
        painter.rect_filled(strip, 0.0, wood);
        label_board(painter, strip, "Top", &tops);
    }
    if !walls.is_empty() {
        let left = egui::Rect::from_min_max(
            outer.left_top(),
            egui::pos2(outer.left() + stile, outer.bottom()),
        );
        painter.rect_filled(left, 0.0, wood_dark);
        label_board(painter, left, "Wall", &walls);
        if walls.len() > 1 {
            let right = egui::Rect::from_min_max(
                egui::pos2(outer.right() - stile, outer.top()),
                outer.right_bottom(),
            );
            painter.rect_filled(right, 0.0, wood_dark);
        }
        for extra in 2..walls.len() {
            let t = extra as f32 / walls.len() as f32;
            let x = egui::lerp(outer.x_range(), t);
            painter.vline(x, outer.y_range(), egui::Stroke::new(3.0, wood_dark));
        }
    }
    if !doors.is_empty() {
        let opening = outer.shrink2(egui::vec2(
            if walls.is_empty() { 8.0 } else { stile + 4.0 },
            if bottoms.is_empty() && tops.is_empty() {
                8.0
            } else {
                rail + 4.0
            },
        ));
        let count = doors.len() as f32;
        for (index, door) in doors.iter().enumerate() {
            let x0 = egui::lerp(opening.x_range(), index as f32 / count);
            let x1 = egui::lerp(opening.x_range(), (index as f32 + 1.0) / count);
            let leaf = egui::Rect::from_min_max(
                egui::pos2(x0 + 3.0, opening.top()),
                egui::pos2(x1 - 3.0, opening.bottom()),
            );
            painter.rect_filled(leaf, 2.0, door_fill);
            painter.rect_stroke(
                leaf,
                2.0,
                egui::Stroke::new(1.5, wood_dark),
                egui::StrokeKind::Inside,
            );
            painter.text(
                leaf.center(),
                egui::Align2::CENTER_CENTER,
                format!("Door\n{:.0} × {:.0}", door.width_mm, door.height_mm),
                egui::FontId::proportional(13.0),
                egui::Color32::from_rgb(40, 32, 24),
            );
        }
    }
    painter.rect_stroke(
        outer,
        2.0,
        egui::Stroke::new(2.0, egui::Color32::from_rgb(230, 220, 200)),
        egui::StrokeKind::Outside,
    );
}

fn label_board(painter: &egui::Painter, rect: egui::Rect, title: &str, boards: &[&SketchBoard]) {
    let size = boards
        .first()
        .map(|board| format!("\n{:.0} × {:.0}", board.width_mm, board.height_mm))
        .unwrap_or_default();
    let text = if boards.len() > 1 {
        format!("{title} × {}{size}", boards.len())
    } else {
        format!("{title}{size}")
    };
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(28, 22, 16),
    );
}
