//! Custom blocks built in the component planner.
//!
//! A [`CustomType`] is a named list of loose [`CustomBoard`]s. Each board has
//! a role (wall, door, leg, ...), a shape (box or triangle), a size, a
//! position, and a rotation. When the block is placed in the room,
//! [`CustomType::scaled_solids`] stretches every board so the whole block
//! fits the width/height/depth typed into the Properties panel.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::geometry::{Solid, Surface};
use super::kind::LegStyle;
use super::units::{finite_or, mm_to_m};

// ---------------------------------------------------------------------------
// Board role and shape
// ---------------------------------------------------------------------------

/// What a loose board is for. Walls, planks, and the rest can still differ
/// in size. A triangle is a gable, a leg is a foot, and a drawer is a front.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomRole {
    Plank,
    Wall,
    Bottom,
    Top,
    Door,
    Back,
    Drawer,
    Leg,
}

impl CustomRole {
    pub const ALL: [Self; 8] = [
        Self::Plank,
        Self::Wall,
        Self::Bottom,
        Self::Top,
        Self::Door,
        Self::Back,
        Self::Drawer,
        Self::Leg,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Plank => "Plank",
            Self::Wall => "Wall",
            Self::Bottom => "Bottom",
            Self::Top => "Top",
            Self::Door => "Door",
            Self::Back => "Back",
            Self::Drawer => "Drawer",
            Self::Leg => "Leg",
        }
    }

    /// Doors and drawer fronts get their own color slot ("Fronts").
    pub fn is_front(self) -> bool {
        matches!(self, Self::Door | Self::Drawer)
    }

    /// A board of this role, sized and placed to fit a 600 × 720 × 560 mm cabinet.
    pub fn default_board(self) -> CustomBoard {
        let (width_mm, height_mm, depth_mm, x_mm, y_mm, z_mm) = match self {
            Self::Wall => (18.0, 720.0, 560.0, -291.0, 360.0, 0.0),
            Self::Plank => (564.0, 18.0, 500.0, 0.0, 360.0, 0.0),
            Self::Bottom => (564.0, 18.0, 560.0, 0.0, 9.0, 0.0),
            Self::Top => (500.0, 18.0, 480.0, 0.0, 711.0, 0.0),
            Self::Door => (564.0, 680.0, 18.0, 0.0, 360.0, 289.0),
            Self::Back => (564.0, 684.0, 18.0, 0.0, 360.0, -271.0),
            Self::Drawer => (500.0, 150.0, 18.0, 0.0, 90.0, 289.0),
            Self::Leg => (40.0, 100.0, 40.0, -250.0, 50.0, -220.0),
        };
        CustomBoard {
            role: self,
            shape: BoardShape::Box,
            leg: if self == Self::Leg {
                LegStyle::Plastic100
            } else {
                LegStyle::None
            },
            width_mm,
            height_mm,
            depth_mm,
            x_mm,
            y_mm,
            z_mm,
            yaw_deg: 0.0,
            pitch_deg: 0.0,
            roll_deg: 0.0,
            turns: 0,
            hole_width_mm: 0.0,
            hole_height_mm: 0.0,
            hole_x_mm: 0.0,
            hole_y_mm: 0.0,
        }
    }
}

/// The outline of a custom board. A triangle stands on its base.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardShape {
    #[default]
    Box,
    /// Isosceles gable. Width is the base and height is the peak.
    Triangle,
    /// Right angle at the lower left. Width is the base and height is the rise.
    RightTriangle,
}

impl BoardShape {
    pub const ALL: [Self; 3] = [Self::Box, Self::Triangle, Self::RightTriangle];

    pub fn label(self) -> &'static str {
        match self {
            Self::Box => "Box",
            Self::Triangle => "Triangle",
            Self::RightTriangle => "Right triangle",
        }
    }
}

