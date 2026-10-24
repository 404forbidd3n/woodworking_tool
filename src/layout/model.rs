//! Furniture, room size, and the millimeters-to-meters conversion.
//!
//! The 3D world is in meters because that is a comfortable scale for the
//! renderer. The sidebar speaks millimeters, which is how a cabinet is
//! actually measured. The room origin is the back-left corner of the floor:
//!
//! - X runs to the right along the back wall
//! - Y runs up
//! - Z runs out into the room
//!
//! A piece is positioned by the center of its base, not by a corner. Its
//! front face points along local +Z. A quarter-turn of 0 keeps that front
//! facing into the room.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const MM_PER_METER: f32 = 1000.0;

/// Sheet thickness used only to draw the carcass. This is not a cut list.
pub const PANEL_METERS: f32 = 0.018;

/// Gap drawn between doors and between drawer fronts.
pub const GAP_METERS: f32 = 0.003;

pub fn mm_to_m(mm: f32) -> f32 {
    mm / MM_PER_METER
}

pub fn m_to_mm(meters: f32) -> f32 {
    meters * MM_PER_METER
}

/// Round `value` to the nearest `step`. A step of zero leaves the value alone,
/// which is how the snap toggle turns the grid off.
pub fn snap_to_step(value: f32, step: f32) -> f32 {
    if step <= f32::EPSILON {
        value
    } else {
        (value / step).round() * step
    }
}

/// Things that can be placed. Existing variant names are kept because they are
/// also the JSON tags used by Stage 1 project files.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Kind {
    /// One cabinet, split into `count` drawer fronts of equal height.
    Drawers {
        count: u8,
    },
    /// A hinged door, or a pair of them.
    HingedDoors {
        doors: u8,
    },
    /// A door hinged along the top, drawn ajar so it is obvious.
    LiftDoor,
    /// Two doors that pass in front of each other.
    SlidingDoors,
    /// A top and four legs. No carcass.
    Table,
    /// A thick kitchen worktop slab, placed independently over base cabinets.
    Countertop,
    /// A lighter standalone top for a custom table base.
    Tabletop,
    /// An open-front cabinet with editable internal shelves.
    #[serde(alias = "box")]
    OpenShelves {
        #[serde(default = "default_shelf_count")]
        shelves: u8,
    },
    Refrigerator,
    Dishwasher,
    ElectricOven,
    CoffeeMachine,
    Microwave,
    SquareSink,
    RoundSink,
    DoubleSink,
    SinkFaucet,
    CentralHeatingBoiler,
    AirConditioner,
    /// A utility pipe, or a pair of them, that pieces must work around.
    Pipe {
        #[serde(default)]
        horizontal: bool,
        #[serde(default = "default_pipe_count")]
        count: u8,
        #[serde(default)]
        color: PipeColor,
    },
    /// An L-shaped cabinet that sits in a wall corner.
    /// Width runs along the back wall and depth along the left wall.
    Corner {
        style: CornerStyle,
    },
    /// A block built in the component planner from loose boards.
    Custom,
}

/// How a corner cabinet is closed. Both styles share the same L-shaped carcass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CornerStyle {
    /// Two small doors, one on each wall, with shelves in the corner bay.
    ShelfDoors,
    /// Two hinged doors across the diagonal opening.
    DoubleDoor,
}

/// Solid painted fronts, or a wood frame with a glass pane.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoorStyle {
    #[default]
    Solid,
    Glass,
}

/// What a loose board is for. The shape is still just a box: a wall can be
/// taller than the one beside it, and a top can be smaller than the bottom.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomRole {
    Plank,
    Wall,
    Bottom,
    Top,
    Door,
    Back,
}

impl CustomRole {
    pub const ALL: [Self; 6] = [
        Self::Plank,
        Self::Wall,
        Self::Bottom,
        Self::Top,
        Self::Door,
        Self::Back,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Plank => "Plank",
            Self::Wall => "Wall",
            Self::Bottom => "Bottom",
            Self::Top => "Top",
            Self::Door => "Door",
            Self::Back => "Back",
        }
    }

    pub fn default_board(self) -> CustomBoard {
        let (width_mm, height_mm, depth_mm, x_mm, y_mm, z_mm) = match self {
            Self::Wall => (18.0, 720.0, 560.0, -291.0, 360.0, 0.0),
            Self::Plank => (564.0, 18.0, 500.0, 0.0, 360.0, 0.0),
            Self::Bottom => (564.0, 18.0, 560.0, 0.0, 9.0, 0.0),
            Self::Top => (500.0, 18.0, 480.0, 0.0, 711.0, 0.0),
            Self::Door => (564.0, 680.0, 18.0, 0.0, 360.0, 289.0),
            Self::Back => (564.0, 684.0, 18.0, 0.0, 360.0, -271.0),
        };
        CustomBoard {
            role: self,
            width_mm,
            height_mm,
            depth_mm,
            x_mm,
            y_mm,
            z_mm,
            yaw_deg: 0.0,
            pitch_deg: 0.0,
            roll_deg: 0.0,
            turns: 0,
        }
    }
}

/// One board inside a custom block. Position is the centre of the board.
/// Y grows up from the builder floor. Yaw turns it around the vertical.
/// Pitch and roll tilt it, so a roof can sit at any slope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomBoard {
    pub role: CustomRole,
    pub width_mm: f32,
    pub height_mm: f32,
    pub depth_mm: f32,
    pub x_mm: f32,
    pub y_mm: f32,
    pub z_mm: f32,
    /// Degrees around Y. Zero keeps the board's width along X.
    #[serde(default)]
    pub yaw_deg: f32,
    /// Degrees around the board's width. A roof uses this.
    #[serde(default)]
    pub pitch_deg: f32,
    /// Degrees around the board's depth.
    #[serde(default)]
    pub roll_deg: f32,
    /// Quarter turns from the first rotation control. Folded into yaw on clean.
    #[serde(default)]
    pub turns: u8,
}

impl CustomBoard {
    pub fn sanitize(&mut self) {
        self.width_mm = finite_or(self.width_mm, 18.0).clamp(1.0, 4000.0);
        self.height_mm = finite_or(self.height_mm, 18.0).clamp(1.0, 4000.0);
        self.depth_mm = finite_or(self.depth_mm, 18.0).clamp(1.0, 4000.0);
        self.x_mm = finite_or(self.x_mm, 0.0).clamp(-10000.0, 10000.0);
        self.y_mm = finite_or(self.y_mm, 0.0).clamp(-10000.0, 10000.0);
        self.z_mm = finite_or(self.z_mm, 0.0).clamp(-10000.0, 10000.0);
        self.yaw_deg = wrap_degrees(finite_or(self.yaw_deg, 0.0) + f32::from(self.turns) * 90.0);
        self.pitch_deg = wrap_degrees(finite_or(self.pitch_deg, 0.0));
        self.roll_deg = wrap_degrees(finite_or(self.roll_deg, 0.0));
        self.turns = 0;
    }

    /// Yaw, then pitch, then roll. Pitch lifts the depth edge, which is the
    /// slope of a roof. An old quarter-turn count still counts as yaw.
    pub fn rotation(&self) -> Quat {
        let yaw = (self.yaw_deg + f32::from(self.turns) * 90.0).to_radians();
        Quat::from_rotation_y(yaw)
            * Quat::from_rotation_x(self.pitch_deg.to_radians())
            * Quat::from_rotation_z(self.roll_deg.to_radians())
    }

    /// Half-size of the world-aligned box around the rotated board, in millimetres.
    pub fn world_half_mm(&self) -> Vec3 {
        let half = Vec3::new(self.width_mm, self.height_mm, self.depth_mm) * 0.5;
        let rotation = self.rotation();
        let mut extent = Vec3::ZERO;
        for x in [-half.x, half.x] {
            for y in [-half.y, half.y] {
                for z in [-half.z, half.z] {
                    extent = extent.max((rotation * Vec3::new(x, y, z)).abs());
                }
            }
        }
        extent
    }

    /// Keep the lowest corner on the floor.
    pub fn rest_on_floor(&mut self) {
        let clearance = self.world_half_mm().y;
        if self.y_mm < clearance {
            self.y_mm = clearance;
        }
    }

    /// Local box size in metres, scaled on the board's own axes, plus its tilt.
    /// The slope stays at the angle it was drawn.
    pub fn oriented(&self, sx: f32, sy: f32, sz: f32) -> (Vec3, Quat) {
        let size = Vec3::new(
            mm_to_m(self.width_mm * sx),
            mm_to_m(self.height_mm * sy),
            mm_to_m(self.depth_mm * sz),
        );
        (size, self.rotation())
    }
}

pub(crate) fn wrap_degrees(degrees: f32) -> f32 {
    let wrapped = degrees.rem_euclid(360.0);
    if wrapped > 180.0 {
        wrapped - 360.0
    } else {
        wrapped
    }
}

/// A saved furniture type. `width_mm`, `height_mm`, and `depth_mm` are the
/// size of the block as designed. Placing it in the room starts at that size,
/// and the properties sliders scale every board from there.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomType {
    pub name: String,
    pub boards: Vec<CustomBoard>,
    pub width_mm: f32,
    pub height_mm: f32,
    pub depth_mm: f32,
}

