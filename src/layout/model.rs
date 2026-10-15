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
    /// An empty box used as a stand-in for a sink, appliance, or anything
    /// the other types do not cover.
    Box,
    Refrigerator,
    Dishwasher,
    ElectricOven,
    CoffeeMachine,
    Microwave,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Drawers { .. } => "Drawer Base Cabinet",
            Kind::HingedDoors { doors: 1 } => "Single-Door Base Cabinet",
            Kind::HingedDoors { .. } => "Double-Door Base Cabinet",
            Kind::LiftDoor => "Wall Lift-Up Cabinet",
            Kind::SlidingDoors => "Sliding-Door Cabinet",
            Kind::Table => "Work Table",
            Kind::Countertop => "Kitchen Worktop",
            Kind::Tabletop => "Tabletop",
            Kind::Box => "Custom Block",
            Kind::Refrigerator => "Refrigerator",
            Kind::Dishwasher => "Dishwasher",
            Kind::ElectricOven => "Electric Oven",
            Kind::CoffeeMachine => "Coffee Machine",
            Kind::Microwave => "Microwave",
        }
    }

    pub fn front_count(self) -> usize {
        match self {
            Kind::Drawers { count } => count.max(1) as usize,
            Kind::HingedDoors { doors } => doors.clamp(1, 2) as usize,
            Kind::LiftDoor => 1,
            Kind::SlidingDoors => 2,
            _ => 0,
        }
    }

    pub fn is_appliance(self) -> bool {
        matches!(
            self,
            Kind::Refrigerator
                | Kind::Dishwasher
                | Kind::ElectricOven
                | Kind::CoffeeMachine
                | Kind::Microwave
        )
    }

    pub fn is_cabinet(self) -> bool {
        matches!(
            self,
            Kind::Drawers { .. } | Kind::HingedDoors { .. } | Kind::LiftDoor | Kind::SlidingDoors
        )
    }

    /// Sizes a woodworker would actually start from, plus how high the base
    /// of the piece sits. Wall cabinets start above a 720 mm base.
    pub fn preset(self) -> (Furniture, f32) {
        let (width, height, depth, y_mm) = match self {
            Kind::Drawers { .. } => (600.0, 720.0, 560.0, 0.0),
            Kind::HingedDoors { .. } => (600.0, 720.0, 560.0, 0.0),
            Kind::LiftDoor => (600.0, 400.0, 350.0, 1400.0),
            Kind::SlidingDoors => (1000.0, 2100.0, 600.0, 0.0),
            Kind::Table => (1400.0, 750.0, 800.0, 0.0),
            Kind::Countertop => (2400.0, 38.0, 635.0, 720.0),
            Kind::Tabletop => (1400.0, 30.0, 800.0, 720.0),
            Kind::Box => (600.0, 720.0, 560.0, 0.0),
            Kind::Refrigerator => (900.0, 1850.0, 700.0, 0.0),
            Kind::Dishwasher => (600.0, 820.0, 600.0, 0.0),
            Kind::ElectricOven => (600.0, 600.0, 570.0, 720.0),
            Kind::CoffeeMachine => (260.0, 350.0, 420.0, 900.0),
            Kind::Microwave => (520.0, 310.0, 400.0, 1200.0),
        };
        (Furniture::new(self, width, height, depth), y_mm)
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
        match self.preset {
            FinishPreset::Oak => [181, 137, 84],
            FinishPreset::Walnut => [104, 70, 45],
            FinishPreset::White => [238, 236, 229],
            FinishPreset::Charcoal => [55, 58, 60],
            FinishPreset::StainlessSteel => [165, 172, 175],
            FinishPreset::Custom => self.color,
        }
    }
}

impl Default for Finish {
    fn default() -> Self {
        Self::new(FinishPreset::Oak, [181, 137, 84])
    }
}

/// Editable finishes for every visible family of parts. Carcass order is
/// left, right, bottom, top, back. Fronts follows the visible front order.
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub width_mm: f32,
    pub height_mm: f32,
    pub depth_mm: f32,
    pub finishes: PartFinishes,
    /// Set when the mesh no longer matches these dimensions.
    pub dirty: bool,
}

