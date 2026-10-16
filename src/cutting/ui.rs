//! The cut planner window.
//!
//! Left side: the supplied sheet, then the list of rectangles to cut.
//! Right side: every sheet the estimate needs, with the parts drawn in color
//! and the sheet size in large type.
//!
//! The window is a real second window, so dragging it does not drag the 3D room.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

use crate::layout::model::Furniture;
use crate::theme::{self, dropdown};

use super::from_room::parts_from_furniture;
use super::job::{CutJob, CutPart, PartRole};
use super::pack::{self, PlacedPart};
use super::CutPlannerCamera;

const PART_COLORS: [[u8; 3]; 8] = [
    [196, 122, 58],
    [62, 122, 168],
    [78, 140, 96],
    [168, 84, 84],
    [128, 96, 168],
    [52, 140, 140],
    [176, 132, 48],
    [120, 86, 62],
];

pub fn draw(
    mut contexts: Query<&mut EguiContext, With<CutPlannerCamera>>,
    mut job: ResMut<CutJob>,
    furniture: Query<&Furniture>,
) {
    let Ok(mut context) = contexts.single_mut() else {
        return;
    };
    let ctx = context.get_mut();
    theme::apply_theme(ctx);

    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "cut_viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    egui::Panel::top("cut_toolbar")
        .frame(theme::toolbar_frame())
        .show(&mut viewport_ui, |ui| draw_toolbar(ui, &mut job));

    egui::Panel::left("cut_parts")
        .resizable(true)
        .default_size(340.0)
        .frame(theme::sidebar_frame())
        .show(&mut viewport_ui, |ui| {
            draw_sidebar(ui, &mut job, &furniture)
        });

    egui::CentralPanel::default()
        .frame(theme::sidebar_frame())
        .show(&mut viewport_ui, |ui| draw_canvas(ui, &job));
}

fn draw_toolbar(ui: &mut egui::Ui, job: &mut CutJob) {
    let cream = egui::Color32::from_rgb(246, 236, 220);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Cut planner")
                .strong()
                .size(16.0)
                .color(cream),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_sized([168.0, 32.0], egui::Button::new("Estimate"))
                .clicked()
            {
                job.estimate = Some(pack::estimate(&job.board, &job.parts));
            }
            if let Some(estimate) = &job.estimate {
                let waste = (estimate.waste_ratio() * 100.0).round();
                let boards = estimate.boards_to_order();
                let noun = if boards == 1 { "board" } else { "boards" };
                let text = if estimate.stale {
                    format!("Out of date · last result {boards} {noun}")
                } else {
                    format!("Order {boards} {noun} · {waste:.0}% offcut")
                };
                ui.label(egui::RichText::new(text).color(cream).strong());
            }
        });
    });
}

fn draw_sidebar(ui: &mut egui::Ui, job: &mut CutJob, furniture: &Query<&Furniture>) {
    egui::ScrollArea::vertical()
        .id_salt("cut_sidebar")
        .show(ui, |ui| {
            dropdown(ui, "Supplied board", true, |ui| {
                ui.weak("The full sheet, before any cuts.");
                let board = &mut job.board;
                let mut changed = mm_row(ui, "Width", &mut board.width_mm, 100.0..=4000.0);
                changed |= mm_row(ui, "Height", &mut board.height_mm, 100.0..=4000.0);
                changed |= mm_row(ui, "Thickness", &mut board.thickness_mm, 3.0..=60.0);
                changed |= mm_row(ui, "Saw kerf", &mut board.kerf_mm, 0.0..=10.0);
                changed |= mm_row(ui, "Edge trim", &mut board.trim_mm, 0.0..=50.0);
                changed |= ui
                    .checkbox(&mut board.allow_rotate, "Parts may turn 90°")
                    .changed();
                if changed {
                    job.mark_stale();
                }
                ui.weak("Kerf is the saw blade. Trim is left clear on each edge.");
            });

            dropdown(ui, "Parts to cut", true, |ui| {
                draw_draft(ui, job);
                ui.separator();
                if job.parts.is_empty() {
                    ui.label("Nothing to cut yet.");
                }
                let mut remove = None;
                for (index, part) in job.parts.iter().enumerate() {
                    if part_row(ui, index, part) {
                        remove = Some(index);
                    }
                }
                if let Some(index) = remove {
                    job.parts.remove(index);
                    job.mark_stale();
                }
                ui.add_space(6.0);
                if ui.button("Add parts from the room").clicked() {
                    for piece in furniture.iter() {
                        job.parts.extend(parts_from_furniture(piece));
                    }
                    job.mark_stale();
                }
                if ui.button("Clear list").clicked() {
                    job.parts.clear();
                    job.mark_stale();
                }
            });
        });
}

