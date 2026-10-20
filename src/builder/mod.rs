//! A sketch for building a furniture block from loose boards.
//!
//! This is not a finished builder. It remembers a few named blocks and draws
//! a front view so the idea is visible. Joining the boards, saving them, and
//! placing the block in the room come later.

mod ui;

use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::window::WindowRef;
use bevy_egui::EguiSchedule;

pub use ui::draw as ui_draw;

/// Runs only for the component planner window.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ComponentPlannerPass;

#[derive(Component)]
pub struct ComponentPlannerWindow;

#[derive(Component)]
pub struct ComponentPlannerCamera;

#[derive(Resource, Default)]
pub struct ComponentWindow {
    window: Option<Entity>,
    camera: Option<Entity>,
    requested: bool,
}

pub fn request_open(state: &mut ComponentWindow) {
    state.requested = true;
}

pub fn open_requested(
    mut commands: Commands,
    mut state: ResMut<ComponentWindow>,
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
    resolution.set(1100.0, 760.0);
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
    let camera = commands
        .spawn((
            ComponentPlannerCamera,
            Camera2d,
            Camera {
                order: 2,
                clear_color: ClearColorConfig::Custom(Color::srgb(0.73, 0.70, 0.64)),
                ..default()
            },
            RenderTarget::Window(WindowRef::Entity(window)),
            EguiSchedule::new(ComponentPlannerPass),
        ))
        .id();
    state.window = Some(window);
    state.camera = Some(camera);
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
            state.window = None;
        }
    }
}

/// One board in a sketched block. Sizes are stored only so the sketch can
/// label them. They do not cut a sheet and they do not build a 3D piece.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoardRole {
    Wall,
    Door,
    Bottom,
    Top,
    Back,
}

impl BoardRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Wall => "Wall",
            Self::Door => "Door",
            Self::Bottom => "Bottom",
            Self::Top => "Top",
            Self::Back => "Back",
        }
    }

    fn default_size(self) -> (f32, f32) {
        match self {
            Self::Wall => (18.0, 720.0),
            Self::Door => (564.0, 684.0),
            Self::Bottom | Self::Top => (564.0, 18.0),
            Self::Back => (564.0, 684.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SketchBoard {
    pub role: BoardRole,
    pub width_mm: f32,
    pub height_mm: f32,
}

impl SketchBoard {
    pub fn new(role: BoardRole) -> Self {
        let (width_mm, height_mm) = role.default_size();
        Self {
            role,
            width_mm,
            height_mm,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockSketch {
    pub name: String,
    pub boards: Vec<SketchBoard>,
}

impl BlockSketch {
    pub fn sample() -> Self {
        Self {
            name: "Base block".into(),
            boards: vec![
                SketchBoard::new(BoardRole::Wall),
                SketchBoard::new(BoardRole::Wall),
                SketchBoard::new(BoardRole::Bottom),
                SketchBoard::new(BoardRole::Top),
                SketchBoard::new(BoardRole::Door),
            ],
        }
    }
}

/// The blocks being sketched. Nothing here is written into the project file.
#[derive(Resource, Clone, Debug)]
pub struct ComponentPlan {
    pub blocks: Vec<BlockSketch>,
    pub selected: usize,
}

impl Default for ComponentPlan {
    fn default() -> Self {
        Self {
            blocks: vec![BlockSketch::sample()],
            selected: 0,
        }
    }
}

impl ComponentPlan {
    pub fn selected_mut(&mut self) -> &mut BlockSketch {
        if self.blocks.is_empty() {
            self.blocks.push(BlockSketch::sample());
            self.selected = 0;
        }
        self.selected = self.selected.min(self.blocks.len() - 1);
        &mut self.blocks[self.selected]
    }

    pub fn add_block(&mut self) {
        let number = self.blocks.len() + 1;
        self.blocks.push(BlockSketch {
            name: format!("Block {number}"),
            boards: Vec::new(),
        });
        self.selected = self.blocks.len() - 1;
    }
}
