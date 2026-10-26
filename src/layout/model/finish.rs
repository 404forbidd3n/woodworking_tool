//! Materials and colors.
//!
//! A [`Finish`] is either a named preset (oak, walnut, blue paint, ...) or a
//! custom RGB color. [`PartFinishes`] stores one finish per visible family of
//! parts on a piece, so the carcass, fronts, handles, top, and appliance body
//! can each be colored separately. These are visual only.

use serde::{Deserialize, Serialize};

/// Named materials. `Custom` uses the color stored in [`Finish::color`].
/// `White`, `Charcoal`, and `StainlessSteel` are used for handles and
/// appliances and are not offered as swatches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishPreset {
    Oak,
    Walnut,
    White,
    Charcoal,
    StainlessSteel,
    Custom,
    Pine,
    WhitePine,
    Poplar,
    Cherry,
    Birch,
    Blue,
    NordicWhite,
    MilitaryGreen,
    Black,
}

/// A named material plus an editable fallback color. Materials are deliberately
/// visual only: this draft does not calculate sheet stock or cut lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Finish {
    pub preset: FinishPreset,
    pub color: [u8; 3],
}

impl Finish {
    pub const fn new(preset: FinishPreset, color: [u8; 3]) -> Self {
        Self { preset, color }
    }

    /// The color to draw: the preset's color, or `color` for a custom finish.
    pub fn rgb(self) -> [u8; 3] {
        if self.preset == FinishPreset::Custom {
            self.color
        } else {
            self.preset.rgb()
        }
    }
}

impl FinishPreset {
    pub const PAINTS: [Self; 4] = [
        Self::Blue,
        Self::NordicWhite,
        Self::MilitaryGreen,
        Self::Black,
    ];
    pub const WOODS: [Self; 7] = [
        Self::Pine,
        Self::Oak,
        Self::Walnut,
        Self::WhitePine,
        Self::Poplar,
        Self::Cherry,
        Self::Birch,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Oak => "Oak",
            Self::Walnut => "Walnut",
            Self::White => "White",
            Self::Charcoal => "Charcoal",
            Self::StainlessSteel => "Stainless",
            Self::Custom => "Custom",
            Self::Pine => "Pine",
            Self::WhitePine => "White pine",
            Self::Poplar => "Poplar",
            Self::Cherry => "Cherry",
            Self::Birch => "Birch",
            Self::Blue => "Blue",
            Self::NordicWhite => "Nordic white",
            Self::MilitaryGreen => "Military green",
            Self::Black => "Black",
        }
    }

    /// Flat color used for paint, and the light tone of a wood.
    pub fn rgb(self) -> [u8; 3] {
        match self {
            Self::Oak => [196, 150, 92],
            Self::Walnut => [112, 74, 46],
            Self::White => [238, 236, 229],
            Self::Charcoal => [55, 58, 60],
            Self::StainlessSteel => [165, 172, 175],
            Self::Custom => [180, 180, 180],
            Self::Pine => [214, 176, 112],
            Self::WhitePine => [232, 214, 184],
            Self::Poplar => [206, 196, 156],
            Self::Cherry => [166, 78, 58],
            Self::Birch => [236, 220, 196],
            Self::Blue => [48, 98, 168],
            Self::NordicWhite => [247, 236, 214],
            Self::MilitaryGreen => [74, 90, 58],
            Self::Black => [22, 24, 26],
        }
    }

    /// Darker streak laid over [`Self::rgb`] to fake grain. Paints return themselves.
    pub fn grain(self) -> [u8; 3] {
        match self {
            Self::Oak => [132, 88, 46],
            Self::Walnut => [62, 38, 24],
            Self::Pine => [160, 112, 58],
            Self::WhitePine => [186, 160, 122],
            Self::Poplar => [168, 156, 118],
            Self::Cherry => [108, 42, 34],
            Self::Birch => [190, 166, 132],
            other => other.rgb(),
        }
    }

    /// Woods get a grain texture; paints are flat.
    pub fn is_wood(self) -> bool {
        matches!(
            self,
            Self::Oak
                | Self::Walnut
                | Self::Pine
                | Self::WhitePine
                | Self::Poplar
                | Self::Cherry
                | Self::Birch
        )
    }
}

impl Default for Finish {
    fn default() -> Self {
        Self::new(FinishPreset::Oak, [181, 137, 84])
    }
}

/// Editable finishes for every visible family of parts. Carcass order is
/// left, right, bottom, top, back. Fronts follows the visible front order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PartFinishes {
    pub carcass: [Finish; 5],
    pub fronts: Vec<Finish>,
    pub handles: Finish,
    pub top: Finish,
    pub appliance: Finish,
}

impl Default for PartFinishes {
    fn default() -> Self {
        Self {
            carcass: [Finish::default(); 5],
            fronts: Vec::new(),
            handles: Finish::new(FinishPreset::Charcoal, [55, 58, 60]),
            top: Finish::new(FinishPreset::Walnut, [104, 70, 45]),
            appliance: Finish::new(FinishPreset::StainlessSteel, [165, 172, 175]),
        }
    }
}