fn draw_draft(ui: &mut egui::Ui, job: &mut CutJob) {
    ui.horizontal(|ui| {
        ui.label("Kind");
        egui::ComboBox::from_id_salt("draft_role")
            .selected_text(job.draft_role.label())
            .width(118.0)
            .show_ui(ui, |ui| {
                for role in PartRole::ALL {
                    ui.selectable_value(&mut job.draft_role, role, role.label());
                }
            });
        ui.label("Name");
        ui.add(egui::TextEdit::singleline(&mut job.draft_name).desired_width(ui.available_width()));
    });
    mm_row(ui, "Width", &mut job.draft_width_mm, 10.0..=4000.0);
    mm_row(ui, "Height", &mut job.draft_height_mm, 10.0..=4000.0);
    ui.horizontal(|ui| {
        ui.add_sized([92.0, 18.0], egui::Label::new("Quantity"));
        ui.add(egui::DragValue::new(&mut job.draft_quantity).range(1..=100));
    });
    if ui.button("Add to list").clicked() {
        job.add_draft();
    }
}

fn part_row(ui: &mut egui::Ui, index: usize, part: &CutPart) -> bool {
    let color = part_color(index);
    let mut remove = false;
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 3.0, color);
        ui.label(
            egui::RichText::new(format!(
                "{} × {}   {:.0} × {:.0}",
                part.quantity,
                part.titled(),
                part.width_mm,
                part.height_mm
            ))
            .color(egui::Color32::BLACK),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Remove").clicked() {
                remove = true;
            }
        });
    });
    remove
}

fn draw_canvas(ui: &mut egui::Ui, job: &CutJob) {
    let available = ui.available_size();
    let Some(estimate) = &job.estimate else {
        ui.allocate_space(available);
        ui.painter().text(
            ui.max_rect().center(),
            egui::Align2::CENTER_CENTER,
            "Add the parts, then press Estimate.",
            egui::FontId::new(18.0, egui::FontFamily::Proportional),
            egui::Color32::BLACK,
        );
        return;
    };
    if estimate.sheets.is_empty() {
        ui.allocate_space(available);
        ui.painter().text(
            ui.max_rect().center(),
            egui::Align2::CENTER_CENTER,
            "No part fitted on the sheet.",
            egui::FontId::new(18.0, egui::FontFamily::Proportional),
            egui::Color32::BLACK,
        );
        return;
    }

    let note_height = if estimate.too_big.is_empty() {
        0.0
    } else {
        ui.label("These do not fit on the sheet, even turned:");
        for index in &estimate.too_big {
            if let Some(part) = job.parts.get(*index) {
                ui.label(format!(
                    "{}   {:.0} × {:.0} mm",
                    part.titled(),
                    part.width_mm,
                    part.height_mm
                ));
            }
        }
        ui.min_rect().height()
    };
    let viewport = egui::vec2(available.x, (available.y - note_height).max(160.0));
    let several = estimate.sheets.len() > 1;
    if several {
        egui::ScrollArea::vertical()
            .id_salt("cut_canvas")
            .show(ui, |ui| {
                for (sheet_index, sheet) in estimate.sheets.iter().enumerate() {
                    ui.label(
                        egui::RichText::new(format!(
                            "Board {} of {}",
                            sheet_index + 1,
                            estimate.sheets.len()
                        ))
                        .strong()
                        .size(16.0)
                        .color(egui::Color32::BLACK),
                    );
                    draw_sheet(ui, job, &sheet.parts, viewport.y - 28.0);
                    ui.add_space(16.0);
                }
            });
    } else {
        draw_sheet(ui, job, &estimate.sheets[0].parts, viewport.y);
    }
}

