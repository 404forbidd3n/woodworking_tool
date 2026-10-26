//! Keeping the meshes in sync with the data.
//!
//! [`sync_meshes`] runs every frame. Any [`Furniture`] marked `dirty` (or the
//! [`Room`], when it is dirty) has all of its child mesh entities thrown away
//! and rebuilt from `model::solids`. Rebuilding is cheap because everything
//! is a handful of boxes.

use bevy::prelude::*;

use super::materials::{surface_material, MaterialCache, Palette};
use super::walls::room_solids;
use crate::layout::model::{Furniture, Room, RoomRoot, Solid, Surface};

/// One child mesh, ready to spawn.
struct Built {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    transform: Transform,
}

/// Rebuild any piece or room whose dimensions changed. Meshes are plain boxes;
/// rebuilding a cabinet is cheap and keeps the draw code obvious.
pub fn sync_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut material_cache: ResMut<MaterialCache>,
    palette: Res<Palette>,
    mut room: ResMut<Room>,
    mut pieces: Query<(Entity, &mut Furniture)>,
    roots: Query<Entity, With<RoomRoot>>,
) {
    if room.dirty {
        room.dirty = false;
        if let Some(root) = roots.iter().next() {
            let parts = room_solids(&room);
            replace_children(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut images,
                &mut material_cache,
                &palette,
                root,
                parts,
                None,
            );
        }
    }

    let pending: Vec<(Entity, Furniture)> = pieces
        .iter()
        .filter(|(_, furniture)| furniture.dirty)
        .map(|(entity, furniture)| (entity, furniture.clone()))
        .collect();

    for (_, mut furniture) in &mut pieces {
        furniture.dirty = false;
    }

    for (entity, furniture) in pending {
        let parts = crate::layout::model::solids(&furniture);
        commands.entity(entity).insert(Name::new(furniture.label()));
        replace_children(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut images,
            &mut material_cache,
            &palette,
            entity,
            parts,
            Some(&furniture),
        );
    }
}

/// Despawn every child mesh of `entity` and spawn one per solid. Pipes and
/// round appliance parts become cylinders; everything else is a box or a
/// triangular prism.
fn replace_children(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    material_cache: &mut MaterialCache,
    palette: &Palette,
    entity: Entity,
    parts: Vec<Solid>,
    furniture: Option<&Furniture>,
) {
    let built: Vec<Built> = parts
        .into_iter()
        .map(|part| {
            let (mesh, rotation) =
                if matches!(part.role, Surface::Pipe | Surface::ApplianceRound(_)) {
                    // The two closest axes are the round cross-section, so a
                    // short disc stays upright instead of turning into a rod.
                    let (x, y, z) = (part.size.x, part.size.y, part.size.z);
                    let xz = (x - z).abs();
                    let yz = (y - z).abs();
                    let xy = (x - y).abs();
                    let (radius, length, spin) = if xz <= yz && xz <= xy {
                        (x.min(z) * 0.5, y, Quat::IDENTITY)
                    } else if yz <= xz && yz <= xy {
                        (
                            y.min(z) * 0.5,
                            x,
                            Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                        )
                    } else {
                        (
                            x.min(y) * 0.5,
                            z,
                            Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                        )
                    };
                    (
                        meshes.add(Cylinder::new(radius.max(0.001), length.max(0.001))),
                        part.rotation * spin,
                    )
                } else {
                    (
                        meshes.add(crate::layout::model::shape_mesh(part.shape, part.size)),
                        part.rotation,
                    )
                };
            Built {
                mesh,
                material: surface_material(
                    materials,
                    images,
                    material_cache,
                    palette,
                    furniture,
                    part.role,
                ),
                transform: Transform {
                    translation: part.translation,
                    rotation,
                    scale: Vec3::ONE,
                },
            }
        })
        .collect();

    commands.entity(entity).despawn_related::<Children>();
    commands.entity(entity).with_children(move |parent| {
        for part in built {
            parent.spawn((
                Mesh3d(part.mesh),
                MeshMaterial3d(part.material),
                part.transform,
            ));
        }
    });
}
