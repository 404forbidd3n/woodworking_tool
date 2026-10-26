//! Orbit, picking, and the meshes for the block under construction.

use bevy::camera::visibility::RenderLayers;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy_egui::EguiContext;

use crate::layout::model::{
    mm_to_m, ray_aabb, ray_horizontal_plane, shape_mesh, snap_to_step, wrap_degrees, CustomBoard,
    CustomRole, Editor, LegStyle,
};

/// Gizmos for the component stage only, so the room camera does not draw them.
#[derive(Default, Reflect, GizmoConfigGroup)]
#[reflect(Default)]
pub struct BuilderGizmos;

use super::{
    BuilderChrome, BuilderDrag, BuilderPreview, ComponentPlan, ComponentPlannerCamera,
    ComponentPlannerWindow, ComponentWindow, DragKind, BUILDER_LAYER, BUILDER_OFFSET,
};

/// Grid step for dragging boards in the planner, in millimeters.
const SNAP_MM: f32 = 10.0;

/// System: mouse input in the planner window.
///
/// While a drag is active it is applied every frame until the button is
/// released. Otherwise a scroll zooms, and a press starts a new drag
/// (see [`begin_drag`]). The camera transform is rewritten at the end.
pub fn pointer(
    mut plan: ResMut<ComponentPlan>,
    mut camera: Query<(&Camera, &GlobalTransform, &mut Transform), With<ComponentPlannerCamera>>,
    windows: Query<&Window, With<ComponentPlannerWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    chrome: Res<BuilderChrome>,
) {
    let Ok((camera, camera_transform, mut transform)) = camera.single_mut() else {
        return;
    };
    let Some(window) = windows.iter().next() else {
        return;
    };

    if plan.drag.kind != DragKind::Idle {
        let button = plan.drag.button;
        if mouse.pressed(button) {
            if let Some(cursor) = window.cursor_position() {
                plan.drag.cursor = cursor;
            }
            plan.drag.moved += motion.delta.length();
            let cursor = plan.drag.cursor;
            apply_drag(
                &mut plan,
                camera,
                camera_transform,
                cursor,
                motion.delta,
                &keys,
            );
        } else {
            plan.drag.kind = DragKind::Idle;
        }
    } else if let Some(cursor) = window.cursor_position() {
        if !over_panels(cursor, &chrome) {
            if scroll_lines(&scroll).abs() > f32::EPSILON {
                let factor = (1.0 - scroll_lines(&scroll) * 0.08).clamp(0.7, 1.4);
                plan.orbit.distance = (plan.orbit.distance * factor).clamp(0.4, 12.0);
            }
            if let Some(button) = just_pressed(&mouse) {
                begin_drag(&mut plan, button, cursor, camera, camera_transform, &keys);
            }
        }
    }

    *transform = plan.orbit.pose();
}

/// System: rebuild the preview meshes when `plan.mesh_dirty` is set.
///
/// Every board becomes a parent entity with the board's position and
/// rotation, and one child mesh per piece (a board with a hole is several
/// pieces). A floor slab is added under everything.
pub fn sync_preview(
    mut commands: Commands,
    mut plan: ResMut<ComponentPlan>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut shades: Local<Option<Shades>>,
    state: Res<ComponentWindow>,
    preview: Query<Entity, With<BuilderPreview>>,
) {
    if !plan.mesh_dirty {
        return;
    }
    let Some(root) = state.preview.filter(|entity| preview.get(*entity).is_ok()) else {
        return;
    };
    plan.mesh_dirty = false;
    let shades = shades.get_or_insert_with(|| Shades::new(&mut materials));
    let boards = plan.boards.clone();
    commands.entity(root).despawn_related::<Children>();
    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(2.4, 0.04, 2.4))),
            MeshMaterial3d(shades.floor.clone()),
            Transform::from_xyz(0.0, -0.02, 0.0),
            RenderLayers::layer(BUILDER_LAYER),
        ));
        for board in &boards {
            let material = match board.role {
                CustomRole::Leg if board.leg == LegStyle::Wood100 => shades.wood.clone(),
                CustomRole::Leg if matches!(board.leg, LegStyle::Metal100 | LegStyle::Metal150) => {
                    shades.metal.clone()
                }
                CustomRole::Leg => shades.plastic.clone(),
                _ => shades.wood.clone(),
            };
            let pieces = board.pieces_mm();
            let shape = if pieces.len() == 1 {
                board.shape
            } else {
                crate::layout::model::BoardShape::Box
            };
            parent
                .spawn((
                    board_transform(board),
                    Visibility::default(),
                    RenderLayers::layer(BUILDER_LAYER),
                ))
                .with_children(|part| {
                    for (size_mm, offset_mm) in pieces {
                        part.spawn((
                            Mesh3d(meshes.add(shape_mesh(
                                shape,
                                Vec3::new(
                                    mm_to_m(size_mm.x),
                                    mm_to_m(size_mm.y),
                                    mm_to_m(size_mm.z),
                                ),
                            ))),
                            MeshMaterial3d(material.clone()),
                            Transform::from_translation(Vec3::new(
                                mm_to_m(offset_mm.x),
                                mm_to_m(offset_mm.y),
                                mm_to_m(offset_mm.z),
                            )),
                            RenderLayers::layer(BUILDER_LAYER),
                        ));
                    }
                });
        }
    });
}

