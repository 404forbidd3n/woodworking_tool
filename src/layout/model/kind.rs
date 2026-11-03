//! What a piece *is*: the [`Kind`] enum and the small option enums that hang
//! off it (corner style, door style, pipe color, floor/wall placement, legs).
//!
//! [`Kind::preset`] is where every piece gets its starting size when it is
//! added from the sidebar.

use serde::{Deserialize, Serialize};

use bevy::prelude::*;

use super::furniture::Furniture;

// ---------------------------------------------------------------------------
// The Kind enum and the style options it carries
// ---------------------------------------------------------------------------

/// Things that can be placed. Existing variant names are kept because they are
/// also the JSON tags used by Stage 1 project files.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Kind {
    /// One cabinet, split into `count` drawer fronts of equal height.
    Drawers {
        count: u8,
    },
    /// A base cabinet with a full-height pull-out "jolly" on the left and a
    /// stack of `drawers` equal drawers on the right. A vertical divider
    /// separates the two bays. `jolly_mm` is the jolly bay's outer width.
    JollyDrawers {
        #[serde(default = "default_jolly_drawers")]
        drawers: u8,
        #[serde(default = "default_jolly_mm")]
        jolly_mm: u16,
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
    /// A flat black glass induction plate with four cooking zones. Starts on
    /// top of the default 758 mm worktop.
    InductionHob,
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

/// Paint color of a utility pipe.
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

    /// The pipe's display color.
    pub fn rgb(self) -> [u8; 3] {
        match self {
            Self::Gray => [150, 154, 158],
            Self::Black => [32, 33, 35],
            Self::Blue => [46, 104, 178],
        }
    }
}

impl Kind {
    /// Display name of the floor version.
    pub fn label(self) -> &'static str {
        self.label_for(Placement::Floor)
    }

    /// Display name, which depends on whether the piece is on the floor or the
    /// wall ("Drawer Base Cabinet" vs "Wall Drawer Cabinet").
    pub fn label_for(self, placement: Placement) -> &'static str {
        match self {
            Kind::Drawers { .. } if placement == Placement::Wall => "Wall Drawer Cabinet",
            Kind::Drawers { .. } => "Drawer Base Cabinet",
            Kind::JollyDrawers { .. } => "Jolly + Drawer Base Cabinet",
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
            Kind::InductionHob => "Induction Hob",
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

    /// Smallest height the piece may be given, in mm. Slabs are measured by
    /// their thickness, so they go well below the 50 mm floor of a cabinet.
    pub fn min_height_mm(self) -> f32 {
        match self {
            Kind::Countertop | Kind::Tabletop | Kind::InductionHob => 4.0,
            Kind::Pipe { .. } => 8.0,
            _ => 50.0,
        }
    }

    /// How many doors or drawer fronts this kind has. Zero for open pieces.
    pub fn front_count(self) -> usize {
        match self {
            Kind::Drawers { count } => count.max(1) as usize,
            // Front 0 is the jolly; fronts 1..=drawers are the drawers.
            Kind::JollyDrawers { drawers, .. } => drawers.max(1) as usize + 1,
            Kind::HingedDoors { doors } => doors.clamp(1, 2) as usize,
            Kind::LiftDoor => 1,
            Kind::SlidingDoors => 2,
            Kind::Corner { .. } => 2,
            _ => 0,
        }
    }

    /// True for anything with a door or drawer front.
    pub fn has_doors(self) -> bool {
        self.front_count() > 0
    }

    /// Bought appliances. They are not part of the cut list.
    pub fn is_appliance(self) -> bool {
        matches!(
            self,
            Kind::Refrigerator
                | Kind::Dishwasher
                | Kind::ElectricOven
                | Kind::CoffeeMachine
                | Kind::Microwave
                | Kind::InductionHob
                | Kind::SquareSink
                | Kind::RoundSink
                | Kind::DoubleSink
                | Kind::SinkFaucet
                | Kind::CentralHeatingBoiler
                | Kind::AirConditioner
        )
    }

    /// Box furniture: gets legs, placement, and carcass colors.
    pub fn is_cabinet(self) -> bool {
        matches!(
            self,
            Kind::Drawers { .. }
                | Kind::JollyDrawers { .. }
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
            // A tall base: the jolly wants the extra height for bottles.
            Kind::JollyDrawers { .. } => (600.0, 780.0, 560.0, 0.0),
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
            // 6 mm of glass resting on the 758 mm top of the default worktop.
            Kind::InductionHob => (590.0, 6.0, 520.0, 758.0),
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

/// Serde default for `OpenShelves::shelves` in older files.
const fn default_shelf_count() -> u8 {
    3
}

/// Serde default for `JollyDrawers::drawers`.
const fn default_jolly_drawers() -> u8 {
    2
}

/// Serde default for `JollyDrawers::jolly_mm`.
const fn default_jolly_mm() -> u16 {
    JOLLY_DEFAULT_MM
}

/// Starting outer width of the jolly bay on a new jolly + drawer cabinet.
pub const JOLLY_DEFAULT_MM: u16 = 200;
/// Narrowest jolly bay, and the narrowest drawer bay left beside it.
pub const JOLLY_MIN_MM: f32 = 100.0;

/// The jolly bay's outer width in mm, measured from the cabinet's left face
/// to the middle of the divider. Kept inside the cabinet so both bays stay at
/// least [`JOLLY_MIN_MM`] wide, or half the cabinet each when it is too narrow.
pub fn jolly_width_mm(cabinet_width_mm: f32, jolly_mm: u16) -> f32 {
    let low = JOLLY_MIN_MM.min(cabinet_width_mm * 0.5);
    let high = (cabinet_width_mm - JOLLY_MIN_MM).max(low);
    (jolly_mm as f32).clamp(low, high)
}

/// Serde default for `Pipe::count` in older files.
const fn default_pipe_count() -> u8 {
    1
}

/// Default pipe thickness. Depth on a pipe piece is this diameter.
pub const PIPE_DIAMETER_MM: f32 = 20.0;
/// Extra width or height between the outer faces of a pair of pipes.
pub const PIPE_PAIR_GAP_MM: f32 = 80.0;

/// Whether a piece stands on the floor or hangs on the wall. Wall pieces
/// have no legs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    #[default]
    Floor,
    Wall,
}

/// Adjustable cabinet feet. The number is the height in millimeters.
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

    /// Leg height in millimeters. Zero for `None`.
    pub fn height_mm(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Plastic100 | Self::Metal100 | Self::Wood100 => 100.0,
            Self::Plastic150 | Self::Metal150 => 150.0,
        }
    }
}
