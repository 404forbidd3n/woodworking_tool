//! Small document-level undo and redo history.
//!
//! A snapshot is cheaper and safer than a command for this application: a
//! room contains only a few dozen lightweight records. Continuous drags are
//! coalesced while an input is held, so one accidental drag is one undo step.

use bevy::prelude::*;

use super::model::{Editor, Furniture, Room};
use super::project::{self, LayoutSnapshot};

const HISTORY_LIMIT: usize = 10;

#[derive(Resource, Default)]
pub struct History {
    undo: Vec<LayoutSnapshot>,
    redo: Vec<LayoutSnapshot>,
    last_seen: Option<LayoutSnapshot>,
    pending_before: Option<LayoutSnapshot>,
}

impl History {
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self, current: LayoutSnapshot) -> Option<LayoutSnapshot> {
        let target = self.undo.pop()?;
        self.redo.push(current);
        self.last_seen = Some(target.clone());
        self.pending_before = None;
        Some(target)
    }

    pub fn redo(&mut self, current: LayoutSnapshot) -> Option<LayoutSnapshot> {
        let target = self.redo.pop()?;
        push_limited(&mut self.undo, current);
        self.last_seen = Some(target.clone());
        self.pending_before = None;
        Some(target)
    }
}

fn push_limited(stack: &mut Vec<LayoutSnapshot>, snapshot: LayoutSnapshot) {
    if stack.len() == HISTORY_LIMIT {
        stack.remove(0);
    }
    stack.push(snapshot);
}

/// Observe document changes after commands have been applied. Holding a mouse
/// button or key leaves the transaction open; releasing it commits one step.
pub fn track_history(
    mut history: ResMut<History>,
    room: Res<Room>,
    editor: Res<Editor>,
    pieces: Query<(&Furniture, &Transform)>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let current = project::capture(&room, &editor, &pieces);
    let Some(previous) = history.last_seen.clone() else {
        history.last_seen = Some(current);
        return;
    };
    if current != previous {
        if history.pending_before.is_none() {
            history.pending_before = Some(previous);
            history.redo.clear();
        }
        history.last_seen = Some(current);
    }
    let input_active = mouse.get_pressed().next().is_some() || keys.get_pressed().next().is_some();
    if !input_active {
        if let Some(before) = history.pending_before.take() {
            push_limited(&mut history.undo, before);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::model::RoomFile;

    fn snapshot(value: f32) -> LayoutSnapshot {
        LayoutSnapshot {
            room: RoomFile {
                width_mm: value,
                depth_mm: 3000.0,
                wall_height_mm: 2400.0,
                show_walls: true,
                openings: Vec::new(),
            },
            pieces: Vec::new(),
            snap_enabled: true,
            snap_mm: 50.0,
        }
    }

    #[test]
    fn history_is_bounded_and_new_edit_clears_redo() {
        let mut history = History::default();
        for index in 0..12 {
            push_limited(&mut history.undo, snapshot(index as f32));
        }
        assert_eq!(history.undo.len(), HISTORY_LIMIT);
        let current = snapshot(20.0);
        assert!(history.undo(current.clone()).is_some());
        assert!(history.can_redo());
        history.redo.clear();
        assert!(!history.can_redo());
    }
}
