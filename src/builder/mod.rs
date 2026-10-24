//! The component planner builds a furniture type from loose boards in 3D.
//!
//! A saved type is written to `customs.json` in the project folder and listed
//! in the Customs section of the room. Width, height, and depth on a placed
//! piece scale every board together.

mod scene;
mod ui;

use bevy::camera::visibility::RenderLayers;
use bevy::camera::RenderTarget;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::window::WindowRef;
use bevy_egui::EguiSchedule;

use crate::layout::model::{project_dir, CustomBoard, CustomType};

pub use scene::{
    configure_gizmos, draw_stage, nudge_selection, pointer, sync_preview, BuilderGizmos,
};
pub use ui::draw as ui_draw;

/// The builder is drawn on this layer so it stays out of the room camera.
pub const BUILDER_LAYER: usize = 1;
/// Parked away from the room as well, in case a mesh misses its layer.
pub const BUILDER_OFFSET: Vec3 = Vec3::new(40.0, 0.0, 0.0);

/// Runs only for the component planner window.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ComponentPlannerPass;

#[derive(Component)]
pub struct ComponentPlannerWindow;

#[derive(Component)]
pub struct ComponentPlannerCamera;

#[derive(Component)]
pub struct BuilderPreview;

#[derive(Resource)]
pub struct ComponentWindow {
    window: Option<Entity>,
    camera: Option<Entity>,
    preview: Option<Entity>,
    light: Option<Entity>,
    requested: bool,
}

impl Default for ComponentWindow {
    fn default() -> Self {
        Self {
            window: None,
            camera: None,
            preview: None,
            light: None,
            requested: false,
        }
    }
}

pub fn request_open(state: &mut ComponentWindow) {
    state.requested = true;
}

pub fn open_requested(
    mut commands: Commands,
    mut state: ResMut<ComponentWindow>,
    mut plan: ResMut<ComponentPlan>,
    windows: Query<Entity, With<ComponentPlannerWindow>>,
) {
    if !state.requested {
        return;
    }
    state.requested = false;
    if let Some(entity) = state.window {
        if windows.is_empty() || windows.get(entity).is_ok() {
            return;
        }
    }
    let mut resolution = bevy::window::WindowResolution::default();
    resolution.set(1280.0, 800.0);
    let window = commands
        .spawn((
            Window {
                title: "Component planner".into(),
                resolution,
                ..default()
            },
            ComponentPlannerWindow,
        ))
        .id();
    let orbit = plan.orbit;
    let camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: 2,
                ..default()
            },
            orbit.pose(),
            ComponentPlannerCamera,
            RenderLayers::layer(BUILDER_LAYER),
            RenderTarget::Window(WindowRef::Entity(window)),
            EguiSchedule::new(ComponentPlannerPass),
        ))
        .id();
    let preview = commands
        .spawn((
            BuilderPreview,
            Name::new("Custom block"),
            Transform::from_translation(BUILDER_OFFSET),
            Visibility::default(),
            RenderLayers::layer(BUILDER_LAYER),
        ))
        .id();
    let light = commands
        .spawn((
            Transform::from_translation(BUILDER_OFFSET),
            Visibility::default(),
            RenderLayers::layer(BUILDER_LAYER),
        ))
        .with_children(|parent| {
            parent.spawn((
                DirectionalLight {
                    illuminance: 11000.0,
                    shadow_maps_enabled: true,
                    ..default()
                },
                Transform::from_xyz(2.2, 3.6, 1.6).looking_at(Vec3::new(0.0, 0.35, 0.0), Vec3::Y),
                RenderLayers::layer(BUILDER_LAYER),
            ));
            parent.spawn((
                DirectionalLight {
                    illuminance: 3500.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_xyz(-1.6, 2.2, 2.4).looking_at(Vec3::new(0.0, 0.35, 0.0), Vec3::Y),
                RenderLayers::layer(BUILDER_LAYER),
            ));
        })
        .id();
    state.window = Some(window);
    state.camera = Some(camera);
    state.preview = Some(preview);
    state.light = Some(light);
    plan.mesh_dirty = true;
}

