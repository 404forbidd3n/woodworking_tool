//! The L-shaped corner cabinet.
//!
//! Width runs along the back wall and depth along the left wall. Both arms
//! are [`corner_arm`] deep, and the two arms share one bottom and top.

use bevy::prelude::*;

use super::cabinet::{door_leaf, handle};
use super::{Solid, Surface};
use crate::layout::model::kind::{CornerStyle, DoorStyle};
use crate::layout::model::units::{GAP_METERS, PANEL_METERS};

/// Depth of each straight run on a corner cabinet. The corner bay is this
/// deep on both walls, so the diagonal is longer than either margin.
fn corner_arm(size: Vec3) -> f32 {
    0.35_f32.min(size.x * 0.62).min(size.z * 0.62)
}

/// The corner carcass plus either two small doors (shelf style) or two
/// doors across the diagonal (double-door style).
pub(super) fn corner_cabinet(size: Vec3, style: CornerStyle, doors: DoorStyle) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let t = PANEL_METERS.min(w * 0.12).min(h * 0.12).min(d * 0.12);
    let arm = corner_arm(size).max(t * 4.0);
    let extra_w = (w - arm).max(0.04);
    let extra_d = (d - arm).max(0.04);
    let back_z = -d * 0.5 + arm * 0.5;
    let left_x = -w * 0.5 + arm * 0.5;
    let return_z = -d * 0.5 + arm + extra_d * 0.5;
    let inner_h = (h - 2.0 * t).max(0.02);
    let mut parts = vec![
        Solid::axis(
            Vec3::new(w, t, arm),
            Vec3::new(0.0, t * 0.5, back_z),
            Surface::Carcass(2),
        ),
        Solid::axis(
            Vec3::new(arm, t, extra_d),
            Vec3::new(left_x, t * 0.5, return_z),
            Surface::Carcass(2),
        ),
        Solid::axis(
            Vec3::new(w, t, arm),
            Vec3::new(0.0, h - t * 0.5, back_z),
            Surface::Carcass(3),
        ),
        Solid::axis(
            Vec3::new(arm, t, extra_d),
            Vec3::new(left_x, h - t * 0.5, return_z),
            Surface::Carcass(3),
        ),
        Solid::axis(
            Vec3::new((w - t).max(0.02), inner_h, t),
            Vec3::new(t * 0.5, h * 0.5, -d * 0.5 + t * 0.5),
            Surface::Carcass(4),
        ),
        Solid::axis(
            Vec3::new(t, inner_h, (d - t).max(0.02)),
            Vec3::new(-w * 0.5 + t * 0.5, h * 0.5, t * 0.5),
            Surface::Carcass(0),
        ),
        Solid::axis(
            Vec3::new(t, inner_h, (arm - t).max(0.02)),
            Vec3::new(w * 0.5 - t * 0.5, h * 0.5, -d * 0.5 + (arm + t) * 0.5),
            Surface::Carcass(1),
        ),
        Solid::axis(
            Vec3::new((arm - t).max(0.02), inner_h, t),
            Vec3::new(-w * 0.5 + (arm + t) * 0.5, h * 0.5, d * 0.5 - t * 0.5),
            Surface::Carcass(1),
        ),
    ];

    let door_h = (h - GAP_METERS * 2.0).max(0.04);
    match style {
        CornerStyle::ShelfDoors => {
            for index in 1..=2 {
                let y = t + inner_h * index as f32 / 3.0;
                parts.push(Solid::axis(
                    Vec3::new((arm - 2.0 * t).max(0.02), t, (arm - 2.0 * t).max(0.02)),
                    Vec3::new(left_x, y, back_z),
                    Surface::Carcass(2),
                ));
            }
            let door_w = (extra_w - GAP_METERS * 2.0).max(0.04);
            let door_x = -w * 0.5 + arm + extra_w * 0.5;
            let door_z = -d * 0.5 + arm + 0.008;
            parts.extend(door_leaf(
                door_w,
                door_h,
                Vec3::new(door_x, h * 0.5, door_z),
                Quat::IDENTITY,
                doors,
                0,
            ));
            parts.push(handle(
                Vec3::new(door_x - door_w * 0.35, h * 0.5, door_z + PANEL_METERS),
                0.08,
            ));

            let door_d = (extra_d - GAP_METERS * 2.0).max(0.04);
            let side_x = -w * 0.5 + arm + 0.008;
            let side_z = -d * 0.5 + arm + extra_d * 0.5;
            // Face the left-wall run toward the room (+X).
            let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
            parts.extend(door_leaf(
                door_d,
                door_h,
                Vec3::new(side_x, h * 0.5, side_z),
                rotation,
                doors,
                1,
            ));
            parts.push(Solid::axis(
                Vec3::new(0.012, 0.012, 0.08),
                Vec3::new(side_x + PANEL_METERS, h * 0.5, side_z - door_d * 0.35),
                Surface::Handle,
            ));
        }
        CornerStyle::DoubleDoor => {
            let ax = w * 0.5;
            let az = -d * 0.5 + arm;
            let bx = -w * 0.5 + arm;
            let bz = d * 0.5;
            let dx = bx - ax;
            let dz = bz - az;
            let length = (dx * dx + dz * dz).sqrt().max(0.05);
            let along_x = dx / length;
            let along_z = dz / length;
            // Points out of the cabinet, into the room.
            let nx = along_z;
            let nz = -along_x;
            let leaf_w = (length * 0.5 - GAP_METERS).max(0.04);
            let shift = leaf_w * 0.5 + GAP_METERS * 0.5;
            let rotation = Quat::from_rotation_y((-along_z).atan2(along_x));
            let mid_x = (ax + bx) * 0.5 + nx * 0.012;
            let mid_z = (az + bz) * 0.5 + nz * 0.012;
            for (index, sign) in [(0u8, -1.0_f32), (1, 1.0)] {
                let center = Vec3::new(
                    mid_x + along_x * shift * sign,
                    h * 0.5,
                    mid_z + along_z * shift * sign,
                );
                parts.extend(door_leaf(leaf_w, door_h, center, rotation, doors, index));
                let handle_along = -sign * (leaf_w * 0.5 - 0.05);
                parts.push(Solid::axis(
                    Vec3::new(0.016, 0.09, 0.016),
                    center
                        + Vec3::new(along_x, 0.0, along_z) * handle_along
                        + Vec3::new(nx, 0.0, nz) * 0.02,
                    Surface::Handle,
                ));
            }
        }
    }
    parts
}
