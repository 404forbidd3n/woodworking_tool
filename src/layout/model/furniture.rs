//! The [`Furniture`] component: one placed piece in the room.
//!
//! The entity that carries a `Furniture` also carries a `Transform`. The
//! component holds the size, the kind, and the colors; the transform holds
//! where the piece stands and which way it faces.

use bevy::prelude::*;

use super::custom::CustomType;
use super::finish::{Finish, PartFinishes};
use super::kind::{DoorStyle, Kind, LegStyle, Placement};
use super::units::mm_to_m;

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
    /// A piece with the given size and default everything else. Lift-up doors
    /// start on the wall; floor cabinets start with 100 mm plastic legs.
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

    /// Make `finishes.fronts` have exactly one entry per door or drawer front.
    /// Call this whenever the number of fronts changes.
    pub fn ensure_front_finishes(&mut self) {
        let count = self.front_slots();
        self.finishes.fronts.resize(count, Finish::default());
        self.finishes.fronts.truncate(count);
    }

    /// Width, height, and depth in meters, with a small minimum so a mesh never
    /// collapses to nothing.
    pub fn size_meters(&self) -> Vec3 {
        // Pipes are much thinner than a cabinet panel, so they keep a smaller floor.
        let minimum = if self.kind.is_pipe() { 0.008 } else { 0.05 };
        let min_height = mm_to_m(self.kind.min_height_mm()).min(minimum);
        Vec3::new(
            mm_to_m(self.width_mm).max(minimum),
            mm_to_m(self.height_mm).max(min_height),
            mm_to_m(self.depth_mm).max(minimum),
        )
    }

    /// How far the legs lift the carcass. Zero for anything that is not a floor
    /// cabinet.
    pub fn leg_height_meters(&self) -> f32 {
        if self.placement == Placement::Floor && self.kind.is_cabinet() {
            mm_to_m(self.legs.height_mm())
        } else {
            0.0
        }
    }

    /// The display name of this kind of piece, e.g. "Wall Drawer Cabinet".
    pub fn label(&self) -> &'static str {
        self.kind.label_for(self.placement)
    }

    /// How many fronts get their own color.
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
