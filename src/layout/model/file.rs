//! The saved project format (`silvius.json` and friends).
//!
//! Everything here is plain serde data in millimeters. [`read_project`] and
//! [`write_project`] do the disk I/O, and [`ProjectFile::sanitized`] clamps
//! every number into a sane range after loading, so a hand-edited or old
//! file can never put the app into a broken state.
//!
//! File versions:
//! - 1: the first release, no placement or legs
//! - 2: added the cut list
//! - 3: added floor/wall placement and legs (current)

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::custom::CustomType;
use super::finish::{Finish, PartFinishes};
use super::kind::{DoorStyle, Kind, LegStyle, Placement};
use super::room::{Opening, OpeningKind, WallSide, WallVisibility};
use super::units::finite_or;

// ---------------------------------------------------------------------------
// File structs
// ---------------------------------------------------------------------------

/// Everything in one saved project.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectFile {
    pub version: u32,
    pub snap_enabled: bool,
    pub snap_mm: f32,
    pub room: RoomFile,
    pub pieces: Vec<PieceFile>,
    /// The cut planner. Missing in older files, which then start from a
    /// standard 2440 × 1220 mm sheet.
    #[serde(default)]
    pub cut_list: CutListFile,
}

/// Saved sheet and parts. The planner that edits this lives in `cutting`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CutListFile {
    pub board_width_mm: f32,
    pub board_height_mm: f32,
    pub thickness_mm: f32,
    pub kerf_mm: f32,
    pub trim_mm: f32,
    pub allow_rotate: bool,
    pub draft_role: String,
    pub draft_name: String,
    pub draft_width_mm: f32,
    pub draft_height_mm: f32,
    pub draft_quantity: u32,
    pub parts: Vec<CutPartFile>,
}

impl Default for CutListFile {
    fn default() -> Self {
        Self {
            board_width_mm: 2440.0,
            board_height_mm: 1220.0,
            thickness_mm: 18.0,
            kerf_mm: 3.0,
            trim_mm: 0.0,
            allow_rotate: true,
            draft_role: "Door".into(),
            draft_name: String::new(),
            draft_width_mm: 600.0,
            draft_height_mm: 720.0,
            draft_quantity: 1,
            parts: Vec::new(),
        }
    }
}

/// One saved line of the cut list. `role` is the text label of the role.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CutPartFile {
    pub name: String,
    pub role: String,
    pub width_mm: f32,
    pub height_mm: f32,
    pub quantity: u32,
}

/// The saved room. See [`Room`](super::Room) for what the fields mean.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoomFile {
    pub width_mm: f32,
    pub depth_mm: f32,
    pub wall_height_mm: f32,
    pub show_walls: bool,
    /// Missing on older files. Those keep three walls and a hidden front.
    #[serde(default)]
    pub walls: Option<WallVisibility>,
    #[serde(default)]
    pub edit_enabled: bool,
    #[serde(default)]
    pub openings: Vec<Opening>,
}

impl RoomFile {
    /// Older files have no per-wall flags. Those keep the back, left, and
    /// right walls and leave the front hidden.
    pub fn resolved_walls(&self) -> WallVisibility {
        self.walls
            .unwrap_or_else(|| WallVisibility::legacy(self.show_walls))
    }
}

/// One saved piece: its [`Furniture`](super::Furniture) data plus its
/// position (millimeters) and facing (quarter turns).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PieceFile {
    pub kind: Kind,
    #[serde(default)]
    pub name: String,
    pub width_mm: f32,
    pub height_mm: f32,
    pub depth_mm: f32,
    pub x_mm: f32,
    pub y_mm: f32,
    pub z_mm: f32,
    /// 0, 1, 2, or 3. See [`quarter_turns`].
    pub turns: u8,
    #[serde(default)]
    pub placement: Placement,
    #[serde(default)]
    pub legs: LegStyle,
    #[serde(default)]
    pub doors: DoorStyle,
    #[serde(default)]
    pub custom: Option<CustomType>,
    #[serde(default)]
    pub finishes: PartFinishes,
}

/// Why a project could not be loaded.
pub enum ReadError {
    Missing,
    Invalid(String),
}

/// Folder this program was compiled from. Saves go here, not into whatever
/// directory the terminal happened to be in when the window opened.
pub fn project_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// The project opened at startup, and saved to when no name is entered.
pub const DEFAULT_LAYOUT_FILE: &str = "silvius.json";

/// A bare file name is saved in the project folder. A full path is used as given.
pub fn resolve_layout_path(entered: &str) -> PathBuf {
    let trimmed = entered.trim();
    let relative = if trimmed.is_empty() {
        PathBuf::from(DEFAULT_LAYOUT_FILE)
    } else {
        PathBuf::from(trimmed)
    };
    if relative.is_absolute() {
        relative
    } else {
        project_dir().join(relative)
    }
}

/// Read and parse a project file. Versions 1 to 3 are accepted. The result
/// is not yet sanitized; call [`ProjectFile::sanitized`] on it.
pub fn read_project(path: &str) -> Result<ProjectFile, ReadError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(ReadError::Missing)
        }
        Err(error) => return Err(ReadError::Invalid(error.to_string())),
    };
    let file: ProjectFile =
        serde_json::from_str(&text).map_err(|error| ReadError::Invalid(error.to_string()))?;
    if !(1..=3).contains(&file.version) {
        return Err(ReadError::Invalid(format!(
            "layout file version is {}, this app reads versions 1, 2, and 3",
            file.version
        )));
    }
    Ok(file)
}

/// Write a project as pretty JSON, creating the folder if needed.
pub fn write_project(path: &Path, file: &ProjectFile) -> Result<(), String> {
    let text = serde_json::to_string_pretty(file).map_err(|error| error.to_string())?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("Could not create {}: {error}", parent.display()))?;
        }
    }
    std::fs::write(path, text)
        .map_err(|error| format!("Could not write {}: {error}", path.display()))
}

