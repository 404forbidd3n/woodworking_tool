//! Open kitchen sinks: rectangular (one or two bowls) and round.
//!
//! A sink is built from thin walls around an open middle, so you can see
//! into the basin from above.

use bevy::prelude::*;

use super::{Solid, Surface};
use crate::layout::model::custom::BoardShape;

/// A drop-in wash basin. The top is an open rim; the bowl hangs below it.
pub(super) fn rectangular_sink(size: Vec3, bowls: u8) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let bowls = bowls.max(1);
    let lip = 0.028_f32.min(w * 0.08).min(d * 0.08);
    let wall = 0.012_f32.min(lip * 0.6).max(0.008);
    let floor_t = 0.01_f32.min(h * 0.12);
    let rim_h = 0.016_f32.min(h * 0.18);
    let metal = Surface::Appliance(1);
    let basin = Surface::Appliance(2);
    let mut parts = Vec::new();

    parts.push(Solid::axis(
        Vec3::new(w, floor_t, d),
        Vec3::new(0.0, floor_t * 0.5, 0.0),
        metal,
    ));

    let side_h = (h - rim_h - floor_t).max(0.04);
    let side_y = floor_t + side_h * 0.5;
    parts.push(Solid::axis(
        Vec3::new(wall, side_h, d),
        Vec3::new(-w * 0.5 + wall * 0.5, side_y, 0.0),
        metal,
    ));
    parts.push(Solid::axis(
        Vec3::new(wall, side_h, d),
        Vec3::new(w * 0.5 - wall * 0.5, side_y, 0.0),
        metal,
    ));
    let span_w = (w - wall * 2.0).max(0.02);
    parts.push(Solid::axis(
        Vec3::new(span_w, side_h, wall),
        Vec3::new(0.0, side_y, d * 0.5 - wall * 0.5),
        metal,
    ));
    parts.push(Solid::axis(
        Vec3::new(span_w, side_h, wall),
        Vec3::new(0.0, side_y, -d * 0.5 + wall * 0.5),
        metal,
    ));

    let open_w = (w - lip * 2.0).max(0.05);
    let open_d = (d - lip * 2.0 - wall * 2.0).max(0.04);
    let bowl_w = ((open_w - wall * (bowls - 1) as f32) / bowls as f32).max(0.04);
    if bowls > 1 {
        for index in 1..bowls {
            let x = -open_w * 0.5 + index as f32 * bowl_w + (index as f32 - 0.5) * wall;
            parts.push(Solid::axis(
                Vec3::new(wall, side_h, open_d),
                Vec3::new(x, side_y, 0.0),
                metal,
            ));
        }
    }

    let rim_y = h - rim_h * 0.5;
    parts.push(Solid::axis(
        Vec3::new(w, rim_h, lip),
        Vec3::new(0.0, rim_y, d * 0.5 - lip * 0.5),
        metal,
    ));
    parts.push(Solid::axis(
        Vec3::new(w, rim_h, lip),
        Vec3::new(0.0, rim_y, -d * 0.5 + lip * 0.5),
        metal,
    ));
    let side_lip_d = (d - lip * 2.0).max(0.02);
    parts.push(Solid::axis(
        Vec3::new(lip, rim_h, side_lip_d),
        Vec3::new(-w * 0.5 + lip * 0.5, rim_y, 0.0),
        metal,
    ));
    parts.push(Solid::axis(
        Vec3::new(lip, rim_h, side_lip_d),
        Vec3::new(w * 0.5 - lip * 0.5, rim_y, 0.0),
        metal,
    ));

    for index in 0..bowls {
        let x = -open_w * 0.5 + bowl_w * 0.5 + index as f32 * (bowl_w + wall);
        parts.push(Solid::axis(
            Vec3::new(
                (bowl_w - 0.012).max(0.03),
                0.004,
                (open_d - 0.012).max(0.03),
            ),
            Vec3::new(x, floor_t + 0.004, 0.0),
            basin,
        ));
        parts.push(Solid::axis(
            Vec3::new(0.038, 0.008, 0.038),
            Vec3::new(x, floor_t + 0.01, open_d * 0.08),
            Surface::ApplianceRound(2),
        ));
    }
    parts
}

/// A round wash basin built as an open ring of wall panels around a floor.
pub(super) fn round_sink(size: Vec3) -> Vec<Solid> {
    let (_w, h, _d) = (size.x, size.y, size.z);
    let radius = size.x.min(size.z) * 0.48;
    let wall = 0.014_f32.min(radius * 0.12);
    let floor_t = 0.01_f32.min(h * 0.12);
    let rim_h = 0.016_f32.min(h * 0.18);
    let segments = 12u8;
    let metal = Surface::Appliance(1);
    let mut parts = vec![Solid::axis(
        Vec3::new(radius * 2.0, floor_t, radius * 2.0),
        Vec3::new(0.0, floor_t * 0.5, 0.0),
        Surface::ApplianceRound(1),
    )];

    let side_h = (h - rim_h - floor_t).max(0.04);
    let side_y = floor_t + side_h * 0.5;
    let step = std::f32::consts::TAU / segments as f32;
    let chord = 2.0 * radius * (step * 0.5).sin() * 1.04;
    for index in 0..segments {
        let angle = index as f32 * step;
        let mid = radius - wall * 0.5;
        parts.push(Solid {
            size: Vec3::new(chord, side_h, wall).max(Vec3::splat(0.001)),
            translation: Vec3::new(angle.cos() * mid, side_y, angle.sin() * mid),
            rotation: Quat::from_rotation_y(-angle),
            role: metal,
            shape: BoardShape::Box,
        });
        let rim_mid = radius - wall * 0.35;
        parts.push(Solid {
            size: Vec3::new(chord, rim_h, wall * 1.7).max(Vec3::splat(0.001)),
            translation: Vec3::new(
                angle.cos() * rim_mid,
                h - rim_h * 0.5,
                angle.sin() * rim_mid,
            ),
            rotation: Quat::from_rotation_y(-angle),
            role: metal,
            shape: BoardShape::Box,
        });
    }

    let inner = (radius - wall * 2.2).max(0.04);
    parts.push(Solid::axis(
        Vec3::new(inner * 2.0, 0.004, inner * 2.0),
        Vec3::new(0.0, floor_t + 0.004, 0.0),
        Surface::ApplianceRound(2),
    ));
    parts.push(Solid::axis(
        Vec3::new(0.04, 0.008, 0.04),
        Vec3::new(0.0, floor_t + 0.01, 0.0),
        Surface::ApplianceRound(2),
    ));
    parts
}