/// One board inside a custom block. Position is the centre of the board.
/// Y grows up from the builder floor. Yaw turns it around the vertical.
/// Pitch and roll tilt it, so a roof can sit at any slope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomBoard {
    pub role: CustomRole,
    /// Missing on older saves. Those stay boxes.
    #[serde(default)]
    pub shape: BoardShape,
    /// Used when the role is a leg. Other boards ignore it.
    #[serde(default)]
    pub leg: LegStyle,
    pub width_mm: f32,
    pub height_mm: f32,
    pub depth_mm: f32,
    pub x_mm: f32,
    pub y_mm: f32,
    pub z_mm: f32,
    /// Degrees around Y. Zero keeps the board's width along X.
    #[serde(default)]
    pub yaw_deg: f32,
    /// Degrees around the board's width. A roof uses this.
    #[serde(default)]
    pub pitch_deg: f32,
    /// Degrees around the board's depth.
    #[serde(default)]
    pub roll_deg: f32,
    /// Quarter turns from the first rotation control. Folded into yaw on clean.
    #[serde(default)]
    pub turns: u8,
    /// A rectangular opening in a box, in millimetres. Zero in either axis means solid.
    #[serde(default)]
    pub hole_width_mm: f32,
    #[serde(default)]
    pub hole_height_mm: f32,
    /// Hole centre, measured from the board centre across its width and height.
    #[serde(default)]
    pub hole_x_mm: f32,
    #[serde(default)]
    pub hole_y_mm: f32,
}

impl CustomBoard {
    /// Clamp every number into a sane range, fold `turns` into `yaw_deg`, and
    /// make sure only legs carry a leg style.
    pub fn sanitize(&mut self) {
        self.width_mm = finite_or(self.width_mm, 18.0).clamp(1.0, 4000.0);
        self.height_mm = finite_or(self.height_mm, 18.0).clamp(1.0, 4000.0);
        self.depth_mm = finite_or(self.depth_mm, 18.0).clamp(1.0, 4000.0);
        self.x_mm = finite_or(self.x_mm, 0.0).clamp(-10000.0, 10000.0);
        self.y_mm = finite_or(self.y_mm, 0.0).clamp(-10000.0, 10000.0);
        self.z_mm = finite_or(self.z_mm, 0.0).clamp(-10000.0, 10000.0);
        self.yaw_deg = wrap_degrees(finite_or(self.yaw_deg, 0.0) + f32::from(self.turns) * 90.0);
        self.pitch_deg = wrap_degrees(finite_or(self.pitch_deg, 0.0));
        self.roll_deg = wrap_degrees(finite_or(self.roll_deg, 0.0));
        self.turns = 0;
        self.hole_width_mm = finite_or(self.hole_width_mm, 0.0).clamp(0.0, 4000.0);
        self.hole_height_mm = finite_or(self.hole_height_mm, 0.0).clamp(0.0, 4000.0);
        self.hole_x_mm = finite_or(self.hole_x_mm, 0.0).clamp(-4000.0, 4000.0);
        self.hole_y_mm = finite_or(self.hole_y_mm, 0.0).clamp(-4000.0, 4000.0);
        if self.role != CustomRole::Leg {
            self.leg = LegStyle::None;
        } else if self.leg == LegStyle::None {
            self.leg = LegStyle::Plastic100;
        }
    }

    /// Yaw, then pitch, then roll. Pitch lifts the depth edge, which is the
    /// slope of a roof. An old quarter-turn count still counts as yaw.
    pub fn rotation(&self) -> Quat {
        let yaw = (self.yaw_deg + f32::from(self.turns) * 90.0).to_radians();
        Quat::from_rotation_y(yaw)
            * Quat::from_rotation_x(self.pitch_deg.to_radians())
            * Quat::from_rotation_z(self.roll_deg.to_radians())
    }

    /// Half-size of the world-aligned box around the rotated board, in millimetres.
    pub fn world_half_mm(&self) -> Vec3 {
        let half = Vec3::new(self.width_mm, self.height_mm, self.depth_mm) * 0.5;
        let rotation = self.rotation();
        let mut extent = Vec3::ZERO;
        for x in [-half.x, half.x] {
            for y in [-half.y, half.y] {
                for z in [-half.z, half.z] {
                    extent = extent.max((rotation * Vec3::new(x, y, z)).abs());
                }
            }
        }
        extent
    }

    /// Keep the lowest corner on the floor.
    pub fn rest_on_floor(&mut self) {
        let clearance = self.world_half_mm().y;
        if self.y_mm < clearance {
            self.y_mm = clearance;
        }
    }

