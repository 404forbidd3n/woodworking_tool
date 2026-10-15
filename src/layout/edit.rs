//! Selecting, moving, and the sidebar.
//!
//! Left drag does the thing under the cursor: a piece moves, empty space
//! orbits. That keeps a trackpad usable, because orbit does not need a middle
//! button. Hold Shift during a move to raise or lower the piece. Exact
//! millimeters, door counts, and saving live in the sidebar.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::input::EguiWantsInput;
use bevy_egui::{egui, EguiContexts};

use crate::cutting::{self, CutJob, CutWindow};
use crate::theme::{self, dropdown, toolbar_frame};

use crate::layout::model::{
    example_layout, m_to_mm, mm_to_m, pick_bounds, place, quarter_turns, ray_aabb,
    ray_horizontal_plane, read_project, resolve_layout_path, rotation_from_turns, snap_to_step,
    write_project, Editor, Finish, FinishPreset, Furniture, Kind, ProjectFile, ReadError, Room,
    Selection,
};
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
            if drag.deselect_if_click && drag.moved_px < 5.0 {
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

    // A drag that starts on the sidebar, or on a slider that is already
    // being dragged, must not orbit the room.
    if over_chrome(&window, &sidebar, &toolbar, &properties) || egui.is_using_pointer() {
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
            if let Some((entity, _)) = pick_piece(origin, direction, &pieces) {
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
    if let Some((entity, transform)) = pick_piece(origin, direction, &pieces) {
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

    selection.retain(|entity| pieces.get(entity).is_ok());
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

pub fn orbit_camera(
    mut camera: Query<(&mut Transform, &mut OrbitCamera)>,
    drag: Res<Drag>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    sidebar: Res<SidebarWidth>,
    toolbar: Res<ToolbarHeight>,
    properties: Res<PropertiesUi>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok((mut transform, mut orbit)) = camera.single_mut() else {
        return;
    };

    if !over_chrome(&window, &sidebar, &toolbar, &properties) {
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
    cut_windows: Query<Entity, With<cutting::CutPlannerWindow>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    theme::apply_theme(ctx);
    // Cmd+S is read here so it still saves when Project is collapsed.
    let save_shortcut =
        ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::S));
    if save_shortcut {
        save_layout(&mut editor, &room, &pieces, &cut_job);
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
                &mut camera,
                &pieces,
                &mut cut_job,
                &mut cut_window,
                &cut_windows,
            );
        });
    toolbar_height.0 = toolbar.response.rect.height();

    let panel = egui::Panel::left("tools")
        .resizable(true)
        .default_size(300.0)
        .frame(theme::sidebar_frame())
        .show(&mut viewport_ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                dropdown(ui, "Project", false, |ui| {
                    draw_project(
                        ui,
                        &mut commands,
                        &mut room,
                        &mut editor,
                        &mut selection,
                        &mut camera,
                        &pieces,
                        &mut cut_job,
                    );
                });

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
                });

                dropdown(ui, "Room", true, |ui| {
                    let mut width = room.width_mm;
                    let mut depth = room.depth_mm;
                    let mut height = room.wall_height_mm;
                    let mut walls = room.show_walls;
                    let room_changed = mm_row(ui, "Width", &mut width, 500.0..=30000.0)
                        | mm_row(ui, "Depth", &mut depth, 500.0..=30000.0)
                        | mm_row(ui, "Wall height", &mut height, 500.0..=6000.0);
                    if ui.checkbox(&mut walls, "Show walls").changed() || room_changed {
                        room.width_mm = width;
                        room.depth_mm = depth;
                        room.wall_height_mm = height;
                        room.show_walls = walls;
                        room.dirty = true;
                    }
                });

                dropdown(ui, "Add Furniture", false, |ui| {
                    let library = [
                        ("Drawer Base Cabinet", Kind::Drawers { count: 3 }),
                        ("Door Base Cabinet", Kind::HingedDoors { doors: 2 }),
                        ("Wall Lift-Up Cabinet", Kind::LiftDoor),
                        ("Sliding-Door Cabinet", Kind::SlidingDoors),
                        ("Work Table", Kind::Table),
                        ("Custom Block", Kind::Box),
                    ];
                    let existing = pieces.iter().len();
                    for (label, kind) in library {
                        if library_button(ui, label, kind).clicked() {
                            let entity = add_piece(&mut commands, &room, kind, existing);
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
                            let entity = add_piece(&mut commands, &room, kind, existing);
                            selection.set_only(entity);
                        }
                    }
                });

                dropdown(ui, "Add Appliances", false, |ui| {
                    let existing = pieces.iter().len();
                    for (label, kind) in [
                        ("Refrigerator", Kind::Refrigerator),
                        ("Dishwasher", Kind::Dishwasher),
                        ("Electric Oven", Kind::ElectricOven),
                        ("Coffee Machine", Kind::CoffeeMachine),
                        ("Microwave", Kind::Microwave),
                    ] {
                        if library_button(ui, label, kind).clicked() {
                            let entity = add_piece(&mut commands, &room, kind, existing);
                            selection.set_only(entity);
                        }
                    }
                });

                dropdown(ui, "In the room", false, |ui| {
                    let rows: Vec<(Entity, String)> = pieces
                        .iter()
                        .map(|(entity, furniture, _)| {
                            (
                                entity,
                                format!(
                                    "{}   {:.0} × {:.0} × {:.0}",
                                    furniture.kind.label(),
                                    furniture.width_mm,
                                    furniture.height_mm,
                                    furniture.depth_mm
                                ),
                            )
                        })
                        .collect();
                    if rows.is_empty() {
                        ui.label("Nothing yet.");
                    }
                    for (entity, label) in &rows {
                        let selected = selection.contains(*entity);
                        if ui.selectable_label(selected, label).clicked() {
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

                dropdown(ui, "Snap", false, |ui| {
                    ui.checkbox(&mut editor.snap_enabled, "Snap moves to a grid");
                    mm_row(ui, "Step", &mut editor.snap_mm, 1.0..=500.0);
                });

                // Help stays last so the working sections stay above the instructions.
                dropdown(ui, "Help", false, |ui| {
                    ui.weak("Millimeters. Origin is the back-left corner of the floor.");
                    ui.weak("Drag empty space to orbit. Drag a piece to move it.");
                    ui.weak("Right-drag, or hold Space and drag, to pan. Scroll zooms.");
                    ui.weak("Hold Shift while moving to raise or lower.");
                    ui.weak("R rotates 90°. D duplicates. Delete removes.");
                    ui.weak("Arrows nudge. Page Up / Page Down change height. F frames.");
                    ui.weak("The corner marks are X red, Y green, Z blue.");
                    ui.weak("Ctrl-click or Cmd-click selects multiple pieces.");
                    ui.weak("Dragging one selected piece moves the whole selection.");
                    ui.weak(
                        "Right-click a piece and choose Properties to edit it in the side panel.",
                    );
                    ui.weak("Right-drag still pans.");
                    ui.weak("Cut planner opens the sheet layout in its own window.");
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
    camera: &mut Query<&mut OrbitCamera>,
    pieces: &Query<(Entity, &mut Furniture, &mut Transform)>,
    cut_job: &mut CutJob,
    cut_window: &mut CutWindow,
    cut_windows: &Query<Entity, With<cutting::CutPlannerWindow>>,
) {
    let cream = egui::Color32::from_rgb(246, 236, 220);
    let muted = egui::Color32::from_rgb(214, 196, 170);
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
            .fill(egui::Color32::from_rgb(96, 74, 56))
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
            if ui.button("Load").clicked() {
                load_layout(commands, pieces, room, editor, selection, camera, cut_job);
            }
            if ui.button("Save").clicked() {
                save_layout(editor, room, pieces, cut_job);
            }
            if ui.button("Cut planner").clicked() {
                cutting::open_window(commands, cut_window, cut_windows);
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
        Kind::ElectricOven | Kind::Microwave | Kind::CoffeeMachine => {
            painter.rect_stroke(
                rect.shrink(5.0),
                egui::CornerRadius::same(1),
                stroke,
                egui::StrokeKind::Inside,
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
        Kind::Box => {}
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
                .color(egui::Color32::BLACK),
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

fn add_piece(commands: &mut Commands, room: &Room, kind: Kind, already_there: usize) -> Entity {
    let (furniture, y_mm) = kind.preset();
    // Step new pieces sideways so a second cabinet does not hide inside the first.
    let column = (already_there % 4) as f32;
    let x_mm = room.width_mm * 0.5 + (column - 1.5) * 180.0;
    let z_mm = room.depth_mm * 0.55;
    commands.spawn((furniture, place(x_mm, y_mm, z_mm, 0))).id()
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
    ui.strong(furniture.kind.label());
    ui.horizontal(|ui| {
        ui.label("Name");
        ui.text_edit_singleline(&mut furniture.name);
    });

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
        Kind::Table => {
            ui.label("Top and four legs. Height is the top surface.");
        }
        Kind::Countertop => {
            ui.label("Standalone thick worktop. Height is its thickness.");
        }
        Kind::Tabletop => {
            ui.label("Standalone tabletop. Height is its thickness.");
        }
        Kind::Box => {
            ui.label("Empty box. Use it for a sink, an appliance, or a gap.");
        }
        Kind::Refrigerator
        | Kind::Dishwasher
        | Kind::ElectricOven
        | Kind::CoffeeMachine
        | Kind::Microwave => {
            ui.label("Stylized appliance model with editable overall dimensions.");
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

    egui::CollapsingHeader::new(egui::RichText::new("Materials & colors").strong())
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
            if matches!(furniture.kind, Kind::Table | Kind::Box)
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
                if finish_row(
                    ui,
                    (entity, "appliance", 0),
                    "Appliance body",
                    &mut furniture.finishes.appliance,
                ) {
                    furniture.dirty = true;
                }
                if finish_row(
                    ui,
                    (entity, "handle", 1),
                    "Handles",
                    &mut furniture.finishes.handles,
                ) {
                    furniture.dirty = true;
                }
            }
        });

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

fn despawn_all(commands: &mut Commands, rows: &[(Entity, String)]) {
    for (entity, _) in rows {
        commands.entity(*entity).despawn();
    }
}

fn draw_project(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    room: &mut Room,
    editor: &mut Editor,
    selection: &mut Selection,
    camera: &mut Query<&mut OrbitCamera>,
    pieces: &Query<(Entity, &mut Furniture, &mut Transform)>,
    cut_job: &mut CutJob,
) {
    ui.weak("File in the project folder");
    ui.add(egui::TextEdit::singleline(&mut editor.path).desired_width(f32::INFINITY));
    ui.horizontal(|ui| {
        if ui.button("Save").clicked() {
            save_layout(editor, room, pieces, cut_job);
        }
        if ui.button("Load").clicked() {
            load_layout(commands, pieces, room, editor, selection, camera, cut_job);
        }
    });
    ui.weak("Cmd+S also saves.");
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

fn write_current(
    path: &std::path::Path,
    snap_enabled: bool,
    snap_mm: f32,
    room: &Room,
    pieces: &Query<(Entity, &mut Furniture, &mut Transform)>,
    cut_job: &CutJob,
) -> Result<(), String> {
    let file = ProjectFile {
        version: 2,
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
) -> Option<(Entity, Transform)> {
    let mut best_distance = f32::MAX;
    let mut best: Option<(Entity, Transform)> = None;
    for (entity, furniture, transform, global) in pieces.iter() {
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
            for entity in existing {
                commands.entity(entity).despawn();
            }
            room.width_mm = file.room.width_mm;
            room.depth_mm = file.room.depth_mm;
            room.wall_height_mm = file.room.wall_height_mm;
            room.show_walls = file.room.show_walls;
            room.dirty = true;
            editor.snap_enabled = file.snap_enabled;
            editor.snap_mm = file.snap_mm;
            *cut_job = CutJob::from_file(&file.cut_list);
            for piece in file.pieces {
                let mut furniture =
                    Furniture::new(piece.kind, piece.width_mm, piece.height_mm, piece.depth_mm);
                furniture.name = piece.name;
                furniture.finishes = piece.finishes;
                furniture.ensure_front_finishes();
                commands.spawn((
                    furniture,
                    place(piece.x_mm, piece.y_mm, piece.z_mm, piece.turns),
                ));
            }
            selection.clear();
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