impl CustomType {
    pub fn empty(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            boards: Vec::new(),
            width_mm: 600.0,
            height_mm: 720.0,
            depth_mm: 560.0,
        }
    }

    pub fn door_count(&self) -> usize {
        self.boards
            .iter()
            .filter(|board| board.role == CustomRole::Door)
            .count()
    }

    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for board in &self.boards {
            let center = Vec3::new(board.x_mm, board.y_mm, board.z_mm);
            let half = board.world_half_mm();
            min = min.min(center - half);
            max = max.max(center + half);
        }
        if self.boards.is_empty() {
            (Vec3::ZERO, Vec3::new(600.0, 720.0, 560.0))
        } else {
            (min, max)
        }
    }

    /// Store the outer size of the boards so later scaling has a stable base.
    pub fn recompute_nominal(&mut self) {
        let (min, max) = self.bounds();
        let size = max - min;
        self.width_mm = finite_or(size.x, 600.0).max(1.0);
        self.height_mm = finite_or(size.y, 720.0).max(1.0);
        self.depth_mm = finite_or(size.z, 560.0).max(1.0);
    }

    pub fn sanitize(&mut self) {
        if self.name.trim().is_empty() {
            self.name = "Custom block".to_owned();
        }
        for board in &mut self.boards {
            board.sanitize();
        }
        self.recompute_nominal();
    }

    pub fn scaled_solids(&self, width_mm: f32, height_mm: f32, depth_mm: f32) -> Vec<Solid> {
        if self.boards.is_empty() {
            return Vec::new();
        }
        let (min, max) = self.bounds();
        let sx = width_mm / self.width_mm.max(1.0);
        let sy = height_mm / self.height_mm.max(1.0);
        let sz = depth_mm / self.depth_mm.max(1.0);
        let center_x = (min.x + max.x) * 0.5;
        let center_z = (min.z + max.z) * 0.5;
        let bottom = min.y;
        let mut door = 0u8;
        self.boards
            .iter()
            .map(|board| {
                let role = if board.role == CustomRole::Door {
                    let index = door;
                    door += 1;
                    Surface::Front(index)
                } else {
                    Surface::Carcass(0)
                };
                let (size, rotation) = board.oriented(sx, sy, sz);
                Solid {
                    size: size.max(Vec3::splat(0.001)),
                    translation: Vec3::new(
                        mm_to_m((board.x_mm - center_x) * sx),
                        mm_to_m((board.y_mm - bottom) * sy),
                        mm_to_m((board.z_mm - center_z) * sz),
                    ),
                    rotation,
                    role,
                }
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipeColor {
    #[default]
    Gray,
    Black,
    Blue,
}

impl PipeColor {
    pub const ALL: [Self; 3] = [Self::Gray, Self::Black, Self::Blue];

    pub fn label(self) -> &'static str {
        match self {
            Self::Gray => "Gray",
            Self::Black => "Black",
            Self::Blue => "Blue",
        }
    }

    pub fn rgb(self) -> [u8; 3] {
        match self {
            Self::Gray => [150, 154, 158],
            Self::Black => [32, 33, 35],
            Self::Blue => [46, 104, 178],
        }
    }
}

impl Kind {
    pub fn label(self) -> &'static str {
        self.label_for(Placement::Floor)
    }

    pub fn label_for(self, placement: Placement) -> &'static str {
        match self {
            Kind::Drawers { .. } if placement == Placement::Wall => "Wall Drawer Cabinet",
            Kind::Drawers { .. } => "Drawer Base Cabinet",
            Kind::HingedDoors { doors: 1 } if placement == Placement::Wall => {
                "Single-Door Wall Cabinet"
            }
            Kind::HingedDoors { .. } if placement == Placement::Wall => "Double-Door Wall Cabinet",
            Kind::HingedDoors { doors: 1 } => "Single-Door Base Cabinet",
            Kind::HingedDoors { .. } => "Double-Door Base Cabinet",
            Kind::LiftDoor => "Wall Lift-Up Cabinet",
            Kind::SlidingDoors => "Sliding-Door Cabinet",
            Kind::Table => "Work Table",
            Kind::Countertop => "Kitchen Worktop",
            Kind::Tabletop => "Tabletop",
            Kind::OpenShelves { .. } => "Open Shelf Cabinet",
            Kind::Refrigerator => "Refrigerator",
            Kind::Dishwasher => "Dishwasher",
            Kind::ElectricOven => "Electric Oven",
            Kind::CoffeeMachine => "Coffee Machine",
            Kind::Microwave => "Microwave",
            Kind::SquareSink => "Square Kitchen Sink",
            Kind::RoundSink => "Round Kitchen Sink",
            Kind::DoubleSink => "Double-Bowl Kitchen Sink",
            Kind::SinkFaucet => "Kitchen Faucet / Water Pipe",
            Kind::CentralHeatingBoiler => "Central Heating Boiler",
            Kind::AirConditioner => "Air Conditioner",
            Kind::Pipe {
                horizontal: false,
                count: 1,
                ..
            } => "Vertical Pipe",
            Kind::Pipe {
                horizontal: false, ..
            } => "Double Vertical Pipe",
            Kind::Pipe { count: 1, .. } => "Horizontal Pipe",
            Kind::Pipe { .. } => "Double Horizontal Pipe",
            Kind::Corner {
                style: CornerStyle::ShelfDoors,
            } if placement == Placement::Wall => "Corner Wall Cabinet, Shelves",
            Kind::Corner {
                style: CornerStyle::DoubleDoor,
            } if placement == Placement::Wall => "Corner Wall Cabinet, Double Door",
            Kind::Corner {
                style: CornerStyle::ShelfDoors,
            } => "Corner Base Cabinet, Shelves",
            Kind::Corner {
                style: CornerStyle::DoubleDoor,
            } => "Corner Base Cabinet, Double Door",
            Kind::Custom => "Custom block",
        }
    }

    pub fn is_pipe(self) -> bool {
        matches!(self, Kind::Pipe { .. })
    }

    pub fn front_count(self) -> usize {
        match self {
            Kind::Drawers { count } => count.max(1) as usize,
            Kind::HingedDoors { doors } => doors.clamp(1, 2) as usize,
            Kind::LiftDoor => 1,
            Kind::SlidingDoors => 2,
            Kind::Corner { .. } => 2,
            _ => 0,
        }
    }

    pub fn has_doors(self) -> bool {
        self.front_count() > 0
    }

    pub fn is_appliance(self) -> bool {
        matches!(
            self,
            Kind::Refrigerator
                | Kind::Dishwasher
                | Kind::ElectricOven
                | Kind::CoffeeMachine
                | Kind::Microwave
                | Kind::SquareSink
                | Kind::RoundSink
                | Kind::DoubleSink
                | Kind::SinkFaucet
                | Kind::CentralHeatingBoiler
                | Kind::AirConditioner
        )
    }

    pub fn is_cabinet(self) -> bool {
        matches!(
            self,
            Kind::Drawers { .. }
                | Kind::HingedDoors { .. }
                | Kind::LiftDoor
                | Kind::SlidingDoors
                | Kind::OpenShelves { .. }
                | Kind::Corner { .. }
                | Kind::Custom
        )
    }

    /// Sizes a woodworker would actually start from, plus how high the base
    /// of the piece sits. Wall cabinets start above a 720 mm base.
    pub fn preset(self, placement: Placement) -> (Furniture, f32) {
        let (width, height, depth, y_mm) = match self {
            Kind::Drawers { .. } | Kind::HingedDoors { .. } => {
                if placement == Placement::Wall {
                    (600.0, 720.0, 350.0, 1400.0)
                } else {
                    (600.0, 720.0, 560.0, 0.0)
                }
            }
            Kind::Custom => (600.0, 720.0, 560.0, 0.0),
            Kind::Corner { .. } => {
                // 600 mm along each wall. The arms are 350 mm deep, so a 560 mm
                // base depth would leave almost no opening on a 600 mm plan.
                if placement == Placement::Wall {
                    (600.0, 720.0, 600.0, 1400.0)
                } else {
                    (600.0, 720.0, 600.0, 0.0)
                }
            }
            Kind::LiftDoor => (600.0, 400.0, 350.0, 1400.0),
            Kind::SlidingDoors => (1000.0, 2100.0, 600.0, 0.0),
            Kind::Table => (1400.0, 750.0, 800.0, 0.0),
            Kind::Countertop => (2400.0, 38.0, 635.0, 720.0),
            Kind::Tabletop => (1400.0, 30.0, 800.0, 720.0),
            Kind::OpenShelves { .. } => (600.0, 720.0, 560.0, 0.0),
            Kind::Refrigerator => (900.0, 1850.0, 700.0, 0.0),
            Kind::Dishwasher => (600.0, 820.0, 600.0, 0.0),
            Kind::ElectricOven => (600.0, 600.0, 570.0, 720.0),
            Kind::CoffeeMachine => (260.0, 350.0, 420.0, 900.0),
            Kind::Microwave => (520.0, 310.0, 400.0, 1200.0),
            // Sink tops align with the 758 mm top of the default worktop.
            Kind::SquareSink => (500.0, 180.0, 450.0, 578.0),
            Kind::RoundSink => (450.0, 180.0, 450.0, 578.0),
            Kind::DoubleSink => (850.0, 200.0, 500.0, 558.0),
            Kind::SinkFaucet => (220.0, 420.0, 240.0, 758.0),
            Kind::CentralHeatingBoiler => (500.0, 700.0, 300.0, 1400.0),
            Kind::AirConditioner => (900.0, 300.0, 230.0, 1900.0),
            Kind::Pipe {
                horizontal, count, ..
            } => {
                let span = if count >= 2 {
                    PIPE_DIAMETER_MM + PIPE_PAIR_GAP_MM
                } else {
                    PIPE_DIAMETER_MM
                };
                if horizontal {
                    (1000.0, span, PIPE_DIAMETER_MM, 500.0)
                } else {
                    (span, 1000.0, PIPE_DIAMETER_MM, 0.0)
                }
            }
        };
        let mut furniture = Furniture::new(self, width, height, depth);
        furniture.placement = placement;
        furniture.name = self.label_for(placement).to_owned();
        furniture.legs = if placement == Placement::Floor && self.is_cabinet() {
            LegStyle::Plastic100
        } else {
            LegStyle::None
        };
        (furniture, y_mm)
    }
}

const fn default_shelf_count() -> u8 {
    3
}

const fn default_pipe_count() -> u8 {
    1
}

/// Default pipe thickness. Depth on a pipe piece is this diameter.
pub const PIPE_DIAMETER_MM: f32 = 20.0;
/// Extra width or height between the outer faces of a pair of pipes.
pub const PIPE_PAIR_GAP_MM: f32 = 80.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    #[default]
    Floor,
    Wall,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegStyle {
    #[default]
    None,
    Plastic100,
    Plastic150,
    Metal100,
    Metal150,
    Wood100,
}

impl LegStyle {
    pub const ALL: [Self; 6] = [
        Self::None,
        Self::Plastic100,
        Self::Plastic150,
        Self::Metal100,
        Self::Metal150,
        Self::Wood100,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Plastic100 => "Plastic · 100 mm",
            Self::Plastic150 => "Plastic · 150 mm",
            Self::Metal100 => "Metal · 100 mm",
            Self::Metal150 => "Metal · 150 mm",
            Self::Wood100 => "Wood · 100 mm",
        }
    }

    pub fn height_mm(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Plastic100 | Self::Metal100 | Self::Wood100 => 100.0,
            Self::Plastic150 | Self::Metal150 => 150.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishPreset {
    Oak,
    Walnut,
    White,
    Charcoal,
    StainlessSteel,
    Custom,
    Pine,
    WhitePine,
    Poplar,
    Cherry,
    Birch,
    Blue,
    NordicWhite,
    MilitaryGreen,
    Black,
}

/// A named material plus an editable fallback color. Materials are deliberately
/// visual only: this draft does not calculate sheet stock or cut lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Finish {
    pub preset: FinishPreset,
    pub color: [u8; 3],
}

impl Finish {
    pub const fn new(preset: FinishPreset, color: [u8; 3]) -> Self {
        Self { preset, color }
    }

    pub fn rgb(self) -> [u8; 3] {
        if self.preset == FinishPreset::Custom {
            self.color
        } else {
            self.preset.rgb()
        }
    }
}

impl FinishPreset {
    pub const PAINTS: [Self; 4] = [
        Self::Blue,
        Self::NordicWhite,
        Self::MilitaryGreen,
        Self::Black,
    ];
    pub const WOODS: [Self; 7] = [
        Self::Pine,
        Self::Oak,
        Self::Walnut,
        Self::WhitePine,
        Self::Poplar,
        Self::Cherry,
        Self::Birch,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Oak => "Oak",
            Self::Walnut => "Walnut",
            Self::White => "White",
            Self::Charcoal => "Charcoal",
            Self::StainlessSteel => "Stainless",
            Self::Custom => "Custom",
            Self::Pine => "Pine",
            Self::WhitePine => "White pine",
            Self::Poplar => "Poplar",
            Self::Cherry => "Cherry",
            Self::Birch => "Birch",
            Self::Blue => "Blue",
            Self::NordicWhite => "Nordic white",
            Self::MilitaryGreen => "Military green",
            Self::Black => "Black",
        }
    }

    /// Flat color used for paint, and the light tone of a wood.
    pub fn rgb(self) -> [u8; 3] {
        match self {
            Self::Oak => [196, 150, 92],
            Self::Walnut => [112, 74, 46],
            Self::White => [238, 236, 229],
            Self::Charcoal => [55, 58, 60],
            Self::StainlessSteel => [165, 172, 175],
            Self::Custom => [180, 180, 180],
            Self::Pine => [214, 176, 112],
            Self::WhitePine => [232, 214, 184],
            Self::Poplar => [206, 196, 156],
            Self::Cherry => [166, 78, 58],
            Self::Birch => [236, 220, 196],
            Self::Blue => [48, 98, 168],
            Self::NordicWhite => [247, 236, 214],
            Self::MilitaryGreen => [74, 90, 58],
            Self::Black => [22, 24, 26],
        }
    }

    /// Darker streak laid over [`Self::rgb`] to fake grain. Paints return themselves.
    pub fn grain(self) -> [u8; 3] {
        match self {
            Self::Oak => [132, 88, 46],
            Self::Walnut => [62, 38, 24],
            Self::Pine => [160, 112, 58],
            Self::WhitePine => [186, 160, 122],
            Self::Poplar => [168, 156, 118],
            Self::Cherry => [108, 42, 34],
            Self::Birch => [190, 166, 132],
            other => other.rgb(),
        }
    }

    pub fn is_wood(self) -> bool {
        matches!(
            self,
            Self::Oak
                | Self::Walnut
                | Self::Pine
                | Self::WhitePine
                | Self::Poplar
                | Self::Cherry
                | Self::Birch
        )
    }
}