/// Startup system: draw [`BuilderGizmos`] only on the planner's layer.
pub fn configure_gizmos(mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<BuilderGizmos>();
    config.render_layers = RenderLayers::layer(BUILDER_LAYER);
    // Pull the wire just in front of the board so the outline stays readable.
    config.depth_bias = -0.15;
}

/// Floor grid, origin axes, and an orange wire on the selected board.
/// Same marks the room stage uses.
pub fn draw_stage(
    mut gizmos: Gizmos<BuilderGizmos>,
    plan: Res<ComponentPlan>,
    state: Res<ComponentWindow>,
) {
    if state.window.is_none() {
        return;
    }
    let major = Color::srgb(0.42, 0.40, 0.36);
    let minor = Color::srgba(0.42, 0.40, 0.36, 0.35);
    let step = 0.1_f32;
    let extent = 1.2_f32;
    let count = (extent / step).round() as i32;
    let y = 0.012;
    for index in -count..=count {
        let along = index as f32 * step;
        let color = if index % 5 == 0 { major } else { minor };
        gizmos.line(
            BUILDER_OFFSET + Vec3::new(along, y, -extent),
            BUILDER_OFFSET + Vec3::new(along, y, extent),
            color,
        );
        gizmos.line(
            BUILDER_OFFSET + Vec3::new(-extent, y, along),
            BUILDER_OFFSET + Vec3::new(extent, y, along),
            color,
        );
    }
    let origin = BUILDER_OFFSET + Vec3::new(0.0, 0.03, 0.0);
    let length = 0.35;
    gizmos.line(
        origin,
        origin + Vec3::X * length,
        Color::srgb(0.75, 0.20, 0.18),
    );
    gizmos.line(
        origin,
        origin + Vec3::Y * length,
        Color::srgb(0.20, 0.55, 0.25),
    );
    gizmos.line(
        origin,
        origin + Vec3::Z * length,
        Color::srgb(0.18, 0.35, 0.75),
    );

    let Some(index) = plan.selected else {
        return;
    };
    let Some(board) = plan.boards.get(index) else {
        return;
    };
    let half = Vec3::new(
        mm_to_m(board.width_mm),
        mm_to_m(board.height_mm),
        mm_to_m(board.depth_mm),
    ) * 0.5;
    wire_box(
        &mut gizmos,
        &GlobalTransform::from(board_transform(board)),
        -half,
        half,
        Color::srgb(0.95, 0.42, 0.08),
    );
}

/// Twelve edges of a box, transformed onto the planner stage.
fn wire_box(
    gizmos: &mut Gizmos<BuilderGizmos>,
    transform: &GlobalTransform,
    min: Vec3,
    max: Vec3,
    color: Color,
) {
    let corner = |x, y, z| transform.transform_point(Vec3::new(x, y, z)) + BUILDER_OFFSET;
    let points = [
        corner(min.x, min.y, min.z),
        corner(max.x, min.y, min.z),
        corner(max.x, min.y, max.z),
        corner(min.x, min.y, max.z),
        corner(min.x, max.y, min.z),
        corner(max.x, max.y, min.z),
        corner(max.x, max.y, max.z),
        corner(min.x, max.y, max.z),
    ];
    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    for (start, end) in edges {
        gizmos.line(points[start], points[end], color);
    }
}

/// The few materials the planner preview uses. Created once and kept in a
/// `Local` inside [`sync_preview`].
pub struct Shades {
    wood: Handle<StandardMaterial>,
    floor: Handle<StandardMaterial>,
    plastic: Handle<StandardMaterial>,
    metal: Handle<StandardMaterial>,
}

impl Shades {
    fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        let mat = |materials: &mut Assets<StandardMaterial>, color: Color| {
            materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.82,
                metallic: 0.0,
                ..default()
            })
        };
        Self {
            wood: mat(materials, Color::srgb(0.62, 0.46, 0.28)),
            floor: mat(materials, Color::srgb(0.78, 0.76, 0.72)),
            plastic: mat(materials, Color::srgb(0.22, 0.22, 0.24)),
            metal: mat(materials, Color::srgb(0.62, 0.64, 0.66)),
        }
    }
}

