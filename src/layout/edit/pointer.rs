//! Mouse handling in the 3D view: clicking, selecting, and dragging pieces.
//!
//! A press decides what the drag will be, and stores it in [`Drag`]:
//!
//! | Press                                   | Drag mode                |
//! |-----------------------------------------|--------------------------|
//! | Left on a piece                         | Move the selection       |
//! | Left on empty space                     | Orbit (click = deselect) |
//! | Right                                   | Pan (click = menu)       |
//! | Middle, or Alt + left                   | Orbit                    |
//! | Shift + middle, or Space + left         | Pan                      |
//!
//! While the button is held, a Move drag slides the pieces across the floor
//! (or up/down with Shift). Orbit and Pan are applied to the camera in
//! `camera.rs`, which reads the same [`Drag`] resource.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::input::EguiWantsInput;

use super::{PropertiesUi, SidebarWidth, ToolbarHeight};
use crate::layout::model::{
    pick_bounds, ray_aabb, ray_horizontal_plane, snap_to_step, Editor, Furniture, Room, Selection,
};
use crate::layout::view::OrbitCamera;

// ---------------------------------------------------------------------------
// Drag state
// ---------------------------------------------------------------------------

/// The mouse drag in progress in the 3D view, from press to release.
#[derive(Resource, Default)]
pub struct Drag {
    pub(super) mode: DragMode,
    button: Option<MouseButton>,
    moved_px: f32,
    /// A click on empty space clears the selection. A drag that starts there
    /// is an orbit, and should leave the selection alone.
    deselect_if_click: bool,
    context_candidate: Option<(Entity, Vec2)>,
}

/// What the current drag does. `Move` remembers where every selected piece
/// started, so the pieces move by the cursor's travel and keep their spacing.
#[derive(Default)]
pub(super) enum DragMode {
    #[default]
    Idle,
    Orbit,
    Pan,
    Move {
        anchor_start: Vec3,
        hit_start: Vec3,
        plane_y: f32,
        vertical_offset: f32,
        starts: Vec<(Entity, Vec3)>,
    },
}

impl Drag {
    /// True from the press until the button is released.
    fn mode_is_active(&self) -> bool {
        !matches!(self.mode, DragMode::Idle)
    }

    /// Back to idle.
    fn reset(&mut self) {
        *self = Self::default();
    }
}

/// System: start, continue, and finish mouse drags in the 3D view.
///
/// The first half handles a drag already in progress (including the click
/// that ends it). The second half handles a fresh press and decides which
/// [`DragMode`] it starts.
pub fn pointer(
    mut drag: ResMut<Drag>,
    mut selection: ResMut<Selection>,
    mut properties: ResMut<PropertiesUi>,
    mut pieces: Query<(Entity, &Furniture, &mut Transform, &GlobalTransform)>,
    camera: Query<(&Camera, &GlobalTransform, &OrbitCamera)>,
    window: Query<&Window, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    motion: Res<AccumulatedMouseMotion>,
    editor: Res<Editor>,
    room: Res<Room>,
    egui: Res<EguiWantsInput>,
    sidebar: Res<SidebarWidth>,
    toolbar: Res<ToolbarHeight>,
) {
    let primary_cursor = window
        .iter()
        .next()
        .and_then(|primary| primary.cursor_position());
    if drag.mode_is_active() {
        let Some(button) = drag.button else {
            drag.reset();
            return;
        };
        if !mouse.pressed(button) {
            // A click that dismisses the color picker must not also clear the selection.
            if drag.deselect_if_click && drag.moved_px < 5.0 && !egui.is_popup_open() {
                selection.clear();
            }
            if button == MouseButton::Right && drag.moved_px < 5.0 {
                if let Some((entity, position)) = drag.context_candidate {
                    selection.set_only(entity);
                    properties.context_target = Some(entity);
                    properties.context_position = position;
                }
            }
            drag.reset();
            return;
        }
        drag.moved_px += motion.delta.length();
        if let DragMode::Move {
            anchor_start,
            hit_start,
            plane_y,
            vertical_offset,
            starts,
        } = &mut drag.mode
        {
            let distance = camera
                .iter()
                .next()
                .map(|(_, _, orbit)| orbit.distance)
                .unwrap_or(6.0);
            let vertical = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
            let delta = if vertical {
                // Screen-up raises the piece. Mouse Y grows downward.
                *vertical_offset += -motion.delta.y * 0.0022 * distance.max(1.0);
                let minimum_y = starts
                    .iter()
                    .map(|(_, start)| start.y)
                    .fold(f32::MAX, f32::min);
                let snapped =
                    snap_to_step(anchor_start.y + *vertical_offset, editor.snap_step_meters());
                Vec3::Y * (snapped - anchor_start.y).max(-minimum_y)
            } else if let Some((origin, direction)) = cursor_ray(&window, &camera) {
                let Some(hit) = ray_horizontal_plane(origin, direction, *plane_y) else {
                    return;
                };
                let desired = *anchor_start + (hit - *hit_start);
                Vec3::new(
                    snap_to_step(desired.x, editor.snap_step_meters()) - anchor_start.x,
                    0.0,
                    snap_to_step(desired.z, editor.snap_step_meters()) - anchor_start.z,
                )
            } else {
                return;
            };
            for (entity, start) in starts.iter().copied() {
                if let Ok((_, _, mut transform, _)) = pieces.get_mut(entity) {
                    transform.translation = start + delta;
                }
            }
        }
        return;
    }

    // A click in the component planner must not also grab the room. The
    // primary window has no cursor while the pointer is over the other window.
    if primary_cursor.is_none() {
        return;
    }

    // The stage is only the rectangle beside the panels. A click or drag on
    // the toolbar, sidebar, Properties, or an open popup stays with egui.
    if over_chrome(&window, &sidebar, &toolbar, &properties) || egui.wants_any_pointer_input() {
        return;
    }

    let button = if mouse.just_pressed(MouseButton::Left) {
        MouseButton::Left
    } else if mouse.just_pressed(MouseButton::Right) {
        MouseButton::Right
    } else if mouse.just_pressed(MouseButton::Middle) {
        MouseButton::Middle
    } else {
        return;
    };

    let space = keys.pressed(KeyCode::Space);
    let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    drag.button = Some(button);
    drag.moved_px = 0.0;
    drag.deselect_if_click = false;
    drag.context_candidate = None;

    if button == MouseButton::Right {
        if let Some((origin, direction)) = cursor_ray(&window, &camera) {
            if let Some((entity, _)) = pick_piece(origin, direction, &pieces, room.edit_enabled) {
                let position = window
                    .iter()
                    .next()
                    .and_then(|window| window.cursor_position())
                    .unwrap_or_default();
                drag.context_candidate = Some((entity, position));
            }
        }
        drag.mode = DragMode::Pan;
        return;
    }
    if (button == MouseButton::Middle && shift) || (button == MouseButton::Left && space) {
        drag.mode = DragMode::Pan;
        return;
    }
    if button == MouseButton::Middle || alt {
        drag.mode = DragMode::Orbit;
        return;
    }

    let Some((origin, direction)) = cursor_ray(&window, &camera) else {
        drag.reset();
        return;
    };
    if let Some((entity, transform)) = pick_piece(origin, direction, &pieces, room.edit_enabled) {
        let additive = keys.pressed(KeyCode::ControlLeft)
            || keys.pressed(KeyCode::ControlRight)
            || keys.pressed(KeyCode::SuperLeft)
            || keys.pressed(KeyCode::SuperRight);
        if additive {
            selection.toggle(entity);
            drag.reset();
            return;
        }
        if !selection.contains(entity) {
            selection.set_only(entity);
        } else {
            selection.primary = Some(entity);
        }
        let hit = ray_horizontal_plane(origin, direction, transform.translation.y)
            .unwrap_or(transform.translation);
        let starts = selection
            .entities
            .iter()
            .filter_map(|selected| {
                pieces
                    .get(*selected)
                    .ok()
                    .filter(|(_, furniture, _, _)| room.edit_enabled || !furniture.kind.is_pipe())
                    .map(|(_, _, transform, _)| (*selected, transform.translation))
            })
            .collect();
        drag.mode = DragMode::Move {
            anchor_start: transform.translation,
            hit_start: hit,
            plane_y: transform.translation.y,
            vertical_offset: 0.0,
            starts,
        };
    } else {
        drag.mode = DragMode::Orbit;
        drag.deselect_if_click = true;
    }
}

