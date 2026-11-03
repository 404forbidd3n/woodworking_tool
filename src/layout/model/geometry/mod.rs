//! Turning a piece into boxes.
//!
//! Nothing here touches Bevy entities. [`solids`] takes a [`Furniture`] and
//! returns a list of [`Solid`]s: plain boxes (or triangular prisms) with a
//! size, position, rotation, and a [`Surface`] tag that tells the renderer
//! which color to paint them. `view.rs` then turns each solid into a mesh.
//!
//! All sizes here are in meters, in the piece's local space: the origin is
//! the center of the base, +Y is up, and +Z is the front.
//!
//! Each family of furniture has its own file:
//!
//! - [`cabinet`] — the five-panel carcass, shelves, legs, drawers, and doors
//! - [`corner`] — the L-shaped corner cabinet
//! - [`appliance`] — fridge, oven, microwave, boiler, and so on
//! - [`sink`] — the open wash basins
//! - [`table`] — the work table
//! - [`pipe`] — utility pipes

mod appliance;
mod cabinet;
mod corner;
mod pipe;
mod sink;
mod table;

use bevy::prelude::*;

use super::custom::BoardShape;
use super::furniture::Furniture;
use super::kind::{jolly_width_mm, DoorStyle, Kind, LegStyle};

pub use cabinet::equal_spans;

use appliance::appliance;
use cabinet::{
    adjustable_legs, carcass, drawer_fronts, hinged_fronts, jolly_fronts, lift_front, open_shelves,
    sliding_fronts,
};
use corner::corner_cabinet;
use pipe::pipes;
use table::table;

// ---------------------------------------------------------------------------
// The output types
// ---------------------------------------------------------------------------

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
    /// The black margin around a door or drawer front. See [`edge_band_size`].
    Edge,
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
    pub shape: BoardShape,
}

impl Solid {
    /// An unrotated box.
    fn axis(size: Vec3, translation: Vec3, role: Surface) -> Self {
        Self {
            size: size.max(Vec3::splat(0.001)),
            translation,
            rotation: Quat::IDENTITY,
            role,
            shape: BoardShape::Box,
        }
    }
}

/// Mesh for a board. Triangles are prisms standing on the width axis.
pub fn shape_mesh(shape: BoardShape, size: Vec3) -> Mesh {
    let size = size.max(Vec3::splat(0.001));
    match shape {
        BoardShape::Box => Mesh::from(Cuboid::new(size.x, size.y, size.z)),
        BoardShape::Triangle | BoardShape::RightTriangle => triangle_prism(size, shape),
    }
}

/// A triangle extruded along Z. Flat normals, so each face shades evenly.
fn triangle_prism(size: Vec3, shape: BoardShape) -> Mesh {
    let (hx, hy, hz) = (size.x * 0.5, size.y * 0.5, size.z * 0.5);
    let (a, b, c) = if shape == BoardShape::RightTriangle {
        ([-hx, -hy], [hx, -hy], [-hx, hy])
    } else {
        ([-hx, -hy], [hx, -hy], [0.0, hy])
    };
    let at = |point: [f32; 2], z: f32| [point[0], point[1], z];
    let mut positions = Vec::new();
    let tri = |positions: &mut Vec<[f32; 3]>, p: [f32; 3], q: [f32; 3], r: [f32; 3]| {
        positions.extend([p, q, r]);
    };
    tri(&mut positions, at(a, hz), at(b, hz), at(c, hz));
    tri(&mut positions, at(a, -hz), at(c, -hz), at(b, -hz));
    // Each boundary edge P -> Q of the counter-clockwise profile.
    for (p, q) in [(a, b), (b, c), (c, a)] {
        let (p0, p1) = (at(p, hz), at(p, -hz));
        let (q0, q1) = (at(q, hz), at(q, -hz));
        tri(&mut positions, p0, p1, q1);
        tri(&mut positions, p0, q1, q0);
    }
    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.compute_flat_normals();
    mesh
}

/// Width of the black margin shown around every door and drawer front.
pub const FRONT_EDGE_METERS: f32 = 0.002;

/// Size of the black band drawn behind a front of `size`, in its own local
/// axes. The two long axes grow by [`FRONT_EDGE_METERS`] on each side, so the
/// band shows as a rim around the front. The thin axis shrinks by a millimeter,
/// so the band stays behind both faces and never covers the front's color.
pub fn edge_band_size(size: Vec3) -> Vec3 {
    let thin = size.min_element();
    let grow = Vec3::splat(FRONT_EDGE_METERS * 2.0);
    let mut band = size + grow;
    let recessed = thin - 0.001_f32.min(thin * 0.5);
    if size.x == thin {
        band.x = recessed;
    } else if size.y == thin {
        band.y = recessed;
    } else {
        band.z = recessed;
    }
    band
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
        | Kind::InductionHob
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
        Kind::JollyDrawers { drawers, jolly_mm } => {
            let jolly_w = jolly_width_mm(furniture.width_mm, jolly_mm) / 1000.0;
            let mut parts = carcass(size);
            parts.extend(jolly_fronts(size, jolly_w, drawers.clamp(1, 10)));
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
    let edges: Vec<Solid> = parts
        .iter()
        .filter(|part| matches!(part.role, Surface::Front(_)))
        .map(|part| Solid {
            size: edge_band_size(part.size),
            role: Surface::Edge,
            ..*part
        })
        .collect();
    parts.extend(edges);
    let leg_height = furniture.leg_height_meters();
    if leg_height > 0.0 {
        for part in &mut parts {
            part.translation.y += leg_height;
        }
        parts.extend(adjustable_legs(size, leg_height, furniture.legs));
    }
    parts
}