/// Keep each door and window inside its wall: not wider or taller than the
/// wall, and not past its end. Doors always start at the floor.
pub fn sanitize_openings(
    room_width_mm: f32,
    room_depth_mm: f32,
    wall_height_mm: f32,
    openings: &mut [Opening],
) {
    for opening in openings {
        let wall_length = match opening.wall {
            WallSide::Back | WallSide::Front => room_width_mm,
            WallSide::Left | WallSide::Right => room_depth_mm,
        };
        opening.width_mm = finite_or(opening.width_mm, 900.0).clamp(100.0, wall_length.max(100.0));
        opening.height_mm = finite_or(opening.height_mm, 1000.0).clamp(100.0, wall_height_mm);
        opening.offset_mm =
            finite_or(opening.offset_mm, 0.0).clamp(0.0, (wall_length - opening.width_mm).max(0.0));
        opening.sill_mm = if opening.kind == OpeningKind::Door {
            0.0
        } else {
            finite_or(opening.sill_mm, 900.0)
                .clamp(0.0, (wall_height_mm - opening.height_mm).max(0.0))
        };
    }
}

impl ProjectFile {
    /// Clamp every value to a sane range and upgrade older files: pieces from
    /// versions 1 and 2 get a placement and legs, front color lists are resized
    /// to match the door count, and custom blocks are cleaned.
    pub fn sanitized(mut self) -> Self {
        self.room.width_mm = finite_or(self.room.width_mm, 4200.0).clamp(500.0, 30000.0);
        self.room.depth_mm = finite_or(self.room.depth_mm, 3000.0).clamp(500.0, 30000.0);
        self.room.wall_height_mm = finite_or(self.room.wall_height_mm, 2400.0).clamp(500.0, 6000.0);
        sanitize_openings(
            self.room.width_mm,
            self.room.depth_mm,
            self.room.wall_height_mm,
            &mut self.room.openings,
        );
        self.snap_mm = finite_or(self.snap_mm, 50.0).clamp(1.0, 500.0);
        for piece in &mut self.pieces {
            if self.version < 3 {
                piece.placement = if piece.kind == Kind::LiftDoor {
                    Placement::Wall
                } else {
                    Placement::Floor
                };
                piece.legs = if piece.placement == Placement::Floor && piece.kind.is_cabinet() {
                    LegStyle::Plastic100
                } else {
                    LegStyle::None
                };
            }
            piece.width_mm = finite_or(piece.width_mm, 600.0).clamp(50.0, 6000.0);
            piece.height_mm =
                finite_or(piece.height_mm, 720.0).clamp(piece.kind.min_height_mm(), 4000.0);
            piece.depth_mm = finite_or(piece.depth_mm, 560.0).clamp(50.0, 3000.0);
            piece.x_mm = finite_or(piece.x_mm, 0.0);
            piece.y_mm = finite_or(piece.y_mm, 0.0);
            piece.z_mm = finite_or(piece.z_mm, 0.0);
            piece.turns %= 4;
            piece.kind = match piece.kind {
                Kind::Drawers { count } => Kind::Drawers {
                    count: count.clamp(1, 10),
                },
                Kind::JollyDrawers { drawers, jolly_mm } => Kind::JollyDrawers {
                    drawers: drawers.clamp(1, 10),
                    jolly_mm: jolly_mm.clamp(50, 4000),
                },
                Kind::HingedDoors { doors } => Kind::HingedDoors {
                    doors: if doors <= 1 { 1 } else { 2 },
                },
                Kind::OpenShelves { shelves } => Kind::OpenShelves {
                    shelves: shelves.clamp(1, 10),
                },
                other => other,
            };
            if piece.name.trim().is_empty() {
                piece.name = piece.kind.label_for(piece.placement).to_owned();
            }
            let fronts = if piece.kind == Kind::Custom {
                let block = piece
                    .custom
                    .get_or_insert_with(|| CustomType::empty(piece.name.clone()));
                block.sanitize();
                piece.width_mm = piece.width_mm.max(1.0);
                piece.height_mm = piece.height_mm.max(1.0);
                piece.depth_mm = piece.depth_mm.max(1.0);
                block.door_count()
            } else {
                piece.custom = None;
                piece.kind.front_count()
            };
            piece.finishes.fronts.resize(fronts, Finish::default());
            piece.finishes.fronts.truncate(fronts);
        }
        let list = &mut self.cut_list;
        list.board_width_mm = finite_or(list.board_width_mm, 2440.0).clamp(100.0, 4000.0);
        list.board_height_mm = finite_or(list.board_height_mm, 1220.0).clamp(100.0, 4000.0);
        list.thickness_mm = finite_or(list.thickness_mm, 18.0).clamp(3.0, 60.0);
        list.kerf_mm = finite_or(list.kerf_mm, 3.0).clamp(0.0, 10.0);
        list.trim_mm = finite_or(list.trim_mm, 0.0).clamp(0.0, 50.0);
        list.draft_width_mm = finite_or(list.draft_width_mm, 600.0).clamp(10.0, 4000.0);
        list.draft_height_mm = finite_or(list.draft_height_mm, 720.0).clamp(10.0, 4000.0);
        list.draft_quantity = list.draft_quantity.clamp(1, 100);
        for part in &mut list.parts {
            part.width_mm = finite_or(part.width_mm, 100.0).clamp(1.0, 4000.0);
            part.height_mm = finite_or(part.height_mm, 100.0).clamp(1.0, 4000.0);
            part.quantity = part.quantity.clamp(1, 100);
        }
        self
    }
}
