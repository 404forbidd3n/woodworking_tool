//! A small 3D layout tool for kitchens and other rooms, plus a sheet cutter.
//!
//! Run it with `cargo run`. The toolbar shows the open file and, in the
//! center, the cut planner and the component planner. The sidebar holds the
//! room, the furniture, and the selection.
//!
//! # Where the code lives
//!
//! ```text
//! src/
//! ├── main.rs            this file: wires every system into the Bevy app
//! ├── theme.rs           shared egui colors, frames, and collapsible sections
//! ├── layout/            the main window: the 3D room
//! │   ├── model/         pure data and math (no drawing, no input)
//! │   │   └── geometry/  turns a piece into boxes, one file per furniture family
//! │   ├── view/          draws the room: camera, meshes, materials, grid
//! │   ├── edit/          changes the room: mouse, keyboard, sidebar, saving
//! │   ├── dimensions.rs  the "600 mm" labels under cabinets
//! │   ├── history.rs     undo / redo
//! │   └── project.rs     snapshots and the native file dialogs
//! ├── cutting/           second window: packs panels onto plywood sheets
//! └── builder/           third window: builds a custom block from boards
//! ```
//!
//! Each folder has a `mod.rs` that explains what is in it, with a table of
//! its files. That is the standard Rust layout: a folder `foo/` with a
//! `foo/mod.rs` is the module `foo`.
//!
//! # Bevy in two minutes
//!
//! The app is built on [Bevy](https://bevyengine.org), an ECS game engine,
//! plus [egui](https://github.com/emilk/egui) for the panels.
//!
//! - An **entity** is an ID. Every placed cabinet is one entity.
//! - A **component** is data attached to an entity. A cabinet entity has a
//!   `Furniture` (its size, kind, colors) and a `Transform` (where it is).
//! - A **resource** is a single global value: `Room`, `Selection`, `Editor`.
//! - A **system** is a plain function that Bevy calls every frame. Its
//!   parameters say what it reads: `Res<Room>` reads the room,
//!   `ResMut<Room>` may change it, `Query<&Furniture>` loops over every piece.
//!
//! The data flows one way: input systems in `layout::edit` change
//! components and resources, and set a `dirty` flag when a shape changed.
//! `layout::view::sync_meshes` then rebuilds the meshes for whatever is
//! dirty. Nothing in `layout::model` knows about input or drawing.
//!
//! # One frame, in order
//!
//! 1. `Update`: open any window that was requested, handle input in the
//!    component planner and rebuild its preview, apply a finished file
//!    dialog, then handle the room's mouse and keyboard and move its camera.
//!    Windows the user closed are cleaned up last.
//! 2. `EguiPrimaryContextPass`: draw the width labels, then the toolbar,
//!    sidebar, and Properties panel of the main window.
//! 3. `CutPlannerPass` / `ComponentPlannerPass`: draw the egui of the other
//!    two windows, if they are open.
//! 4. `PostUpdate`: rebuild changed meshes, record an undo step, and draw
//!    the grid and selection outline.

mod builder;
mod cutting;
mod layout;
mod theme;

use bevy::prelude::*;
use bevy::window::{MonitorSelection, WindowMode};
use bevy::winit::WinitSettings;
use bevy_egui::EguiPlugin;

use crate::builder::{
    configure_gizmos, draw_stage, load_saved, nudge_selection,
    on_window_closed as component_window_closed, open_requested as open_component,
    pointer as builder_pointer, sync_preview, BuilderChrome, BuilderGizmos, ComponentPlan,
    ComponentPlannerPass, ComponentWindow, CustomLibrary,
};
use crate::cutting::{on_window_closed, open_requested, CutJob, CutPlannerPass, CutWindow};
use crate::layout::edit::{
    finish_native_dialog, orbit_camera, pointer, shortcuts, sidebar, PropertiesUi, SidebarWidth,
    ToolbarHeight,
};
use crate::layout::history::{track_history, History};
use crate::layout::model::{Editor, Room, Selection};
use crate::layout::view::{draw_guides, setup_world, sync_meshes};

/// Build the Bevy app: register every resource, add the plugins, and
/// schedule every system. See the top of this file for the frame order.
fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.73, 0.70, 0.64)))
        // Redraw on input instead of spinning the GPU while the window sits idle.
        .insert_resource(WinitSettings::desktop_app())
        .insert_resource(Room::default())
        .insert_resource(Editor::default())
        .insert_resource(Selection::default())
        .insert_resource(layout::edit::Drag::default())
        .insert_resource(PropertiesUi::default())
        .insert_resource(SidebarWidth::default())
        .insert_resource(ToolbarHeight::default())
        .insert_resource(CutJob::default())
        .insert_resource(CutWindow::default())
        .insert_resource(ComponentPlan::default())
        .insert_resource(ComponentWindow::default())
        .insert_resource(CustomLibrary::default())
        .insert_resource(BuilderChrome::default())
        .insert_resource(History::default())
        .insert_non_send(layout::project::NativeDialog::default())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Woodworking layout".into(),
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .init_gizmo_group::<BuilderGizmos>()
        .add_systems(Startup, (setup_world, load_saved, configure_gizmos))
        // Pointer runs first so a click and a camera move in the same frame
        // agree about what is selected. The camera transform is written last.
        .add_systems(
            Update,
            (
                open_requested,
                open_component,
                builder_pointer,
                nudge_selection,
                sync_preview,
                finish_native_dialog,
                pointer,
                shortcuts,
                orbit_camera,
                on_window_closed,
                component_window_closed,
            )
                .chain(),
        )
        .add_systems(
            bevy_egui::EguiPrimaryContextPass,
            (layout::dimensions::draw_widths, sidebar).chain(),
        )
        .add_systems(CutPlannerPass, cutting::ui_draw)
        .add_systems(ComponentPlannerPass, builder::ui_draw)
        // New boxes have to exist before transforms propagate, or a resized
        // cabinet would lag a frame behind its outline.
        .add_systems(PostUpdate, draw_stage)
        .add_systems(PostUpdate, sync_meshes.before(TransformSystems::Propagate))
        .add_systems(PostUpdate, track_history.after(sync_meshes))
        .add_systems(PostUpdate, draw_guides.after(TransformSystems::Propagate))
        .run();
}