/// Decide what a mouse press starts. Pan and orbit follow the same buttons
/// as the room. A left press on a board selects it and starts moving it;
/// a left press on empty space orbits.
fn begin_drag(
    plan: &mut ComponentPlan,
    button: MouseButton,
    cursor: Vec2,
    camera: &Camera,
    transform: &GlobalTransform,
    keys: &ButtonInput<KeyCode>,
) {
    plan.drag = BuilderDrag {
        button,
        cursor,
        moved: 0.0,
        ..BuilderDrag::default()
    };
    let space = keys.pressed(KeyCode::Space);
    let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    // Same buttons as the room: right-drag, Shift+middle, and Space+left pan.
    // Middle-drag and Alt+left orbit, even when the cursor is on a board.
    if button == MouseButton::Right
        || (button == MouseButton::Middle && shift)
        || (button == MouseButton::Left && space)
    {
        plan.drag.kind = DragKind::Pan;
        return;
    }
    if button == MouseButton::Middle || (button == MouseButton::Left && alt) {
        plan.drag.kind = DragKind::Orbit;
        return;
    }
    let Some((origin, direction)) = ray_from(camera, transform, cursor) else {
        plan.drag.kind = DragKind::Orbit;
        return;
    };
    if let Some(index) = pick_board(origin, direction, &plan.boards) {
        plan.selected = Some(index);
        let board = &plan.boards[index];
        plan.drag.kind = DragKind::Move;
        plan.drag.index = Some(index);
        plan.drag.start_mm = Vec3::new(board.x_mm, board.y_mm, board.z_mm);
        plan.drag.plane_y = BUILDER_OFFSET.y + mm_to_m(board.y_mm);
        plan.drag.hit = ray_horizontal_plane(origin, direction, plan.drag.plane_y)
            .unwrap_or(BUILDER_OFFSET + local_center(board));
    } else {
        plan.drag.kind = DragKind::Orbit;
    }
}

/// Apply one frame of the current drag. Moving snaps to [`SNAP_MM`] and
/// never sinks a board below the floor. Holding Shift lifts instead of slides.
fn apply_drag(
    plan: &mut ComponentPlan,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    cursor: Vec2,
    delta: Vec2,
    keys: &ButtonInput<KeyCode>,
) {
    match plan.drag.kind {
        DragKind::Orbit => {
            plan.orbit.yaw -= delta.x * 0.005;
            plan.orbit.pitch = (plan.orbit.pitch - delta.y * 0.005).clamp(0.08, 1.45);
        }
        DragKind::Pan => {
            let scale = plan.orbit.distance * 0.001;
            let right = camera_transform.rotation() * Vec3::X;
            let up = camera_transform.rotation() * Vec3::Y;
            plan.orbit.focus -= right * delta.x * scale;
            plan.orbit.focus -= up * delta.y * scale;
        }
        DragKind::Move => {
            let Some(index) = plan.drag.index else {
                return;
            };
            let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
            if shift {
                plan.drag.lift_mm -= delta.y * 1.2;
            }
            let lift = plan.drag.lift_mm;
            let start = plan.drag.start_mm;
            let hit = plan.drag.hit;
            let plane_y = plan.drag.plane_y;
            let floor = plan
                .boards
                .get(index)
                .map(|board| board.world_half_mm().y)
                .unwrap_or(0.0);
            let mut next_x = start.x;
            let mut next_z = start.z;
            if let Some((origin, direction)) = ray_from(camera, camera_transform, cursor) {
                if let Some(hit_now) = ray_horizontal_plane(origin, direction, plane_y) {
                    let moved = hit_now - hit;
                    next_x = snap_to_step(start.x + m_to_mm(moved.x), SNAP_MM);
                    next_z = snap_to_step(start.z + m_to_mm(moved.z), SNAP_MM);
                }
            }
            if let Some(board) = plan.boards.get_mut(index) {
                if !shift {
                    board.x_mm = next_x;
                    board.z_mm = next_z;
                }
                board.y_mm = snap_to_step(start.y + lift, SNAP_MM).max(floor);
                plan.mesh_dirty = true;
            }
        }
        DragKind::Idle => {}
    }
}

/// Meters to millimeters.
fn m_to_mm(meters: f32) -> f32 {
    meters * 1000.0
}

/// Arrow-key pan. Returns `true` if any arrow was pressed.
fn pan_builder(orbit: &mut super::BuilderOrbit, keys: &ButtonInput<KeyCode>) -> bool {
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
        return false;
    }
    orbit.pan_screen(x, y);
    true
}