/// True when the cursor is over the toolbar, the sidebar, the Properties
/// panel, or the right-click menu.
pub(super) fn over_chrome(
    window: &Query<&Window, With<PrimaryWindow>>,
    sidebar: &SidebarWidth,
    toolbar: &ToolbarHeight,
    properties: &PropertiesUi,
) -> bool {
    window
        .iter()
        .next()
        .and_then(|window| window.cursor_position())
        .is_some_and(|cursor| {
            cursor.x < sidebar.0
                || cursor.y < toolbar.0
                || properties
                    .menu_rect
                    .is_some_and(|menu| menu.contains(cursor))
        })
}

/// World-space ray through the cursor, as (origin, normalized direction).
fn cursor_ray(
    window: &Query<&Window, With<PrimaryWindow>>,
    camera: &Query<(&Camera, &GlobalTransform, &OrbitCamera)>,
) -> Option<(Vec3, Vec3)> {
    let cursor = window.iter().next()?.cursor_position()?;
    let (camera, transform, _) = camera.iter().next()?;
    let ray = camera.viewport_to_world(transform, cursor).ok()?;
    let direction = Vec3::new(ray.direction.x, ray.direction.y, ray.direction.z);
    let direction = direction.normalize_or_zero();
    if direction.length_squared() < 0.5 {
        None
    } else {
        Some((ray.origin, direction))
    }
}

/// Closest piece whose box contains the ray. The test is in the piece's local
/// space, so a cabinet that has been turned 90° still selects on its real faces.
fn pick_piece(
    origin: Vec3,
    direction: Vec3,
    pieces: &Query<(Entity, &Furniture, &mut Transform, &GlobalTransform)>,
    allow_pipes: bool,
) -> Option<(Entity, Transform)> {
    let mut best_distance = f32::MAX;
    let mut best: Option<(Entity, Transform)> = None;
    for (entity, furniture, transform, global) in pieces.iter() {
        if furniture.kind.is_pipe() && !allow_pipes {
            continue;
        }
        let (min, max) = pick_bounds(furniture);
        let inverse = global.affine().inverse();
        let local_origin = inverse.transform_point3(origin);
        let local_direction = inverse.transform_vector3(direction).normalize_or_zero();
        if local_direction.length_squared() < 0.5 {
            continue;
        }
        if let Some(distance) = ray_aabb(local_origin, local_direction, min, max) {
            if distance < best_distance {
                best_distance = distance;
                best = Some((entity, *transform));
            }
        }
    }
    best
}
