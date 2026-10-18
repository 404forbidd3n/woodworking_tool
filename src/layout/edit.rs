//! Selecting, moving, and the sidebar.
//!
//! Left drag does the thing under the cursor: a piece moves, empty space
//! orbits. That keeps a trackpad usable, because orbit does not need a middle
//! button. Hold Shift during a move to raise or lower the piece. Exact
//! millimeters, door counts, and saving live in the sidebar.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, RequestRedraw};
use bevy_egui::input::EguiWantsInput;
use bevy_egui::{egui, EguiContexts};

use crate::cutting::{self, CutJob, CutWindow};
use crate::theme::{self, dropdown, toolbar_frame};

use crate::layout::history::History;
use crate::layout::model::{
    example_layout, m_to_mm, mm_to_m, pick_bounds, place, quarter_turns, ray_aabb,
    ray_horizontal_plane, read_project, resolve_layout_path, rotation_from_turns,
    sanitize_openings, snap_to_step, write_project, Editor, Finish, FinishPreset, Furniture, Kind,
    LegStyle, Opening, OpeningKind, PipeColor, Placement, ProjectFile, ReadError, Room, Selection,
    WallSide, PIPE_PAIR_GAP_MM,
};
use crate::layout::project::{self, DialogRequest, NativeDialog};
use crate::layout::view::{self, OrbitCamera};

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

#[derive(Resource, Default)]
pub struct Drag {
    mode: DragMode,
    button: Option<MouseButton>,
    moved_px: f32,
    /// A click on empty space clears the selection. A drag that starts there
    /// is an orbit, and should leave the selection alone.
    deselect_if_click: bool,
    context_candidate: Option<(Entity, Vec2)>,
}

#[derive(Default)]
enum DragMode {
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

#[derive(Resource, Default)]
pub struct PropertiesUi {
    pub context_target: Option<Entity>,
    pub context_position: Vec2,
    /// Screen rect of the right-click menu, so a click on it does not also
    /// reach the 3D view behind it.
    pub menu_rect: Option<Rect>,
    /// While set, the docked Properties panel shows the primary selection.
    pub open: bool,
}

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

impl Drag {
    fn mode_is_active(&self) -> bool {
        !matches!(self.mode, DragMode::Idle)
    }

