//! Cabinet parts: the carcass, shelves, legs, drawer fronts, and doors.
//!
//! A cabinet is always [`carcass`] plus one of the "fronts" functions below.
//! `solids()` in the parent module picks which ones to combine.

use bevy::prelude::*;

use super::{Solid, Surface};
use crate::layout::model::custom::BoardShape;
use crate::layout::model::kind::{DoorStyle, LegStyle};
use crate::layout::model::units::{GAP_METERS, PANEL_METERS};

// ---------------------------------------------------------------------------
// The box: carcass, shelves, legs
// ---------------------------------------------------------------------------

/// The five-panel box every cabinet starts from. Surface indexes:
/// 0 left, 1 right, 2 bottom, 3 top, 4 back (same order as the color slots).
pub(super) fn carcass(size: Vec3) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let t = PANEL_METERS.min(w * 0.2).min(h * 0.2).min(d * 0.2);
    let inner_w = (w - 2.0 * t).max(0.01);
    let inner_h = (h - 2.0 * t).max(0.01);
    vec![
        Solid::axis(
            Vec3::new(t, h, d),
            Vec3::new(-w * 0.5 + t * 0.5, h * 0.5, 0.0),
            Surface::Carcass(0),
        ),
        Solid::axis(
            Vec3::new(t, h, d),
            Vec3::new(w * 0.5 - t * 0.5, h * 0.5, 0.0),
            Surface::Carcass(1),
        ),
        Solid::axis(
            Vec3::new(inner_w, t, d),
            Vec3::new(0.0, t * 0.5, 0.0),
            Surface::Carcass(2),
        ),
        Solid::axis(
            Vec3::new(inner_w, t, d),
            Vec3::new(0.0, h - t * 0.5, 0.0),
            Surface::Carcass(3),
        ),
        Solid::axis(
            Vec3::new(inner_w, inner_h, t),
            Vec3::new(0.0, h * 0.5, -d * 0.5 + t * 0.5),
            Surface::Carcass(4),
        ),
    ]
}

/// `count` shelves spaced evenly between the bottom and the top.
pub(super) fn open_shelves(size: Vec3, count: u8) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let t = PANEL_METERS.min(w * 0.2).min(h * 0.2).min(d * 0.2);
    let inner_w = (w - 2.0 * t).max(0.01);
    (1..=count)
        .map(|index| {
            let y = t + (h - 2.0 * t) * index as f32 / (count as f32 + 1.0);
            Solid::axis(
                Vec3::new(inner_w, t, d),
                Vec3::new(0.0, y, 0.0),
                Surface::Carcass(2),
            )
        })
        .collect()
}

/// Four feet, one near each corner, under a carcass lifted by `height`.
pub(super) fn adjustable_legs(size: Vec3, height: f32, style: LegStyle) -> Vec<Solid> {
    let foot = 0.045_f32.min(size.x * 0.15).min(size.z * 0.15);
    let inset_x = (foot + 0.05).min(size.x * 0.35);
    let inset_z = (foot + 0.05).min(size.z * 0.35);
    let mut parts = Vec::with_capacity(4);
    for x_sign in [-1.0, 1.0] {
        for z_sign in [-1.0, 1.0] {
            parts.push(Solid::axis(
                Vec3::new(foot, height, foot),
                Vec3::new(
                    x_sign * (size.x * 0.5 - inset_x),
                    height * 0.5,
                    z_sign * (size.z * 0.5 - inset_z),
                ),
                Surface::Leg(style),
            ));
        }
    }
    parts
}

/// Equal bands from the bottom of the front to the top.
/// `count` bands are separated by `gap`, and the first and last band sit
/// `gap` in from the outer edges as well.
pub fn equal_spans(length: f32, count: u8, gap: f32) -> Vec<(f32, f32)> {
    let count = count.max(1) as f32;
    let usable = (length - gap * (count + 1.0)).max(count * 0.001);
    let band = usable / count;
    (0..count as usize)
        .map(|index| {
            let i = index as f32;
            let start = gap + i * (band + gap);
            (start + band * 0.5, band)
        })
        .collect()
}

/// Drawer fronts stacked from bottom to top, each with a handle.
pub(super) fn drawer_fronts(size: Vec3, count: u8) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let z = d * 0.5 + 0.004;
    let spans = equal_spans(h, count, GAP_METERS);
    let mut parts = Vec::with_capacity(spans.len() * 2);
    for (index, (center_y, band_h)) in spans.into_iter().enumerate() {
        parts.push(Solid::axis(
            Vec3::new((w - GAP_METERS * 2.0).max(0.02), band_h, PANEL_METERS),
            Vec3::new(0.0, center_y, z),
            Surface::Front(index as u8),
        ));
        parts.push(handle(Vec3::new(0.0, center_y, z + PANEL_METERS * 0.5), w));
    }
    parts
}