fn draw_sheet(ui: &mut egui::Ui, job: &CutJob, placed: &[PlacedPart], height: f32) {
    let board_w = job.board.width_mm.max(1.0);
    let board_h = job.board.height_mm.max(1.0);
    // Wide enough for "4000 mm" at the large size, so the height is not clipped.
    let left_band = 128.0;
    let top_band = 40.0;
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height.max(top_band + 80.0)),
        egui::Sense::hover(),
    );
    let inner_w = (rect.width() - left_band).max(40.0);
    let inner_h = (rect.height() - top_band).max(40.0);
    let scale = (inner_w / board_w).min(inner_h / board_h);
    let sheet_px = egui::vec2(board_w * scale, board_h * scale);
    let sheet_origin = rect.min
        + egui::vec2(
            left_band + (inner_w - sheet_px.x) * 0.5,
            top_band + (inner_h - sheet_px.y) * 0.5,
        );
    let sheet = egui::Rect::from_min_size(sheet_origin, sheet_px);
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(sheet, 4.0, egui::Color32::from_rgb(236, 228, 214));
    painter.rect_stroke(
        sheet,
        4.0,
        egui::Stroke::new(2.0, egui::Color32::BLACK),
        egui::StrokeKind::Inside,
    );

    let font = egui::FontId::new(22.0, egui::FontFamily::Proportional);
    painter.text(
        egui::pos2(sheet.center().x, sheet.top() - 8.0),
        egui::Align2::CENTER_BOTTOM,
        format!("{board_w:.0} mm"),
        font.clone(),
        egui::Color32::BLACK,
    );
    painter.text(
        egui::pos2(sheet.left() - 10.0, sheet.center().y),
        egui::Align2::RIGHT_CENTER,
        format!("{board_h:.0} mm"),
        font,
        egui::Color32::BLACK,
    );

    for piece in placed {
        let origin = sheet.min
            + egui::vec2(
                piece.x_mm * scale,
                (board_h - piece.y_mm - piece.height_mm) * scale,
            );
        let size = egui::vec2(piece.width_mm * scale, piece.height_mm * scale);
        let piece_rect = egui::Rect::from_min_size(origin, size);
        let fill = part_color(piece.part_index);
        painter.rect_filled(piece_rect, 2.0, fill);
        painter.rect_stroke(
            piece_rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::from_black_alpha(80)),
            egui::StrokeKind::Inside,
        );
        if size.x > 46.0 && size.y > 28.0 {
            let caption = job
                .parts
                .get(piece.part_index)
                .map(|part| part.titled())
                .unwrap_or_default();
            painter.text(
                piece_rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("{caption}\n{:.0}×{:.0}", piece.width_mm, piece.height_mm),
                egui::FontId::proportional(12.0),
                text_on(fill),
            );
        }
    }
}

fn part_color(index: usize) -> egui::Color32 {
    let rgb = PART_COLORS[index % PART_COLORS.len()];
    egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

fn text_on(color: egui::Color32) -> egui::Color32 {
    let brightness = color.r() as u16 + color.g() as u16 + color.b() as u16;
    if brightness > 420 {
        egui::Color32::BLACK
    } else {
        egui::Color32::WHITE
    }
}

fn mm_row(
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