    fn reset(&mut self) {
        *self = Self::default();
    }
}

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
    if egui.wants_any_keyboard_input() || over_chrome(&window, &sidebar, &toolbar, &properties) {
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
                let mut placed = *transform;
                placed.translation.x += 0.1;
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

    if !over_chrome(&window, &sidebar, &toolbar, &properties) && !egui.wants_any_pointer_input() {
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

pub fn sidebar(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut pieces: Query<(Entity, &mut Furniture, &mut Transform)>,
    mut selection: ResMut<Selection>,
    mut room: ResMut<Room>,
    mut editor: ResMut<Editor>,
    mut properties: ResMut<PropertiesUi>,
    mut camera: Query<&mut OrbitCamera>,
    mut sidebar_width: ResMut<SidebarWidth>,
    mut toolbar_height: ResMut<ToolbarHeight>,
    mut cut_job: ResMut<CutJob>,
    mut cut_window: ResMut<CutWindow>,
    mut history: ResMut<History>,
    mut dialogs: NonSendMut<NativeDialog>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    theme::apply_theme(ctx);
    // Cmd+S is read here so it still saves when Project is collapsed.
    let save_shortcut =
        ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::S));
    let redo_modifiers = egui::Modifiers {
        command: true,
        shift: true,
        ..default()
    };
    let redo_shortcut = ctx.input_mut(|input| {
        input.consume_key(redo_modifiers, egui::Key::Z)
            || input.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)
    });
    let undo_shortcut = !redo_shortcut
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::Z));
    if save_shortcut {
        save_layout(&mut editor, &room, &pieces, &cut_job);
    }
    if undo_shortcut {
        perform_history(
            false,
            &mut history,
            &mut commands,
            &mut room,
            &mut editor,
            &mut selection,
            &pieces,
        );
    } else if redo_shortcut {
        perform_history(
            true,
            &mut history,
            &mut commands,
            &mut room,
            &mut editor,
            &mut selection,
            &pieces,
        );
    }

    // egui 0.36 draws panels into a root Ui instead of straight onto the context.
    // The toolbar is added first, so it spans the full width above the sidebar.
    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    let toolbar = egui::Panel::top("toolbar")
        .frame(toolbar_frame())
        .show(&mut viewport_ui, |ui| {
            draw_toolbar(
                ui,
                &mut commands,
                &mut room,
                &mut editor,
                &mut selection,
                &pieces,
                &mut cut_job,
                &mut cut_window,
                &mut history,
                &mut dialogs,
            );
        });
    toolbar_height.0 = toolbar.response.rect.height();

    let panel = egui::Panel::left("tools")
        .resizable(true)
        .default_size(300.0)
        .frame(theme::sidebar_frame())
        .show(&mut viewport_ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                dropdown(ui, "View", false, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Corner").clicked() {
                            if let Ok(mut orbit) = camera.single_mut() {
                                orbit.corner_view();
                            }
                        }
                        if ui.button("Front").clicked() {
                            if let Ok(mut orbit) = camera.single_mut() {
                                orbit.front_view();
                            }
                        }
                        if ui.button("Top").clicked() {
                            if let Ok(mut orbit) = camera.single_mut() {
                                orbit.top_view();
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Frame room").clicked() {
                            if let Ok(mut orbit) = camera.single_mut() {
                                orbit.frame_room(&room);
                            }
                        }
                        if ui.button("Frame piece").clicked() {
                            if let Some(entity) = selection.primary {
                                if let Ok((_, furniture, transform)) = pieces.get(entity) {
                                    let base = transform.translation;
                                    let size = furniture.size_meters();
                                    if let Ok(mut orbit) = camera.single_mut() {
                                        orbit.frame_piece(base, size);
                                    }
                                }
                            }
                        }
                    });
                    ui.separator();
                    ui.checkbox(&mut editor.snap_enabled, "Snap moves to a grid");
                    mm_row(ui, "Step", &mut editor.snap_mm, 1.0..=500.0);
                });

                dropdown(ui, "Room", true, |ui| {
                    let mut show_walls = room.show_walls;
                    if ui.checkbox(&mut show_walls, "Show walls").changed() {
                        room.show_walls = show_walls;
                        room.dirty = true;
                    }
                    ui.checkbox(&mut room.edit_enabled, "Enable room editing");
                    ui.checkbox(&mut room.show_measurements, "Show room measurements");
                    ui.add_enabled_ui(room.show_walls, |ui| {
                        for side in WallSide::ALL {
                            let mut visible = room.walls.visible(side);
                            if ui.checkbox(&mut visible, side.label()).changed() {
                                room.walls.set(side, visible);
                                room.dirty = true;
                            }
                        }
                    });
                    ui.weak("Hide a wall to look through that side.");
                    ui.add_enabled_ui(room.edit_enabled, |ui| {
                        let mut width = room.width_mm;
                        let mut depth = room.depth_mm;
                        let mut height = room.wall_height_mm;
                        if mm_row(ui, "Width", &mut width, 500.0..=30000.0)
                            | mm_row(ui, "Depth", &mut depth, 500.0..=30000.0)
                            | mm_row(ui, "Wall height", &mut height, 500.0..=6000.0)
                        {
                            room.width_mm = width;
                            room.depth_mm = depth;
                            room.wall_height_mm = height;
                            room.dirty = true;
                        }
                        ui.separator();
                        ui.strong("Doors & windows");
                        let mut remove = None;
                        let mut openings_changed = false;
                        for (index, opening) in room.openings.iter_mut().enumerate() {
                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    egui::ComboBox::from_id_salt(("opening_kind", index))
                                        .selected_text(match opening.kind {
                                            OpeningKind::Door => "Door",
                                            OpeningKind::Window => "Window",
                                        })
                                        .show_ui(ui, |ui| {
                                            openings_changed |= ui
                                                .selectable_value(
                                                    &mut opening.kind,
                                                    OpeningKind::Door,
                                                    "Door",
                                                )
                                                .changed();
                                            openings_changed |= ui
                                                .selectable_value(
                                                    &mut opening.kind,
                                                    OpeningKind::Window,
                                                    "Window",
                                                )
                                                .changed();
                                        });
                                    egui::ComboBox::from_id_salt(("opening_wall", index))
                                        .selected_text(opening.wall.label())
                                        .show_ui(ui, |ui| {
                                            for wall in WallSide::ALL {
                                                openings_changed |= ui
                                                    .selectable_value(
                                                        &mut opening.wall,
                                                        wall,
                                                        wall.label(),
                                                    )
                                                    .changed();
                                            }
                                        });
                                    if ui.button("Remove").clicked() {
                                        remove = Some(index);
                                    }
                                });
                                openings_changed |=
                                    mm_row(ui, "Position", &mut opening.offset_mm, 0.0..=30000.0);
                                openings_changed |=
                                    mm_row(ui, "Width", &mut opening.width_mm, 100.0..=10000.0);
                                openings_changed |=
                                    mm_row(ui, "Height", &mut opening.height_mm, 100.0..=6000.0);
                                if opening.kind == OpeningKind::Window {
                                    openings_changed |= mm_row(
                                        ui,
                                        "Sill height",
                                        &mut opening.sill_mm,
                                        0.0..=5000.0,
                                    );
                                } else {
                                    opening.sill_mm = 0.0;
                                }
                            });
                        }
                        room.dirty |= openings_changed;
                        if let Some(index) = remove {
                            room.openings.remove(index);
                            room.dirty = true;
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Add door").clicked() {
                                room.openings.push(Opening::door());
                                room.dirty = true;
                            }
                            if ui.button("Add window").clicked() {
                                room.openings.push(Opening::window());
                                room.dirty = true;
                            }
                        });
                        if room.dirty {
                            sanitize_openings(
                                room.width_mm,
                                room.depth_mm,
                                room.wall_height_mm,
                                &mut room.openings,
                            );
                        }
                        ui.separator();
                        ui.strong("Utility pipes");
                        ui.weak("Drag pipes like cabinets while room editing is on.");
                        let existing = pieces.iter().len();
                        ui.horizontal(|ui| {
                            for (label, horizontal) in
                                [("Add vertical pipe", false), ("Add horizontal pipe", true)]
                            {
                                if ui.button(label).clicked() {
                                    let kind = Kind::Pipe {
                                        horizontal,
                                        count: 1,
                                        color: PipeColor::Gray,
                                    };
                                    let entity = add_piece(
                                        &mut commands,
                                        &room,
                                        kind,
                                        Placement::Floor,
                                        existing,
                                    );
                                    selection.set_only(entity);
                                }
                            }
                        });
                    });
                });

                dropdown(ui, "Add Furniture", false, |ui| {
                    let existing = pieces.iter().len();
                    ui.strong("Floor");
                    for (label, kind) in [
                        ("Drawer Base Cabinet", Kind::Drawers { count: 3 }),
                        ("Door Base Cabinet", Kind::HingedDoors { doors: 2 }),
                        ("Sliding-Door Cabinet", Kind::SlidingDoors),
                        ("Open Shelf Cabinet", Kind::OpenShelves { shelves: 3 }),
                        ("Work Table", Kind::Table),
                    ] {
                        if library_button(ui, label, kind).clicked() {
                            let entity =
                                add_piece(&mut commands, &room, kind, Placement::Floor, existing);
                            selection.set_only(entity);
                        }
                    }
                    ui.add_space(6.0);
                    ui.strong("Wall mounted");
                    for (label, kind) in [
                        ("Wall Door Cabinet", Kind::HingedDoors { doors: 2 }),
                        ("Wall Lift-Up Cabinet", Kind::LiftDoor),
                        ("Wall Drawer Cabinet", Kind::Drawers { count: 3 }),
                    ] {
                        if library_button(ui, label, kind).clicked() {
                            let entity =
                                add_piece(&mut commands, &room, kind, Placement::Wall, existing);
                            selection.set_only(entity);
                        }
                    }
                });

                dropdown(ui, "Add Surfaces", false, |ui| {
                    let existing = pieces.iter().len();
                    for (label, kind) in [
                        ("Kitchen Worktop", Kind::Countertop),
                        ("Tabletop", Kind::Tabletop),
                    ] {
                        if library_button(ui, label, kind).clicked() {
                            let entity =
                                add_piece(&mut commands, &room, kind, Placement::Floor, existing);
                            selection.set_only(entity);
                        }
                    }
                });

                dropdown(ui, "Add Appliances", false, |ui| {
                    let existing = pieces.iter().len();
                    for (label, kind, placement) in [
                        ("Refrigerator", Kind::Refrigerator, Placement::Floor),
                        ("Dishwasher", Kind::Dishwasher, Placement::Floor),
                        ("Electric Oven", Kind::ElectricOven, Placement::Floor),
                        ("Coffee Machine", Kind::CoffeeMachine, Placement::Floor),
                        ("Microwave", Kind::Microwave, Placement::Floor),
                        ("Square Kitchen Sink", Kind::SquareSink, Placement::Floor),
                        ("Round Kitchen Sink", Kind::RoundSink, Placement::Floor),
                        (
                            "Double-Bowl Kitchen Sink",
                            Kind::DoubleSink,
                            Placement::Floor,
                        ),
                        (
                            "Kitchen Faucet / Water Pipe",
                            Kind::SinkFaucet,
                            Placement::Floor,
                        ),
                        (
                            "Central Heating Boiler",
                            Kind::CentralHeatingBoiler,
                            Placement::Wall,
                        ),
                        ("Air Conditioner", Kind::AirConditioner, Placement::Wall),
                    ] {
                        if library_button(ui, label, kind).clicked() {
                            let entity = add_piece(&mut commands, &room, kind, placement, existing);
                            selection.set_only(entity);
                        }
                    }
                });

                dropdown(ui, "In the room", false, |ui| {
                    let rows: Vec<(Entity, String, bool)> = pieces
                        .iter()
                        .map(|(entity, furniture, _)| {
                            (
                                entity,
                                format!(
                                    "{}   {:.0} × {:.0} × {:.0}",
                                    furniture.label(),
                                    furniture.width_mm,
                                    furniture.height_mm,
                                    furniture.depth_mm
                                ),
                                furniture.kind.is_pipe(),
                            )
                        })
                        .collect();
                    if rows.is_empty() {
                        ui.label("Nothing yet.");
                    }
                    for (entity, label, is_pipe) in &rows {
                        let selected = selection.contains(*entity);
                        let row = ui.add_enabled(
                            room.edit_enabled || !is_pipe,
                            egui::Button::selectable(selected, label),
                        );
                        if row.clicked() {
                            selection.set_only(*entity);
                        }
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Example kitchen").clicked() {
                            despawn_all(&mut commands, &rows);
                            for (furniture, transform) in example_layout() {
                                commands.spawn((furniture, transform));
                            }
                            selection.clear();
                            if let Ok(mut orbit) = camera.single_mut() {
                                orbit.frame_room(&room);
                            }
                            editor.status = "Example kitchen placed.".into();
                        }
                        if ui.button("Clear").clicked() {
                            despawn_all(&mut commands, &rows);
                            selection.clear();
                            editor.status = "Room cleared.".into();
                        }
                    });
                });

                dropdown(ui, "Selected", false, |ui| {
                    let primary = selection.primary.and_then(|entity| pieces.get(entity).ok());
                    if let Some((_, furniture, _)) = primary {
                        ui.label(furniture.name.as_str());
                        if selection.entities.len() > 1 {
                            ui.weak(format!("{} pieces selected", selection.entities.len()));
                        }
                    } else {
                        ui.label("Click a piece, or add one.");
                    }
                    if ui.button("Open Properties").clicked() {
                        properties.open = true;
                    }
                });
            });
        });
    // Properties docks beside the sidebar, so the 3D view treats both as one strip.
    let mut chrome_right = panel.response.rect.right();
    if properties.open {
        let docked = egui::Panel::left("properties")
            .resizable(true)
            .default_size(340.0)
            .frame(theme::sidebar_frame())
            .show(&mut viewport_ui, |ui| {
                draw_properties_panel(
                    ui,
                    &mut properties,
                    &mut commands,
                    &mut pieces,
                    &mut selection,
                );
            });
        chrome_right = docked.response.rect.right();
    }
    sidebar_width.0 = chrome_right;
    draw_context_menu(ctx, &mut properties);
}

