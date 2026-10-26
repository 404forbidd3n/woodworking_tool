//! Saving, loading, the native file dialogs, and undo / redo.
//!
//! Save writes straight to the current path. "Save As" and "Load" open a
//! native file dialog on another thread ([`NativeDialog`]); the
//! [`finish_native_dialog`] system checks every frame whether the user has
//! picked a file and then saves or loads it.

use bevy::prelude::*;
use bevy::window::RequestRedraw;

use crate::cutting::CutJob;
use crate::layout::history::History;
use crate::layout::model::{
    read_project, resolve_layout_path, write_project, Editor, Furniture, ProjectFile, ReadError,
    Room, Selection,
};
use crate::layout::project::{self, DialogRequest, NativeDialog};
use crate::layout::view::{self, OrbitCamera};

use super::toolbar::layout_label;

// ---------------------------------------------------------------------------
// Save and load
// ---------------------------------------------------------------------------

/// Save to the current path (`layout.json` if none). The result goes to the
/// status message in the toolbar.
pub(super) fn save_layout(
    editor: &mut Editor,
    room: &Room,
    pieces: &Query<(Entity, &mut Furniture, &mut Transform)>,
    cut_job: &CutJob,
) {
    if editor.path.trim().is_empty() {
        editor.path = "layout.json".into();
    }
    let path = resolve_layout_path(&editor.path);
    editor.status = match write_current(
        &path,
        editor.snap_enabled,
        editor.snap_mm,
        room,
        pieces,
        cut_job,
    ) {
        Ok(()) => "Saved".into(),
        Err(message) => message,
    };
    info!("{}", editor.status);
}

/// Build a [`ProjectFile`] from the live room and write it to `path`.
fn write_current(
    path: &std::path::Path,
    snap_enabled: bool,
    snap_mm: f32,
    room: &Room,
    pieces: &Query<(Entity, &mut Furniture, &mut Transform)>,
    cut_job: &CutJob,
) -> Result<(), String> {
    let file = ProjectFile {
        version: 3,
        snap_enabled,
        snap_mm,
        room: view::saved_room(room),
        pieces: pieces
            .iter()
            .map(|(_, furniture, transform)| view::piece_file(furniture, &transform))
            .collect(),
        cut_list: cut_job.to_file(),
    };
    write_project(path, &file)
}

/// Replace the room with the file at the current path, and frame the camera
/// on it. A missing or broken file leaves the room alone and says why.
fn load_layout(
    commands: &mut Commands,
    pieces: &Query<(Entity, &mut Furniture, &mut Transform)>,
    room: &mut Room,
    editor: &mut Editor,
    selection: &mut Selection,
    camera: &mut Query<&mut OrbitCamera>,
    cut_job: &mut CutJob,
) {
    if editor.path.trim().is_empty() {
        editor.path = "layout.json".into();
    }
    let path = resolve_layout_path(&editor.path);
    let existing: Vec<Entity> = pieces.iter().map(|(entity, _, _)| entity).collect();
    match read_project(&path.to_string_lossy()) {
        Ok(file) => {
            let file = file.sanitized();
            *cut_job = CutJob::from_file(&file.cut_list);
            project::restore(
                project::from_project(&file),
                commands,
                room,
                editor,
                selection,
                &existing,
            );
            if let Ok(mut orbit) = camera.single_mut() {
                orbit.frame_room(room);
            }
            editor.status = "Loaded".into();
        }
        Err(ReadError::Missing) => {
            editor.status = format!("No file named {}", layout_label(&editor.path));
        }
        Err(ReadError::Invalid(message)) => {
            editor.status = message;
        }
    }
}

/// Open the native Save As or Open dialog, unless one is already open.
pub(super) fn begin_dialog(
    dialogs: &mut NativeDialog,
    editor: &mut Editor,
    request: DialogRequest,
) {
    if dialogs.is_open() {
        return;
    }
    match request {
        DialogRequest::Save => dialogs.begin_save(&editor.path),
        DialogRequest::Open => dialogs.begin_open(&editor.path),
    }
    editor.status = "Choose a file".into();
}

/// Applies a finished Load or Save As dialog. The panel itself stays open
/// without stopping the 3D window.
pub fn finish_native_dialog(
    mut dialogs: NonSendMut<NativeDialog>,
    mut redraws: MessageWriter<RequestRedraw>,
    mut commands: Commands,
    pieces: Query<(Entity, &mut Furniture, &mut Transform)>,
    mut room: ResMut<Room>,
    mut editor: ResMut<Editor>,
    mut selection: ResMut<Selection>,
    mut camera: Query<&mut OrbitCamera>,
    mut cut_job: ResMut<CutJob>,
    mut history: ResMut<History>,
) {
    if dialogs.is_open() {
        redraws.write(RequestRedraw);
    }
    let Some((request, path)) = dialogs.poll() else {
        return;
    };
    let Some(path) = path else {
        editor.status = "Cancelled".into();
        return;
    };
    editor.path = path.display().to_string();
    match request {
        DialogRequest::Save => save_layout(&mut editor, &room, &pieces, &cut_job),
        DialogRequest::Open => {
            load_layout(
                &mut commands,
                &pieces,
                &mut room,
                &mut editor,
                &mut selection,
                &mut camera,
                &mut cut_job,
            );
            *history = History::default();
        }
    }
}

/// Undo (`redo == false`) or redo one step. The current state is pushed on
/// the opposite stack so the step can be reversed.
pub(super) fn perform_history(
    redo: bool,
    history: &mut History,
    commands: &mut Commands,
    room: &mut Room,
    editor: &mut Editor,
    selection: &mut Selection,
    pieces: &Query<(Entity, &mut Furniture, &mut Transform)>,
) {
    let files = pieces
        .iter()
        .map(|(_, furniture, transform)| view::piece_file(furniture, transform))
        .collect();
    let current = project::from_files(room, editor, files);
    let target = if redo {
        history.redo(current)
    } else {
        history.undo(current)
    };
    if let Some(target) = target {
        let existing: Vec<_> = pieces.iter().map(|(entity, _, _)| entity).collect();
        project::restore(target, commands, room, editor, selection, &existing);
        editor.status = if redo { "Redone" } else { "Undone" }.into();
    }
}
