//! Ray math used to click on pieces and drag them across the floor.

use bevy::prelude::*;

use super::furniture::Furniture;
use super::kind::Kind;

/// Local-space box used both for clicking and for the selection outline.
/// The origin is the base center. Fronts stick out a little past `depth`.
pub fn pick_bounds(furniture: &Furniture) -> (Vec3, Vec3) {
    let size = furniture.size_meters();
    let leg_height = furniture.leg_height_meters();
    let extra_z = if matches!(furniture.kind, Kind::LiftDoor) {
        0.25
    } else {
        0.04
    };
    let extra_y = if matches!(furniture.kind, Kind::LiftDoor) {
        0.12
    } else {
        0.0
    };
    // A 20 mm pipe is hard to grab. The outline and the click target grow a little.
    let grab = if furniture.kind.is_pipe() { 0.03 } else { 0.0 };
    (
        Vec3::new(-size.x * 0.5 - grab, 0.0, -size.z * 0.5 - grab),
        Vec3::new(
            size.x * 0.5 + grab,
            size.y + leg_height + extra_y + grab,
            size.z * 0.5 + extra_z + grab,
        ),
    )
}

/// Ray against an axis-aligned box. `direction` must be normalized.
/// Returns the distance along the ray, or `None` when the ray misses.
pub fn ray_aabb(origin: Vec3, direction: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    let mut near = 0.0_f32;
    let mut far = f32::MAX;
    for axis in 0..3 {
        let origin_axis = origin[axis];
        let direction_axis = direction[axis];
        if direction_axis.abs() < 1.0e-8 {
            if origin_axis < min[axis] || origin_axis > max[axis] {
                return None;
            }
            continue;
        }
        let mut t1 = (min[axis] - origin_axis) / direction_axis;
        let mut t2 = (max[axis] - origin_axis) / direction_axis;
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
        }
        near = near.max(t1);
        far = far.min(t2);
        if near > far {
            return None;
        }
    }
    if far < 0.0 {
        None
    } else {
        Some(near.max(0.0))
    }
}

/// Where a ray crosses a horizontal plane. Used to slide a piece across the floor.
pub fn ray_horizontal_plane(origin: Vec3, direction: Vec3, plane_y: f32) -> Option<Vec3> {
    if direction.y.abs() < 1.0e-6 {
        return None;
    }
    let distance = (plane_y - origin.y) / direction.y;
    if distance < 0.0 {
        None
    } else {
        Some(origin + direction * distance)
    }
}