fn draw_toolbar(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    room: &mut Room,
    editor: &mut Editor,
    selection: &mut Selection,
    pieces: &Query<(Entity, &mut Furniture, &mut Transform)>,
    cut_job: &mut CutJob,
    cut_window: &mut CutWindow,
    history: &mut History,
    dialogs: &mut NativeDialog,
) {
    let cream = theme::ink();
    let muted = theme::muted();
    let error = editor.status.starts_with("Could") || editor.status.starts_with("No file");
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Woodworking")
                .strong()
                .size(16.0)
                .color(cream),
        );
        ui.add_space(6.0);
        egui::Frame::new()
            .fill(theme::chip())
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(10, 4))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(layout_label(&editor.path))
                        .strong()
                        .color(cream),
                );
            });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(history.can_redo(), egui::Button::new("Redo"))
                .clicked()
            {
                perform_history(true, history, commands, room, editor, selection, pieces);
            }
            if ui
                .add_enabled(history.can_undo(), egui::Button::new("Undo"))
                .clicked()
            {
                perform_history(false, history, commands, room, editor, selection, pieces);
            }
            if ui.button("Load").clicked() {
                begin_dialog(dialogs, editor, DialogRequest::Open);
            }
            if ui.button("Save").clicked() {
                save_layout(editor, room, pieces, cut_job);
            }
            if ui.button("Save As").clicked() {
                begin_dialog(dialogs, editor, DialogRequest::Save);
            }
            if ui.button("Cut planner").clicked() {
                cutting::request_open(cut_window);
            }
            if !editor.status.is_empty() {
                let color = if error {
                    egui::Color32::from_rgb(255, 186, 150)
                } else {
                    muted
                };
                ui.label(egui::RichText::new(editor.status.as_str()).color(color));
            }
        });
    });
}