impl Furniture {
    pub fn new(kind: Kind, width_mm: f32, height_mm: f32, depth_mm: f32) -> Self {
        let mut furniture = Self {
            kind,
            name: kind.label().to_owned(),
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
        let count = self.kind.front_count();
        self.finishes.fronts.resize(count, Finish::default());
        self.finishes.fronts.truncate(count);
    }

    pub fn size_meters(&self) -> Vec3 {
        Vec3::new(
            mm_to_m(self.width_mm).max(0.05),
            mm_to_m(self.height_mm).max(0.05),
            mm_to_m(self.depth_mm).max(0.05),
        )
    }
}

/// Floor rectangle and the three walls around it. The open side is +Z,
/// which is where the camera starts, so a kitchen can be seen from the front.
#[derive(Resource, Clone, Debug)]
pub struct Room {
    pub width_mm: f32,
    pub depth_mm: f32,
    pub wall_height_mm: f32,
    pub show_walls: bool,
    pub dirty: bool,
}

impl Default for Room {
    fn default() -> Self {
        Self {
            width_mm: 4200.0,
            depth_mm: 3000.0,
            wall_height_mm: 2400.0,
            show_walls: true,
            dirty: true,
        }
    }
}

impl Room {
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
    match furniture.kind {
        Kind::Table => table(size),
        Kind::Countertop | Kind::Tabletop => vec![Solid::axis(
            size,
            Vec3::new(0.0, size.y * 0.5, 0.0),
            Surface::Top,
        )],
        Kind::Box => vec![Solid::axis(
            size,
            Vec3::new(0.0, size.y * 0.5, 0.0),
            Surface::Carcass(0),
        )],
        kind @ (Kind::Refrigerator
        | Kind::Dishwasher
        | Kind::ElectricOven
        | Kind::CoffeeMachine
        | Kind::Microwave) => appliance(size, kind),
        Kind::Drawers { count } => {
            let mut parts = carcass(size);
            parts.extend(drawer_fronts(size, count.max(1)));
            parts
        }
        Kind::HingedDoors { doors } => {
            let mut parts = carcass(size);
            parts.extend(hinged_fronts(size, doors.clamp(1, 2)));
            parts
        }
        Kind::LiftDoor => {
            let mut parts = carcass(size);
            parts.extend(lift_front(size));
            parts
        }
        Kind::SlidingDoors => {
            let mut parts = carcass(size);
            parts.extend(sliding_fronts(size));
            parts
        }
    }
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

fn hinged_fronts(size: Vec3, doors: u8) -> Vec<Solid> {
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
        parts.push(Solid::axis(
            Vec3::new(band_w, front_h, PANEL_METERS),
            Vec3::new(x, center_y, z),
            Surface::Front(index as u8),
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
fn lift_front(size: Vec3) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let front_h = (h - GAP_METERS * 2.0).max(0.02);
    let front_w = (w - GAP_METERS * 2.0).max(0.02);
    let hinge = Vec3::new(0.0, h - GAP_METERS, d * 0.5 + 0.004);
    let rotation = Quat::from_rotation_x(-0.65);
    let door_center = hinge + rotation * Vec3::new(0.0, -front_h * 0.5, 0.0);
    let handle_point =
        hinge + rotation * Vec3::new(0.0, -front_h + 0.04, PANEL_METERS * 0.5 + 0.004);
    vec![
        Solid {
            size: Vec3::new(front_w, front_h, PANEL_METERS).max(Vec3::splat(0.001)),
            translation: door_center,
            rotation,
            role: Surface::Front(0),
        },
        Solid {
            size: Vec3::new(0.12, 0.012, 0.012),
            translation: handle_point,
            rotation,
            role: Surface::Handle,
        },
    ]
}

fn sliding_fronts(size: Vec3) -> Vec<Solid> {
    let (w, h, d) = (size.x, size.y, size.z);
    let front_h = (h - GAP_METERS * 2.0).max(0.02);
    // Each leaf is a little wider than half so they overlap, and one sits
    // a few millimeters in front of the other so the overlap is readable.
    let leaf_w = w * 0.55;
    let center_y = h * 0.5;
    let back_z = d * 0.5 + 0.002;
    let front_z = back_z + 0.012;
    vec![
        Solid::axis(
            Vec3::new(leaf_w, front_h, PANEL_METERS),
            Vec3::new(-w * 0.22, center_y, back_z),
            Surface::Front(0),
        ),
        handle(Vec3::new(-0.04, center_y, back_z + PANEL_METERS), 0.02),
        Solid::axis(
            Vec3::new(leaf_w, front_h, PANEL_METERS),
            Vec3::new(w * 0.22, center_y, front_z),
            Surface::Front(1),
        ),
        handle(Vec3::new(0.04, center_y, front_z + PANEL_METERS), 0.02),
    ]
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
                Vec3::new(w * 0.46, h * 0.38, 0.02),
                Vec3::new(0.0, h * 0.55, front_z),
                Surface::Appliance(2),
            ),
            Solid::axis(
                Vec3::new(w * 0.55, 0.018, d * 0.34),
                Vec3::new(0.0, h * 0.08, d * 0.28),
                Surface::Appliance(1),
            ),
        ],
        Kind::Microwave => vec![
            body,
            Solid::axis(
                Vec3::new(w * 0.68, h * 0.68, 0.016),
                Vec3::new(-w * 0.08, h * 0.5, front_z),
                Surface::Appliance(2),
            ),
            Solid::axis(
                Vec3::new(w * 0.12, h * 0.68, 0.016),
                Vec3::new(w * 0.39, h * 0.5, front_z),
                Surface::Appliance(1),
            ),
        ],
        _ => vec![body],
    }
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
    (
        Vec3::new(-size.x * 0.5, 0.0, -size.z * 0.5),
        Vec3::new(size.x * 0.5, size.y + extra_y, size.z * 0.5 + extra_z),
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
        piece(Kind::Box, 600.0, 720.0, 560.0, 2250.0, 0.0, 280.0),
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

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct RoomFile {
    pub width_mm: f32,
    pub depth_mm: f32,
    pub wall_height_mm: f32,
    pub show_walls: bool,
}

#[derive(Serialize, Deserialize)]
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
    if !(1..=2).contains(&file.version) {
        return Err(ReadError::Invalid(format!(
            "layout file version is {}, this app reads versions 1 and 2",
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

impl ProjectFile {
    pub fn sanitized(mut self) -> Self {
        self.room.width_mm = finite_or(self.room.width_mm, 4200.0).clamp(500.0, 30000.0);
        self.room.depth_mm = finite_or(self.room.depth_mm, 3000.0).clamp(500.0, 30000.0);
        self.room.wall_height_mm = finite_or(self.room.wall_height_mm, 2400.0).clamp(500.0, 6000.0);
        self.snap_mm = finite_or(self.snap_mm, 50.0).clamp(1.0, 500.0);
        for piece in &mut self.pieces {
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
                other => other,
            };
            if piece.name.trim().is_empty() {
                piece.name = piece.kind.label().to_owned();
            }
            piece
                .finishes
                .fronts
                .resize(piece.kind.front_count(), Finish::default());
            piece.finishes.fronts.truncate(piece.kind.front_count());
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
        assert!(centers[0] < 200.0 / MM_PER_METER);
        assert!(centers[2] > 500.0 / MM_PER_METER);
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
            (Kind::Drawers { count: 3 }, 11),
            (Kind::HingedDoors { doors: 1 }, 7),
            (Kind::HingedDoors { doors: 2 }, 9),
            (Kind::LiftDoor, 7),
            (Kind::SlidingDoors, 9),
            (Kind::Table, 5),
            (Kind::Countertop, 1),
            (Kind::Tabletop, 1),
            (Kind::Box, 1),
            (Kind::Refrigerator, 4),
            (Kind::Dishwasher, 3),
            (Kind::ElectricOven, 4),
            (Kind::CoffeeMachine, 3),
            (Kind::Microwave, 3),
        ];
        for (kind, expected) in cases {
            let furniture = Furniture::new(kind, 600.0, 720.0, 560.0);
            assert_eq!(solids(&furniture).len(), expected, "{kind:?}");
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
}
