//! Colors and materials for the 3D meshes.
//!
//! - [`Palette`] holds the fixed colors used for the room (floor, walls) and
//!   as a fallback.
//! - [`MaterialCache`] creates one GPU material per distinct finish and reuses
//!   it, so fifty oak cabinets share one material.
//! - [`surface_material`] picks the right material for one box of a piece,
//!   based on its [`Surface`] tag and the piece's chosen finishes.
//!
//! Wood finishes get a small procedurally generated grain texture, so the
//! app needs no image files.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::layout::model::{Finish, FinishPreset, Furniture, Surface};

// ---------------------------------------------------------------------------
// Fixed palette
// ---------------------------------------------------------------------------

/// Stock colors. They are only there so a door reads differently from a side,
/// and so a placeholder box does not look like a finished cabinet.
#[derive(Resource)]
pub struct Palette {
    pub carcass: Handle<StandardMaterial>,
    pub front: Handle<StandardMaterial>,
    pub handle: Handle<StandardMaterial>,
    pub table: Handle<StandardMaterial>,
    pub placeholder: Handle<StandardMaterial>,
    pub floor: Handle<StandardMaterial>,
    pub wall: Handle<StandardMaterial>,
    pub glass: Handle<StandardMaterial>,
}

impl Palette {
    /// Create the fixed materials.
    pub(super) fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        let mat = |materials: &mut Assets<StandardMaterial>, color: Color| {
            materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.82,
                metallic: 0.0,
                ..default()
            })
        };
        Self {
            carcass: mat(materials, Color::srgb(0.62, 0.46, 0.28)),
            front: mat(materials, Color::srgb(0.78, 0.64, 0.44)),
            handle: mat(materials, Color::srgb(0.20, 0.14, 0.10)),
            table: mat(materials, Color::srgb(0.50, 0.36, 0.22)),
            placeholder: mat(materials, Color::srgb(0.55, 0.60, 0.64)),
            floor: mat(materials, Color::srgb(0.78, 0.76, 0.72)),
            wall: mat(materials, Color::srgb(0.92, 0.91, 0.88)),
            glass: materials.add(StandardMaterial {
                base_color: Color::srgba(0.72, 0.84, 0.90, 0.38),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.06,
                metallic: 0.05,
                ..default()
            }),
        }
    }

    /// The fixed material for a surface. Used for boxes with no piece (the room).
    fn handle(&self, role: Surface) -> Handle<StandardMaterial> {
        match role {
            Surface::Carcass(_) => self.carcass.clone(),
            Surface::Front(_) => self.front.clone(),
            Surface::Handle => self.handle.clone(),
            Surface::Top => self.table.clone(),
            Surface::Appliance(_) => self.placeholder.clone(),
            Surface::ApplianceRound(_) => self.placeholder.clone(),
            Surface::Leg(_) => self.handle.clone(),
            Surface::Pipe => self.handle.clone(),
            Surface::Floor => self.floor.clone(),
            Surface::Wall => self.wall.clone(),
            Surface::Glass => self.glass.clone(),
            Surface::Edge => self.handle.clone(),
        }
    }
}

/// Reuses GPU materials across pieces that have the same visible finish.
#[derive(Resource, Default)]
pub struct MaterialCache {
    entries: HashMap<([u8; 3], [u8; 3], bool), Handle<StandardMaterial>>,
    glass: Option<Handle<StandardMaterial>>,
}

impl MaterialCache {
    /// The material for a finish, created on first use.
    fn get(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
        images: &mut Assets<Image>,
        finish: Finish,
    ) -> Handle<StandardMaterial> {
        let rgb = finish.rgb();
        let metallic = finish.preset == FinishPreset::StainlessSteel;
        let grain = if finish.preset.is_wood() {
            finish.preset.grain()
        } else {
            rgb
        };
        let wood = finish.preset.is_wood();
        self.entries
            .entry((rgb, grain, metallic))
            .or_insert_with(|| {
                let texture = wood.then(|| images.add(wood_grain(rgb, grain)));
                materials.add(StandardMaterial {
                    base_color: if wood {
                        Color::WHITE
                    } else {
                        Color::srgb_u8(rgb[0], rgb[1], rgb[2])
                    },
                    base_color_texture: texture,
                    perceptual_roughness: if metallic {
                        0.32
                    } else if wood {
                        0.62
                    } else {
                        0.78
                    },
                    metallic: if metallic { 0.86 } else { 0.0 },
                    ..default()
                })
            })
            .clone()
    }