/// The name shown in the toolbar. A file inside the project folder is shown
/// without the long folder path.
fn layout_label(path: &str) -> String {
    let resolved = resolve_layout_path(path);
    resolved
        .strip_prefix(crate::layout::model::project_dir())
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| resolved.display().to_string())
}

fn library_button(ui: &mut egui::Ui, label: &str, kind: Kind) -> egui::Response {
    let desired = egui::vec2(ui.available_width(), 36.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let visuals = ui.style().interact(&response);
    ui.painter().rect(
        rect,
        visuals.corner_radius,
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );
    let icon = egui::Rect::from_min_size(rect.min + egui::vec2(8.0, 7.0), egui::vec2(24.0, 22.0));
    paint_kind_icon(ui.painter(), icon, kind, visuals.fg_stroke);
    ui.painter().text(
        egui::pos2(rect.left() + 42.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.0),
        visuals.fg_stroke.color,
    );
    response
}

fn paint_kind_icon(painter: &egui::Painter, rect: egui::Rect, kind: Kind, stroke: egui::Stroke) {
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(2),
        stroke,
        egui::StrokeKind::Inside,
    );
    let line = |a: egui::Pos2, b: egui::Pos2| painter.line_segment([a, b], stroke);
    match kind {
        Kind::Drawers { .. } => {
            for fraction in [0.33, 0.66] {
                line(
                    egui::pos2(rect.left(), egui::lerp(rect.y_range(), fraction)),
                    egui::pos2(rect.right(), egui::lerp(rect.y_range(), fraction)),
                );
            }
        }
        Kind::HingedDoors { .. } | Kind::SlidingDoors => {
            line(
                egui::pos2(rect.center().x, rect.top()),
                egui::pos2(rect.center().x, rect.bottom()),
            );
        }
        Kind::LiftDoor => {
            line(rect.left_bottom(), rect.right_top());
        }
        Kind::Table => {
            line(
                egui::pos2(rect.left() + 4.0, rect.top() + 6.0),
                egui::pos2(rect.left() + 4.0, rect.bottom()),
            );
            line(
                egui::pos2(rect.right() - 4.0, rect.top() + 6.0),
                egui::pos2(rect.right() - 4.0, rect.bottom()),
            );
        }
        Kind::Countertop | Kind::Tabletop => {
            line(
                egui::pos2(rect.left(), rect.top() + 6.0),
                egui::pos2(rect.right(), rect.top() + 6.0),
            );
        }
        Kind::Pipe { horizontal, .. } => {
            if horizontal {
                line(rect.left_center(), rect.right_center());
            } else {
                line(rect.center_top(), rect.center_bottom());
            }
        }
        Kind::ElectricOven | Kind::Microwave | Kind::CoffeeMachine => {
            painter.rect_stroke(
                rect.shrink(5.0),
                egui::CornerRadius::same(1),
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        Kind::SquareSink => {
            painter.rect_stroke(
                rect.shrink(5.0),
                egui::CornerRadius::same(2),
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        Kind::RoundSink => {
            painter.circle_stroke(
                rect.center(),
                rect.width().min(rect.height()) * 0.32,
                stroke,
            );
        }
        Kind::DoubleSink => {
            let left = egui::Rect::from_min_max(
                rect.min + egui::vec2(3.0, 5.0),
                egui::pos2(rect.center().x - 1.0, rect.bottom() - 5.0),
            );
            let right = egui::Rect::from_min_max(
                egui::pos2(rect.center().x + 1.0, rect.top() + 5.0),
                rect.max - egui::vec2(3.0, 5.0),
            );
            painter.rect_stroke(left, 1.0, stroke, egui::StrokeKind::Inside);
            painter.rect_stroke(right, 1.0, stroke, egui::StrokeKind::Inside);
        }
        Kind::SinkFaucet => {
            line(rect.center_bottom(), rect.center_top());
            line(
                rect.center_top(),
                egui::pos2(rect.right() - 3.0, rect.top()),
            );
            line(
                egui::pos2(rect.right() - 3.0, rect.top()),
                egui::pos2(rect.right() - 3.0, rect.center().y),
            );
        }
        Kind::CentralHeatingBoiler => {
            painter.rect_stroke(
                rect.shrink(4.0),
                egui::CornerRadius::same(2),
                stroke,
                egui::StrokeKind::Inside,
            );
            line(
                egui::pos2(rect.left() + 6.0, rect.bottom() - 5.0),
                egui::pos2(rect.right() - 6.0, rect.bottom() - 5.0),
            );
        }
        Kind::AirConditioner => {
            line(
                egui::pos2(rect.left() + 3.0, rect.bottom() - 6.0),
                egui::pos2(rect.right() - 3.0, rect.bottom() - 6.0),
            );
            line(
                egui::pos2(rect.left() + 5.0, rect.bottom() - 3.0),
                egui::pos2(rect.right() - 5.0, rect.bottom() - 3.0),
            );
        }
        Kind::Refrigerator => {
            line(
                egui::pos2(rect.left(), rect.center().y),
                egui::pos2(rect.right(), rect.center().y),
            );
        }
        Kind::Dishwasher => {
            line(
                egui::pos2(rect.left() + 3.0, rect.top() + 5.0),
                egui::pos2(rect.right() - 3.0, rect.top() + 5.0),
            );
        }
        Kind::OpenShelves { shelves } => {
            for index in 1..=shelves {
                let fraction = index as f32 / (shelves as f32 + 1.0);
                line(
                    egui::pos2(rect.left(), egui::lerp(rect.y_range(), fraction)),
                    egui::pos2(rect.right(), egui::lerp(rect.y_range(), fraction)),
                );
            }
        }
    }
}

/// The docked editor. It follows the selection, so clicking another piece in
/// the room switches what the panel edits without reopening it.
fn draw_properties_panel(
    ui: &mut egui::Ui,
    properties: &mut PropertiesUi,
    commands: &mut Commands,
    pieces: &mut Query<(Entity, &mut Furniture, &mut Transform)>,
    selection: &mut Selection,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Properties")
                .strong()
                .size(16.0)
                .color(theme::ink()),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Close").clicked() {
                properties.open = false;
            }
        });
    });
    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt("properties_scroll")
        .show(ui, |ui| {
            let Some(target) = selection
                .primary
                .filter(|entity| pieces.get(*entity).is_ok())
            else {
                ui.label("Select a piece to edit it here.");
                return;
            };
            if selection.entities.len() > 1 {
                ui.weak(format!(
                    "{} pieces selected. Editing the last one clicked.",
                    selection.entities.len()
                ));
                ui.separator();
            }
            draw_properties(ui, commands, pieces, selection, target);
        });
}

