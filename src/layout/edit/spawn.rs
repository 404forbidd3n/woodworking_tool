//! Adding and removing pieces.
//!
//! New pieces always appear in the middle of the room's floor plan. The
//! piece's mesh is built later by `view::sync_meshes`, because the new
//! [`Furniture`] starts out `dirty`.

use bevy::prelude::*;

use crate::layout::model::{place, CustomType, Furniture, Kind, LegStyle, Placement, Room};

/// Add a built-in piece with its preset size. Wall pieces start at the
/// preset height on the wall; floor pieces start on the floor.
pub(super) fn add_piece(
    commands: &mut Commands,
    room: &Room,
    kind: Kind,
    placement: Placement,
) -> Entity {
    let (furniture, y_mm) = kind.preset(placement);
    commands
        .spawn((furniture, place_at_center(room, y_mm, 0)))
        .id()
}

/// Add a custom block at the size it was designed.
pub(super) fn add_custom(commands: &mut Commands, room: &Room, block: &CustomType) -> Entity {
    let mut furniture = Furniture::new(
        Kind::Custom,
        block.width_mm,
        block.height_mm,
        block.depth_mm,
    );
    furniture.name = block.name.clone();
    furniture.custom = Some(block.clone());
    furniture.placement = Placement::Floor;
    furniture.legs = LegStyle::Plastic100;
    furniture.ensure_front_finishes();
    commands
        .spawn((furniture, place_at_center(room, 0.0, 0)))
        .id()
}

/// Footprint centre of the room. Height and facing stay with the piece.
pub(super) fn place_at_center(room: &Room, y_mm: f32, turns: u8) -> Transform {
    place(room.width_mm * 0.5, y_mm, room.depth_mm * 0.5, turns)
}

/// Pipes belong to the room, so clearing the furniture leaves them in place.
pub(super) fn despawn_all(commands: &mut Commands, rows: &[(Entity, String, bool)]) {
    for (entity, _, is_pipe) in rows {
        if !is_pipe {
            commands.entity(*entity).despawn();
        }
    }
}