    /// Local boxes before rotation. A hole becomes the frame around the opening.
    /// A triangle stays one piece; the mesh carries the shape.
    pub fn pieces_mm(&self) -> Vec<(Vec3, Vec3)> {
        let size = Vec3::new(self.width_mm, self.height_mm, self.depth_mm);
        if self.shape != BoardShape::Box || self.hole_width_mm < 1.0 || self.hole_height_mm < 1.0 {
            return vec![(size, Vec3::ZERO)];
        }
        let w = self.width_mm;
        let h = self.height_mm;
        let d = self.depth_mm;
        let hw = self.hole_width_mm.min((w - 2.0).max(1.0));
        let hh = self.hole_height_mm.min((h - 2.0).max(1.0));
        let max_x = ((w - hw) * 0.5).max(0.0);
        let max_y = ((h - hh) * 0.5).max(0.0);
        let ox = self.hole_x_mm.clamp(-max_x, max_x);
        let oy = self.hole_y_mm.clamp(-max_y, max_y);
        let left = ox - hw * 0.5;
        let right = ox + hw * 0.5;
        let bottom = oy - hh * 0.5;
        let top = oy + hh * 0.5;
        let x0 = -w * 0.5;
        let x1 = w * 0.5;
        let y0 = -h * 0.5;
        let y1 = h * 0.5;
        let mut parts = Vec::new();
        let left_w = left - x0;
        if left_w > 0.5 {
            parts.push((
                Vec3::new(left_w, h, d),
                Vec3::new(x0 + left_w * 0.5, 0.0, 0.0),
            ));
        }
        let right_w = x1 - right;
        if right_w > 0.5 {
            parts.push((
                Vec3::new(right_w, h, d),
                Vec3::new(right + right_w * 0.5, 0.0, 0.0),
            ));
        }
        let inner_w = right - left;
        let bottom_h = bottom - y0;
        if bottom_h > 0.5 && inner_w > 0.5 {
            parts.push((
                Vec3::new(inner_w, bottom_h, d),
                Vec3::new(ox, y0 + bottom_h * 0.5, 0.0),
            ));
        }
        let top_h = y1 - top;
        if top_h > 0.5 && inner_w > 0.5 {
            parts.push((
                Vec3::new(inner_w, top_h, d),
                Vec3::new(ox, top + top_h * 0.5, 0.0),
            ));
        }
        if parts.is_empty() {
            vec![(size, Vec3::ZERO)]
        } else {
            parts
        }
    }

    /// A drawer front plus the box behind it. `index` stacks the next one above.
    pub fn drawer_set(index: usize) -> Vec<CustomBoard> {
        let y = 90.0 + index as f32 * 170.0;
        vec![
            placed(CustomRole::Drawer, 500.0, 150.0, 18.0, 0.0, y, 289.0),
            placed(CustomRole::Wall, 18.0, 110.0, 420.0, -241.0, y - 20.0, 60.0),
            placed(CustomRole::Wall, 18.0, 110.0, 420.0, 241.0, y - 20.0, 60.0),
            placed(CustomRole::Bottom, 464.0, 16.0, 420.0, 0.0, y - 70.0, 60.0),
            placed(CustomRole::Back, 464.0, 90.0, 16.0, 0.0, y - 25.0, -150.0),
        ]
    }

    /// A leg for slot `index`: the four corners first, then shifted copies.
    pub fn leg_at(style: LegStyle, index: usize) -> Self {
        let style = if style == LegStyle::None {
            LegStyle::Plastic100
        } else {
            style
        };
        let height = style.height_mm().max(40.0);
        let corners = [
            (-250.0, -220.0),
            (250.0, -220.0),
            (-250.0, 220.0),
            (250.0, 220.0),
        ];
        let (x, z) = corners[index % 4];
        let mut board = CustomRole::Leg.default_board();
        board.leg = style;
        board.height_mm = height;
        board.y_mm = height * 0.5;
        board.x_mm = x + (index / 4) as f32 * 50.0;
        board.z_mm = z;
        board
    }
}

/// A board of `role` with an explicit size and position.
fn placed(
    role: CustomRole,
    width_mm: f32,
    height_mm: f32,
    depth_mm: f32,
    x_mm: f32,
    y_mm: f32,
    z_mm: f32,
) -> CustomBoard {
    let mut board = role.default_board();
    board.width_mm = width_mm;
    board.height_mm = height_mm;
    board.depth_mm = depth_mm;
    board.x_mm = x_mm;
    board.y_mm = y_mm;
    board.z_mm = z_mm;
    board
}

