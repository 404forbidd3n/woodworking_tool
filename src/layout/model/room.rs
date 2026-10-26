//! The room itself: floor size, wall height, which walls are visible, and the
//! doors and windows cut into the walls.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::units::mm_to_m;

/// Floor rectangle and four walls. Each wall can be hidden so the camera can
/// look through that side.
#[derive(Resource, Clone, Debug)]
pub struct Room {
    pub width_mm: f32,
    pub depth_mm: f32,
    pub wall_height_mm: f32,
    /// Master switch. Individual walls stay as they were when this is off.
    pub show_walls: bool,
    pub walls: WallVisibility,
    /// A safety lock for dimensions, openings, and utility pipes.
    pub edit_enabled: bool,
    /// View-only: wall, door, and window dimension lines. Not saved.
    pub show_measurements: bool,
    pub openings: Vec<Opening>,
    pub dirty: bool,
}

impl Default for Room {
    fn default() -> Self {
        Self {
            width_mm: 4200.0,
            depth_mm: 3000.0,
            wall_height_mm: 2400.0,
            show_walls: true,
            walls: WallVisibility::default(),
            edit_enabled: false,
            show_measurements: false,
            openings: Vec::new(),
            dirty: true,
        }
    }
}

/// A door (always starts at the floor) or a window (has a sill height).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpeningKind {
    Door,
    Window,
}

/// The four walls. Back runs along X at Z = 0; Left runs along Z at X = 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WallSide {
    Back,
    Left,
    Right,
    Front,
}

impl WallSide {
    pub const ALL: [Self; 4] = [Self::Back, Self::Left, Self::Right, Self::Front];

    pub fn label(self) -> &'static str {
        match self {
            Self::Back => "Back wall",
            Self::Left => "Left wall",
            Self::Right => "Right wall",
            Self::Front => "Front wall",
        }
    }
}

/// Which of the four walls are drawn. The front starts hidden so the room can
/// still be seen from the camera's starting side.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WallVisibility {
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub front: bool,
}

impl Default for WallVisibility {
    fn default() -> Self {
        Self {
            back: true,
            left: true,
            right: true,
            front: false,
        }
    }
}

impl WallVisibility {
    /// Is this wall switched on?
    pub fn visible(self, side: WallSide) -> bool {
        match side {
            WallSide::Back => self.back,
            WallSide::Left => self.left,
            WallSide::Right => self.right,
            WallSide::Front => self.front,
        }
    }

    /// Switch one wall on or off.
    pub fn set(&mut self, side: WallSide, visible: bool) {
        match side {
            WallSide::Back => self.back = visible,
            WallSide::Left => self.left = visible,
            WallSide::Right => self.right = visible,
            WallSide::Front => self.front = visible,
        }
    }

    /// Rooms saved before the front wall existed.
    pub fn legacy(show_walls: bool) -> Self {
        Self {
            back: show_walls,
            left: show_walls,
            right: show_walls,
            front: false,
        }
    }
}

/// A door or window cut into a wall. All sizes are millimeters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Opening {
    pub kind: OpeningKind,
    pub wall: WallSide,
    /// Distance from the wall's left/back end.
    pub offset_mm: f32,
    pub width_mm: f32,
    pub height_mm: f32,
    /// Doors always sanitize this to zero.
    pub sill_mm: f32,
}

impl Opening {
    /// A standard 900 × 2050 mm door on the back wall.
    pub fn door() -> Self {
        Self {
            kind: OpeningKind::Door,
            wall: WallSide::Back,
            offset_mm: 800.0,
            width_mm: 900.0,
            height_mm: 2050.0,
            sill_mm: 0.0,
        }
    }

    /// A 1200 × 1000 mm window on the back wall, 900 mm off the floor.
    pub fn window() -> Self {
        Self {
            kind: OpeningKind::Window,
            wall: WallSide::Back,
            offset_mm: 1000.0,
            width_mm: 1200.0,
            height_mm: 1000.0,
            sill_mm: 900.0,
        }
    }
}

impl Room {
    /// A wall is drawn only if both the master switch and its own switch are on.
    pub fn wall_shown(&self, side: WallSide) -> bool {
        self.show_walls && self.walls.visible(side)
    }

    /// Width, wall height, and depth in meters.
    pub fn size_meters(&self) -> Vec3 {
        Vec3::new(
            mm_to_m(self.width_mm).max(0.5),
            mm_to_m(self.wall_height_mm).max(0.5),
            mm_to_m(self.depth_mm).max(0.5),
        )
    }
}
