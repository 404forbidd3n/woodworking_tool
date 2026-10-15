//! A small 3D layout tool for kitchens and other rooms.
//!
//! Run it with `cargo run`. The toolbar shows the open file. The sidebar
//! holds room size, the six piece types, and millimeters for the selection.
//! Flatma-style cut lists, materials, and drilling plans are intentionally
//! not here.
//!
//! The code is split so each file has one job:
//!
//! - `model` — millimeters, cabinet types, and how a cabinet is broken into boxes
//! - `view` — camera, room shell, meshes, grid
//! - `edit` — mouse, keyboard, sidebar, saving `layout.json`

mod edit;
mod model;
mod view;

use bevy::prelude::*;
use bevy::winit::WinitSettings;
use bevy_egui::EguiPlugin;

use crate::edit::{
    orbit_camera, pointer, shortcuts, sidebar, PropertiesUi, SidebarWidth, ToolbarHeight,
};
use crate::model::{Editor, Room, Selection};
use crate::view::{draw_guides, setup_world, sync_meshes};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.73, 0.70, 0.64)))
        // Redraw on input instead of spinning the GPU while the window sits idle.
        .insert_resource(WinitSettings::desktop_app())
        .insert_resource(Room::default())
        .insert_resource(Editor::default())
        .insert_resource(Selection::default())
        .insert_resource(edit::Drag::default())
        .insert_resource(PropertiesUi::default())
        .insert_resource(SidebarWidth::default())
        .insert_resource(ToolbarHeight::default())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Woodworking layout".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, setup_world)
        // Pointer runs first so a click and a camera move in the same frame
        // agree about what is selected. The camera transform is written last.
        .add_systems(Update, (pointer, shortcuts, orbit_camera).chain())
        .add_systems(bevy_egui::EguiPrimaryContextPass, sidebar)
        // New boxes have to exist before transforms propagate, or a resized
        // cabinet would lag a frame behind its outline.
        .add_systems(PostUpdate, sync_meshes.before(TransformSystems::Propagate))
        .add_systems(PostUpdate, draw_guides.after(TransformSystems::Propagate))
        .run();
}
