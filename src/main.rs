//! A small 3D layout tool for kitchens and other rooms, plus a sheet cutter.
//!
//! Run it with `cargo run`. The toolbar shows the open file and, in the
//! center, the cut planner and the component planner. The sidebar holds the
//! room, the furniture, and the selection.
//!
//! Where the code lives:
//!
//! - `layout` — the 3D room: data, drawing, and the sidebar
//! - `cutting` — the separate window that nests parts on plywood
//! - `builder` — the sketch window for a custom furniture block
//! - `theme` — the shared colors, buttons, and section headers

mod builder;
mod cutting;
mod layout;
mod theme;

use bevy::prelude::*;
use bevy::winit::WinitSettings;
use bevy_egui::EguiPlugin;

use crate::builder::{
    on_window_closed as component_window_closed, open_requested as open_component,
    ComponentPlan, ComponentPlannerPass, ComponentWindow,
};
use crate::cutting::{on_window_closed, open_requested, CutJob, CutPlannerPass, CutWindow};
use crate::layout::edit::{
    finish_native_dialog, orbit_camera, pointer, shortcuts, sidebar, PropertiesUi, SidebarWidth,
    ToolbarHeight,
};
use crate::layout::history::{track_history, History};
use crate::layout::model::{Editor, Room, Selection};
use crate::layout::view::{draw_guides, setup_world, sync_meshes};

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
        .insert_resource(History::default())
        .insert_non_send(layout::project::NativeDialog::default())
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
        .add_systems(
            Update,
            (
                open_requested,
                open_component,
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
        .add_systems(PostUpdate, sync_meshes.before(TransformSystems::Propagate))
        .add_systems(PostUpdate, track_history.after(sync_meshes))
        .add_systems(PostUpdate, draw_guides.after(TransformSystems::Propagate))
        .run();
}