pub fn on_window_closed(
    mut removed: RemovedComponents<ComponentPlannerWindow>,
    mut state: ResMut<ComponentWindow>,
    mut commands: Commands,
) {
    for entity in removed.read() {
        if state.window == Some(entity) {
            if let Some(camera) = state.camera.take() {
                commands.entity(camera).despawn();
            }
            if let Some(preview) = state.preview.take() {
                commands.entity(preview).despawn();
            }
            if let Some(light) = state.light.take() {
                commands.entity(light).despawn();
            }
            state.window = None;
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum DragKind {
    #[default]
    Idle,
    Orbit,
    Pan,
    Move,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BuilderDrag {
    pub kind: DragKind,
    pub button: MouseButton,
    pub moved: f32,
    pub index: Option<usize>,
    pub start_mm: Vec3,
    pub hit: Vec3,
    pub plane_y: f32,
    pub cursor: Vec2,
    pub lift_mm: f32,
}

impl Default for BuilderDrag {
    fn default() -> Self {
        Self {
            kind: DragKind::Idle,
            button: MouseButton::Left,
            moved: 0.0,
            index: None,
            start_mm: Vec3::ZERO,
            hit: Vec3::ZERO,
            plane_y: 0.0,
            cursor: Vec2::ZERO,
            lift_mm: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BuilderOrbit {
    pub focus: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Default for BuilderOrbit {
    fn default() -> Self {
        Self {
            focus: BUILDER_OFFSET + Vec3::Y * 0.4,
            yaw: 2.2,
            pitch: 0.55,
            distance: 2.6,
        }
    }
}

impl BuilderOrbit {
    pub fn pose(self) -> Transform {
        let pitch = self.pitch.clamp(0.08, 1.45);
        let horizontal = self.distance * pitch.cos();
        let eye = self.focus
            + Vec3::new(
                horizontal * self.yaw.cos(),
                self.distance * pitch.sin(),
                horizontal * self.yaw.sin(),
            );
        Transform::from_translation(eye).looking_at(self.focus, Vec3::Y)
    }
}

/// The block currently being edited. It is not in the room until it is saved
/// and then chosen from Customs.
#[derive(Resource)]
pub struct ComponentPlan {
    pub name: String,
    pub boards: Vec<CustomBoard>,
    pub selected: Option<usize>,
    pub mesh_dirty: bool,
    pub status: String,
    pub(crate) drag: BuilderDrag,
    pub(crate) orbit: BuilderOrbit,
}

impl Default for ComponentPlan {
    fn default() -> Self {
        Self {
            name: "New block".into(),
            boards: Vec::new(),
            selected: None,
            mesh_dirty: true,
            status: String::new(),
            drag: BuilderDrag::default(),
            orbit: BuilderOrbit::default(),
        }
    }
}

impl ComponentPlan {
    pub fn load_type(&mut self, block: &CustomType) {
        self.name = block.name.clone();
        self.boards = block.boards.clone();
        self.selected = None;
        self.mesh_dirty = true;
        self.status = format!("Editing {}", block.name);
    }
}

/// Saved custom types. `customs.json` lives next to the program and is
/// gitignored with the other project files.
#[derive(Resource, Clone, Debug, Default)]
pub struct CustomLibrary {
    pub types: Vec<CustomType>,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct BuilderChrome {
    pub sidebar: f32,
    pub toolbar: f32,
}

impl Default for BuilderChrome {
    fn default() -> Self {
        Self {
            sidebar: 360.0,
            toolbar: 48.0,
        }
    }
}

impl CustomLibrary {
    pub fn load() -> Self {
        let path = customs_path();
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        let Ok(file) = serde_json::from_str::<CustomLibraryFile>(&text) else {
            return Self::default();
        };
        let mut library = Self { types: file.types };
        for block in &mut library.types {
            block.sanitize();
        }
        library
    }

    pub fn save_plan(&mut self, plan: &mut ComponentPlan) {
        let name = plan.name.trim();
        if name.is_empty() {
            plan.status = "Name the block before saving.".into();
            return;
        }
        if plan.boards.is_empty() {
            plan.status = "Add a board before saving.".into();
            return;
        }
        let mut block = CustomType {
            name: name.to_owned(),
            boards: plan.boards.clone(),
            width_mm: 1.0,
            height_mm: 1.0,
            depth_mm: 1.0,
        };
        block.sanitize();
        plan.name = block.name.clone();
        if let Some(existing) = self.types.iter_mut().find(|saved| saved.name == block.name) {
            *existing = block;
            plan.status = format!("Updated {}.", plan.name);
        } else {
            plan.status = format!("Saved {}.", block.name);
            self.types.push(block);
        }
        if let Err(error) = self.write() {
            plan.status = format!("Could not save ({error}).");
        }
    }

    fn write(&self) -> std::io::Result<()> {
        let file = CustomLibraryFile {
            types: self.types.clone(),
        };
        let text = serde_json::to_string_pretty(&file).unwrap_or_else(|_| "{}".into());
        std::fs::write(customs_path(), text)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CustomLibraryFile {
    types: Vec<CustomType>,
}

use serde::{Deserialize, Serialize};

fn customs_path() -> std::path::PathBuf {
    project_dir().join("customs.json")
}

pub fn load_saved(mut library: ResMut<CustomLibrary>) {
    *library = CustomLibrary::load();
}