impl Default for Finish {
    fn default() -> Self {
        Self::new(FinishPreset::Oak, [181, 137, 84])
    }
}

/// Editable finishes for every visible family of parts. Carcass order is
/// left, right, bottom, top, back. Fronts follows the visible front order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PartFinishes {
    pub carcass: [Finish; 5],
    pub fronts: Vec<Finish>,
    pub handles: Finish,
    pub top: Finish,
    pub appliance: Finish,
}

impl Default for PartFinishes {
    fn default() -> Self {
        Self {
            carcass: [Finish::default(); 5],
            fronts: Vec::new(),
            handles: Finish::new(FinishPreset::Charcoal, [55, 58, 60]),
            top: Finish::new(FinishPreset::Walnut, [104, 70, 45]),
            appliance: Finish::new(FinishPreset::StainlessSteel, [165, 172, 175]),
        }
    }
}

/// Dimensions live here. The position and facing live on [`Transform`],
/// so dragging a piece does not have to rebuild its mesh.
///
/// `Visibility` is required because the panels are child entities. Bevy will
/// not propagate visibility through a parent that does not have it, and the
/// cabinets would fail to draw.
#[derive(Component, Clone, Debug)]
#[require(Visibility)]
pub struct Furniture {
    pub kind: Kind,
    pub name: String,
    pub placement: Placement,
    pub legs: LegStyle,
    /// Solid or glass fronts. Drawers and open pieces ignore this.
    pub doors: DoorStyle,
    /// Boards for [`Kind::Custom`]. Other kinds leave this empty.
    pub custom: Option<CustomType>,
    pub width_mm: f32,
    pub height_mm: f32,
    pub depth_mm: f32,
    pub finishes: PartFinishes,
    /// Set when the mesh no longer matches these dimensions.
    pub dirty: bool,
}

impl Furniture {
    pub fn new(kind: Kind, width_mm: f32, height_mm: f32, depth_mm: f32) -> Self {
        let placement = if kind == Kind::LiftDoor {
            Placement::Wall
        } else {
            Placement::Floor
        };
        let mut furniture = Self {
            kind,
            name: kind.label().to_owned(),
            placement,
            legs: if placement == Placement::Floor && kind.is_cabinet() {
                LegStyle::Plastic100
            } else {
                LegStyle::None
            },
            doors: DoorStyle::Solid,
            custom: None,
            width_mm,
            height_mm,
            depth_mm,
            finishes: PartFinishes::default(),
            dirty: true,
        };
        furniture.ensure_front_finishes();
        furniture
    }

    pub fn ensure_front_finishes(&mut self) {
        let count = self.front_slots();
        self.finishes.fronts.resize(count, Finish::default());
        self.finishes.fronts.truncate(count);
    }

    pub fn size_meters(&self) -> Vec3 {
        // Pipes are much thinner than a cabinet panel, so they keep a smaller floor.
        let minimum = if self.kind.is_pipe() { 0.008 } else { 0.05 };
        Vec3::new(
            mm_to_m(self.width_mm).max(minimum),
            mm_to_m(self.height_mm).max(minimum),
            mm_to_m(self.depth_mm).max(minimum),
        )
    }

    pub fn leg_height_meters(&self) -> f32 {
        if self.placement == Placement::Floor && self.kind.is_cabinet() {
            mm_to_m(self.legs.height_mm())
        } else {
            0.0
        }
    }

    pub fn label(&self) -> &'static str {
        self.kind.label_for(self.placement)
    }

    pub fn front_slots(&self) -> usize {
        if self.kind == Kind::Custom {
            self.custom
                .as_ref()
                .map(|block| block.door_count())
                .unwrap_or(0)
        } else {
            self.kind.front_count()
        }
    }
}

/// Floor rectangle and four walls. Each wall can be hidden so the camera can
/// look through that side.
#[derive(Resource, Clone, Debug)]
pub struct Room {
    pub width_mm: f32,
    pub depth_mm: f32,
    pub wall_height_mm: f32,
    /// Master switch. Individual walls stay as they were when this is off.
    pub show_walls: bool,
    pub walls: WallVisibility,
    /// A safety lock for dimensions, openings, and utility pipes.
    pub edit_enabled: bool,
    /// View-only: wall, door, and window dimension lines. Not saved.
    pub show_measurements: bool,
    pub openings: Vec<Opening>,
    pub dirty: bool,
}

impl Default for Room {
    fn default() -> Self {
        Self {
            width_mm: 4200.0,
            depth_mm: 3000.0,
            wall_height_mm: 2400.0,
            show_walls: true,
            walls: WallVisibility::default(),
            edit_enabled: false,
            show_measurements: false,
            openings: Vec::new(),
            dirty: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpeningKind {
    Door,
    Window,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WallSide {
    Back,
    Left,
    Right,
    Front,
}

impl WallSide {
    pub const ALL: [Self; 4] = [Self::Back, Self::Left, Self::Right, Self::Front];

    pub fn label(self) -> &'static str {
        match self {
            Self::Back => "Back wall",
            Self::Left => "Left wall",
            Self::Right => "Right wall",
            Self::Front => "Front wall",
        }
    }
}

/// Which of the four walls are drawn. The front starts hidden so the room can
/// still be seen from the camera's starting side.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WallVisibility {
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub front: bool,
}

impl Default for WallVisibility {
    fn default() -> Self {
        Self {
            back: true,
            left: true,
            right: true,
            front: false,
        }
    }
}

impl WallVisibility {
    pub fn visible(self, side: WallSide) -> bool {
        match side {
            WallSide::Back => self.back,
            WallSide::Left => self.left,
            WallSide::Right => self.right,
            WallSide::Front => self.front,
        }
    }

    pub fn set(&mut self, side: WallSide, visible: bool) {
        match side {
            WallSide::Back => self.back = visible,
            WallSide::Left => self.left = visible,
            WallSide::Right => self.right = visible,
            WallSide::Front => self.front = visible,
        }
    }

    /// Rooms saved before the front wall existed.
    pub fn legacy(show_walls: bool) -> Self {
        Self {
            back: show_walls,
            left: show_walls,
            right: show_walls,
            front: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Opening {
    pub kind: OpeningKind,
    pub wall: WallSide,
    /// Distance from the wall's left/back end.
    pub offset_mm: f32,
    pub width_mm: f32,
    pub height_mm: f32,
    /// Doors always sanitize this to zero.
    pub sill_mm: f32,
}

impl Opening {
    pub fn door() -> Self {
        Self {
            kind: OpeningKind::Door,
            wall: WallSide::Back,
            offset_mm: 800.0,
            width_mm: 900.0,
            height_mm: 2050.0,
            sill_mm: 0.0,
        }
    }

    pub fn window() -> Self {
        Self {
            kind: OpeningKind::Window,
            wall: WallSide::Back,
            offset_mm: 1000.0,
            width_mm: 1200.0,
            height_mm: 1000.0,
            sill_mm: 900.0,
        }
    }
}

impl Room {
    pub fn wall_shown(&self, side: WallSide) -> bool {
        self.show_walls && self.walls.visible(side)
    }

    pub fn size_meters(&self) -> Vec3 {
        Vec3::new(
            mm_to_m(self.width_mm).max(0.5),
            mm_to_m(self.wall_height_mm).max(0.5),
            mm_to_m(self.depth_mm).max(0.5),
        )
    }
}

#[derive(Resource, Debug)]
pub struct Editor {
    pub snap_enabled: bool,
    pub snap_mm: f32,
    pub path: String,
    pub status: String,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            snap_enabled: true,
            snap_mm: 50.0,
            // A short name. Saving resolves it inside the project folder.
            path: "layout.json".into(),
            status: String::new(),
        }
    }
}

impl Editor {
    /// Snap step in meters, or zero when snapping is off.
    pub fn snap_step_meters(&self) -> f32 {
        if self.snap_enabled {
            mm_to_m(self.snap_mm.max(1.0))
        } else {
            0.0
        }
    }
}

#[derive(Resource, Default, Debug)]
pub struct Selection {
    pub entities: Vec<Entity>,
    pub primary: Option<Entity>,
}

impl Selection {
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.entities.contains(&entity)
    }

    pub fn set_only(&mut self, entity: Entity) {
        self.entities.clear();
        self.entities.push(entity);
        self.primary = Some(entity);
    }

    pub fn toggle(&mut self, entity: Entity) {
        if let Some(index) = self.entities.iter().position(|item| *item == entity) {
            self.entities.remove(index);
            if self.primary == Some(entity) {
                self.primary = self.entities.last().copied();
            }
        } else {
            self.entities.push(entity);
            self.primary = Some(entity);
        }
    }

    pub fn clear(&mut self) {
        self.entities.clear();
        self.primary = None;
    }

    pub fn retain(&mut self, mut keep: impl FnMut(Entity) -> bool) {
        self.entities.retain(|entity| keep(*entity));
        if self
            .primary
            .is_some_and(|entity| !self.entities.contains(&entity))
        {
            self.primary = self.entities.last().copied();
        }
    }
}

/// Marks the entity whose children are the floor and walls.
#[derive(Component)]
pub struct RoomRoot;

/// Which stock color a generated box should use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Carcass(u8),
    Front(u8),
    Handle,
    Top,
    Appliance(u8),
    /// A round appliance part. Size uses diameter × height × diameter.
    ApplianceRound(u8),
    /// A clear pane set into a door frame.
    Glass,
    Leg(LegStyle),
    Pipe,
    Floor,
    Wall,
}

/// One box in a piece, in the piece's local space.
/// The local origin is the center of the base.
#[derive(Clone, Copy, Debug)]
pub struct Solid {
    pub size: Vec3,
    pub translation: Vec3,
    pub rotation: Quat,
    pub role: Surface,
}

impl Solid {
    fn axis(size: Vec3, translation: Vec3, role: Surface) -> Self {
        Self {
            size: size.max(Vec3::splat(0.001)),
            translation,
            rotation: Quat::IDENTITY,
            role,
        }
    }
}

/// Build the boxes that represent a piece. Every cabinet is a five-panel
/// carcass (two sides, top, bottom, back) plus fronts. Panels meet at edges
/// and do not share faces, so the seams do not flicker.
pub fn solids(furniture: &Furniture) -> Vec<Solid> {
    let size = furniture.size_meters();
    let mut parts = match furniture.kind {
        Kind::Table => table(size),
        Kind::Countertop | Kind::Tabletop => vec![Solid::axis(
            size,
            Vec3::new(0.0, size.y * 0.5, 0.0),
            Surface::Top,
        )],
        Kind::OpenShelves { shelves } => {
            let mut parts = carcass(size);
            parts.extend(open_shelves(size, shelves.clamp(1, 10)));
            parts
        }
        kind @ (Kind::Refrigerator
        | Kind::Dishwasher
        | Kind::ElectricOven
        | Kind::CoffeeMachine
        | Kind::Microwave
        | Kind::SquareSink
        | Kind::RoundSink
        | Kind::DoubleSink
        | Kind::SinkFaucet
        | Kind::CentralHeatingBoiler
        | Kind::AirConditioner) => appliance(size, kind),
        Kind::Pipe {
            horizontal, count, ..
        } => pipes(size, horizontal, count),
        Kind::Drawers { count } => {
            let mut parts = carcass(size);
            parts.extend(drawer_fronts(size, count.max(1)));
            parts
        }
        Kind::HingedDoors { doors } => {
            let mut parts = carcass(size);
            if furniture.doors == DoorStyle::Glass {
                parts.extend(open_shelves(size, 2));
            }
            parts.extend(hinged_fronts(size, doors.clamp(1, 2), furniture.doors));
            parts
        }
        Kind::LiftDoor => {
            let mut parts = carcass(size);
            parts.extend(lift_front(size, furniture.doors));
            parts
        }
        Kind::SlidingDoors => {
            let mut parts = carcass(size);
            parts.extend(sliding_fronts(size, furniture.doors));
            parts
        }
        Kind::Corner { style } => corner_cabinet(size, style, furniture.doors),
        Kind::Custom => furniture
            .custom
            .as_ref()
            .map(|block| {
                block.scaled_solids(furniture.width_mm, furniture.height_mm, furniture.depth_mm)
            })
            .unwrap_or_default(),
    };
    let leg_height = furniture.leg_height_meters();
    if leg_height > 0.0 {
        for part in &mut parts {
            part.translation.y += leg_height;
        }
        parts.extend(adjustable_legs(size, leg_height, furniture.legs));
    }
    parts
}

/// Pipe diameter is the piece depth. Double pipes sit at the two ends of the
/// cross-section so the outer box still bounds them.
fn pipes(size: Vec3, horizontal: bool, count: u8) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    if horizontal {
        let d = d.min(h);
        let ys: Vec<f32> = if count >= 2 {
            vec![d * 0.5, h - d * 0.5]
        } else {
            vec![h * 0.5]
        };
        ys.into_iter()
            .map(|y| Solid::axis(Vec3::new(w, d, d), Vec3::new(0.0, y, 0.0), Surface::Pipe))
            .collect()
    } else {
        let d = d.min(w);
        let xs: Vec<f32> = if count >= 2 {
            vec![-(w - d) * 0.5, (w - d) * 0.5]
        } else {
            vec![0.0]
        };
        xs.into_iter()
            .map(|x| {
                Solid::axis(
                    Vec3::new(d, h, d),
                    Vec3::new(x, h * 0.5, 0.0),
                    Surface::Pipe,
                )
            })
            .collect()
    }
}