/// Small menu at the cursor after a right-click on a piece. A press anywhere
/// outside it closes it.
fn draw_context_menu(ctx: &egui::Context, properties: &mut PropertiesUi) {
    if properties.context_target.is_none() {
        properties.menu_rect = None;
        return;
    }
    let shown = egui::Area::new("piece_context_menu".into())
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(
            properties.context_position.x,
            properties.context_position.y,
        ))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .show(ui, |ui| ui.button("Properties").clicked())
                .inner
        });
    let rect = shown.response.rect;
    let pressed_outside = ctx.input(|input| {
        input.pointer.any_pressed()
            && input
                .pointer
                .interact_pos()
                .is_some_and(|position| !rect.contains(position))
    });
    if shown.inner {
        properties.open = true;
    }
    if shown.inner || pressed_outside {
        properties.context_target = None;
        properties.menu_rect = None;
    } else {
        properties.menu_rect = Some(Rect::new(rect.min.x, rect.min.y, rect.max.x, rect.max.y));
    }
}

/// Returns true when the number was edited. The value is already clamped.
fn mm_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) -> bool {
    ui.horizontal(|ui| {
        ui.add_sized([92.0, 18.0], egui::Label::new(label));
        ui.add(
            egui::DragValue::new(value)
                .speed(5.0)
                .range(range)
                .suffix(" mm"),
        )
    })
    .inner
    .changed()
}

fn add_piece(
    commands: &mut Commands,
    room: &Room,
    kind: Kind,
    placement: Placement,
    already_there: usize,
) -> Entity {
    let (furniture, y_mm) = kind.preset(placement);
    // Step new pieces sideways so a second cabinet does not hide inside the first.
    let column = (already_there % 4) as f32;
    let x_mm = room.width_mm * 0.5 + (column - 1.5) * 180.0;
    let z_mm = if kind.is_pipe() {
        furniture.depth_mm * 0.5 + 10.0
    } else {
        room.depth_mm * 0.55
    };
    commands.spawn((furniture, place(x_mm, y_mm, z_mm, 0))).id()
}

fn draw_pipe_size(ui: &mut egui::Ui, furniture: &mut Furniture) {
    let Kind::Pipe {
        horizontal, count, ..
    } = furniture.kind
    else {
        return;
    };
    let mut length = if horizontal {
        furniture.width_mm
    } else {
        furniture.height_mm
    };
    let mut diameter = furniture.depth_mm;
    if mm_row(ui, "Length", &mut length, 100.0..=10000.0) {
        if horizontal {
            furniture.width_mm = length;
        } else {
            furniture.height_mm = length;
        }
        furniture.dirty = true;
    }
    if mm_row(ui, "Diameter", &mut diameter, 8.0..=200.0) {
        furniture.depth_mm = diameter;
        if count < 2 {
            if horizontal {
                furniture.height_mm = diameter;
            } else {
                furniture.width_mm = diameter;
            }
        } else if horizontal {
            furniture.height_mm = furniture.height_mm.max(diameter + PIPE_PAIR_GAP_MM);
        } else {
            furniture.width_mm = furniture.width_mm.max(diameter + PIPE_PAIR_GAP_MM);
        }
        furniture.dirty = true;
    }
}

