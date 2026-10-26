//! Everything the user does to the room: the mouse, the keyboard, the
//! sidebar, and saving.
//!
//! Left drag does the thing under the cursor: a piece moves, empty space
//! orbits. That keeps a trackpad usable, because orbit does not need a middle
//! button. Hold Shift during a move to raise or lower the piece. Exact
//! millimeters, door counts, and saving live in the sidebar.
//!
//! | File              | What is in it                                        |
//! |-------------------|------------------------------------------------------|
//! | `pointer.rs`      | Mouse clicks and drags in the 3D view                |
//! | `camera.rs`       | Zoom, orbit, and pan                                 |
//! | `keyboard.rs`     | Delete / R / D / F / arrow-key shortcuts             |
//! | `sidebar.rs`      | The egui system that lays out every panel            |
//! | `toolbar.rs`      | The bar across the top                               |
//! | `library.rs`      | The "Add ..." buttons and their little icons         |
//! | `properties.rs`   | The Properties panel and the right-click menu        |
//! | `materials.rs`    | Paint and wood swatch pickers                        |
//! | `widgets.rs`      | Shared egui helpers (the millimeter number field)    |
//! | `spawn.rs`        | Adding and removing pieces                           |
//! | `files.rs`        | Save, load, file dialogs, undo / redo                |
//!
//! The four Bevy systems that `main.rs` schedules are [`pointer`],
//! [`shortcuts`], [`orbit_camera`], [`sidebar`], plus
//! [`finish_native_dialog`] for the file dialogs.

mod camera;
mod files;
mod keyboard;
mod library;
mod materials;
mod pointer;
mod properties;
mod sidebar;
mod spawn;
mod toolbar;
mod widgets;

use bevy::prelude::*;

pub use camera::orbit_camera;
pub use files::finish_native_dialog;
pub use keyboard::shortcuts;
pub use pointer::{pointer, Drag};
pub use sidebar::sidebar;

// ---------------------------------------------------------------------------
// Shared UI state
//
// These resources are written by the egui pass and read by the mouse and
// keyboard systems, so that a click on a panel never also reaches the 3D view.
// ---------------------------------------------------------------------------

/// Width of the sidebar in logical pixels. The 3D view ignores the pointer
/// only while it is actually inside this strip, so orbit and zoom keep working
/// on the rest of the window.
#[derive(Resource)]
pub struct SidebarWidth(pub f32);

impl Default for SidebarWidth {
    fn default() -> Self {
        Self(320.0)
    }
}

/// Height of the top toolbar. Clicks and scroll there belong to the file bar,
/// not to the 3D view underneath it.
#[derive(Resource)]
pub struct ToolbarHeight(pub f32);

impl Default for ToolbarHeight {
    fn default() -> Self {
        Self(40.0)
    }
}

/// State of the Properties panel and the right-click menu.
#[derive(Resource, Default)]
pub struct PropertiesUi {
    pub context_target: Option<Entity>,
    pub context_position: Vec2,
    /// Screen rect of the right-click menu, so a click on it does not also
    /// reach the 3D view behind it.
    pub menu_rect: Option<Rect>,
    /// While set, the docked Properties panel shows the primary selection.
    pub open: bool,
    /// The materials section opens on body and fronts. This switches to one
    /// swatch row per board.
    pub materials_detail: bool,
}
