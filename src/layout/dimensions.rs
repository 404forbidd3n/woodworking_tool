//! Screen-readable width dimensions for cabinets.
//!
//! Endpoints live in 3D and are projected every frame. The label itself is
//! painted with egui, so it remains upright and legible while the camera moves.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use super::model::{mm_to_m, Furniture};
use super::view::OrbitCamera;

pub fn draw_widths(
    mut contexts: EguiContexts,
    camera: Query<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    pieces: Query<(&Furniture, &GlobalTransform)>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Background,
        egui::Id::new("cabinet_widths"),
    ));
    let screen = ctx.viewport_rect();

    for (furniture, transform) in &pieces {
        if !furniture.kind.is_cabinet() {
            continue;
        }
        let half_width = mm_to_m(furniture.width_mm) * 0.5;
        let front = mm_to_m(furniture.depth_mm) * 0.5 + 0.06;
        let left_world = transform.transform_point(Vec3::new(-half_width, 0.025, front));
        let right_world = transform.transform_point(Vec3::new(half_width, 0.025, front));
        let (Ok(left), Ok(right)) = (
            camera.world_to_viewport(camera_transform, left_world),
            camera.world_to_viewport(camera_transform, right_world),
        ) else {
            continue;
        };
        let left = egui::pos2(left.x, left.y);
        let right = egui::pos2(right.x, right.y);
        if !screen.intersects(egui::Rect::from_two_pos(left, right).expand(30.0)) {
            continue;
        }
        let delta = right - left;
        let length = delta.length();
        if length < 24.0 {
            continue;
        }
        let direction = delta / length;
        let normal = egui::vec2(-direction.y, direction.x);
        let stroke = egui::Stroke::new(1.6, egui::Color32::BLACK);
        painter.line_segment([left, right], stroke);
        let arrow = 8.0;
        painter.line_segment([left, left + direction * arrow + normal * 4.0], stroke);
        painter.line_segment([left, left + direction * arrow - normal * 4.0], stroke);
        painter.line_segment([right, right - direction * arrow + normal * 4.0], stroke);
        painter.line_segment([right, right - direction * arrow - normal * 4.0], stroke);

        let center = left + delta * 0.5;
        let text = format!("{:.0} mm", furniture.width_mm);
        let font = egui::FontId::new(13.0, egui::FontFamily::Proportional);
        let galley = painter.layout_no_wrap(text, font, egui::Color32::BLACK);
        let background =
            egui::Rect::from_center_size(center, galley.size() + egui::vec2(10.0, 4.0));
        painter.rect_filled(background, 4.0, egui::Color32::from_white_alpha(220));
        let text_pos = background.center() - galley.size() * 0.5;
        painter.galley(
            text_pos + egui::vec2(0.45, 0.0),
            galley.clone(),
            egui::Color32::BLACK,
        );
        painter.galley(text_pos, galley, egui::Color32::BLACK);
    }
}
