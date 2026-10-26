//! Converting between live pieces (a [`Furniture`] plus a `Transform`, in
//! meters) and saved records ([`PieceFile`], [`RoomFile`], in millimeters).
//!
//! Saving, loading, and undo/redo all go through these four functions.

use bevy::prelude::*;

use crate::layout::model::{Furniture, PieceFile, Room, RoomFile};

// ---------------------------------------------------------------------------
// File -> live
// ---------------------------------------------------------------------------

/// Copy a saved room onto the live [`Room`] and mark it for a redraw.
pub(crate) fn apply_saved_room(room: &mut Room, saved: &RoomFile) {
    room.width_mm = saved.width_mm;
    room.depth_mm = saved.depth_mm;
    room.wall_height_mm = saved.wall_height_mm;
    room.show_walls = saved.show_walls;
    room.walls = saved.resolved_walls();
    room.edit_enabled = saved.edit_enabled;
    room.openings = saved.openings.clone();
    room.dirty = true;
}

/// A saved piece as a ready-to-spawn (Furniture, Transform) pair.
pub(crate) fn placement_from_file(piece: PieceFile) -> (Furniture, Transform) {
    let mut furniture = Furniture::new(piece.kind, piece.width_mm, piece.height_mm, piece.depth_mm);
    furniture.name = piece.name;
    furniture.placement = piece.placement;
    furniture.legs = piece.legs;
    furniture.doors = piece.doors;
    furniture.custom = piece.custom;
    furniture.finishes = piece.finishes;
    furniture.ensure_front_finishes();
    (
        furniture,
        crate::layout::model::place(piece.x_mm, piece.y_mm, piece.z_mm, piece.turns),
    )
}

/// The live room as a save record.
pub fn saved_room(room: &Room) -> RoomFile {
    RoomFile {
        width_mm: room.width_mm,
        depth_mm: room.depth_mm,
        wall_height_mm: room.wall_height_mm,
        show_walls: room.show_walls,
        walls: Some(room.walls),
        edit_enabled: room.edit_enabled,
        openings: room.openings.clone(),
    }
}

/// A live piece as a save record, in millimeters and quarter turns.
pub fn piece_file(furniture: &Furniture, transform: &Transform) -> PieceFile {
    PieceFile {
        kind: furniture.kind,
        name: furniture.name.clone(),
        width_mm: furniture.width_mm,
        height_mm: furniture.height_mm,
        depth_mm: furniture.depth_mm,
        x_mm: crate::layout::model::m_to_mm(transform.translation.x),
        y_mm: crate::layout::model::m_to_mm(transform.translation.y),
        z_mm: crate::layout::model::m_to_mm(transform.translation.z),
        turns: crate::layout::model::quarter_turns(transform.rotation),
        placement: furniture.placement,
        legs: furniture.legs,
        doors: furniture.doors,
        custom: furniture.custom.clone(),
        finishes: furniture.finishes.clone(),
    }
}
