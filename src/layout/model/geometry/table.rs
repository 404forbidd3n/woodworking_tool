//! The work table: one top and four legs, no carcass.

use bevy::prelude::*;

use super::{Solid, Surface};

/// A top and four legs inset from the corners. Height is the top surface.
pub(super) fn table(size: Vec3) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let top_t = 0.03_f32.min(h * 0.4);
    let leg = 0.05_f32.min(w * 0.15).min(d * 0.15);
    let inset = (leg * 0.5 + 0.03).min(w * 0.3).min(d * 0.3);
    let leg_h = (h - top_t).max(0.05);
    let mut parts = vec![Solid::axis(
        Vec3::new(w, top_t, d),
        Vec3::new(0.0, h - top_t * 0.5, 0.0),
        Surface::Top,
    )];
    for x_sign in [-1.0, 1.0] {
        for z_sign in [-1.0, 1.0] {
            parts.push(Solid::axis(
                Vec3::new(leg, leg_h, leg),
                Vec3::new(
                    x_sign * (w * 0.5 - inset),
                    leg_h * 0.5,
                    z_sign * (d * 0.5 - inset),
                ),
                Surface::Carcass(0),
            ));
        }
    }
    parts
}