/// Wrap an angle into -180°..=180°.
pub(crate) fn wrap_degrees(degrees: f32) -> f32 {
    let wrapped = degrees.rem_euclid(360.0);
    if wrapped > 180.0 {
        wrapped - 360.0
    } else {
        wrapped
    }
}

/// A saved furniture type. `width_mm`, `height_mm`, and `depth_mm` are the
/// size of the block as designed. Placing it in the room starts at that size,
/// and the properties sliders scale every board from there.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomType {
    pub name: String,
    pub boards: Vec<CustomBoard>,
    pub width_mm: f32,
    pub height_mm: f32,
    pub depth_mm: f32,
}

impl CustomType {
    /// A block with no boards and the default cabinet size.
    pub fn empty(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            boards: Vec::new(),
            width_mm: 600.0,
            height_mm: 720.0,
            depth_mm: 560.0,
        }
    }

    /// How many boards are fronts (doors or drawer fronts).
    pub fn door_count(&self) -> usize {
        self.boards
            .iter()
            .filter(|board| board.role.is_front())
            .count()
    }

    /// The (min, max) corners of the box around every board, in millimeters.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for board in &self.boards {
            let center = Vec3::new(board.x_mm, board.y_mm, board.z_mm);
            let half = board.world_half_mm();
            min = min.min(center - half);
            max = max.max(center + half);
        }
        if self.boards.is_empty() {
            (Vec3::ZERO, Vec3::new(600.0, 720.0, 560.0))
        } else {
            (min, max)
        }
    }

    /// Store the outer size of the boards so later scaling has a stable base.
    pub fn recompute_nominal(&mut self) {
        let (min, max) = self.bounds();
        let size = max - min;
        self.width_mm = finite_or(size.x, 600.0).max(1.0);
        self.height_mm = finite_or(size.y, 720.0).max(1.0);
        self.depth_mm = finite_or(size.z, 560.0).max(1.0);
    }

    /// Give the block a name, clean every board, and recompute its size.
    pub fn sanitize(&mut self) {
        if self.name.trim().is_empty() {
            self.name = "Custom block".to_owned();
        }
        for board in &mut self.boards {
            board.sanitize();
        }
        self.recompute_nominal();
    }

    /// The block's boards as renderable boxes, stretched to the given size.
    ///
    /// Each axis is scaled separately, so a wider block has its boards spread
    /// further apart and the boards along that axis get longer. The block's
    /// bottom stays on the floor and it is centered on X and Z.
    pub fn scaled_solids(&self, width_mm: f32, height_mm: f32, depth_mm: f32) -> Vec<Solid> {
        if self.boards.is_empty() {
            return Vec::new();
        }
        let (min, max) = self.bounds();
        let sx = width_mm / self.width_mm.max(1.0);
        let sy = height_mm / self.height_mm.max(1.0);
        let sz = depth_mm / self.depth_mm.max(1.0);
        let center_x = (min.x + max.x) * 0.5;
        let center_z = (min.z + max.z) * 0.5;
        let bottom = min.y;
        let mut door = 0u8;
        let mut solids = Vec::new();
        for board in &self.boards {
            let role = if board.role.is_front() {
                let index = door;
                door += 1;
                Surface::Front(index)
            } else if board.role == CustomRole::Leg {
                Surface::Leg(board.leg)
            } else {
                Surface::Carcass(0)
            };
            let rotation = board.rotation();
            let origin = Vec3::new(
                mm_to_m((board.x_mm - center_x) * sx),
                mm_to_m((board.y_mm - bottom) * sy),
                mm_to_m((board.z_mm - center_z) * sz),
            );
            let pieces = board.pieces_mm();
            let shape = if pieces.len() == 1 {
                board.shape
            } else {
                BoardShape::Box
            };
            for (size_mm, offset_mm) in pieces {
                let local = Vec3::new(
                    mm_to_m(offset_mm.x * sx),
                    mm_to_m(offset_mm.y * sy),
                    mm_to_m(offset_mm.z * sz),
                );
                solids.push(Solid {
                    size: Vec3::new(
                        mm_to_m(size_mm.x * sx),
                        mm_to_m(size_mm.y * sy),
                        mm_to_m(size_mm.z * sz),
                    )
                    .max(Vec3::splat(0.001)),
                    translation: origin + rotation * local,
                    rotation,
                    role,
                    shape,
                });
            }
        }
        solids
    }
}
