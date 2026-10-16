//! Native project-file selection.
//!
//! Serialization stays in `model`; this module only owns the operating-system
//! dialogs so the editor UI does not need platform-specific file logic.
//!
//! The dialogs are asynchronous. A blocking save panel, opened from inside
//! the window event loop, freezes the macOS app.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use bevy::prelude::*;

use super::model::{
    project_dir, Editor, Furniture, PieceFile, ProjectFile, Room, RoomFile, Selection,
};
use super::view;

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutSnapshot {
    pub room: RoomFile,
    pub pieces: Vec<PieceFile>,
    pub snap_enabled: bool,
    pub snap_mm: f32,
}

pub fn capture(
    room: &Room,
    editor: &Editor,
    pieces: &Query<(&Furniture, &Transform)>,
) -> LayoutSnapshot {
    from_files(
        room,
        editor,
        pieces
            .iter()
            .map(|(furniture, transform)| view::piece_file(furniture, transform))
            .collect(),
    )
}

pub fn from_files(room: &Room, editor: &Editor, pieces: Vec<PieceFile>) -> LayoutSnapshot {
    LayoutSnapshot {
        room: view::saved_room(room),
        pieces,
        snap_enabled: editor.snap_enabled,
        snap_mm: editor.snap_mm,
    }
}

pub fn from_project(file: &ProjectFile) -> LayoutSnapshot {
    LayoutSnapshot {
        room: file.room.clone(),
        pieces: file.pieces.clone(),
        snap_enabled: file.snap_enabled,
        snap_mm: file.snap_mm,
    }
}

pub fn restore(
    snapshot: LayoutSnapshot,
    commands: &mut Commands,
    room: &mut Room,
    editor: &mut Editor,
    selection: &mut Selection,
    existing: &[Entity],
) {
    for entity in existing {
        commands.entity(*entity).despawn();
    }
    view::apply_saved_room(room, &snapshot.room);
    editor.snap_enabled = snapshot.snap_enabled;
    editor.snap_mm = snapshot.snap_mm;
    selection.clear();
    for piece in snapshot.pieces {
        commands.spawn(view::placement_from_file(piece));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogRequest {
    Save,
    Open,
}

struct ActiveDialog {
    request: DialogRequest,
    future: Pin<Box<dyn Future<Output = Option<rfd::FileHandle>> + Send>>,
}

/// One native dialog at a time. The window keeps running while it is open.
///
/// This stays on the main thread. The dialog future is `Send` but not `Sync`.
#[derive(Default)]
pub struct NativeDialog {
    active: Option<ActiveDialog>,
}

impl NativeDialog {
    pub fn begin_save(&mut self, current: &str) {
        self.begin(DialogRequest::Save, save_dialog(current).save_file());
    }

    pub fn begin_open(&mut self, current: &str) {
        self.begin(DialogRequest::Open, open_dialog(current).pick_file());
    }

    fn begin(
        &mut self,
        request: DialogRequest,
        future: impl Future<Output = Option<rfd::FileHandle>> + Send + 'static,
    ) {
        if self.active.is_some() {
            return;
        }
        self.active = Some(ActiveDialog {
            request,
            future: Box::pin(future),
        });
    }

    /// `Some` once the user confirms or cancels. `None` while the panel is open.
    pub fn poll(&mut self) -> Option<(DialogRequest, Option<PathBuf>)> {
        let active = self.active.as_mut()?;
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        match active.future.as_mut().poll(&mut context) {
            Poll::Pending => None,
            Poll::Ready(file) => {
                let request = self.active.take().unwrap().request;
                let path = file.map(|handle| {
                    let path = handle.path().to_path_buf();
                    if request == DialogRequest::Save {
                        ensure_json_extension(path)
                    } else {
                        path
                    }
                });
                Some((request, path))
            }
        }
    }

    pub fn is_open(&self) -> bool {
        self.active.is_some()
    }
}

fn save_dialog(current: &str) -> rfd::AsyncFileDialog {
    let current = Path::new(current);
    let mut dialog = rfd::AsyncFileDialog::new()
        .add_filter("Woodworking project", &["json"])
        .set_title("Save woodworking project")
        .set_directory(
            current
                .parent()
                .filter(|path| path.is_dir())
                .unwrap_or(project_dir()),
        );
    if let Some(name) = current.file_name().and_then(|name| name.to_str()) {
        dialog = dialog.set_file_name(name);
    }
    dialog
}

fn open_dialog(current: &str) -> rfd::AsyncFileDialog {
    let current = Path::new(current);
    let directory = current
        .parent()
        .filter(|path| path.is_dir())
        .unwrap_or(project_dir());
    rfd::AsyncFileDialog::new()
        .add_filter("Woodworking project", &["json"])
        .set_title("Open woodworking project")
        .set_directory(directory)
}

fn ensure_json_extension(mut path: PathBuf) -> PathBuf {
    if path.extension().is_none() {
        path.set_extension("json");
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_name_gets_json_extension() {
        assert_eq!(
            ensure_json_extension(PathBuf::from("/tmp/kitchen")),
            PathBuf::from("/tmp/kitchen.json")
        );
        assert_eq!(
            ensure_json_extension(PathBuf::from("/tmp/kitchen.json")),
            PathBuf::from("/tmp/kitchen.json")
        );
    }
}