fn open_shelves(size: Vec3, count: u8) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let t = PANEL_METERS.min(w * 0.2).min(h * 0.2).min(d * 0.2);
    let inner_w = (w - 2.0 * t).max(0.01);
    (1..=count)
        .map(|index| {
            let y = t + (h - 2.0 * t) * index as f32 / (count as f32 + 1.0);
            Solid::axis(
                Vec3::new(inner_w, t, d),
                Vec3::new(0.0, y, 0.0),
                Surface::Carcass(2),
            )
        })
        .collect()
}

fn adjustable_legs(size: Vec3, height: f32, style: LegStyle) -> Vec<Solid> {
    let foot = 0.045_f32.min(size.x * 0.15).min(size.z * 0.15);
    let inset_x = (foot + 0.05).min(size.x * 0.35);
    let inset_z = (foot + 0.05).min(size.z * 0.35);
    let mut parts = Vec::with_capacity(4);
    for x_sign in [-1.0, 1.0] {
        for z_sign in [-1.0, 1.0] {
            parts.push(Solid::axis(
                Vec3::new(foot, height, foot),
                Vec3::new(
                    x_sign * (size.x * 0.5 - inset_x),
                    height * 0.5,
                    z_sign * (size.z * 0.5 - inset_z),
                ),
                Surface::Leg(style),
            ));
        }
    }
    parts
}

fn carcass(size: Vec3) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let t = PANEL_METERS.min(w * 0.2).min(h * 0.2).min(d * 0.2);
    let inner_w = (w - 2.0 * t).max(0.01);
    let inner_h = (h - 2.0 * t).max(0.01);
    vec![
        Solid::axis(
            Vec3::new(t, h, d),
            Vec3::new(-w * 0.5 + t * 0.5, h * 0.5, 0.0),
            Surface::Carcass(0),
        ),
        Solid::axis(
            Vec3::new(t, h, d),
            Vec3::new(w * 0.5 - t * 0.5, h * 0.5, 0.0),
            Surface::Carcass(1),
        ),
        Solid::axis(
            Vec3::new(inner_w, t, d),
            Vec3::new(0.0, t * 0.5, 0.0),
            Surface::Carcass(2),
        ),
        Solid::axis(
            Vec3::new(inner_w, t, d),
            Vec3::new(0.0, h - t * 0.5, 0.0),
            Surface::Carcass(3),
        ),
        Solid::axis(
            Vec3::new(inner_w, inner_h, t),
            Vec3::new(0.0, h * 0.5, -d * 0.5 + t * 0.5),
            Surface::Carcass(4),
        ),
    ]
}

/// Equal bands from the bottom of the front to the top.
/// `count` bands are separated by `gap`, and the first and last band sit
/// `gap` in from the outer edges as well.
pub fn equal_spans(length: f32, count: u8, gap: f32) -> Vec<(f32, f32)> {
    let count = count.max(1) as f32;
    let usable = (length - gap * (count + 1.0)).max(count * 0.001);
    let band = usable / count;
    (0..count as usize)
        .map(|index| {
            let i = index as f32;
            let start = gap + i * (band + gap);
            (start + band * 0.5, band)
        })
        .collect()
}

fn drawer_fronts(size: Vec3, count: u8) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let z = d * 0.5 + 0.004;
    let spans = equal_spans(h, count, GAP_METERS);
    let mut parts = Vec::with_capacity(spans.len() * 2);
    for (index, (center_y, band_h)) in spans.into_iter().enumerate() {
        parts.push(Solid::axis(
            Vec3::new((w - GAP_METERS * 2.0).max(0.02), band_h, PANEL_METERS),
            Vec3::new(0.0, center_y, z),
            Surface::Front(index as u8),
        ));
        parts.push(handle(Vec3::new(0.0, center_y, z + PANEL_METERS * 0.5), w));
    }
    parts
}

fn hinged_fronts(size: Vec3, doors: u8, style: DoorStyle) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let z = d * 0.5 + 0.004;
    let doors = doors.clamp(1, 2);
    let spans = equal_spans(w, doors, GAP_METERS);
    let front_h = (h - GAP_METERS * 2.0).max(0.02);
    let center_y = h * 0.5;
    let mut parts = Vec::new();
    for (index, (center_x, band_w)) in spans.into_iter().enumerate() {
        // `equal_spans` measures from the left edge. Shift onto the centered origin.
        let x = center_x - w * 0.5;
        parts.extend(door_leaf(
            band_w,
            front_h,
            Vec3::new(x, center_y, z),
            Quat::IDENTITY,
            style,
            index as u8,
        ));
        // A single door gets the pull on the right. A pair gets pulls toward the middle.
        let pull_x = if doors == 1 {
            x + band_w * 0.5 - 0.04
        } else if index == 0 {
            x + band_w * 0.5 - 0.04
        } else {
            x - band_w * 0.5 + 0.04
        };
        parts.push(handle(
            Vec3::new(pull_x, center_y, z + PANEL_METERS * 0.5),
            0.12,
        ));
    }
    parts
}

/// The lift-up door is rotated open around its top edge. Negative rotation
/// around X swings the bottom of the door out toward +Z (into the room)
/// and upward, which is the motion of a top-hinged door.
fn lift_front(size: Vec3, style: DoorStyle) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let front_h = (h - GAP_METERS * 2.0).max(0.02);
    let front_w = (w - GAP_METERS * 2.0).max(0.02);
    let hinge = Vec3::new(0.0, h - GAP_METERS, d * 0.5 + 0.004);
    let rotation = Quat::from_rotation_x(-0.65);
    let door_center = hinge + rotation * Vec3::new(0.0, -front_h * 0.5, 0.0);
    let handle_point =
        hinge + rotation * Vec3::new(0.0, -front_h + 0.04, PANEL_METERS * 0.5 + 0.004);
    let mut parts = door_leaf(front_w, front_h, door_center, rotation, style, 0);
    parts.push(Solid {
        size: Vec3::new(0.12, 0.012, 0.012),
        translation: handle_point,
        rotation,
        role: Surface::Handle,
    });
    parts
}

fn sliding_fronts(size: Vec3, style: DoorStyle) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let front_h = (h - GAP_METERS * 2.0).max(0.02);
    // Each leaf is a little wider than half so they overlap, and one sits
    // a few millimeters in front of the other so the overlap is readable.
    let leaf_w = w * 0.55;
    let center_y = h * 0.5;
    let back_z = d * 0.5 + 0.002;
    let front_z = back_z + 0.012;
    let mut parts = door_leaf(
        leaf_w,
        front_h,
        Vec3::new(-w * 0.22, center_y, back_z),
        Quat::IDENTITY,
        style,
        0,
    );
    parts.push(handle(
        Vec3::new(-0.04, center_y, back_z + PANEL_METERS),
        0.02,
    ));
    parts.extend(door_leaf(
        leaf_w,
        front_h,
        Vec3::new(w * 0.22, center_y, front_z),
        Quat::IDENTITY,
        style,
        1,
    ));
    parts.push(handle(
        Vec3::new(0.04, center_y, front_z + PANEL_METERS),
        0.02,
    ));
    parts
}

/// A solid slab, or a wood frame with a glass pane in the middle.
/// Local X is the leaf width, local Y the height, and local +Z the outer face.
fn door_leaf(
    width: f32,
    height: f32,
    center: Vec3,
    rotation: Quat,
    style: DoorStyle,
    index: u8,
) -> Vec<Solid> {
    let thickness = PANEL_METERS;
    if style != DoorStyle::Glass {
        return vec![Solid {
            size: Vec3::new(width, height, thickness).max(Vec3::splat(0.001)),
            translation: center,
            rotation,
            role: Surface::Front(index),
        }];
    }
    let rail = 0.042_f32.min(width * 0.18).min(height * 0.18).max(0.008);
    let frame = |sx: f32, sy: f32, ox: f32, oy: f32| Solid {
        size: Vec3::new(sx, sy, thickness).max(Vec3::splat(0.001)),
        translation: center + rotation * Vec3::new(ox, oy, 0.0),
        rotation,
        role: Surface::Front(index),
    };
    let inner_w = (width - rail * 2.0).max(0.01);
    let inner_h = (height - rail * 2.0).max(0.01);
    vec![
        frame(rail, height, -width * 0.5 + rail * 0.5, 0.0),
        frame(rail, height, width * 0.5 - rail * 0.5, 0.0),
        frame(inner_w, rail, 0.0, height * 0.5 - rail * 0.5),
        frame(inner_w, rail, 0.0, -height * 0.5 + rail * 0.5),
        Solid {
            size: Vec3::new(inner_w, inner_h, 0.004),
            translation: center + rotation * Vec3::new(0.0, 0.0, 0.002),
            rotation,
            role: Surface::Glass,
        },
    ]
}

/// Depth of each straight run on a corner cabinet. The corner bay is this
/// deep on both walls, so the diagonal is longer than either margin.
fn corner_arm(size: Vec3) -> f32 {
    0.35_f32.min(size.x * 0.62).min(size.z * 0.62)
}