fn draw_properties(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    pieces: &mut Query<(Entity, &mut Furniture, &mut Transform)>,
    selection: &mut Selection,
    entity: Entity,
) {
    let Ok((_, mut furniture, mut transform)) = pieces.get_mut(entity) else {
        return;
    };
    ui.strong(furniture.label());
    ui.horizontal(|ui| {
        ui.label("Name");
        ui.text_edit_singleline(&mut furniture.name);
    });

    if furniture.kind.is_pipe() {
        draw_pipe_size(ui, &mut furniture);
    } else {
        let mut width = furniture.width_mm;
        let mut height = furniture.height_mm;
        let mut depth = furniture.depth_mm;
        if mm_row(ui, "Width", &mut width, 50.0..=6000.0) {
            furniture.width_mm = width;
            furniture.dirty = true;
        }
        if mm_row(ui, "Height", &mut height, 50.0..=4000.0) {
            furniture.height_mm = height;
            furniture.dirty = true;
        }
        if mm_row(ui, "Depth", &mut depth, 50.0..=3000.0) {
            furniture.depth_mm = depth;
            furniture.dirty = true;
        }
    }

    match furniture.kind {
        Kind::Drawers { count } => {
            let mut drawers = count as i32;
            if ui
                .add(egui::Slider::new(&mut drawers, 1..=10).text("Drawers"))
                .changed()
            {
                furniture.kind = Kind::Drawers {
                    count: drawers as u8,
                };
                furniture.ensure_front_finishes();
                furniture.dirty = true;
            }
            ui.label("Drawers split the height into equal fronts.");
        }
        Kind::HingedDoors { doors } => {
            ui.horizontal(|ui| {
                if ui.selectable_label(doors == 1, "1 door").clicked() && doors != 1 {
                    furniture.kind = Kind::HingedDoors { doors: 1 };
                    furniture.ensure_front_finishes();
                    furniture.dirty = true;
                }
                if ui.selectable_label(doors != 1, "2 doors").clicked() && doors == 1 {
                    furniture.kind = Kind::HingedDoors { doors: 2 };
                    furniture.ensure_front_finishes();
                    furniture.dirty = true;
                }
            });
        }
        Kind::LiftDoor => {
            ui.label("Hinge is on the top edge. Drawn open so it reads as a lift-up.");
        }
        Kind::SlidingDoors => {
            ui.label("Two leaves, one in front of the other.");
        }
        Kind::OpenShelves { shelves } => {
            let mut count = shelves as i32;
            if ui
                .add(egui::Slider::new(&mut count, 1..=10).text("Shelves"))
                .changed()
            {
                furniture.kind = Kind::OpenShelves {
                    shelves: count as u8,
                };
                furniture.dirty = true;
            }
            ui.label("Open-front cabinet with evenly spaced shelves.");
        }
        Kind::Table => {
            ui.label("Top and four legs. Height is the top surface.");
        }
        Kind::Countertop => {
            ui.label("Standalone thick worktop. Height is its thickness.");
        }
        Kind::Tabletop => {
            ui.label("Standalone tabletop. Height is its thickness.");
        }
        Kind::Refrigerator
        | Kind::Dishwasher
        | Kind::ElectricOven
        | Kind::CoffeeMachine
        | Kind::Microwave
        | Kind::CentralHeatingBoiler
        | Kind::AirConditioner => {
            ui.label("Stylized appliance model with editable overall dimensions.");
        }
        Kind::SquareSink => {
            ui.label("Open rectangular basin for washing dishes. The rim sits at worktop height.");
        }
        Kind::RoundSink => {
            ui.label("Open round basin for washing dishes. The rim sits at worktop height.");
        }
        Kind::DoubleSink => {
            ui.label("Large open sink with two rectangular basins.");
        }
        Kind::SinkFaucet => {
            ui.label("Movable water pipe / faucet for a kitchen sink.");
        }
        Kind::Pipe {
            horizontal,
            count,
            color,
        } => {
            let mut next = (horizontal, count, color);
            ui.horizontal(|ui| {
                if ui.selectable_label(!horizontal, "Vertical").clicked() {
                    next.0 = false;
                }
                if ui.selectable_label(horizontal, "Horizontal").clicked() {
                    next.0 = true;
                }
            });
            ui.horizontal(|ui| {
                if ui.selectable_label(count < 2, "1 pipe").clicked() {
                    next.1 = 1;
                }
                if ui.selectable_label(count >= 2, "2 pipes").clicked() {
                    next.1 = 2;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Color");
                for option in PipeColor::ALL {
                    if ui
                        .selectable_label(color == option, option.label())
                        .clicked()
                    {
                        next.2 = option;
                    }
                }
            });
            if next != (horizontal, count, color) {
                if next.0 != horizontal {
                    let width = furniture.width_mm;
                    furniture.width_mm = furniture.height_mm;
                    furniture.height_mm = width;
                }
                if next.1 != count {
                    let span = if next.1 >= 2 {
                        furniture.depth_mm + PIPE_PAIR_GAP_MM
                    } else {
                        furniture.depth_mm
                    };
                    if next.0 {
                        furniture.height_mm = span;
                    } else {
                        furniture.width_mm = span;
                    }
                }
                furniture.kind = Kind::Pipe {
                    horizontal: next.0,
                    count: next.1,
                    color: next.2,
                };
                if furniture.name.ends_with("Pipe") {
                    furniture.name = furniture.label().to_owned();
                }
                furniture.dirty = true;
            }
        }
    }

    if furniture.kind.is_cabinet() {
        ui.horizontal(|ui| {
            ui.label("Placement");
            ui.strong(match furniture.placement {
                Placement::Floor => "Floor",
                Placement::Wall => "Wall mounted",
            });
        });
        if furniture.placement == Placement::Floor {
            ui.horizontal(|ui| {
                ui.label("Legs");
                egui::ComboBox::from_id_salt((entity, "legs"))
                    .selected_text(furniture.legs.label())
                    .show_ui(ui, |ui| {
                        for style in LegStyle::ALL {
                            if ui
                                .selectable_value(&mut furniture.legs, style, style.label())
                                .changed()
                            {
                                furniture.dirty = true;
                            }
                        }
                    });
            });
            ui.weak("Cabinet height is the carcass; legs add to installed height.");
        }
    }

    ui.add_space(4.0);
    let mut x_mm = m_to_mm(transform.translation.x);
    let mut y_mm = m_to_mm(transform.translation.y);
    let mut z_mm = m_to_mm(transform.translation.z);
    if mm_row(ui, "X right", &mut x_mm, -2000.0..=30000.0) {
        transform.translation.x = mm_to_m(x_mm);
    }
    if mm_row(ui, "Y up", &mut y_mm, 0.0..=6000.0) {
        transform.translation.y = mm_to_m(y_mm);
    }
    if mm_row(ui, "Z into room", &mut z_mm, -2000.0..=30000.0) {
        transform.translation.z = mm_to_m(z_mm);
    }

    ui.label("Facing. 0° looks into the room.");
    let turns = quarter_turns(transform.rotation);
    ui.horizontal(|ui| {
        for turn in 0..4u8 {
            // `turn` is a u8 so it can be compared with the stored facing.
            // Multiply in a wider type: 3 * 90 does not fit in a u8.
            let degrees = i32::from(turn) * 90;
            if ui
                .selectable_label(turns == turn, format!("{degrees}°"))
                .clicked()
            {
                transform.rotation = rotation_from_turns(turn);
            }
        }
    });

    if !furniture.kind.is_pipe() {
        egui::CollapsingHeader::new(
            egui::RichText::new("Materials & colors")
                .strong()
                .size(15.0)
                .color(theme::ink()),
        )
        .default_open(true)
        .show(ui, |ui| {
            if furniture.kind.is_cabinet() {
                let wall_names = ["Left side", "Right side", "Bottom", "Top", "Back"];
                for (index, label) in wall_names.into_iter().enumerate() {
                    if finish_row(
                        ui,
                        (entity, "wall", index),
                        label,
                        &mut furniture.finishes.carcass[index],
                    ) {
                        furniture.dirty = true;
                    }
                }
                for index in 0..furniture.finishes.fronts.len() {
                    if finish_row(
                        ui,
                        (entity, "front", index),
                        &format!("Front {}", index + 1),
                        &mut furniture.finishes.fronts[index],
                    ) {
                        furniture.dirty = true;
                    }
                }
                if finish_row(
                    ui,
                    (entity, "handle", 0),
                    "Handles",
                    &mut furniture.finishes.handles,
                ) {
                    furniture.dirty = true;
                }
            }
            if matches!(furniture.kind, Kind::Table)
                && finish_row(
                    ui,
                    (entity, "frame", 0),
                    if furniture.kind == Kind::Table {
                        "Frame / legs"
                    } else {
                        "Block"
                    },
                    &mut furniture.finishes.carcass[0],
                )
            {
                furniture.dirty = true;
            }
            if matches!(
                furniture.kind,
                Kind::Table | Kind::Countertop | Kind::Tabletop
            ) && finish_row(ui, (entity, "top", 0), "Top", &mut furniture.finishes.top)
            {
                furniture.dirty = true;
            }
            if furniture.kind.is_appliance() {
                if appliance_finish_row(ui, &mut furniture.finishes.appliance) {
                    furniture.dirty = true;
                }
            }
        });
    }

    let mut duplicate = None;
    let mut delete = false;
    ui.horizontal(|ui| {
        if ui.button("Duplicate").clicked() {
            let mut copy = furniture.clone();
            copy.dirty = true;
            let mut placed = *transform;
            placed.translation +=
                transform.rotation * Vec3::X * (mm_to_m(furniture.width_mm) + 0.02);
            duplicate = Some((copy, placed));
        }
        if ui.button("Delete").clicked() {
            delete = true;
        }
    });
    // The mutable piece borrow has to end before spawn or despawn.
    drop(furniture);
    drop(transform);
    if let Some((copy, placed)) = duplicate {
        selection.set_only(commands.spawn((copy, placed)).id());
    }
    if delete {
        commands.entity(entity).despawn();
        selection.entities.retain(|selected| *selected != entity);
        selection.primary = selection.entities.last().copied();
    }
}

fn finish_row(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    label: &str,
    finish: &mut Finish,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add_sized([78.0, 18.0], egui::Label::new(label));
        egui::ComboBox::from_id_salt(id)
            .selected_text(finish_name(finish.preset))
            .width(116.0)
            .show_ui(ui, |ui| {
                for preset in [
                    FinishPreset::Oak,
                    FinishPreset::Walnut,
                    FinishPreset::White,
                    FinishPreset::Charcoal,
                    FinishPreset::StainlessSteel,
                    FinishPreset::Custom,
                ] {
                    changed |= ui
                        .selectable_value(&mut finish.preset, preset, finish_name(preset))
                        .changed();
                }
            });
        if finish.preset == FinishPreset::Custom {
            changed |= ui.color_edit_button_srgb(&mut finish.color).changed();
        } else {
            let rgb = finish.rgb();
            ui.colored_label(egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]), "■");
        }
    });
    changed
}