/// One or two hinged doors side by side, with pulls near the opening edge.
pub(super) fn hinged_fronts(size: Vec3, doors: u8, style: DoorStyle) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let z = d * 0.5 + 0.004;
    let doors = doors.clamp(1, 2);
    let spans = equal_spans(w, doors, GAP_METERS);
    let front_h = (h - GAP_METERS * 2.0).max(0.02);
    let center_y = h * 0.5;
    let mut parts = Vec::new();
    for (index, (center_x, band_w)) in spans.into_iter().enumerate() {
        // `equal_spans` measures from the left edge. Shift onto the centered origin.
        let x = center_x - w * 0.5;
        parts.extend(door_leaf(
            band_w,
            front_h,
            Vec3::new(x, center_y, z),
            Quat::IDENTITY,
            style,
            index as u8,
        ));
        // A single door gets the pull on the right. A pair gets pulls toward the middle.
        let pull_x = if doors == 1 {
            x + band_w * 0.5 - 0.04
        } else if index == 0 {
            x + band_w * 0.5 - 0.04
        } else {
            x - band_w * 0.5 + 0.04
        };
        parts.push(handle(
            Vec3::new(pull_x, center_y, z + PANEL_METERS * 0.5),
            0.12,
        ));
    }
    parts
}

/// The lift-up door is rotated open around its top edge. Negative rotation
/// around X swings the bottom of the door out toward +Z (into the room)
/// and upward, which is the motion of a top-hinged door.
pub(super) fn lift_front(size: Vec3, style: DoorStyle) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let front_h = (h - GAP_METERS * 2.0).max(0.02);
    let front_w = (w - GAP_METERS * 2.0).max(0.02);
    let hinge = Vec3::new(0.0, h - GAP_METERS, d * 0.5 + 0.004);
    let rotation = Quat::from_rotation_x(-0.65);
    let door_center = hinge + rotation * Vec3::new(0.0, -front_h * 0.5, 0.0);
    let handle_point =
        hinge + rotation * Vec3::new(0.0, -front_h + 0.04, PANEL_METERS * 0.5 + 0.004);
    let mut parts = door_leaf(front_w, front_h, door_center, rotation, style, 0);
    parts.push(Solid {
        size: Vec3::new(0.12, 0.012, 0.012),
        translation: handle_point,
        rotation,
        role: Surface::Handle,
        shape: BoardShape::Box,
    });
    parts
}

/// Two overlapping sliding doors on separate tracks.
pub(super) fn sliding_fronts(size: Vec3, style: DoorStyle) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let front_h = (h - GAP_METERS * 2.0).max(0.02);
    // Each leaf is a little wider than half so they overlap, and one sits
    // a few millimeters in front of the other so the overlap is readable.
    let leaf_w = w * 0.55;
    let center_y = h * 0.5;
    let back_z = d * 0.5 + 0.002;
    let front_z = back_z + 0.012;
    let mut parts = door_leaf(
        leaf_w,
        front_h,
        Vec3::new(-w * 0.22, center_y, back_z),
        Quat::IDENTITY,
        style,
        0,
    );
    parts.push(handle(
        Vec3::new(-0.04, center_y, back_z + PANEL_METERS),
        0.02,
    ));
    parts.extend(door_leaf(
        leaf_w,
        front_h,
        Vec3::new(w * 0.22, center_y, front_z),
        Quat::IDENTITY,
        style,
        1,
    ));
    parts.push(handle(
        Vec3::new(0.04, center_y, front_z + PANEL_METERS),
        0.02,
    ));
    parts
}

/// A solid slab, or a wood frame with a glass pane in the middle.
/// Local X is the leaf width, local Y the height, and local +Z the outer face.
pub(super) fn door_leaf(
    width: f32,
    height: f32,
    center: Vec3,
    rotation: Quat,
    style: DoorStyle,
    index: u8,
) -> Vec<Solid> {
    let thickness = PANEL_METERS;
    if style != DoorStyle::Glass {
        return vec![Solid {
            size: Vec3::new(width, height, thickness).max(Vec3::splat(0.001)),
            translation: center,
            rotation,
            role: Surface::Front(index),
            shape: BoardShape::Box,
        }];
    }
    let rail = 0.042_f32.min(width * 0.18).min(height * 0.18).max(0.008);
    let frame = |sx: f32, sy: f32, ox: f32, oy: f32| Solid {
        size: Vec3::new(sx, sy, thickness).max(Vec3::splat(0.001)),
        translation: center + rotation * Vec3::new(ox, oy, 0.0),
        rotation,
        role: Surface::Front(index),
        shape: BoardShape::Box,
    };
    let inner_w = (width - rail * 2.0).max(0.01);
    let inner_h = (height - rail * 2.0).max(0.01);
    vec![
        frame(rail, height, -width * 0.5 + rail * 0.5, 0.0),
        frame(rail, height, width * 0.5 - rail * 0.5, 0.0),
        frame(inner_w, rail, 0.0, height * 0.5 - rail * 0.5),
        frame(inner_w, rail, 0.0, -height * 0.5 + rail * 0.5),
        Solid {
            size: Vec3::new(inner_w, inner_h, 0.004),
            translation: center + rotation * Vec3::new(0.0, 0.0, 0.002),
            rotation,
            role: Surface::Glass,
            shape: BoardShape::Box,
        },
    ]
}

/// A small bar pull. `width` is clamped to 40–180 mm.
pub(super) fn handle(at: Vec3, width: f32) -> Solid {
    let width = width.clamp(0.04, 0.18);
    Solid::axis(
        Vec3::new(width, 0.012, 0.012),
        at + Vec3::new(0.0, 0.0, 0.008),
        Surface::Handle,
    )
}