fn corner_cabinet(size: Vec3, style: CornerStyle, doors: DoorStyle) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let t = PANEL_METERS.min(w * 0.12).min(h * 0.12).min(d * 0.12);
    let arm = corner_arm(size).max(t * 4.0);
    let extra_w = (w - arm).max(0.04);
    let extra_d = (d - arm).max(0.04);
    let back_z = -d * 0.5 + arm * 0.5;
    let left_x = -w * 0.5 + arm * 0.5;
    let return_z = -d * 0.5 + arm + extra_d * 0.5;
    let inner_h = (h - 2.0 * t).max(0.02);
    let mut parts = vec![
        Solid::axis(
            Vec3::new(w, t, arm),
            Vec3::new(0.0, t * 0.5, back_z),
            Surface::Carcass(2),
        ),
        Solid::axis(
            Vec3::new(arm, t, extra_d),
            Vec3::new(left_x, t * 0.5, return_z),
            Surface::Carcass(2),
        ),
        Solid::axis(
            Vec3::new(w, t, arm),
            Vec3::new(0.0, h - t * 0.5, back_z),
            Surface::Carcass(3),
        ),
        Solid::axis(
            Vec3::new(arm, t, extra_d),
            Vec3::new(left_x, h - t * 0.5, return_z),
            Surface::Carcass(3),
        ),
        Solid::axis(
            Vec3::new((w - t).max(0.02), inner_h, t),
            Vec3::new(t * 0.5, h * 0.5, -d * 0.5 + t * 0.5),
            Surface::Carcass(4),
        ),
        Solid::axis(
            Vec3::new(t, inner_h, (d - t).max(0.02)),
            Vec3::new(-w * 0.5 + t * 0.5, h * 0.5, t * 0.5),
            Surface::Carcass(0),
        ),
        Solid::axis(
            Vec3::new(t, inner_h, (arm - t).max(0.02)),
            Vec3::new(w * 0.5 - t * 0.5, h * 0.5, -d * 0.5 + (arm + t) * 0.5),
            Surface::Carcass(1),
        ),
        Solid::axis(
            Vec3::new((arm - t).max(0.02), inner_h, t),
            Vec3::new(-w * 0.5 + (arm + t) * 0.5, h * 0.5, d * 0.5 - t * 0.5),
            Surface::Carcass(1),
        ),
    ];

    let door_h = (h - GAP_METERS * 2.0).max(0.04);
    match style {
        CornerStyle::ShelfDoors => {
            for index in 1..=2 {
                let y = t + inner_h * index as f32 / 3.0;
                parts.push(Solid::axis(
                    Vec3::new((arm - 2.0 * t).max(0.02), t, (arm - 2.0 * t).max(0.02)),
                    Vec3::new(left_x, y, back_z),
                    Surface::Carcass(2),
                ));
            }
            let door_w = (extra_w - GAP_METERS * 2.0).max(0.04);
            let door_x = -w * 0.5 + arm + extra_w * 0.5;
            let door_z = -d * 0.5 + arm + 0.008;
            parts.extend(door_leaf(
                door_w,
                door_h,
                Vec3::new(door_x, h * 0.5, door_z),
                Quat::IDENTITY,
                doors,
                0,
            ));
            parts.push(handle(
                Vec3::new(door_x - door_w * 0.35, h * 0.5, door_z + PANEL_METERS),
                0.08,
            ));

            let door_d = (extra_d - GAP_METERS * 2.0).max(0.04);
            let side_x = -w * 0.5 + arm + 0.008;
            let side_z = -d * 0.5 + arm + extra_d * 0.5;
            // Face the left-wall run toward the room (+X).
            let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
            parts.extend(door_leaf(
                door_d,
                door_h,
                Vec3::new(side_x, h * 0.5, side_z),
                rotation,
                doors,
                1,
            ));
            parts.push(Solid::axis(
                Vec3::new(0.012, 0.012, 0.08),
                Vec3::new(side_x + PANEL_METERS, h * 0.5, side_z - door_d * 0.35),
                Surface::Handle,
            ));
        }
        CornerStyle::DoubleDoor => {
            let ax = w * 0.5;
            let az = -d * 0.5 + arm;
            let bx = -w * 0.5 + arm;
            let bz = d * 0.5;
            let dx = bx - ax;
            let dz = bz - az;
            let length = (dx * dx + dz * dz).sqrt().max(0.05);
            let along_x = dx / length;
            let along_z = dz / length;
            // Points out of the cabinet, into the room.
            let nx = along_z;
            let nz = -along_x;
            let leaf_w = (length * 0.5 - GAP_METERS).max(0.04);
            let shift = leaf_w * 0.5 + GAP_METERS * 0.5;
            let rotation = Quat::from_rotation_y((-along_z).atan2(along_x));
            let mid_x = (ax + bx) * 0.5 + nx * 0.012;
            let mid_z = (az + bz) * 0.5 + nz * 0.012;
            for (index, sign) in [(0u8, -1.0_f32), (1, 1.0)] {
                let center = Vec3::new(
                    mid_x + along_x * shift * sign,
                    h * 0.5,
                    mid_z + along_z * shift * sign,
                );
                parts.extend(door_leaf(leaf_w, door_h, center, rotation, doors, index));
                let handle_along = -sign * (leaf_w * 0.5 - 0.05);
                parts.push(Solid::axis(
                    Vec3::new(0.016, 0.09, 0.016),
                    center
                        + Vec3::new(along_x, 0.0, along_z) * handle_along
                        + Vec3::new(nx, 0.0, nz) * 0.02,
                    Surface::Handle,
                ));
            }
        }
    }
    parts
}

fn handle(at: Vec3, width: f32) -> Solid {
    let width = width.clamp(0.04, 0.18);
    Solid::axis(
        Vec3::new(width, 0.012, 0.012),
        at + Vec3::new(0.0, 0.0, 0.008),
        Surface::Handle,
    )
}

fn table(size: Vec3) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let top_t = 0.03_f32.min(h * 0.4);
    let leg = 0.05_f32.min(w * 0.15).min(d * 0.15);
    let inset = (leg * 0.5 + 0.03).min(w * 0.3).min(d * 0.3);
    let leg_h = (h - top_t).max(0.05);
    let mut parts = vec![Solid::axis(
        Vec3::new(w, top_t, d),
        Vec3::new(0.0, h - top_t * 0.5, 0.0),
        Surface::Top,
    )];
    for x_sign in [-1.0, 1.0] {
        for z_sign in [-1.0, 1.0] {
            parts.push(Solid::axis(
                Vec3::new(leg, leg_h, leg),
                Vec3::new(
                    x_sign * (w * 0.5 - inset),
                    leg_h * 0.5,
                    z_sign * (d * 0.5 - inset),
                ),
                Surface::Carcass(0),
            ));
        }
    }
    parts
}

/// Recognizable but deliberately lightweight appliance models. They are made
/// from boxes so projects stay self-contained and do not depend on asset files.
fn appliance(size: Vec3, kind: Kind) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let body = Solid::axis(size, Vec3::new(0.0, h * 0.5, 0.0), Surface::Appliance(0));
    let front_z = d * 0.5 + 0.006;
    match kind {
        Kind::Refrigerator => vec![
            body,
            Solid::axis(
                Vec3::new((w - 0.02).max(0.02), (h * 0.58 - 0.01).max(0.02), 0.012),
                Vec3::new(0.0, h * 0.71, front_z),
                Surface::Appliance(1),
            ),
            Solid::axis(
                Vec3::new((w - 0.02).max(0.02), (h * 0.42 - 0.01).max(0.02), 0.012),
                Vec3::new(0.0, h * 0.21, front_z),
                Surface::Appliance(1),
            ),
            appliance_handle(Vec3::new(w * 0.36, h * 0.7, front_z + 0.015), h * 0.34),
        ],
        Kind::Dishwasher => vec![
            body,
            Solid::axis(
                Vec3::new((w - 0.02).max(0.02), (h - 0.04).max(0.02), 0.014),
                Vec3::new(0.0, h * 0.5, front_z),
                Surface::Appliance(1),
            ),
            appliance_handle(Vec3::new(0.0, h * 0.86, front_z + 0.015), w * 0.65),
        ],
        Kind::ElectricOven => vec![
            body,
            Solid::axis(
                Vec3::new(w * 0.78, h * 0.56, 0.016),
                Vec3::new(0.0, h * 0.42, front_z),
                Surface::Appliance(2),
            ),
            Solid::axis(
                Vec3::new(w * 0.86, h * 0.17, 0.016),
                Vec3::new(0.0, h * 0.86, front_z),
                Surface::Appliance(1),
            ),
            appliance_handle(Vec3::new(0.0, h * 0.7, front_z + 0.02), w * 0.64),
        ],
        Kind::CoffeeMachine => vec![
            body,
            Solid::axis(
                Vec3::new(w * 0.62, h * 0.48, 0.02),
                Vec3::new(0.0, h * 0.57, front_z),
                Surface::Appliance(2),
            ),
            Solid::axis(
                Vec3::new(w * 0.7, 0.025, d * 0.42),
                Vec3::new(0.0, h * 0.07, d * 0.25),
                Surface::Appliance(1),
            ),
            // Twin coffee spouts make this silhouette unlike a microwave.
            Solid::axis(
                Vec3::new(0.025, h * 0.18, 0.025),
                Vec3::new(-w * 0.1, h * 0.34, front_z + 0.025),
                Surface::Handle,
            ),
            Solid::axis(
                Vec3::new(0.025, h * 0.18, 0.025),
                Vec3::new(w * 0.1, h * 0.34, front_z + 0.025),
                Surface::Handle,
            ),
            Solid::axis(
                Vec3::new(w * 0.48, h * 0.12, d * 0.5),
                Vec3::new(0.0, h * 0.94, -d * 0.08),
                Surface::Appliance(1),
            ),
        ],
        Kind::Microwave => vec![
            body,
            Solid::axis(
                Vec3::new(w * 0.7, h * 0.7, 0.016),
                Vec3::new(-w * 0.09, h * 0.5, front_z),
                Surface::Appliance(2),
            ),
            Solid::axis(
                Vec3::new(w * 0.12, h * 0.68, 0.016),
                Vec3::new(w * 0.39, h * 0.5, front_z),
                Surface::Appliance(1),
            ),
            appliance_handle(Vec3::new(w * 0.27, h * 0.5, front_z + 0.02), h * 0.48),
        ],
        Kind::SquareSink => rectangular_sink(size, 1),
        Kind::RoundSink => round_sink(size),
        Kind::DoubleSink => rectangular_sink(size, 2),
        Kind::SinkFaucet => {
            let tube = w.min(d).min(0.045).max(0.018);
            vec![
                Solid::axis(
                    Vec3::new(tube * 2.2, 0.025, tube * 2.2),
                    Vec3::new(0.0, 0.0125, 0.0),
                    Surface::ApplianceRound(1),
                ),
                Solid::axis(
                    Vec3::new(tube, h * 0.82, tube),
                    Vec3::new(0.0, h * 0.41, 0.0),
                    Surface::ApplianceRound(1),
                ),
                Solid::axis(
                    Vec3::new(tube, tube, d * 0.72),
                    Vec3::new(0.0, h * 0.82, d * 0.28),
                    Surface::ApplianceRound(1),
                ),
                Solid::axis(
                    Vec3::new(tube, h * 0.18, tube),
                    Vec3::new(0.0, h * 0.75, d * 0.64),
                    Surface::ApplianceRound(1),
                ),
            ]
        }
        Kind::CentralHeatingBoiler => vec![
            body,
            Solid::axis(
                Vec3::new(w * 0.42, h * 0.13, 0.018),
                Vec3::new(0.0, h * 0.28, front_z),
                Surface::Appliance(2),
            ),
            Solid::axis(
                Vec3::new(w * 0.74, 0.025, 0.025),
                Vec3::new(0.0, h * 0.1, front_z + 0.015),
                Surface::Handle,
            ),
            // Visible service pipes under the boiler.
            Solid::axis(
                Vec3::new(0.025, h * 0.2, 0.025),
                Vec3::new(-w * 0.22, -h * 0.1, 0.0),
                Surface::ApplianceRound(1),
            ),
            Solid::axis(
                Vec3::new(0.025, h * 0.2, 0.025),
                Vec3::new(w * 0.22, -h * 0.1, 0.0),
                Surface::ApplianceRound(1),
            ),
        ],
        Kind::AirConditioner => {
            let mut parts = vec![
                body,
                Solid::axis(
                    Vec3::new(w * 0.88, h * 0.18, 0.018),
                    Vec3::new(0.0, h * 0.18, front_z),
                    Surface::Appliance(2),
                ),
            ];
            for index in 0..5 {
                let x = -w * 0.32 + index as f32 * w * 0.16;
                parts.push(Solid::axis(
                    Vec3::new(0.012, h * 0.13, 0.022),
                    Vec3::new(x, h * 0.18, front_z + 0.014),
                    Surface::Appliance(1),
                ));
            }
            parts
        }
        _ => vec![body],
    }
}

