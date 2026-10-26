//! Moving the camera: scroll to zoom, and applying orbit / pan drags.
//!
//! The camera itself ([`OrbitCamera`]) is defined in `view.rs`. It orbits a
//! focus point at a distance, with a yaw and a pitch. This file only changes
//! those numbers in response to input.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::input::EguiWantsInput;

use super::pointer::{over_chrome, Drag, DragMode};
use super::{PropertiesUi, SidebarWidth, ToolbarHeight};
use crate::layout::view::OrbitCamera;

/// Draw the room only in the area that is not covered by the toolbar or panels.
pub fn orbit_camera(
    mut camera: Query<(&mut Transform, &mut OrbitCamera)>,
    drag: Res<Drag>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    sidebar: Res<SidebarWidth>,
    toolbar: Res<ToolbarHeight>,
    properties: Res<PropertiesUi>,
    egui: Res<EguiWantsInput>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok((mut transform, mut orbit)) = camera.single_mut() else {
        return;
    };

    let cursor_here = window
        .iter()
        .next()
        .and_then(|primary| primary.cursor_position());
    if cursor_here.is_some()
        && !over_chrome(&window, &sidebar, &toolbar, &properties)
        && !egui.wants_any_pointer_input()
    {
        let lines = match scroll.unit {
            MouseScrollUnit::Line => scroll.delta.y,
            MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
        };
        if lines.abs() > f32::EPSILON {
            let factor = (1.0 - lines * 0.06).clamp(0.72, 1.35);
            orbit.distance = (orbit.distance * factor).clamp(0.35, 80.0);
        }
    }

    match &drag.mode {
        DragMode::Orbit => {
            orbit.yaw -= motion.delta.x * 0.005;
            orbit.pitch = (orbit.pitch - motion.delta.y * 0.005).clamp(0.08, 1.45);
        }
        DragMode::Pan => {
            // Grab the room: the point under the cursor follows the cursor.
            let scale = orbit.distance * 0.0009;
            let right = transform.rotation * Vec3::X;
            let up = transform.rotation * Vec3::Y;
            orbit.focus -= right * motion.delta.x * scale;
            // Motion Y grows downward, and camera-up is toward the top of the screen.
            orbit.focus -= up * motion.delta.y * scale;
        }
        DragMode::Idle | DragMode::Move { .. } => {}
    }

    *transform = orbit.pose();
}

/// Slide the camera the way a right-drag does: left and right across the
/// view, up and down toward the top and bottom of the screen.
pub(super) fn pan_camera(orbit: &mut OrbitCamera, keys: &ButtonInput<KeyCode>) {
    let mut x = 0.0;
    let mut y = 0.0;
    if keys.just_pressed(KeyCode::ArrowLeft) {
        x -= 1.0;
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        x += 1.0;
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        y += 1.0;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        y -= 1.0;
    }
    if x == 0.0 && y == 0.0 {
        return;
    }
    let pose = orbit.pose();
    let right = pose.rotation * Vec3::X;
    let up = pose.rotation * Vec3::Y;
    let step = (orbit.distance * 0.08).clamp(0.05, 2.0);
    orbit.focus += right * x * step + up * y * step;
}
