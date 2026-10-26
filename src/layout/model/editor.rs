//! Editor-wide state that is not part of the room: the snap grid, the open
//! file path, the status message, and which pieces are selected.

use bevy::prelude::*;

use super::units::mm_to_m;

/// Settings that apply to the whole editor, not to the room.
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

/// Which pieces are selected. `primary` is the one clicked last; the
/// Properties panel edits that one.
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

    /// Select just this piece.
    pub fn set_only(&mut self, entity: Entity) {
        self.entities.clear();
        self.entities.push(entity);
        self.primary = Some(entity);
    }

    /// Ctrl/Cmd-click: add the piece, or remove it if it was already selected.
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

    /// Drop pieces that fail `keep` (deleted, or locked pipes).
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