/// A drop-in wash basin. The top is an open rim; the bowl hangs below it.
fn rectangular_sink(size: Vec3, bowls: u8) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let bowls = bowls.max(1);
    let lip = 0.028_f32.min(w * 0.08).min(d * 0.08);
    let wall = 0.012_f32.min(lip * 0.6).max(0.008);
    let floor_t = 0.01_f32.min(h * 0.12);
    let rim_h = 0.016_f32.min(h * 0.18);
    let metal = Surface::Appliance(1);
    let basin = Surface::Appliance(2);
    let mut parts = Vec::new();

    parts.push(Solid::axis(
        Vec3::new(w, floor_t, d),
        Vec3::new(0.0, floor_t * 0.5, 0.0),
        metal,
    ));

    let side_h = (h - rim_h - floor_t).max(0.04);
    let side_y = floor_t + side_h * 0.5;
    parts.push(Solid::axis(
        Vec3::new(wall, side_h, d),
        Vec3::new(-w * 0.5 + wall * 0.5, side_y, 0.0),
        metal,
    ));
    parts.push(Solid::axis(
        Vec3::new(wall, side_h, d),
        Vec3::new(w * 0.5 - wall * 0.5, side_y, 0.0),
        metal,
    ));
    let span_w = (w - wall * 2.0).max(0.02);
    parts.push(Solid::axis(
        Vec3::new(span_w, side_h, wall),
        Vec3::new(0.0, side_y, d * 0.5 - wall * 0.5),
        metal,
    ));
    parts.push(Solid::axis(
        Vec3::new(span_w, side_h, wall),
        Vec3::new(0.0, side_y, -d * 0.5 + wall * 0.5),
        metal,
    ));

    let open_w = (w - lip * 2.0).max(0.05);
    let open_d = (d - lip * 2.0 - wall * 2.0).max(0.04);
    let bowl_w = ((open_w - wall * (bowls - 1) as f32) / bowls as f32).max(0.04);
    if bowls > 1 {
        for index in 1..bowls {
            let x = -open_w * 0.5 + index as f32 * bowl_w + (index as f32 - 0.5) * wall;
            parts.push(Solid::axis(
                Vec3::new(wall, side_h, open_d),
                Vec3::new(x, side_y, 0.0),
                metal,
            ));
        }
    }

    let rim_y = h - rim_h * 0.5;
    parts.push(Solid::axis(
        Vec3::new(w, rim_h, lip),
        Vec3::new(0.0, rim_y, d * 0.5 - lip * 0.5),
        metal,
    ));
    parts.push(Solid::axis(
        Vec3::new(w, rim_h, lip),
        Vec3::new(0.0, rim_y, -d * 0.5 + lip * 0.5),
        metal,
    ));
    let side_lip_d = (d - lip * 2.0).max(0.02);
    parts.push(Solid::axis(
        Vec3::new(lip, rim_h, side_lip_d),
        Vec3::new(-w * 0.5 + lip * 0.5, rim_y, 0.0),
        metal,
    ));
    parts.push(Solid::axis(
        Vec3::new(lip, rim_h, side_lip_d),
        Vec3::new(w * 0.5 - lip * 0.5, rim_y, 0.0),
        metal,
    ));

    for index in 0..bowls {
        let x = -open_w * 0.5 + bowl_w * 0.5 + index as f32 * (bowl_w + wall);
        parts.push(Solid::axis(
            Vec3::new(
                (bowl_w - 0.012).max(0.03),
                0.004,
                (open_d - 0.012).max(0.03),
            ),
            Vec3::new(x, floor_t + 0.004, 0.0),
            basin,
        ));
        parts.push(Solid::axis(
            Vec3::new(0.038, 0.008, 0.038),
            Vec3::new(x, floor_t + 0.01, open_d * 0.08),
            Surface::ApplianceRound(2),
        ));
    }
    parts
}

/// A round wash basin built as an open ring of wall panels around a floor.
fn round_sink(size: Vec3) -> Vec<Solid> {
    let (_w, h, _d) = (size.x, size.y, size.z);
    let radius = size.x.min(size.z) * 0.48;
    let wall = 0.014_f32.min(radius * 0.12);
    let floor_t = 0.01_f32.min(h * 0.12);
    let rim_h = 0.016_f32.min(h * 0.18);
    let segments = 12u8;
    let metal = Surface::Appliance(1);
    let mut parts = vec![Solid::axis(
        Vec3::new(radius * 2.0, floor_t, radius * 2.0),
        Vec3::new(0.0, floor_t * 0.5, 0.0),
        Surface::ApplianceRound(1),
    )];

    let side_h = (h - rim_h - floor_t).max(0.04);
    let side_y = floor_t + side_h * 0.5;
    let step = std::f32::consts::TAU / segments as f32;
    let chord = 2.0 * radius * (step * 0.5).sin() * 1.04;
    for index in 0..segments {
        let angle = index as f32 * step;
        let mid = radius - wall * 0.5;
        parts.push(Solid {
            size: Vec3::new(chord, side_h, wall).max(Vec3::splat(0.001)),
            translation: Vec3::new(angle.cos() * mid, side_y, angle.sin() * mid),
            rotation: Quat::from_rotation_y(-angle),
            role: metal,
        });
        let rim_mid = radius - wall * 0.35;
        parts.push(Solid {
            size: Vec3::new(chord, rim_h, wall * 1.7).max(Vec3::splat(0.001)),
            translation: Vec3::new(
                angle.cos() * rim_mid,
                h - rim_h * 0.5,
                angle.sin() * rim_mid,
            ),
            rotation: Quat::from_rotation_y(-angle),
            role: metal,
        });
    }

    let inner = (radius - wall * 2.2).max(0.04);
    parts.push(Solid::axis(
        Vec3::new(inner * 2.0, 0.004, inner * 2.0),
        Vec3::new(0.0, floor_t + 0.004, 0.0),
        Surface::ApplianceRound(2),
    ));
    parts.push(Solid::axis(
        Vec3::new(0.04, 0.008, 0.04),
        Vec3::new(0.0, floor_t + 0.01, 0.0),
        Surface::ApplianceRound(2),
    ));
    parts
}

fn appliance_handle(at: Vec3, length: f32) -> Solid {
    Solid::axis(
        Vec3::new(length.max(0.04), 0.018, 0.018),
        at,
        Surface::Handle,
    )
}

/// Local-space box used both for clicking and for the selection outline.
/// The origin is the base center. Fronts stick out a little past `depth`.
pub fn pick_bounds(furniture: &Furniture) -> (Vec3, Vec3) {
    let size = furniture.size_meters();
    let leg_height = furniture.leg_height_meters();
    let extra_z = if matches!(furniture.kind, Kind::LiftDoor) {
        0.25
    } else {
        0.04
    };
    let extra_y = if matches!(furniture.kind, Kind::LiftDoor) {
        0.12
    } else {
        0.0
    };
    // A 20 mm pipe is hard to grab. The outline and the click target grow a little.
    let grab = if furniture.kind.is_pipe() { 0.03 } else { 0.0 };
    (
        Vec3::new(-size.x * 0.5 - grab, 0.0, -size.z * 0.5 - grab),
        Vec3::new(
            size.x * 0.5 + grab,
            size.y + leg_height + extra_y + grab,
            size.z * 0.5 + extra_z + grab,
        ),
    )
}

/// Facing stored as 0, 1, 2, or 3 quarter-turns around Y.
/// 0 means the front points along +Z, into the room.
pub fn quarter_turns(rotation: Quat) -> u8 {
    let (yaw, _, _) = rotation.to_euler(EulerRot::YXZ);
    let turns = (yaw / std::f32::consts::FRAC_PI_2).round() as i32;
    turns.rem_euclid(4) as u8
}

pub fn rotation_from_turns(turns: u8) -> Quat {
    Quat::from_rotation_y(turns as f32 * std::f32::consts::FRAC_PI_2)
}

pub fn place(x_mm: f32, y_mm: f32, z_mm: f32, turns: u8) -> Transform {
    Transform {
        translation: Vec3::new(mm_to_m(x_mm), mm_to_m(y_mm), mm_to_m(z_mm)),
        rotation: rotation_from_turns(turns),
        scale: Vec3::ONE,
    }
}

/// A small kitchen that shows every type once. It is only a starting point.
pub fn example_layout() -> Vec<(Furniture, Transform)> {
    fn piece(
        kind: Kind,
        width: f32,
        height: f32,
        depth: f32,
        x: f32,
        y: f32,
        z: f32,
    ) -> (Furniture, Transform) {
        (
            Furniture::new(kind, width, height, depth),
            place(x, y, z, 0),
        )
    }

    vec![
        piece(
            Kind::Drawers { count: 3 },
            600.0,
            720.0,
            560.0,
            350.0,
            0.0,
            280.0,
        ),
        piece(
            Kind::HingedDoors { doors: 2 },
            800.0,
            720.0,
            560.0,
            1050.0,
            0.0,
            280.0,
        ),
        piece(
            Kind::Drawers { count: 4 },
            500.0,
            720.0,
            560.0,
            1700.0,
            0.0,
            280.0,
        ),
        piece(
            Kind::OpenShelves { shelves: 3 },
            600.0,
            720.0,
            560.0,
            2250.0,
            0.0,
            280.0,
        ),
        piece(Kind::LiftDoor, 800.0, 420.0, 340.0, 1050.0, 1480.0, 170.0),
        piece(Kind::SlidingDoors, 900.0, 2100.0, 580.0, 3200.0, 0.0, 290.0),
        piece(Kind::Table, 1500.0, 750.0, 850.0, 1900.0, 0.0, 1700.0),
    ]
}

/// Ray against an axis-aligned box. `direction` must be normalized.
/// Returns the distance along the ray, or `None` when the ray misses.
pub fn ray_aabb(origin: Vec3, direction: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    let mut near = 0.0_f32;
    let mut far = f32::MAX;
    for axis in 0..3 {
        let origin_axis = origin[axis];
        let direction_axis = direction[axis];
        if direction_axis.abs() < 1.0e-8 {
            if origin_axis < min[axis] || origin_axis > max[axis] {
                return None;
            }
            continue;
        }
        let mut t1 = (min[axis] - origin_axis) / direction_axis;
        let mut t2 = (max[axis] - origin_axis) / direction_axis;
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
        }
        near = near.max(t1);
        far = far.min(t2);
        if near > far {
            return None;
        }
    }
    if far < 0.0 {
        None
    } else {
        Some(near.max(0.0))
    }
}

/// Where a ray crosses a horizontal plane. Used to slide a piece across the floor.
pub fn ray_horizontal_plane(origin: Vec3, direction: Vec3, plane_y: f32) -> Option<Vec3> {
    if direction.y.abs() < 1.0e-6 {
        return None;
    }
    let distance = (plane_y - origin.y) / direction.y;
    if distance < 0.0 {
        None
    } else {
        Some(origin + direction * distance)
    }
}

// --- project files -----------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectFile {
    pub version: u32,
    pub snap_enabled: bool,
    pub snap_mm: f32,
    pub room: RoomFile,
    pub pieces: Vec<PieceFile>,
    /// The cut planner. Missing in older files, which then start from a
    /// standard 2440 × 1220 mm sheet.
    #[serde(default)]
    pub cut_list: CutListFile,
}

/// Saved sheet and parts. The planner that edits this lives in `cutting`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CutListFile {
    pub board_width_mm: f32,
    pub board_height_mm: f32,
    pub thickness_mm: f32,
    pub kerf_mm: f32,
    pub trim_mm: f32,
    pub allow_rotate: bool,
    pub draft_role: String,
    pub draft_name: String,
    pub draft_width_mm: f32,
    pub draft_height_mm: f32,
    pub draft_quantity: u32,
    pub parts: Vec<CutPartFile>,
}