/// Board center in meters, relative to the stage origin.
fn local_center(board: &CustomBoard) -> Vec3 {
    Vec3::new(
        mm_to_m(board.x_mm),
        mm_to_m(board.y_mm),
        mm_to_m(board.z_mm),
    )
}

/// Position and rotation of a board on the stage.
fn board_transform(board: &CustomBoard) -> Transform {
    Transform {
        translation: local_center(board),
        rotation: board.rotation(),
        scale: Vec3::ONE,
    }
}

/// Arrow keys and R, matching the room: left and right move sideways, up is
/// toward the back wall, and Page Up / Page Down change height. R adds 90° of yaw.
pub fn nudge_selection(
    mut plan: ResMut<ComponentPlan>,
    windows: Query<&Window, With<ComponentPlannerWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
    editor: Res<Editor>,
    chrome: Res<BuilderChrome>,
    mut contexts: Query<&mut EguiContext, With<ComponentPlannerCamera>>,
) {
    let Some(window) = windows.iter().next() else {
        return;
    };
    if !window.focused || plan.drag.kind != DragKind::Idle {
        return;
    }
    if contexts
        .iter_mut()
        .next()
        .is_some_and(|mut context| context.get_mut().egui_wants_keyboard_input())
    {
        return;
    }
    if (keys.just_pressed(KeyCode::Delete) || keys.just_pressed(KeyCode::Backspace))
        && plan.selected.is_some()
    {
        if let Some(index) = plan.selected {
            plan.remove_board(index);
        }
        return;
    }
    if window
        .cursor_position()
        .is_some_and(|cursor| over_panels(cursor, &chrome))
    {
        return;
    }
    let control = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if plan.selected.is_none() || control {
        if pan_builder(&mut plan.orbit, &keys) || plan.selected.is_none() {
            return;
        }
    }
    let Some(index) = plan.selected.filter(|index| *index < plan.boards.len()) else {
        return;
    };
    let step = if editor.snap_enabled {
        editor.snap_mm.max(1.0)
    } else {
        SNAP_MM
    };
    let board = &mut plan.boards[index];
    let mut changed = false;
    if keys.just_pressed(KeyCode::ArrowLeft) {
        board.x_mm -= step;
        changed = true;
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        board.x_mm += step;
        changed = true;
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        board.z_mm -= step;
        changed = true;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        board.z_mm += step;
        changed = true;
    }
    if keys.just_pressed(KeyCode::PageUp) {
        board.y_mm += step;
        changed = true;
    }
    if keys.just_pressed(KeyCode::PageDown) {
        board.y_mm -= step;
        changed = true;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        board.yaw_deg = wrap_degrees(board.yaw_deg + 90.0);
        changed = true;
    }
    if changed {
        board.rest_on_floor();
        plan.mesh_dirty = true;
    }
}

/// The board nearest the camera under a ray, if any.
fn pick_board(origin: Vec3, direction: Vec3, boards: &[CustomBoard]) -> Option<usize> {
    let mut best_distance = f32::MAX;
    let mut best = None;
    for (index, board) in boards.iter().enumerate() {
        let center = BUILDER_OFFSET + local_center(board);
        let half = board.world_half_mm().map(mm_to_m);
        let Some(distance) = ray_aabb(origin, direction, center - half, center + half) else {
            continue;
        };
        if distance < best_distance {
            best_distance = distance;
            best = Some(index);
        }
    }
    best
}

/// World-space ray through a cursor position, as (origin, normalized direction).
fn ray_from(camera: &Camera, transform: &GlobalTransform, cursor: Vec2) -> Option<(Vec3, Vec3)> {
    let ray = camera.viewport_to_world(transform, cursor).ok()?;
    let direction =
        Vec3::new(ray.direction.x, ray.direction.y, ray.direction.z).normalize_or_zero();
    if direction.length_squared() < 0.5 {
        None
    } else {
        Some((ray.origin, direction))
    }
}

/// True when the cursor is over the egui toolbar or sidebar.
fn over_panels(cursor: Vec2, chrome: &BuilderChrome) -> bool {
    cursor.x < chrome.sidebar || cursor.y < chrome.toolbar
}

/// Scroll amount in lines, whether the device reports lines or pixels.
fn scroll_lines(scroll: &AccumulatedMouseScroll) -> f32 {
    match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
    }
}

/// The mouse button pressed this frame, if any.
fn just_pressed(mouse: &ButtonInput<MouseButton>) -> Option<MouseButton> {
    if mouse.just_pressed(MouseButton::Left) {
        Some(MouseButton::Left)
    } else if mouse.just_pressed(MouseButton::Right) {
        Some(MouseButton::Right)
    } else if mouse.just_pressed(MouseButton::Middle) {
        Some(MouseButton::Middle)
    } else {
        None
    }
}
