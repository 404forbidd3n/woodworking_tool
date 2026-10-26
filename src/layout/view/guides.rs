//! Overlay lines drawn with gizmos: the floor grid, the X/Y/Z axis marker,
//! and the orange outline around selected pieces.

use bevy::prelude::*;

use crate::layout::model::{Furniture, Room};

/// Floor grid, a short axis marker at the back-left corner, and an outline
/// around the selected piece. Drawn as gizmos so they stay visible on top of
/// the floor without becoming another mesh to maintain.
pub fn draw_guides(
    mut gizmos: Gizmos,
    room: Res<Room>,
    selection: Res<crate::layout::model::Selection>,
    pieces: Query<(&Furniture, &GlobalTransform)>,
) {
    draw_grid(&mut gizmos, &room);
    draw_axes(&mut gizmos);

    for selected in &selection.entities {
        let Ok((furniture, transform)) = pieces.get(*selected) else {
            continue;
        };
        let (min, max) = crate::layout::model::pick_bounds(furniture);
        let color = if selection.primary == Some(*selected) {
            Color::srgb(0.95, 0.42, 0.08)
        } else {
            Color::srgb(1.0, 0.72, 0.2)
        };
        wire_box(&mut gizmos, transform, min, max, color);
    }
}

/// Floor grid: 100 mm lines (500 mm in a big room), darker every 500 mm.
fn draw_grid(gizmos: &mut Gizmos, room: &Room) {
    let size = room.size_meters();
    // A 100 mm grid is useful in a normal room and noisy in a very large one.
    let step = if size.x <= 8.0 && size.z <= 8.0 {
        0.1
    } else {
        0.5
    };
    let major_every = (0.5_f32 / step).round() as i32;
    let y = 0.012;
    let x_count = (size.x / step).round() as i32;
    let z_count = (size.z / step).round() as i32;

    for index in 0..=x_count {
        let x = index as f32 * step;
        let color = if index % major_every == 0 {
            Color::srgb(0.42, 0.40, 0.36)
        } else {
            Color::srgba(0.42, 0.40, 0.36, 0.35)
        };
        gizmos.line(Vec3::new(x, y, 0.0), Vec3::new(x, y, size.z), color);
    }
    for index in 0..=z_count {
        let z = index as f32 * step;
        let color = if index % major_every == 0 {
            Color::srgb(0.42, 0.40, 0.36)
        } else {
            Color::srgba(0.42, 0.40, 0.36, 0.35)
        };
        gizmos.line(Vec3::new(0.0, y, z), Vec3::new(size.x, y, z), color);
    }
}

/// Red X, green Y, and blue Z, 350 mm long, at the room origin.
fn draw_axes(gizmos: &mut Gizmos) {
    let origin = Vec3::new(0.0, 0.03, 0.0);
    let length = 0.35;
    gizmos.line(
        origin,
        origin + Vec3::X * length,
        Color::srgb(0.75, 0.20, 0.18),
    );
    gizmos.line(
        origin,
        origin + Vec3::Y * length,
        Color::srgb(0.20, 0.55, 0.25),
    );
    gizmos.line(
        origin,
        origin + Vec3::Z * length,
        Color::srgb(0.18, 0.35, 0.75),
    );
}

/// Twelve edges of a box in a piece's local space.
fn wire_box(gizmos: &mut Gizmos, transform: &GlobalTransform, min: Vec3, max: Vec3, color: Color) {
    let corner = |x, y, z| transform.transform_point(Vec3::new(x, y, z));
    let points = [
        corner(min.x, min.y, min.z),
        corner(max.x, min.y, min.z),
        corner(max.x, min.y, max.z),
        corner(min.x, min.y, max.z),
        corner(min.x, max.y, min.z),
        corner(max.x, max.y, min.z),
        corner(max.x, max.y, max.z),
        corner(min.x, max.y, max.z),
    ];
    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    for (start, end) in edges {
        gizmos.line(points[start], points[end], color);
    }
}
