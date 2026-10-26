//! Units, snapping, and facing.
//!
//! The 3D world is in meters, the sidebar and the save files are in
//! millimeters. Everything that converts between the two lives here, along
//! with the grid snap and the "quarter turn" facing used by every piece.

use bevy::prelude::*;

/// Millimeters in a meter.
pub const MM_PER_METER: f32 = 1000.0;

/// Sheet thickness used only to draw the carcass. This is not a cut list.
pub const PANEL_METERS: f32 = 0.018;

/// Gap drawn between doors and between drawer fronts.
pub const GAP_METERS: f32 = 0.003;

/// Millimeters to meters.
pub fn mm_to_m(mm: f32) -> f32 {
    mm / MM_PER_METER
}

/// Meters to millimeters.
pub fn m_to_mm(meters: f32) -> f32 {
    meters * MM_PER_METER
}

/// Round `value` to the nearest `step`. A step of zero leaves the value alone,
/// which is how the snap toggle turns the grid off.
pub fn snap_to_step(value: f32, step: f32) -> f32 {
    if step <= f32::EPSILON {
        value
    } else {
        (value / step).round() * step
    }
}

/// Facing stored as 0, 1, 2, or 3 quarter-turns around Y.
/// 0 means the front points along +Z, into the room.
pub fn quarter_turns(rotation: Quat) -> u8 {
    let (yaw, _, _) = rotation.to_euler(EulerRot::YXZ);
    let turns = (yaw / std::f32::consts::FRAC_PI_2).round() as i32;
    turns.rem_euclid(4) as u8
}

/// The rotation for 0–3 quarter turns around Y.
pub fn rotation_from_turns(turns: u8) -> Quat {
    Quat::from_rotation_y(turns as f32 * std::f32::consts::FRAC_PI_2)
}

/// A transform from a position in millimeters and a facing in quarter turns.
pub fn place(x_mm: f32, y_mm: f32, z_mm: f32, turns: u8) -> Transform {
    Transform {
        translation: Vec3::new(mm_to_m(x_mm), mm_to_m(y_mm), mm_to_m(z_mm)),
        rotation: rotation_from_turns(turns),
        scale: Vec3::ONE,
    }
}

/// `value`, or `fallback` if it is NaN or infinite (a broken save file).
pub(crate) fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}
