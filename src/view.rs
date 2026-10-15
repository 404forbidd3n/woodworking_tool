//! The room, the orbit camera, and the meshes that stand in for furniture.
//!
//! Navigation is a normal modelling camera, aimed at a laptop trackpad as well
//! as a mouse:
//!
//! - drag empty space to orbit
//! - drag a piece to move it
//! - right-drag, or Space-drag, to pan
//! - scroll to zoom
//!
//! The pointer logic itself lives in `edit`. This module only stores the
//! camera angles and writes the camera transform from them.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::model::{
    example_layout, read_project, resolve_layout_path, Editor, Finish, FinishPreset, Furniture,
    PieceFile, ReadError, Room, RoomFile, RoomRoot, Solid, Surface,
};

/// Stock colors. They are only there so a door reads differently from a side,
/// and so a placeholder box does not look like a finished cabinet.
#[derive(Resource)]
pub struct Palette {
    pub carcass: Handle<StandardMaterial>,
    pub front: Handle<StandardMaterial>,
    pub handle: Handle<StandardMaterial>,
    pub table: Handle<StandardMaterial>,
    pub placeholder: Handle<StandardMaterial>,
    pub floor: Handle<StandardMaterial>,
    pub wall: Handle<StandardMaterial>,
}

impl Palette {
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
            carcass: mat(materials, Color::srgb(0.62, 0.46, 0.28)),
            front: mat(materials, Color::srgb(0.78, 0.64, 0.44)),
            handle: mat(materials, Color::srgb(0.20, 0.14, 0.10)),
            table: mat(materials, Color::srgb(0.50, 0.36, 0.22)),
            placeholder: mat(materials, Color::srgb(0.55, 0.60, 0.64)),
            floor: mat(materials, Color::srgb(0.78, 0.76, 0.72)),
            wall: mat(materials, Color::srgb(0.92, 0.91, 0.88)),
        }
    }

    fn handle(&self, role: Surface) -> Handle<StandardMaterial> {
        match role {
            Surface::Carcass(_) => self.carcass.clone(),
            Surface::Front(_) => self.front.clone(),
            Surface::Handle => self.handle.clone(),
            Surface::Top => self.table.clone(),
            Surface::Appliance(_) => self.placeholder.clone(),
            Surface::Floor => self.floor.clone(),
            Surface::Wall => self.wall.clone(),
        }
    }
}

/// Reuses GPU materials across pieces that have the same visible finish.
#[derive(Resource, Default)]
pub struct MaterialCache {
    entries: HashMap<([u8; 3], bool), Handle<StandardMaterial>>,
}

impl MaterialCache {
    fn get(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
        finish: Finish,
    ) -> Handle<StandardMaterial> {
        let rgb = finish.rgb();
        let metallic = finish.preset == FinishPreset::StainlessSteel;
        self.entries
            .entry((rgb, metallic))
            .or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: Color::srgb_u8(rgb[0], rgb[1], rgb[2]),
                    perceptual_roughness: if metallic { 0.32 } else { 0.78 },
                    metallic: if metallic { 0.86 } else { 0.0 },
                    ..default()
                })
            })
            .clone()
    }
}