fn appliance_finish_row(ui: &mut egui::Ui, finish: &mut Finish) -> bool {
    let choices = [
        (
            "Silver metal",
            Finish::new(FinishPreset::StainlessSteel, [165, 172, 175]),
        ),
        ("Black", Finish::new(FinishPreset::Custom, [25, 27, 29])),
        ("Dark gray", Finish::new(FinishPreset::Custom, [70, 73, 76])),
    ];
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.label("Appliance color");
        for (label, option) in choices {
            if ui.selectable_label(*finish == option, label).clicked() && *finish != option {
                *finish = option;
                changed = true;
            }
        }
    });
    changed
}

fn finish_name(preset: FinishPreset) -> &'static str {
    match preset {
        FinishPreset::Oak => "Oak",
        FinishPreset::Walnut => "Walnut",
        FinishPreset::White => "White",
        FinishPreset::Charcoal => "Charcoal",
        FinishPreset::StainlessSteel => "Stainless",
        FinishPreset::Custom => "Custom",
    }
}

/// Pipes belong to the room, so clearing the furniture leaves them in place.
fn despawn_all(commands: &mut Commands, rows: &[(Entity, String, bool)]) {
    for (entity, _, is_pipe) in rows {
        if !is_pipe {
            commands.entity(*entity).despawn();
        }
    }
}