    /// The shared see-through material for door panes.
    fn glass(&mut self, materials: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
        self.glass
            .get_or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: Color::srgba(0.78, 0.88, 0.94, 0.34),
                    alpha_mode: AlphaMode::Blend,
                    perceptual_roughness: 0.04,
                    metallic: 0.08,
                    ..default()
                })
            })
            .clone()
    }
}

/// Pick the material for one box. Room boxes use the [`Palette`]. Piece
/// boxes look up the matching finish on the piece: carcass panel, front,
/// handle, top, appliance body, leg, or pipe.
pub(super) fn surface_material(
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    cache: &mut MaterialCache,
    palette: &Palette,
    furniture: Option<&Furniture>,
    role: Surface,
) -> Handle<StandardMaterial> {
    let Some(furniture) = furniture else {
        return palette.handle(role);
    };
    let finish = match role {
        Surface::Carcass(index) => furniture
            .finishes
            .carcass
            .get(index as usize)
            .copied()
            .unwrap_or_default(),
        Surface::Front(index) => furniture
            .finishes
            .fronts
            .get(index as usize)
            .copied()
            .unwrap_or_default(),
        Surface::Handle => furniture.finishes.handles,
        Surface::Top => furniture.finishes.top,
        Surface::Appliance(2) => Finish::new(FinishPreset::Charcoal, [28, 32, 34]),
        Surface::Appliance(_) => furniture.finishes.appliance,
        Surface::ApplianceRound(2) => Finish::new(FinishPreset::Charcoal, [28, 32, 34]),
        Surface::ApplianceRound(_) => furniture.finishes.appliance,
        Surface::Leg(crate::layout::model::LegStyle::Wood100) => furniture.finishes.carcass[0],
        Surface::Leg(crate::layout::model::LegStyle::Metal100)
        | Surface::Leg(crate::layout::model::LegStyle::Metal150) => {
            Finish::new(FinishPreset::StainlessSteel, [165, 172, 175])
        }
        Surface::Leg(_) => Finish::new(FinishPreset::Charcoal, [48, 50, 52]),
        Surface::Pipe => match furniture.kind {
            crate::layout::model::Kind::Pipe { color, .. } => {
                Finish::new(FinishPreset::Custom, color.rgb())
            }
            _ => return palette.handle(role),
        },
        Surface::Glass => return cache.glass(materials),
        Surface::Edge => Finish::new(FinishPreset::Charcoal, [10, 10, 10]),
        Surface::Floor | Surface::Wall => return palette.handle(role),
    };
    cache.get(materials, images, finish)
}

/// A small generated grain image. No external texture files: each wood is a
/// pair of colors with vertical streaks and a few pores.
fn wood_grain(base: [u8; 3], streak: [u8; 3]) -> Image {
    let size = 96u32;
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let n = grain_hash(x, y);
            let wander = (grain_hash(x / 4, 0) - 0.5) * 5.0;
            let band = ((x as f32 + wander) * 0.42).sin();
            let pore = if grain_hash(x / 2, y / 7) > 0.86 {
                0.45
            } else {
                0.0
            };
            let mix = ((band * 0.5 + 0.5) * 0.7 + pore + (n - 0.5) * 0.12).clamp(0.0, 1.0);
            let channel = |light: u8, dark: u8| {
                (light as f32 + (dark as f32 - light as f32) * mix).round() as u8
            };
            data.extend_from_slice(&[
                channel(base[0], streak[0]),
                channel(base[1], streak[1]),
                channel(base[2], streak[2]),
                255,
            ]);
        }
    }
    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// Cheap repeatable noise in 0..=1 for the grain texture.
fn grain_hash(x: u32, y: u32) -> f32 {
    let mut n = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    (n & 255) as f32 / 255.0
}
