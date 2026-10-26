//! The 3D room planner (the main window).
//!
//! The room is split the classic way, into data, drawing, and input:
//!
//! - [`model`] — millimeters, cabinet kinds, room size, the save format, and
//!   the math that turns a cabinet into boxes. Pure data; no Bevy systems.
//! - [`view`] — draws the room: the orbit camera, the meshes, the materials,
//!   and the floor grid.
//! - [`edit`] — changes the room: mouse, keyboard, the sidebar, the
//!   Properties panel, and saving and loading.
//!
//! And three smaller helpers:
//!
//! - [`dimensions`] — the "600 mm" labels under cabinets and on the walls
//! - [`history`] — undo and redo, by snapshotting the whole room
//! - [`project`] — room snapshots, and the native Save / Open dialogs

pub mod dimensions;
pub mod edit;
pub mod history;
pub mod model;
pub mod project;
pub mod view;
