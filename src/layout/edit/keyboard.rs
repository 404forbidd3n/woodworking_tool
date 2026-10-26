//! Keyboard shortcuts in the 3D view.
//!
//! With something selected:
//!
//! - Delete / Backspace: remove it
//! - R: turn it a quarter turn
//! - D: duplicate it into the middle of the room
//! - F: frame the camera on it
//! - Arrow keys: nudge it by one snap step (Page Up / Down for height)
//! - Ctrl + arrows: pan the camera instead
//!
//! With nothing selected, F frames the whole room and the arrows pan.
//!
//! Undo, redo, and save (Cmd+Z / Cmd+Shift+Z / Cmd+S) are read by egui in
//! `sidebar.rs` instead, because egui consumes those keys first.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::input::EguiWantsInput;

use super::camera::pan_camera;
use super::pointer::over_chrome;
use super::spawn::place_at_center;
use super::{PropertiesUi, SidebarWidth, ToolbarHeight};
use crate::layout::model::{
    m_to_mm, mm_to_m, quarter_turns, rotation_from_turns, Editor, Furniture, Room, Selection,
};
use crate::layout::view::OrbitCamera;

/// System: the keyboard shortcuts listed at the top of this file.
pub fn shortcuts(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<Selection>,
    mut pieces: Query<(Entity, &Furniture, &mut Transform)>,
    mut camera: Query<&mut OrbitCamera>,
    room: Res<Room>,
    editor: Res<Editor>,
    egui: Res<EguiWantsInput>,
    sidebar: Res<SidebarWidth>,
    toolbar: Res<ToolbarHeight>,
    properties: Res<PropertiesUi>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    // Number fields use the arrow keys. Don't also nudge a cabinet.
    // Keys typed in the component planner belong to that window.
    let main_focused = window.iter().next().is_some_and(|window| window.focused);
    if !main_focused
        || egui.wants_any_keyboard_input()
        || over_chrome(&window, &sidebar, &toolbar, &properties)
    {
        return;
    }

    selection.retain(|entity| {
        pieces
            .get(entity)
            .is_ok_and(|(_, furniture, _)| room.edit_enabled || !furniture.kind.is_pipe())
    });
    if selection.is_empty() {
        if keys.just_pressed(KeyCode::KeyF) {
            if let Ok(mut orbit) = camera.single_mut() {
                orbit.frame_room(&room);
            }
        }
        if let Ok(mut orbit) = camera.single_mut() {
            pan_camera(&mut orbit, &keys);
        }
        return;
    }

    if keys.just_pressed(KeyCode::Delete) || keys.just_pressed(KeyCode::Backspace) {
        for entity in selection.entities.drain(..) {
            commands.entity(entity).despawn();
        }
        selection.primary = None;
        return;
    }

    if keys.just_pressed(KeyCode::KeyR) {
        for entity in selection.entities.iter().copied() {
            if let Ok((_, _, mut transform)) = pieces.get_mut(entity) {
                let turns = (quarter_turns(transform.rotation) + 1) % 4;
                transform.rotation = rotation_from_turns(turns);
            }
        }
    }

    if keys.just_pressed(KeyCode::KeyD) {
        let mut copies = Vec::new();
        for entity in selection.entities.iter().copied() {
            if let Ok((_, furniture, transform)) = pieces.get(entity) {
                let mut copy = furniture.clone();
                copy.dirty = true;
                copy.name = format!("{} copy", copy.name);
                let placed = place_at_center(
                    &room,
                    m_to_mm(transform.translation.y),
                    quarter_turns(transform.rotation),
                );
                copies.push(commands.spawn((copy, placed)).id());
            }
        }
        if !copies.is_empty() {
            selection.entities = copies;
            selection.primary = selection.entities.last().copied();
        }
    }

    if keys.just_pressed(KeyCode::KeyF) {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for entity in selection.entities.iter().copied() {
            if let Ok((_, furniture, transform)) = pieces.get(entity) {
                let size = furniture.size_meters();
                min = min
                    .min(transform.translation - Vec3::X * size.x * 0.5 - Vec3::Z * size.z * 0.5);
                max =
                    max.max(transform.translation + Vec3::new(size.x * 0.5, size.y, size.z * 0.5));
            }
        }
        if min.x < f32::MAX {
            let size = max - min;
            if let Ok(mut orbit) = camera.single_mut() {
                orbit.frame_piece(
                    Vec3::new((min.x + max.x) * 0.5, min.y, (min.z + max.z) * 0.5),
                    size,
                );
            }
        }
    }

    if control_held(&keys) {
        if let Ok(mut orbit) = camera.single_mut() {
            pan_camera(&mut orbit, &keys);
        }
        return;
    }

    let step = if editor.snap_enabled {
        mm_to_m(editor.snap_mm.max(1.0))
    } else {
        0.01
    };
    for entity in selection.entities.iter().copied() {
        let Ok((_, _, mut transform)) = pieces.get_mut(entity) else {
            continue;
        };
        // On a plan, "up" is toward the back wall, which is -Z.
        if keys.just_pressed(KeyCode::ArrowLeft) {
            transform.translation.x -= step;
        }
        if keys.just_pressed(KeyCode::ArrowRight) {
            transform.translation.x += step;
        }
        if keys.just_pressed(KeyCode::ArrowUp) {
            transform.translation.z -= step;
        }
        if keys.just_pressed(KeyCode::ArrowDown) {
            transform.translation.z += step;
        }
        if keys.just_pressed(KeyCode::PageUp) {
            transform.translation.y = (transform.translation.y + step).max(0.0);
        }
        if keys.just_pressed(KeyCode::PageDown) {
            transform.translation.y = (transform.translation.y - step).max(0.0);
        }
    }
}

/// Either Control key is down.
fn control_held(keys: &ButtonInput<KeyCode>) -> bool {
    keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)
}
