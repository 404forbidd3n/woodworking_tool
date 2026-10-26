//! The floor slab and the four walls, as boxes.
//!
//! A wall with doors or windows is cut into a grid of panels along every
//! opening edge, and the panels that fall inside an opening are skipped.
//! That leaves a real hole you can see through.

use bevy::prelude::*;

use crate::layout::model::{mm_to_m, Room, Solid, Surface};

/// The floor slab plus every visible wall.
pub(super) fn room_solids(room: &Room) -> Vec<Solid> {
    let size = room.size_meters();
    let (w, _h, d) = (size.x, size.y, size.z);
    let t = 0.08;
    let mut parts = vec![Solid {
        // The top of the floor slab is y = 0, so pieces sit on it.
        size: Vec3::new(w, 0.04, d),
        translation: Vec3::new(w * 0.5, -0.02, d * 0.5),
        rotation: Quat::IDENTITY,
        role: Surface::Floor,
        shape: crate::layout::model::BoardShape::Box,
    }];
    for side in crate::layout::model::WallSide::ALL {
        if room.wall_shown(side) {
            parts.extend(wall_panels(room, side, t));
        }
    }
    parts
}

/// One wall, split into panels around its doors and windows.
fn wall_panels(room: &Room, side: crate::layout::model::WallSide, thickness: f32) -> Vec<Solid> {
    use crate::layout::model::WallSide;
    let length_mm = match side {
        WallSide::Back | WallSide::Front => room.width_mm,
        WallSide::Left | WallSide::Right => room.depth_mm,
    };
    let mut xs = vec![0.0, length_mm];
    let mut ys = vec![0.0, room.wall_height_mm];
    let openings: Vec<_> = room
        .openings
        .iter()
        .filter(|opening| opening.wall == side)
        .collect();
    for opening in &openings {
        xs.extend([opening.offset_mm, opening.offset_mm + opening.width_mm]);
        ys.extend([opening.sill_mm, opening.sill_mm + opening.height_mm]);
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    xs.dedup_by(|a, b| (*a - *b).abs() < 0.1);
    ys.dedup_by(|a, b| (*a - *b).abs() < 0.1);

    let mut panels = Vec::new();
    for x_pair in xs.windows(2) {
        for y_pair in ys.windows(2) {
            let (x0, x1) = (x_pair[0], x_pair[1]);
            let (y0, y1) = (y_pair[0], y_pair[1]);
            let center = ((x0 + x1) * 0.5, (y0 + y1) * 0.5);
            if openings.iter().any(|opening| {
                center.0 > opening.offset_mm
                    && center.0 < opening.offset_mm + opening.width_mm
                    && center.1 > opening.sill_mm
                    && center.1 < opening.sill_mm + opening.height_mm
            }) {
                continue;
            }
            let span = mm_to_m(x1 - x0);
            let panel_h = mm_to_m(y1 - y0);
            let along = mm_to_m((x0 + x1) * 0.5);
            let center_y = mm_to_m((y0 + y1) * 0.5);
            let solid = match side {
                WallSide::Back => wall(
                    Vec3::new(span, panel_h, thickness),
                    Vec3::new(along, center_y, -thickness * 0.5),
                ),
                WallSide::Left => wall(
                    Vec3::new(thickness, panel_h, span),
                    Vec3::new(-thickness * 0.5, center_y, along),
                ),
                WallSide::Right => wall(
                    Vec3::new(thickness, panel_h, span),
                    Vec3::new(mm_to_m(room.width_mm) + thickness * 0.5, center_y, along),
                ),
                WallSide::Front => wall(
                    Vec3::new(span, panel_h, thickness),
                    Vec3::new(along, center_y, mm_to_m(room.depth_mm) + thickness * 0.5),
                ),
            };
            panels.push(solid);
        }
    }
    panels
}

/// A wall panel box.
fn wall(size: Vec3, translation: Vec3) -> Solid {
    Solid {
        size,
        translation,
        rotation: Quat::IDENTITY,
        role: Surface::Wall,
        shape: crate::layout::model::BoardShape::Box,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::model::{Opening, OpeningKind, WallSide};

    #[test]
    fn wall_is_split_around_a_real_opening() {
        let mut room = Room::default();
        room.openings.push(Opening {
            kind: OpeningKind::Door,
            wall: WallSide::Back,
            offset_mm: 1000.0,
            width_mm: 900.0,
            height_mm: 2050.0,
            sill_mm: 0.0,
        });
        let panels = wall_panels(&room, WallSide::Back, 0.08);
        assert!(panels.len() > 1);
        room.walls.front = true;
        let front = wall_panels(&room, WallSide::Front, 0.08);
        let depth = mm_to_m(room.depth_mm);
        assert!(!front.is_empty());
        assert!(front.iter().all(|panel| panel.translation.z > depth));
        room.walls.back = false;
        let drawn = room_solids(&room);
        assert!(drawn
            .iter()
            .filter(|solid| solid.role == Surface::Wall)
            .all(|panel| panel.translation.z > -0.01));
        assert!(panels.iter().all(|panel| {
            let x = panel.translation.x;
            let y = panel.translation.y;
            !(x > 1.0 && x < 1.9 && y > 0.0 && y < 2.05)
        }));
    }
}
