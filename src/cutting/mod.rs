//! Sheet cutting.
//!
//! This is separate from the 3D room. The room decides where a cabinet stands.
//! This module decides how its panels fit on the plywood you order.
//!
//! - [`job`] is the sheet and the parts list
//! - [`pack`] places those parts and counts the sheets
//! - [`from_room`] reads panels off the cabinets already in the room
//! - [`ui`] is the second window

mod from_room;
mod job;
mod pack;
mod ui;

use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::window::WindowRef;
use bevy_egui::EguiSchedule;

pub use job::CutJob;
pub use ui::draw as ui_draw;

/// Runs only for the cut planner window. The 3D window keeps its own pass.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CutPlannerPass;

/// Marks the cut planner's OS window, so closing it can drop the camera too.
#[derive(Component)]
pub struct CutPlannerWindow;

/// Marks the camera that egui draws the cut planner into.
#[derive(Component)]
pub struct CutPlannerCamera;

#[derive(Resource, Default)]
pub struct CutWindow {
    window: Option<Entity>,
    camera: Option<Entity>,
    requested: bool,
}

pub fn request_open(state: &mut CutWindow) {
    state.requested = true;
}

/// Spawn from `Update`, before bevy_egui begins its pass loop. Creating the
/// Egui context from inside another Egui pass leaves it without output for one
/// frame and produces an avoidable runtime error.
pub fn open_requested(
    mut commands: Commands,
    mut state: ResMut<CutWindow>,
    windows: Query<Entity, With<CutPlannerWindow>>,
) {
    if !state.requested {
        return;
    }
    state.requested = false;
    // The new window entity is not visible to this query until next frame,
    // so a second click in the same frame must not open another one.
    if let Some(entity) = state.window {
        if windows.is_empty() || windows.get(entity).is_ok() {
            return;
        }
    }
    let mut resolution = bevy::window::WindowResolution::default();
    resolution.set(1240.0, 840.0);
    let window = commands
        .spawn((
            Window {
                title: "Cut planner".into(),
                resolution,
                ..default()
            },
            CutPlannerWindow,
        ))
        .id();
    // A 2D camera keeps this window from drawing the room. EguiSchedule tells
    // bevy_egui to run CutPlannerPass for this camera only.
    let camera = commands
        .spawn((
            CutPlannerCamera,
            Camera2d,
            Camera {
                order: 1,
                clear_color: ClearColorConfig::Custom(Color::srgb(0.73, 0.70, 0.64)),
                ..default()
            },
            RenderTarget::Window(WindowRef::Entity(window)),
            EguiSchedule::new(CutPlannerPass),
        ))
        .id();
    state.window = Some(window);
    state.camera = Some(camera);
}

/// The OS window is already gone. Drop the camera that was aimed at it.
pub fn on_window_closed(
    mut removed: RemovedComponents<CutPlannerWindow>,
    mut state: ResMut<CutWindow>,
    mut commands: Commands,
) {
    for entity in removed.read() {
        if state.window == Some(entity) {
            if let Some(camera) = state.camera.take() {
                commands.entity(camera).despawn();
            }
            state.window = None;
        }
    }
}
