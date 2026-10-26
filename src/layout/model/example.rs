//! The "Example kitchen" button in the sidebar spawns this layout.

use bevy::prelude::*;

use super::furniture::Furniture;
use super::kind::Kind;
use super::units::place;

/// A small kitchen that shows every type once. It is only a starting point.
pub fn example_layout() -> Vec<(Furniture, Transform)> {
    /// Shorthand: a piece of `kind` at (x, y, z) in millimeters, facing the room.
    fn piece(
        kind: Kind,
        width: f32,
        height: f32,
        depth: f32,
        x: f32,
        y: f32,
        z: f32,
    ) -> (Furniture, Transform) {
        (
            Furniture::new(kind, width, height, depth),
            place(x, y, z, 0),
        )
    }

    vec![
        piece(
            Kind::Drawers { count: 3 },
            600.0,
            720.0,
            560.0,
            350.0,
            0.0,
            280.0,
        ),
        piece(
            Kind::HingedDoors { doors: 2 },
            800.0,
            720.0,
            560.0,
            1050.0,
            0.0,
            280.0,
        ),
        piece(
            Kind::Drawers { count: 4 },
            500.0,
            720.0,
            560.0,
            1700.0,
            0.0,
            280.0,
        ),
        piece(
            Kind::OpenShelves { shelves: 3 },
            600.0,
            720.0,
            560.0,
            2250.0,
            0.0,
            280.0,
        ),
        piece(Kind::LiftDoor, 800.0, 420.0, 340.0, 1050.0, 1480.0, 170.0),
        piece(Kind::SlidingDoors, 900.0, 2100.0, 580.0, 3200.0, 0.0, 290.0),
        piece(Kind::Table, 1500.0, 750.0, 850.0, 1900.0, 0.0, 1700.0),
    ]
}