impl Default for CutListFile {
    fn default() -> Self {
        Self {
            board_width_mm: 2440.0,
            board_height_mm: 1220.0,
            thickness_mm: 18.0,
            kerf_mm: 3.0,
            trim_mm: 0.0,
            allow_rotate: true,
            draft_role: "Door".into(),
            draft_name: String::new(),
            draft_width_mm: 600.0,
            draft_height_mm: 720.0,
            draft_quantity: 1,
            parts: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CutPartFile {
    pub name: String,
    pub role: String,
    pub width_mm: f32,
    pub height_mm: f32,
    pub quantity: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoomFile {
    pub width_mm: f32,
    pub depth_mm: f32,
    pub wall_height_mm: f32,
    pub show_walls: bool,
    /// Missing on older files. Those keep three walls and a hidden front.
    #[serde(default)]
    pub walls: Option<WallVisibility>,
    #[serde(default)]
    pub edit_enabled: bool,
    #[serde(default)]
    pub openings: Vec<Opening>,
}

impl RoomFile {
    /// Older files have no per-wall flags. Those keep the back, left, and
    /// right walls and leave the front hidden.
    pub fn resolved_walls(&self) -> WallVisibility {
        self.walls
            .unwrap_or_else(|| WallVisibility::legacy(self.show_walls))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PieceFile {
    pub kind: Kind,
    #[serde(default)]
    pub name: String,
    pub width_mm: f32,
    pub height_mm: f32,
    pub depth_mm: f32,
    pub x_mm: f32,
    pub y_mm: f32,
    pub z_mm: f32,
    /// 0, 1, 2, or 3. See [`quarter_turns`].
    pub turns: u8,
    #[serde(default)]
    pub placement: Placement,
    #[serde(default)]
    pub legs: LegStyle,
    #[serde(default)]
    pub doors: DoorStyle,
    #[serde(default)]
    pub custom: Option<CustomType>,
    #[serde(default)]
    pub finishes: PartFinishes,
}

pub enum ReadError {
    Missing,
    Invalid(String),
}

/// Folder this program was compiled from. Saves go here, not into whatever
/// directory the terminal happened to be in when the window opened.
pub fn project_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// A bare file name is saved in the project folder. A full path is used as given.
pub fn resolve_layout_path(entered: &str) -> PathBuf {
    let trimmed = entered.trim();
    let relative = if trimmed.is_empty() {
        PathBuf::from("layout.json")
    } else {
        PathBuf::from(trimmed)
    };
    if relative.is_absolute() {
        relative
    } else {
        project_dir().join(relative)
    }
}

pub fn read_project(path: &str) -> Result<ProjectFile, ReadError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(ReadError::Missing)
        }
        Err(error) => return Err(ReadError::Invalid(error.to_string())),
    };
    let file: ProjectFile =
        serde_json::from_str(&text).map_err(|error| ReadError::Invalid(error.to_string()))?;
    if !(1..=3).contains(&file.version) {
        return Err(ReadError::Invalid(format!(
            "layout file version is {}, this app reads versions 1, 2, and 3",
            file.version
        )));
    }
    Ok(file)
}

pub fn write_project(path: &Path, file: &ProjectFile) -> Result<(), String> {
    let text = serde_json::to_string_pretty(file).map_err(|error| error.to_string())?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("Could not create {}: {error}", parent.display()))?;
        }
    }
    std::fs::write(path, text)
        .map_err(|error| format!("Could not write {}: {error}", path.display()))
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

pub fn sanitize_openings(
    room_width_mm: f32,
    room_depth_mm: f32,
    wall_height_mm: f32,
    openings: &mut [Opening],
) {
    for opening in openings {
        let wall_length = match opening.wall {
            WallSide::Back | WallSide::Front => room_width_mm,
            WallSide::Left | WallSide::Right => room_depth_mm,
        };
        opening.width_mm = finite_or(opening.width_mm, 900.0).clamp(100.0, wall_length.max(100.0));
        opening.height_mm = finite_or(opening.height_mm, 1000.0).clamp(100.0, wall_height_mm);
        opening.offset_mm =
            finite_or(opening.offset_mm, 0.0).clamp(0.0, (wall_length - opening.width_mm).max(0.0));
        opening.sill_mm = if opening.kind == OpeningKind::Door {
            0.0
        } else {
            finite_or(opening.sill_mm, 900.0)
                .clamp(0.0, (wall_height_mm - opening.height_mm).max(0.0))
        };
    }
}

impl ProjectFile {
    pub fn sanitized(mut self) -> Self {
        self.room.width_mm = finite_or(self.room.width_mm, 4200.0).clamp(500.0, 30000.0);
        self.room.depth_mm = finite_or(self.room.depth_mm, 3000.0).clamp(500.0, 30000.0);
        self.room.wall_height_mm = finite_or(self.room.wall_height_mm, 2400.0).clamp(500.0, 6000.0);
        sanitize_openings(
            self.room.width_mm,
            self.room.depth_mm,
            self.room.wall_height_mm,
            &mut self.room.openings,
        );
        self.snap_mm = finite_or(self.snap_mm, 50.0).clamp(1.0, 500.0);
        for piece in &mut self.pieces {
            if self.version < 3 {
                piece.placement = if piece.kind == Kind::LiftDoor {
                    Placement::Wall
                } else {
                    Placement::Floor
                };
                piece.legs = if piece.placement == Placement::Floor && piece.kind.is_cabinet() {
                    LegStyle::Plastic100
                } else {
                    LegStyle::None
                };
            }
            piece.width_mm = finite_or(piece.width_mm, 600.0).clamp(50.0, 6000.0);
            piece.height_mm = finite_or(piece.height_mm, 720.0).clamp(50.0, 4000.0);
            piece.depth_mm = finite_or(piece.depth_mm, 560.0).clamp(50.0, 3000.0);
            piece.x_mm = finite_or(piece.x_mm, 0.0);
            piece.y_mm = finite_or(piece.y_mm, 0.0);
            piece.z_mm = finite_or(piece.z_mm, 0.0);
            piece.turns %= 4;
            piece.kind = match piece.kind {
                Kind::Drawers { count } => Kind::Drawers {
                    count: count.clamp(1, 10),
                },
                Kind::HingedDoors { doors } => Kind::HingedDoors {
                    doors: if doors <= 1 { 1 } else { 2 },
                },
                Kind::OpenShelves { shelves } => Kind::OpenShelves {
                    shelves: shelves.clamp(1, 10),
                },
                other => other,
            };
            if piece.name.trim().is_empty() {
                piece.name = piece.kind.label_for(piece.placement).to_owned();
            }
            let fronts = if piece.kind == Kind::Custom {
                let block = piece
                    .custom
                    .get_or_insert_with(|| CustomType::empty(piece.name.clone()));
                block.sanitize();
                piece.width_mm = piece.width_mm.max(1.0);
                piece.height_mm = piece.height_mm.max(1.0);
                piece.depth_mm = piece.depth_mm.max(1.0);
                block.door_count()
            } else {
                piece.custom = None;
                piece.kind.front_count()
            };
            piece.finishes.fronts.resize(fronts, Finish::default());
            piece.finishes.fronts.truncate(fronts);
        }
        let list = &mut self.cut_list;
        list.board_width_mm = finite_or(list.board_width_mm, 2440.0).clamp(100.0, 4000.0);
        list.board_height_mm = finite_or(list.board_height_mm, 1220.0).clamp(100.0, 4000.0);
        list.thickness_mm = finite_or(list.thickness_mm, 18.0).clamp(3.0, 60.0);
        list.kerf_mm = finite_or(list.kerf_mm, 3.0).clamp(0.0, 10.0);
        list.trim_mm = finite_or(list.trim_mm, 0.0).clamp(0.0, 50.0);
        list.draft_width_mm = finite_or(list.draft_width_mm, 600.0).clamp(10.0, 4000.0);
        list.draft_height_mm = finite_or(list.draft_height_mm, 720.0).clamp(10.0, 4000.0);
        list.draft_quantity = list.draft_quantity.clamp(1, 100);
        for part in &mut list.parts {
            part.width_mm = finite_or(part.width_mm, 100.0).clamp(1.0, 4000.0);
            part.height_mm = finite_or(part.height_mm, 100.0).clamp(1.0, 4000.0);
            part.quantity = part.quantity.clamp(1, 100);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point_in_solid(part: &Solid, point: Vec3) -> bool {
        let local = part.rotation.inverse() * (point - part.translation);
        local.x.abs() <= part.size.x * 0.5
            && local.y.abs() <= part.size.y * 0.5
            && local.z.abs() <= part.size.z * 0.5
    }

    #[test]
    fn drawers_split_the_front_into_equal_bands() {
        let furniture = Furniture::new(Kind::Drawers { count: 3 }, 600.0, 720.0, 560.0);
        let fronts: Vec<_> = solids(&furniture)
            .into_iter()
            .filter(|solid| matches!(solid.role, Surface::Front(_)))
            .collect();
        assert_eq!(fronts.len(), 3);
        let mut centers: Vec<_> = fronts.iter().map(|solid| solid.translation.y).collect();
        centers.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let first_gap = centers[1] - centers[0];
        let second_gap = centers[2] - centers[1];
        assert!((first_gap - second_gap).abs() < 1.0e-4);
        // The stack starts near the bottom and ends near the top, not bunched in the middle.
        let leg_height = furniture.leg_height_meters();
        assert!(centers[0] - leg_height < 200.0 / MM_PER_METER);
        assert!(centers[2] - leg_height > 500.0 / MM_PER_METER);
        let roles: Vec<_> = solids(&furniture)
            .into_iter()
            .filter_map(|solid| match solid.role {
                Surface::Front(index) => Some(index),
                _ => None,
            })
            .collect();
        assert_eq!(roles, vec![0, 1, 2]);
    }

    #[test]
    fn each_kind_has_the_expected_number_of_boxes() {
        let cases = [
            (Kind::Drawers { count: 3 }, 15),
            (Kind::HingedDoors { doors: 1 }, 11),
            (Kind::HingedDoors { doors: 2 }, 13),
            (Kind::LiftDoor, 7),
            (Kind::SlidingDoors, 13),
            (Kind::Table, 5),
            (Kind::Countertop, 1),
            (Kind::Tabletop, 1),
            (Kind::OpenShelves { shelves: 3 }, 12),
            (Kind::Refrigerator, 4),
            (Kind::Dishwasher, 3),
            (Kind::ElectricOven, 4),
            (Kind::CoffeeMachine, 6),
            (Kind::Microwave, 4),
            (Kind::SquareSink, 11),
            (Kind::RoundSink, 27),
            (Kind::DoubleSink, 14),
            (Kind::SinkFaucet, 4),
            (Kind::CentralHeatingBoiler, 5),
            (Kind::AirConditioner, 7),
        ];
        for (kind, expected) in cases {
            let furniture = Furniture::new(kind, 600.0, 720.0, 560.0);
            assert_eq!(solids(&furniture).len(), expected, "{kind:?}");
        }
    }

    #[test]
    fn kitchen_sinks_are_open_wash_basins() {
        for kind in [Kind::SquareSink, Kind::RoundSink, Kind::DoubleSink] {
            let furniture = Furniture::new(kind, 600.0, 180.0, 500.0);
            let parts = solids(&furniture);
            let top = parts
                .iter()
                .map(|part| part.translation.y + part.size.y * 0.5)
                .fold(0.0, f32::max);
            let sample = Vec3::new(0.12, top - 0.03, 0.0);
            assert!(
                parts.iter().all(|part| !point_in_solid(part, sample)),
                "{kind:?} covers the top of its basin"
            );
            assert!(
                parts.iter().any(|part| part.translation.y < 0.02),
                "{kind:?} has no basin floor"
            );
        }
    }

    #[test]
    fn ray_hits_a_box_in_front_and_misses_one_beside_it() {
        let min = Vec3::new(-0.3, 0.0, -0.3);
        let max = Vec3::new(0.3, 0.7, 0.3);
        let hit = ray_aabb(
            Vec3::new(0.0, 0.3, 2.0),
            Vec3::new(0.0, 0.0, -1.0),
            min,
            max,
        );
        assert!(hit.is_some());
        let miss = ray_aabb(
            Vec3::new(2.0, 0.3, 2.0),
            Vec3::new(0.0, 0.0, -1.0),
            min,
            max,
        );
        assert!(miss.is_none());
    }

    #[test]
    fn layout_file_is_in_the_project_folder() {
        let path = resolve_layout_path("layout.json");
        assert!(path.is_absolute());
        assert_eq!(path.parent(), Some(project_dir()));
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("layout.json")
        );
        let nested = resolve_layout_path("jobs/client.json");
        assert_eq!(
            nested.parent().and_then(|dir| dir.file_name()),
            Some(std::ffi::OsStr::new("jobs"))
        );
        assert!(nested.starts_with(project_dir()));
    }

    #[test]
    fn snap_and_quarter_turns_stay_on_the_grid() {
        assert!((snap_to_step(0.126, 0.05) - 0.15).abs() < 1.0e-5);
        assert!((snap_to_step(0.126, 0.0) - 0.126).abs() < 1.0e-5);
        let rotation = rotation_from_turns(3);
        assert_eq!(quarter_turns(rotation), 3);
        assert_eq!(quarter_turns(rotation_from_turns(4)), 0);
    }

    #[test]
    fn stage_one_json_loads_with_stage_two_defaults() {
        let json = r#"{
            "version": 1,
            "snap_enabled": true,
            "snap_mm": 50.0,
            "room": {
                "width_mm": 4200.0,
                "depth_mm": 3000.0,
                "wall_height_mm": 2400.0,
                "show_walls": true
            },
            "pieces": [{
                "kind": {"type": "drawers", "count": 3},
                "width_mm": 600.0,
                "height_mm": 720.0,
                "depth_mm": 560.0,
                "x_mm": 300.0,
                "y_mm": 0.0,
                "z_mm": 280.0,
                "turns": 0
            }]
        }"#;
        let file: ProjectFile = serde_json::from_str(json).unwrap();
        let file = file.sanitized();
        assert_eq!(file.version, 1);
        assert_eq!(file.pieces[0].name, "Drawer Base Cabinet");
        assert_eq!(file.pieces[0].finishes.fronts.len(), 3);
        assert_eq!(file.pieces[0].placement, Placement::Floor);
        assert_eq!(file.pieces[0].legs, LegStyle::Plastic100);
        assert_eq!(file.cut_list.board_width_mm, 2440.0);
        assert!(file.cut_list.parts.is_empty());
    }