/// Angles for the modelling camera. The transform on the same entity is
/// rewritten from these every frame, so the sidebar can change the view by
/// editing the angles alone.
#[derive(Component)]
pub struct OrbitCamera {
    pub focus: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl OrbitCamera {
    pub fn looking_at_room(room: &Room) -> Self {
        let mut camera = Self {
            focus: Vec3::ZERO,
            // From the front-left corner, a little above the cabinets.
            yaw: 2.25,
            pitch: 0.62,
            distance: 8.0,
        };
        camera.frame_room(room);
        camera
    }

    pub fn frame_room(&mut self, room: &Room) {
        let size = room.size_meters();
        self.focus = Vec3::new(size.x * 0.5, size.y * 0.28, size.z * 0.45);
        self.distance = size.x.hypot(size.z).max(2.0) * 0.95;
    }

    pub fn frame_piece(&mut self, base: Vec3, size: Vec3) {
        self.focus = base + Vec3::Y * size.y * 0.5;
        self.distance = size.x.max(size.y).max(size.z).max(0.4) * 3.2;
    }

    pub fn corner_view(&mut self) {
        self.yaw = 2.25;
        self.pitch = 0.62;
    }

    pub fn front_view(&mut self) {
        // yaw of a quarter turn puts the camera on the +Z side, looking back
        // toward the back wall.
        self.yaw = std::f32::consts::FRAC_PI_2;
        self.pitch = 0.18;
    }

    pub fn top_view(&mut self) {
        // Steep, but not straight down: a vertical look direction makes the
        // camera's "up" undefined.
        self.yaw = std::f32::consts::FRAC_PI_2;
        self.pitch = 1.42;
    }

    pub fn pose(&self) -> Transform {
        Transform::from_translation(self.eye()).looking_at(self.focus, Vec3::Y)
    }

    fn eye(&self) -> Vec3 {
        let pitch = self.pitch.clamp(0.08, 1.45);
        let horizontal = self.distance * pitch.cos();
        self.focus
            + Vec3::new(
                horizontal * self.yaw.cos(),
                self.distance * pitch.sin(),
                horizontal * self.yaw.sin(),
            )
    }
}

pub fn setup_world(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut room: ResMut<Room>,
    mut editor: ResMut<Editor>,
) {
    ambient.color = Color::srgb(1.0, 0.98, 0.94);
    ambient.brightness = 90.0;

    commands.insert_resource(Palette::new(&mut materials));
    commands.insert_resource(MaterialCache::default());
    commands.spawn((
        RoomRoot,
        Name::new("Room"),
        Transform::default(),
        Visibility::default(),
    ));

    // A warm key light plus a cooler fill, so a cabinet's side is readable
    // without hunting for the light direction.
    commands.spawn((
        DirectionalLight {
            illuminance: 11000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(8.0, 14.0, 6.0).looking_at(Vec3::new(2.0, 0.5, 1.5), Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 3500.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(-6.0, 8.0, 10.0).looking_at(Vec3::new(2.0, 0.8, 1.0), Vec3::Y),
    ));

    let layout_path = resolve_layout_path(&editor.path);
    let loaded = match read_project(&layout_path.to_string_lossy()) {
        Ok(file) => {
            let file = file.sanitized();
            apply_saved_room(&mut room, &file.room);
            editor.snap_enabled = file.snap_enabled;
            editor.snap_mm = file.snap_mm;
            editor.status = "Loaded".into();
            Some(file.pieces)
        }
        Err(ReadError::Missing) => {
            editor.status = "Not saved yet".into();
            None
        }
        Err(ReadError::Invalid(message)) => {
            editor.status = format!("Could not read the file ({message}). Showing the example.");
            None
        }
    };

    let placements = loaded
        .map(|pieces| pieces.into_iter().map(placement_from_file).collect())
        .unwrap_or_else(example_layout);

    for (furniture, transform) in placements {
        commands.spawn((furniture, transform));
    }

    let camera = OrbitCamera::looking_at_room(&room);
    commands.spawn((
        Camera3d::default(),
        camera.pose(),
        camera,
        Name::new("Camera"),
    ));
}

fn apply_saved_room(room: &mut Room, saved: &RoomFile) {
    room.width_mm = saved.width_mm;
    room.depth_mm = saved.depth_mm;
    room.wall_height_mm = saved.wall_height_mm;
    room.show_walls = saved.show_walls;
    room.dirty = true;
}

fn placement_from_file(piece: PieceFile) -> (Furniture, Transform) {
    let mut furniture = Furniture::new(piece.kind, piece.width_mm, piece.height_mm, piece.depth_mm);
    furniture.name = piece.name;
    furniture.finishes = piece.finishes;
    furniture.ensure_front_finishes();
    (
        furniture,
        crate::model::place(piece.x_mm, piece.y_mm, piece.z_mm, piece.turns),
    )
}

struct Built {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    transform: Transform,
}

/// Rebuild any piece or room whose dimensions changed. Meshes are plain boxes;
/// rebuilding a cabinet is cheap and keeps the draw code obvious.
pub fn sync_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut material_cache: ResMut<MaterialCache>,
    palette: Res<Palette>,
    mut room: ResMut<Room>,
    mut pieces: Query<(Entity, &mut Furniture)>,
    roots: Query<Entity, With<RoomRoot>>,
) {
    if room.dirty {
        room.dirty = false;
        if let Some(root) = roots.iter().next() {
            let parts = room_solids(&room);
            replace_children(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut material_cache,
                &palette,
                root,
                parts,
                None,
            );
        }
    }

    let pending: Vec<(Entity, Furniture)> = pieces
        .iter()
        .filter(|(_, furniture)| furniture.dirty)
        .map(|(entity, furniture)| (entity, furniture.clone()))
        .collect();

    for (_, mut furniture) in &mut pieces {
        furniture.dirty = false;
    }

    for (entity, furniture) in pending {
        let parts = crate::model::solids(&furniture);
        commands
            .entity(entity)
            .insert(Name::new(furniture.kind.label()));
        replace_children(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut material_cache,
            &palette,
            entity,
            parts,
            Some(&furniture),
        );
    }
}

fn room_solids(room: &Room) -> Vec<Solid> {
    let size = room.size_meters();
    let (w, h, d) = (size.x, size.y, size.z);
    let t = 0.08;
    let mut parts = vec![Solid {
        // The top of the floor slab is y = 0, so pieces sit on it.
        size: Vec3::new(w, 0.04, d),
        translation: Vec3::new(w * 0.5, -0.02, d * 0.5),
        rotation: Quat::IDENTITY,
        role: Surface::Floor,
    }];
    if room.show_walls {
        // Walls sit just outside the floor rectangle, so a 600 mm cabinet
        // placed at the back wall is really 600 mm of floor.
        parts.push(wall(
            Vec3::new(w + t * 2.0, h, t),
            Vec3::new(w * 0.5, h * 0.5, -t * 0.5),
        ));
        parts.push(wall(
            Vec3::new(t, h, d),
            Vec3::new(-t * 0.5, h * 0.5, d * 0.5),
        ));
        parts.push(wall(
            Vec3::new(t, h, d),
            Vec3::new(w + t * 0.5, h * 0.5, d * 0.5),
        ));
    }
    parts
}

fn wall(size: Vec3, translation: Vec3) -> Solid {
    Solid {
        size,
        translation,
        rotation: Quat::IDENTITY,
        role: Surface::Wall,
    }
}

fn replace_children(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    material_cache: &mut MaterialCache,
    palette: &Palette,
    entity: Entity,
    parts: Vec<Solid>,
    furniture: Option<&Furniture>,
) {
    let built: Vec<Built> = parts
        .into_iter()
        .map(|part| Built {
            mesh: meshes.add(Cuboid::new(part.size.x, part.size.y, part.size.z)),
            material: surface_material(materials, material_cache, palette, furniture, part.role),
            transform: Transform {
                translation: part.translation,
                rotation: part.rotation,
                scale: Vec3::ONE,
            },
        })
        .collect();

    commands.entity(entity).despawn_related::<Children>();
    commands.entity(entity).with_children(move |parent| {
        for part in built {
            parent.spawn((
                Mesh3d(part.mesh),
                MeshMaterial3d(part.material),
                part.transform,
            ));
        }
    });
}

fn surface_material(
    materials: &mut Assets<StandardMaterial>,
    cache: &mut MaterialCache,
    palette: &Palette,
    furniture: Option<&Furniture>,
    role: Surface,
) -> Handle<StandardMaterial> {
    let Some(furniture) = furniture else {
        return palette.handle(role);
    };
    let finish = match role {
        Surface::Carcass(index) => furniture
            .finishes
            .carcass
            .get(index as usize)
            .copied()
            .unwrap_or_default(),
        Surface::Front(index) => furniture
            .finishes
            .fronts
            .get(index as usize)
            .copied()
            .unwrap_or_default(),
        Surface::Handle => furniture.finishes.handles,
        Surface::Top => furniture.finishes.top,
        Surface::Appliance(2) => Finish::new(FinishPreset::Charcoal, [28, 32, 34]),
        Surface::Appliance(_) => furniture.finishes.appliance,
        Surface::Floor | Surface::Wall => return palette.handle(role),
    };
    cache.get(materials, finish)
}

/// Floor grid, a short axis marker at the back-left corner, and an outline
/// around the selected piece. Drawn as gizmos so they stay visible on top of
/// the floor without becoming another mesh to maintain.
pub fn draw_guides(
    mut gizmos: Gizmos,
    room: Res<Room>,
    selection: Res<crate::model::Selection>,
    pieces: Query<(&Furniture, &GlobalTransform)>,
) {
    draw_grid(&mut gizmos, &room);
    draw_axes(&mut gizmos);

    for selected in &selection.entities {
        let Ok((furniture, transform)) = pieces.get(*selected) else {
            continue;
        };
        let (min, max) = crate::model::pick_bounds(furniture);
        let color = if selection.primary == Some(*selected) {
            Color::srgb(0.95, 0.42, 0.08)
        } else {
            Color::srgb(1.0, 0.72, 0.2)
        };
        wire_box(&mut gizmos, transform, min, max, color);
    }
}

fn draw_grid(gizmos: &mut Gizmos, room: &Room) {
    let size = room.size_meters();
    // A 100 mm grid is useful in a normal room and noisy in a very large one.
    let step = if size.x <= 8.0 && size.z <= 8.0 {
        0.1
    } else {
        0.5
    };
    let major_every = (0.5_f32 / step).round() as i32;
    let y = 0.012;
    let x_count = (size.x / step).round() as i32;
    let z_count = (size.z / step).round() as i32;

    for index in 0..=x_count {
        let x = index as f32 * step;
        let color = if index % major_every == 0 {
            Color::srgb(0.42, 0.40, 0.36)
        } else {
            Color::srgba(0.42, 0.40, 0.36, 0.35)
        };
        gizmos.line(Vec3::new(x, y, 0.0), Vec3::new(x, y, size.z), color);
    }
    for index in 0..=z_count {
        let z = index as f32 * step;
        let color = if index % major_every == 0 {
            Color::srgb(0.42, 0.40, 0.36)
        } else {
            Color::srgba(0.42, 0.40, 0.36, 0.35)
        };
        gizmos.line(Vec3::new(0.0, y, z), Vec3::new(size.x, y, z), color);
    }
}

fn draw_axes(gizmos: &mut Gizmos) {
    let origin = Vec3::new(0.0, 0.03, 0.0);
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
}

fn wire_box(gizmos: &mut Gizmos, transform: &GlobalTransform, min: Vec3, max: Vec3, color: Color) {
    let corner = |x, y, z| transform.transform_point(Vec3::new(x, y, z));
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

pub fn saved_room(room: &Room) -> RoomFile {
    RoomFile {
        width_mm: room.width_mm,
        depth_mm: room.depth_mm,
        wall_height_mm: room.wall_height_mm,
        show_walls: room.show_walls,
    }
}

pub fn piece_file(furniture: &Furniture, transform: &Transform) -> PieceFile {
    PieceFile {
        kind: furniture.kind,
        name: furniture.name.clone(),
        width_mm: furniture.width_mm,
        height_mm: furniture.height_mm,
        depth_mm: furniture.depth_mm,
        x_mm: crate::model::m_to_mm(transform.translation.x),
        y_mm: crate::model::m_to_mm(transform.translation.y),
        z_mm: crate::model::m_to_mm(transform.translation.z),
        turns: crate::model::quarter_turns(transform.rotation),
        finishes: furniture.finishes.clone(),
    }
}
