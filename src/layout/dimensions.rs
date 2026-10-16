//! Screen-readable width dimensions for cabinets, and optional room dimensions.
//!
//! Endpoints live in 3D and are projected every frame. The label itself is
//! painted with egui, so it remains upright and legible while the camera moves.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use super::model::{mm_to_m, Furniture, Room, WallSide};
use super::view::OrbitCamera;

pub fn draw_widths(
    mut contexts: EguiContexts,
    camera: Query<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    pieces: Query<(&Furniture, &GlobalTransform)>,
    room: Res<Room>,
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

    if room.show_measurements {
        draw_room(&painter, screen, camera, camera_transform, &room);
    }

    for (furniture, transform) in &pieces {
        if !furniture.kind.is_cabinet() {
            continue;
        }
        let half_width = mm_to_m(furniture.width_mm) * 0.5;
        let front = mm_to_m(furniture.depth_mm) * 0.5 + 0.06;
        let left_world = transform.transform_point(Vec3::new(-half_width, 0.025, front));
        let right_world = transform.transform_point(Vec3::new(half_width, 0.025, front));
        draw_dimension(
            &painter,
            screen,
            camera,
            camera_transform,
            left_world,
            right_world,
            furniture.width_mm,
        );
    }
}

/// Wall lengths above the walls, plus width and height of every opening.
fn draw_room(
    painter: &egui::Painter,
    screen: egui::Rect,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    room: &Room,
) {
    let wall_height = mm_to_m(room.wall_height_mm) + 0.14;
    let room_width = mm_to_m(room.width_mm);
    let room_depth = mm_to_m(room.depth_mm);
    if room.wall_shown(WallSide::Back) {
        draw_dimension(
            painter,
            screen,
            camera,
            camera_transform,
            Vec3::new(0.0, wall_height, 0.02),
            Vec3::new(room_width, wall_height, 0.02),
            room.width_mm,
        );
    }
    if room.wall_shown(WallSide::Front) {
        draw_dimension(
            painter,
            screen,
            camera,
            camera_transform,
            Vec3::new(0.0, wall_height, room_depth - 0.02),
            Vec3::new(room_width, wall_height, room_depth - 0.02),
            room.width_mm,
        );
    }
    if room.wall_shown(WallSide::Left) {
        draw_dimension(
            painter,
            screen,
            camera,
            camera_transform,
            Vec3::new(-0.02, wall_height, 0.0),
            Vec3::new(-0.02, wall_height, room_depth),
            room.depth_mm,
        );
    }
    if room.wall_shown(WallSide::Right) {
        draw_dimension(
            painter,
            screen,
            camera,
            camera_transform,
            Vec3::new(room_width + 0.02, wall_height, 0.0),
            Vec3::new(room_width + 0.02, wall_height, room_depth),
            room.depth_mm,
        );
    }

    for opening in &room.openings {
        if !room.wall_shown(opening.wall) {
            continue;
        }
        let top = opening.sill_mm + opening.height_mm;
        let width_start = wall_point(room, opening.wall, opening.offset_mm, top + 70.0);
        let width_end = wall_point(
            room,
            opening.wall,
            opening.offset_mm + opening.width_mm,
            top + 70.0,
        );
        draw_dimension(
            painter,
            screen,
            camera,
            camera_transform,
            width_start,
            width_end,
            opening.width_mm,
        );
        let height_along = opening.offset_mm - 70.0;
        draw_dimension(
            painter,
            screen,
            camera,
            camera_transform,
            wall_point(room, opening.wall, height_along, opening.sill_mm),
            wall_point(room, opening.wall, height_along, top),
            opening.height_mm,
        );
    }
}

fn wall_point(room: &Room, wall: WallSide, along_mm: f32, height_mm: f32) -> Vec3 {
    let along = mm_to_m(along_mm);
    let height = mm_to_m(height_mm);
    match wall {
        WallSide::Back => Vec3::new(along, height, 0.04),
        WallSide::Left => Vec3::new(0.04, height, along),
        WallSide::Right => Vec3::new(mm_to_m(room.width_mm) - 0.04, height, along),
        WallSide::Front => Vec3::new(along, height, mm_to_m(room.depth_mm) - 0.04),
    }
}

fn draw_dimension(
    painter: &egui::Painter,
    screen: egui::Rect,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    start_world: Vec3,
    end_world: Vec3,
    millimeters: f32,
) {
    let (Ok(start), Ok(end)) = (
        camera.world_to_viewport(camera_transform, start_world),
        camera.world_to_viewport(camera_transform, end_world),
    ) else {
        return;
    };
    let start = egui::pos2(start.x, start.y);
    let end = egui::pos2(end.x, end.y);
    if !screen.intersects(egui::Rect::from_two_pos(start, end).expand(30.0)) {
        return;
    }
    let delta = end - start;
    let length = delta.length();
    if length < 24.0 {
        return;
    }
    let direction = delta / length;
    let normal = egui::vec2(-direction.y, direction.x);
    let stroke = egui::Stroke::new(1.6, egui::Color32::BLACK);
    painter.line_segment([start, end], stroke);
    let arrow = 8.0;
    painter.line_segment([start, start + direction * arrow + normal * 4.0], stroke);
    painter.line_segment([start, start + direction * arrow - normal * 4.0], stroke);
    painter.line_segment([end, end - direction * arrow + normal * 4.0], stroke);
    painter.line_segment([end, end - direction * arrow - normal * 4.0], stroke);

    let center = start + delta * 0.5;
    let text = format!("{millimeters:.0} mm");
    let font = egui::FontId::new(13.0, egui::FontFamily::Proportional);
    let galley = painter.layout_no_wrap(text, font, egui::Color32::BLACK);
    let background = egui::Rect::from_center_size(center, galley.size() + egui::vec2(10.0, 4.0));
    painter.rect_filled(background, 4.0, egui::Color32::from_white_alpha(220));
    let text_pos = background.center() - galley.size() * 0.5;
    painter.galley(
        text_pos + egui::vec2(0.45, 0.0),
        galley.clone(),
        egui::Color32::BLACK,
    );
    painter.galley(text_pos, galley, egui::Color32::BLACK);
}
