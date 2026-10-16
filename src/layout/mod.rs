//! The 3D room planner.
//!
//! - [`model`] keeps millimeters, cabinet kinds, and the saved project
//! - [`view`] draws the room, the camera, and the furniture meshes
//! - [`edit`] handles the pointer, the sidebar, and saving

pub mod dimensions;
pub mod edit;
pub mod history;
pub mod model;
pub mod project;
pub mod view;