fn perform_history(
    redo: bool,
    history: &mut History,
    commands: &mut Commands,
    room: &mut Room,
    editor: &mut Editor,
    selection: &mut Selection,
    pieces: &Query<(Entity, &mut Furniture, &mut Transform)>,
) {
    let files = pieces
        .iter()
        .map(|(_, furniture, transform)| view::piece_file(furniture, transform))
        .collect();
    let current = project::from_files(room, editor, files);
    let target = if redo {
        history.redo(current)
    } else {
        history.undo(current)
    };
    if let Some(target) = target {
        let existing: Vec<_> = pieces.iter().map(|(entity, _, _)| entity).collect();
        project::restore(target, commands, room, editor, selection, &existing);
        editor.status = if redo { "Redone" } else { "Undone" }.into();
    }
}

fn save_layout(
    editor: &mut Editor,
    room: &Room,
    pieces: &Query<(Entity, &mut Furniture, &mut Transform)>,
    cut_job: &CutJob,
) {
    if editor.path.trim().is_empty() {
        editor.path = "layout.json".into();
    }
    let path = resolve_layout_path(&editor.path);
    editor.status = match write_current(
        &path,
        editor.snap_enabled,
        editor.snap_mm,
        room,
        pieces,
        cut_job,
    ) {
        Ok(()) => "Saved".into(),
        Err(message) => message,
    };
    info!("{}", editor.status);
}

fn begin_dialog(dialogs: &mut NativeDialog, editor: &mut Editor, request: DialogRequest) {
    if dialogs.is_open() {
        return;
    }
    match request {
        DialogRequest::Save => dialogs.begin_save(&editor.path),
        DialogRequest::Open => dialogs.begin_open(&editor.path),
    }
    editor.status = "Choose a file".into();
}

/// Applies a finished Load or Save As dialog. The panel itself stays open
/// without stopping the 3D window.
pub fn finish_native_dialog(
    mut dialogs: NonSendMut<NativeDialog>,
    mut redraws: MessageWriter<RequestRedraw>,
    mut commands: Commands,
    pieces: Query<(Entity, &mut Furniture, &mut Transform)>,
    mut room: ResMut<Room>,
    mut editor: ResMut<Editor>,
    mut selection: ResMut<Selection>,
    mut camera: Query<&mut OrbitCamera>,
    mut cut_job: ResMut<CutJob>,
    mut history: ResMut<History>,
) {
    if dialogs.is_open() {
        redraws.write(RequestRedraw);
    }
    let Some((request, path)) = dialogs.poll() else {
        return;
    };
    let Some(path) = path else {
        editor.status = "Cancelled".into();
        return;
    };
    editor.path = path.display().to_string();
    match request {
        DialogRequest::Save => save_layout(&mut editor, &room, &pieces, &cut_job),
        DialogRequest::Open => {
            load_layout(
                &mut commands,
                &pieces,
                &mut room,
                &mut editor,
                &mut selection,
                &mut camera,
                &mut cut_job,
            );
            *history = History::default();
        }
    }
}

fn write_current(
    path: &std::path::Path,
    snap_enabled: bool,
    snap_mm: f32,
    room: &Room,
    pieces: &Query<(Entity, &mut Furniture, &mut Transform)>,
    cut_job: &CutJob,
) -> Result<(), String> {
    let file = ProjectFile {
        version: 3,
        snap_enabled,
        snap_mm,
        room: view::saved_room(room),
        pieces: pieces
            .iter()
            .map(|(_, furniture, transform)| view::piece_file(furniture, &transform))
            .collect(),
        cut_list: cut_job.to_file(),
    };
    write_project(path, &file)
}

fn over_chrome(
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

fn load_layout(
    commands: &mut Commands,
    pieces: &Query<(Entity, &mut Furniture, &mut Transform)>,
    room: &mut Room,
    editor: &mut Editor,
    selection: &mut Selection,
    camera: &mut Query<&mut OrbitCamera>,
    cut_job: &mut CutJob,
) {
    if editor.path.trim().is_empty() {
        editor.path = "layout.json".into();
    }
    let path = resolve_layout_path(&editor.path);
    let existing: Vec<Entity> = pieces.iter().map(|(entity, _, _)| entity).collect();
    match read_project(&path.to_string_lossy()) {
        Ok(file) => {
            let file = file.sanitized();
            *cut_job = CutJob::from_file(&file.cut_list);
            project::restore(
                project::from_project(&file),
                commands,
                room,
                editor,
                selection,
                &existing,
            );
            if let Ok(mut orbit) = camera.single_mut() {
                orbit.frame_room(room);
            }
            editor.status = "Loaded".into();
        }
        Err(ReadError::Missing) => {
            editor.status = format!("No file named {}", layout_label(&editor.path));
        }
        Err(ReadError::Invalid(message)) => {
            editor.status = message;
        }
    }
}
