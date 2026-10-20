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
use super::job::{CutJob, CutListGroup, CutPart, PartRole};
use super::pack;
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

    let side_max = (ctx.viewport_rect().width() * 0.42).clamp(300.0, 460.0);
    egui::Panel::left("cut_parts")
        .resizable(true)
        .default_size(320.0)
        .max_size(side_max)
        .frame(theme::sidebar_frame())
        .show(&mut viewport_ui, |ui| {
            draw_sidebar(ui, &mut job, &furniture)
        });

    egui::CentralPanel::default()
        .frame(theme::sidebar_frame())
        .show(&mut viewport_ui, |ui| draw_canvas(ui, &mut job));
}

fn draw_toolbar(ui: &mut egui::Ui, job: &mut CutJob) {
    let cream = theme::ink();
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
                job.sheet_page = 0;
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
                draw_list_tabs(ui, job);
                let group = job.list_group;
                let showing = job.parts.iter().any(|part| part.role.group() == group);
                if !showing {
                    let empty = match group {
                        CutListGroup::Doors => "No doors or drawer fronts yet.",
                        CutListGroup::Parts => "No carcass parts yet.",
                    };
                    ui.label(empty);
                }
                let mut remove = None;
                for (index, part) in job.parts.iter().enumerate() {
                    if part.role.group() != group {
                        continue;
                    }
                    if part_row(ui, index, part) {
                        remove = Some(index);
                    }
                }
                if let Some(index) = remove {
                    job.parts.remove(index);
                    job.mark_stale();
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let clear_label = match group {
                        CutListGroup::Doors => "Clear doors",
                        CutListGroup::Parts => "Clear parts",
                    };
                    if ui.button(clear_label).clicked() {
                        job.clear_group(group);
                    }
                    if ui.button("Clear list").clicked() {
                        job.parts.clear();
                        job.mark_stale();
                    }
                });
                if ui.button("Add parts from the room").clicked() {
                    for piece in furniture.iter() {
                        job.parts.extend(parts_from_furniture(piece));
                    }
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

fn draw_list_tabs(ui: &mut egui::Ui, job: &mut CutJob) {
    let doors = job.parts.iter().filter(|part| part.role.is_front()).count();
    let carcass = job.parts.len() - doors;
    ui.horizontal(|ui| {
        ui.selectable_value(
            &mut job.list_group,
            CutListGroup::Doors,
            format!("Doors ({doors})"),
        );
        ui.selectable_value(
            &mut job.list_group,
            CutListGroup::Parts,
            format!("Parts ({carcass})"),
        );
    });
    let note = match job.list_group {
        CutListGroup::Doors => {
            "Doors and drawer fronts. These are often MDF, not the carcass sheet."
        }
        CutListGroup::Parts => "Sides, tops, bottoms, shelves, backs, and the rest of the carcass.",
    };
    ui.weak(note);
}

fn part_row(ui: &mut egui::Ui, index: usize, part: &CutPart) -> bool {
    let color = part_color(index);
    let mut remove = false;
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 3.0, color);
        ui.label(
            egui::RichText::new(part.role.tag())
                .size(12.0)
                .color(theme::ink())
                .background_color(theme::chip()),
        );
        let text_width = (ui.available_width() - 84.0).max(40.0);
        ui.allocate_ui_with_layout(
            egui::vec2(text_width, 36.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(format!("{} × {}", part.quantity, part.titled()))
                            .color(theme::ink()),
                    )
                    .truncate(),
                );
                ui.weak(format!("{:.0} × {:.0} mm", part.width_mm, part.height_mm));
            },
        );
        if ui.button("Remove").clicked() {
            remove = true;
        }
    });
    remove
}

fn draw_canvas(ui: &mut egui::Ui, job: &mut CutJob) {
    let available = ui.available_size();
    if job.estimate.is_none() {
        paint_canvas_message(ui, available, "Add the parts, then press Estimate.");
        return;
    }
    if job
        .estimate
        .as_ref()
        .is_some_and(|estimate| estimate.sheets.is_empty())
    {
        paint_canvas_message(ui, available, "No part fitted on the sheet.");
        return;
    }

    let too_big: Vec<usize> = job
        .estimate
        .as_ref()
        .map(|estimate| estimate.too_big.clone())
        .unwrap_or_default();
    if !too_big.is_empty() {
        ui.label("These do not fit on the sheet, even turned:");
        for index in &too_big {
            if let Some(part) = job.parts.get(*index) {
                ui.label(format!(
                    "{}   {:.0} × {:.0} mm",
                    part.titled(),
                    part.width_mm,
                    part.height_mm
                ));
            }
        }
    }

    let sheet_count = job
        .estimate
        .as_ref()
        .map(|estimate| estimate.sheets.len())
        .unwrap_or(0);
    if sheet_count == 0 {
        return;
    }
    if job.sheet_page >= sheet_count {
        job.sheet_page = sheet_count - 1;
    }
    if sheet_count > 1 {
        let page = job.sheet_page;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(page > 0, egui::Button::new("Previous"))
                .clicked()
            {
                job.sheet_page = page - 1;
            }
            ui.label(
                egui::RichText::new(format!("Board {} of {}", page + 1, sheet_count))
                    .strong()
                    .size(16.0)
                    .color(theme::ink()),
            );
            if ui
                .add_enabled(page + 1 < sheet_count, egui::Button::new("Next"))
                .clicked()
            {
                job.sheet_page = page + 1;
            }
        });
    }

    let mut height = ui.available_height();
    if !height.is_finite() || height < 240.0 {
        height = 240.0;
    }
    let page = job.sheet_page;
    draw_sheet(ui, job, page, height);
}

fn paint_canvas_message(ui: &mut egui::Ui, available: egui::Vec2, message: &str) {
    let size = if available.x.is_finite() && available.y.is_finite() {
        available
    } else {
        ui.max_rect().size()
    };
    ui.allocate_space(size);
    ui.painter().text(
        ui.max_rect().center(),
        egui::Align2::CENTER_CENTER,
        message,
        egui::FontId::new(18.0, egui::FontFamily::Proportional),
        theme::ink(),
    );
}

fn draw_sheet(ui: &mut egui::Ui, job: &CutJob, page: usize, height: f32) {
    let Some(placed) = job
        .estimate
        .as_ref()
        .and_then(|estimate| estimate.sheets.get(page))
        .map(|sheet| sheet.parts.as_slice())
    else {
        return;
    };
    let board_w = job.board.width_mm.max(1.0);
    let board_h = job.board.height_mm.max(1.0);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(80.0), height),
        egui::Sense::hover(),
    );
    // Leave room for the millimetre captions without giving them a fixed
    // band that can shrink the board down to a thumbnail.
    let left_band = (rect.width() * 0.08).clamp(64.0, 96.0);
    let top_band = 32.0;
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
        theme::ink(),
    );
    painter.text(
        egui::pos2(sheet.left() - 10.0, sheet.center().y),
        egui::Align2::RIGHT_CENTER,
        format!("{board_h:.0} mm"),
        font,
        theme::ink(),
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
            egui::Stroke::new(2.0, egui::Color32::from_black_alpha(160)),
            egui::StrokeKind::Inside,
        );
        if size.x > 28.0 && size.y > 16.0 {
            let caption = job
                .parts
                .get(piece.part_index)
                .map(|part| part.titled())
                .unwrap_or_default();
            let font_size = (size.y * 0.28).clamp(10.0, 16.0);
            painter.text(
                piece_rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("{caption}\n{:.0}×{:.0}", piece.width_mm, piece.height_mm),
                egui::FontId::proportional(font_size),
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
