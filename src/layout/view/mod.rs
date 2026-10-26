//! The room, the orbit camera, and the meshes that stand in for furniture.
//!
//! Navigation is a normal modelling camera, aimed at a laptop trackpad as well
//! as a mouse:
//!
//! - drag empty space to orbit
//! - drag a piece to move it
//! - right-drag, or Space-drag, to pan
//! - scroll to zoom
//!
//! The pointer logic itself lives in `edit`. This module only stores the
//! camera angles and writes the camera transform from them.
//!
//! | File            | What is in it                                          |
//! |-----------------|--------------------------------------------------------|
//! | `mod.rs`        | [`setup_world`]: lights, camera, and the first load    |
//! | `camera.rs`     | [`OrbitCamera`] angles and view presets                |
//! | `meshes.rs`     | [`sync_meshes`]: rebuilds meshes for changed pieces    |
//! | `walls.rs`      | Floor and wall boxes, with holes for doors and windows |
//! | `materials.rs`  | Colors, wood grain, and the material cache             |
//! | `guides.rs`     | [`draw_guides`]: floor grid, axes, selection outline   |
//! | `convert.rs`    | Live pieces <-> saved file records                     |

mod camera;
mod convert;
mod guides;
mod materials;
mod meshes;
mod walls;

use bevy::prelude::*;

use crate::layout::model::{
    example_layout, read_project, resolve_layout_path, Editor, ReadError, Room, RoomRoot,
};

pub use camera::OrbitCamera;
pub(crate) use convert::{apply_saved_room, placement_from_file};
pub use convert::{piece_file, saved_room};
pub use guides::draw_guides;
pub use meshes::sync_meshes;

use materials::{MaterialCache, Palette};

/// Startup system: lights, the room root entity, the camera, and the pieces
/// from the last saved layout (or the example kitchen if there is none).
pub fn setup_world(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut room: ResMut<Room>,
    mut editor: ResMut<Editor>,
    mut cut_job: ResMut<crate::cutting::CutJob>,
) {
    ambient.color = Color::srgb(1.0, 0.98, 0.94);
    ambient.brightness = 90.0;

    commands.insert_resource(Palette::new(&mut materials));
    commands.insert_resource(MaterialCache::default());
    commands.spawn((
        RoomRoot,
        Name::new("Room"),
        Transform::default(),
        Visibility::default(),
    ));

    // A warm key light plus a cooler fill, so a cabinet's side is readable
    // without hunting for the light direction.
    commands.spawn((
        DirectionalLight {
            illuminance: 11000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(8.0, 14.0, 6.0).looking_at(Vec3::new(2.0, 0.5, 1.5), Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 3500.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(-6.0, 8.0, 10.0).looking_at(Vec3::new(2.0, 0.8, 1.0), Vec3::Y),
    ));

    let layout_path = resolve_layout_path(&editor.path);
    let loaded = match read_project(&layout_path.to_string_lossy()) {
        Ok(file) => {
            let file = file.sanitized();
            apply_saved_room(&mut room, &file.room);
            editor.snap_enabled = file.snap_enabled;
            editor.snap_mm = file.snap_mm;
            editor.status = "Loaded".into();
            *cut_job = crate::cutting::CutJob::from_file(&file.cut_list);
            Some(file.pieces)
        }
        Err(ReadError::Missing) => {
            editor.status = "Not saved yet".into();
            None
        }
        Err(ReadError::Invalid(message)) => {
            editor.status = format!("Could not read the file ({message}). Showing the example.");
            None
        }
    };

    let placements = loaded
        .map(|pieces| pieces.into_iter().map(placement_from_file).collect())
        .unwrap_or_else(example_layout);

    for (furniture, transform) in placements {
        commands.spawn((furniture, transform));
    }

    let camera = OrbitCamera::looking_at_room(&room);
    commands.spawn((
        Camera3d::default(),
        camera.pose(),
        camera,
        Name::new("Camera"),
    ));
}
