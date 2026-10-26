//! Utility pipes: one or two round runs, vertical or horizontal.

use bevy::prelude::*;

use super::{Solid, Surface};

/// Pipe diameter is the piece depth. Double pipes sit at the two ends of the
/// cross-section so the outer box still bounds them.
pub(super) fn pipes(size: Vec3, horizontal: bool, count: u8) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    if horizontal {
        let d = d.min(h);
        let ys: Vec<f32> = if count >= 2 {
            vec![d * 0.5, h - d * 0.5]
        } else {
            vec![h * 0.5]
        };
        ys.into_iter()
            .map(|y| Solid::axis(Vec3::new(w, d, d), Vec3::new(0.0, y, 0.0), Surface::Pipe))
            .collect()
    } else {
        let d = d.min(w);
        let xs: Vec<f32> = if count >= 2 {
            vec![-(w - d) * 0.5, (w - d) * 0.5]
        } else {
            vec![0.0]
        };
        xs.into_iter()
            .map(|x| {
                Solid::axis(
                    Vec3::new(d, h, d),
                    Vec3::new(x, h * 0.5, 0.0),
                    Surface::Pipe,
                )
            })
            .collect()
    }
}