    #[test]
    fn finish_and_selection_helpers_round_trip() {
        let mut finish = Finish::default();
        finish.preset = FinishPreset::Custom;
        finish.color = [12, 34, 56];
        let encoded = serde_json::to_string(&finish).unwrap();
        let decoded: Finish = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.rgb(), [12, 34, 56]);

        let mut world = World::new();
        let first = world.spawn_empty().id();
        let second = world.spawn_empty().id();
        let mut selection = Selection::default();
        selection.set_only(first);
        selection.toggle(second);
        assert!(selection.contains(first) && selection.contains(second));
        assert_eq!(selection.primary, Some(second));
        selection.toggle(second);
        assert_eq!(selection.primary, Some(first));
    }

    #[test]
    fn stage_four_presets_and_legacy_box_migrate() {
        let legacy: Kind = serde_json::from_str(r#"{"type":"box"}"#).unwrap();
        assert_eq!(legacy, Kind::OpenShelves { shelves: 3 });

        let (floor, floor_y) = Kind::Drawers { count: 3 }.preset(Placement::Floor);
        assert_eq!(floor_y, 0.0);
        assert_eq!(floor.legs, LegStyle::Plastic100);
        assert_eq!(floor.depth_mm, 560.0);

        let (wall, wall_y) = Kind::Drawers { count: 3 }.preset(Placement::Wall);
        assert_eq!(wall_y, 1400.0);
        assert_eq!(wall.legs, LegStyle::None);
        assert_eq!(wall.depth_mm, 350.0);
    }

    #[test]
    fn room_openings_are_clamped_to_their_wall() {
        let mut openings = vec![Opening {
            kind: OpeningKind::Window,
            wall: WallSide::Left,
            offset_mm: 9000.0,
            width_mm: 4000.0,
            height_mm: 2000.0,
            sill_mm: 1800.0,
        }];
        sanitize_openings(4200.0, 3000.0, 2400.0, &mut openings);
        assert_eq!(openings[0].width_mm, 3000.0);
        assert_eq!(openings[0].offset_mm, 0.0);
        assert_eq!(openings[0].sill_mm, 400.0);
    }

    #[test]
    fn pipes_are_round_pieces_with_one_or_two_members() {
        let vertical = Kind::Pipe {
            horizontal: false,
            count: 2,
            color: PipeColor::Gray,
        };
        let (furniture, y) = vertical.preset(Placement::Floor);
        assert_eq!(y, 0.0);
        assert_eq!(furniture.legs, LegStyle::None);
        let parts = solids(&furniture);
        assert_eq!(parts.len(), 2);
        assert!(parts.iter().all(|part| part.role == Surface::Pipe));
        assert!(parts.iter().all(|part| part.size.y > part.size.x));

        let horizontal = Kind::Pipe {
            horizontal: true,
            count: 1,
            color: PipeColor::Blue,
        };
        let (furniture, y) = horizontal.preset(Placement::Floor);
        assert_eq!(y, 500.0);
        let parts = solids(&furniture);
        assert_eq!(parts.len(), 1);
        assert!(parts[0].size.x > parts[0].size.y);
        assert!((parts[0].size.y - 0.02).abs() < 0.001);

        let saved: RoomFile = serde_json::from_str(
            r#"{
                "width_mm": 4200.0,
                "depth_mm": 3000.0,
                "wall_height_mm": 2400.0,
                "show_walls": true
            }"#,
        )
        .unwrap();
        let walls = saved.resolved_walls();
        assert!(walls.back && walls.left && walls.right);
        assert!(!walls.front);

        let minimal: Kind = serde_json::from_str(r#"{"type":"pipe"}"#).unwrap();
        assert_eq!(
            minimal,
            Kind::Pipe {
                horizontal: false,
                count: 1,
                color: PipeColor::Gray
            }
        );
    }

    #[test]
    fn corner_shelf_cabinet_has_two_doors_and_shelves() {
        let furniture = Furniture::new(
            Kind::Corner {
                style: CornerStyle::ShelfDoors,
            },
            600.0,
            720.0,
            600.0,
        );
        let parts = solids(&furniture);
        let fronts = parts
            .iter()
            .filter(|solid| matches!(solid.role, Surface::Front(_)))
            .count();
        assert_eq!(fronts, 2);
        let shelves = parts
            .iter()
            .filter(|solid| {
                solid.role == Surface::Carcass(2) && solid.size.x < 0.4 && solid.size.z < 0.4
            })
            .count();
        assert!(shelves >= 2, "{shelves}");
    }

    #[test]
    fn corner_double_door_spans_the_opening() {
        let furniture = Furniture::new(
            Kind::Corner {
                style: CornerStyle::DoubleDoor,
            },
            600.0,
            720.0,
            600.0,
        );
        let parts = solids(&furniture);
        let fronts: Vec<_> = parts
            .into_iter()
            .filter(|solid| matches!(solid.role, Surface::Front(_)))
            .collect();
        assert_eq!(fronts.len(), 2);
        let delta = fronts[1].translation - fronts[0].translation;
        assert!(delta.x.abs() > 0.05);
        assert!(delta.z.abs() > 0.05);
        assert_ne!(fronts[0].rotation, Quat::IDENTITY);
    }

    #[test]
    fn glass_door_adds_a_pane_inside_a_frame() {
        let mut furniture = Furniture::new(Kind::HingedDoors { doors: 1 }, 600.0, 720.0, 560.0);
        furniture.doors = DoorStyle::Glass;
        let parts = solids(&furniture);
        assert!(parts.iter().any(|solid| solid.role == Surface::Glass));
        assert!(
            parts
                .iter()
                .filter(|solid| matches!(solid.role, Surface::Front(_)))
                .count()
                >= 4
        );
    }

    #[test]
    fn older_pieces_load_with_solid_doors() {
        let piece: PieceFile = serde_json::from_str(
            r#"{
                "kind": {"type": "hinged_doors", "doors": 2},
                "width_mm": 600.0,
                "height_mm": 720.0,
                "depth_mm": 560.0,
                "x_mm": 0.0,
                "y_mm": 0.0,
                "z_mm": 0.0,
                "turns": 0
            }"#,
        )
        .unwrap();
        assert_eq!(piece.doors, DoorStyle::Solid);
    }

    #[test]
    fn custom_block_scales_from_the_designed_size() {
        let mut block = CustomType::empty("Wide");
        block.boards.push(CustomBoard {
            role: CustomRole::Wall,
            width_mm: 18.0,
            height_mm: 100.0,
            depth_mm: 40.0,
            x_mm: -50.0,
            y_mm: 50.0,
            z_mm: 0.0,
            yaw_deg: 0.0,
            pitch_deg: 0.0,
            roll_deg: 0.0,
            turns: 0,
        });
        block.boards.push(CustomBoard {
            role: CustomRole::Wall,
            width_mm: 18.0,
            height_mm: 100.0,
            depth_mm: 40.0,
            x_mm: 50.0,
            y_mm: 50.0,
            z_mm: 0.0,
            yaw_deg: 0.0,
            pitch_deg: 0.0,
            roll_deg: 0.0,
            turns: 0,
        });
        block.recompute_nominal();
        let designed = block.scaled_solids(block.width_mm, block.height_mm, block.depth_mm);
        let wider = block.scaled_solids(block.width_mm * 2.0, block.height_mm, block.depth_mm);
        let designed_gap = (designed[1].translation.x - designed[0].translation.x).abs();
        let wider_gap = (wider[1].translation.x - wider[0].translation.x).abs();
        assert!((wider_gap - designed_gap * 2.0).abs() < 0.001);
        assert!((wider[0].size.y - designed[0].size.y).abs() < 0.001);
        // The bottoms of both walls stay on the floor when only the width changes.
        let designed_bottom = designed[0].translation.y - designed[0].size.y * 0.5;
        let wider_bottom = wider[0].translation.y - wider[0].size.y * 0.5;
        assert!(designed_bottom.abs() < 0.001);
        assert!(wider_bottom.abs() < 0.001);
    }

    #[test]
    fn a_quarter_turn_swaps_a_boards_width_and_depth() {
        let mut block = CustomType::empty("Turned");
        block.boards.push(CustomBoard {
            role: CustomRole::Plank,
            width_mm: 200.0,
            height_mm: 18.0,
            depth_mm: 40.0,
            x_mm: 0.0,
            y_mm: 9.0,
            z_mm: 0.0,
            yaw_deg: 0.0,
            pitch_deg: 0.0,
            roll_deg: 0.0,
            turns: 1,
        });
        block.recompute_nominal();
        assert!((block.width_mm - 40.0).abs() < 0.1);
        assert!((block.depth_mm - 200.0).abs() < 0.1);
        block.sanitize();
        assert_eq!(block.boards[0].turns, 0);
        assert!((block.boards[0].yaw_deg - 90.0).abs() < 0.01);
        let solid = &block.scaled_solids(block.width_mm, block.height_mm, block.depth_mm)[0];
        assert_ne!(solid.rotation, Quat::IDENTITY);
    }

    #[test]
    fn a_pitched_board_stands_up_like_a_roof() {
        let board = CustomBoard {
            role: CustomRole::Plank,
            width_mm: 80.0,
            height_mm: 18.0,
            depth_mm: 200.0,
            x_mm: 0.0,
            y_mm: 0.0,
            z_mm: 0.0,
            yaw_deg: 0.0,
            pitch_deg: 90.0,
            roll_deg: 0.0,
            turns: 0,
        };
        let half = board.world_half_mm();
        assert!((half.x - 40.0).abs() < 0.2);
        assert!((half.y - 100.0).abs() < 0.2);
        assert!((half.z - 9.0).abs() < 0.2);
    }
}
