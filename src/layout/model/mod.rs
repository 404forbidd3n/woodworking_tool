//! Furniture, room size, and the millimeters-to-meters conversion.
//!
//! The 3D world is in meters because that is a comfortable scale for the
//! renderer. The sidebar speaks millimeters, which is how a cabinet is
//! actually measured. The room origin is the back-left corner of the floor:
//!
//! - X runs to the right along the back wall
//! - Y runs up
//! - Z runs out into the room
//!
//! A piece is positioned by the center of its base, not by a corner. Its
//! front face points along local +Z. A quarter-turn of 0 keeps that front
//! facing into the room.
//!
//! This module is pure data and math. It never draws anything and never reads
//! input; `view` draws it and `edit` changes it.
//!
//! | File            | What is in it                                            |
//! |-----------------|----------------------------------------------------------|
//! | `units.rs`      | mm <-> m, grid snap, quarter-turn facing                 |
//! | `kind.rs`       | [`Kind`] (what a piece is), door/corner/leg/pipe options |
//! | `furniture.rs`  | [`Furniture`], the component on every placed piece       |
//! | `custom.rs`     | Custom blocks from the component planner                 |
//! | `finish.rs`     | Materials and colors                                     |
//! | `room.rs`       | [`Room`], walls, doors and windows                       |
//! | `editor.rs`     | [`Editor`] settings and the [`Selection`]                |
//! | `geometry/`     | Turning a piece into boxes for the renderer              |
//! | `picking.rs`    | Ray tests for clicking and dragging                      |
//! | `example.rs`    | The example kitchen                                      |
//! | `file.rs`       | The JSON save format, loading and saving                 |
//!
//! Everything is re-exported here, so the rest of the app just writes
//! `use crate::layout::model::Furniture` without caring which file it is in.

mod custom;
mod editor;
mod example;
mod file;
mod finish;
mod furniture;
mod geometry;
mod kind;
mod picking;
mod room;
mod units;

#[cfg(test)]
mod tests;

pub use custom::*;
pub use editor::*;
pub use example::*;
pub use file::*;
pub use finish::*;
pub use furniture::*;
pub use geometry::*;
pub use kind::*;
pub use picking::*;
pub use room::*;
pub use units::*;
