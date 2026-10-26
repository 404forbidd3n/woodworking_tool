//! The sheet and the parts waiting to be cut.
//!
//! This is the data the cut planner edits. The packing itself lives in
//! [`super::pack`], and the window that draws it lives in [`super::ui`].

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

use crate::layout::model::{CutListFile, CutPartFile};

/// What a rectangle is, so the list and the canvas can name it.
/// The names are the ones a cabinet drawing actually uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartRole {
    Door,
    DrawerFront,
    Shelf,
    Top,
    Bottom,
    Side,
    Back,
    Divider,
    Custom,
}

impl PartRole {
    pub const ALL: [PartRole; 9] = [
        PartRole::Door,
        PartRole::DrawerFront,
        PartRole::Shelf,
        PartRole::Top,
        PartRole::Bottom,
        PartRole::Side,
        PartRole::Back,
        PartRole::Divider,
        PartRole::Custom,
    ];

    /// Name shown in the UI. Also the string written to the save file.
    pub fn label(self) -> &'static str {
        match self {
            PartRole::Door => "Door",
            PartRole::DrawerFront => "Drawer front",
            PartRole::Shelf => "Shelf",
            PartRole::Top => "Top",
            PartRole::Bottom => "Bottom",
            PartRole::Side => "Side",
            PartRole::Back => "Back",
            PartRole::Divider => "Divider",
            PartRole::Custom => "Custom",
        }
    }

    /// Inverse of [`Self::label`]. Unknown text becomes `Custom`.
    pub fn from_label(label: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|role| role.label() == label)
            .unwrap_or(PartRole::Custom)
    }

    /// Short name drawn in front of a list row.
    pub fn tag(self) -> &'static str {
        match self {
            PartRole::DrawerFront => "Drawer",
            other => other.label(),
        }
    }

    /// Doors and drawer fronts are usually a different sheet, such as MDF.
    pub fn is_front(self) -> bool {
        matches!(self, PartRole::Door | PartRole::DrawerFront)
    }

    /// Which tab of the cut list this part belongs to.
    pub fn group(self) -> CutListGroup {
        if self.is_front() {
            CutListGroup::Doors
        } else if self == PartRole::Back {
            CutListGroup::Backs
        } else {
            CutListGroup::Parts
        }
    }
}

/// The lists inside the cut planner. Fronts and backs stay apart from the
/// carcass because each is usually a different sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CutListGroup {
    Doors,
    Backs,
    Parts,
}

/// One line in the parts list. `quantity` is how many identical rectangles
/// the estimate has to place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CutPart {
    pub name: String,
    pub role: PartRole,
    pub width_mm: f32,
    pub height_mm: f32,
    pub quantity: u32,
}

impl CutPart {
    /// The part's name, or its role when it has no name.
    pub fn titled(&self) -> String {
        if self.name.trim().is_empty() {
            self.role.label().to_owned()
        } else {
            self.name.clone()
        }
    }
}

/// The supplied sheet. Width and height are the face you cut on.
/// Thickness is recorded so the order note is complete, but it does not
/// change the 2D layout.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StockBoard {
    pub width_mm: f32,
    pub height_mm: f32,
    pub thickness_mm: f32,
    /// Saw kerf, left as a gap between rectangles.
    pub kerf_mm: f32,
    /// Trim taken off each edge before any part is placed.
    pub trim_mm: f32,
    /// Plywood can turn 90°. Leave this off when the grain has to stay put.
    pub allow_rotate: bool,
}

impl Default for StockBoard {
    fn default() -> Self {
        Self {
            // A common full sheet. The numbers stay editable.
            width_mm: 2440.0,
            height_mm: 1220.0,
            thickness_mm: 18.0,
            kerf_mm: 3.0,
            trim_mm: 0.0,
            allow_rotate: true,
        }
    }
}

/// Everything the cut planner is working on, including the half-typed part
/// at the bottom of the list.
#[derive(Resource, Clone, Debug)]
pub struct CutJob {
    pub board: StockBoard,
    pub parts: Vec<CutPart>,
    pub draft_role: PartRole,
    pub draft_name: String,
    pub draft_width_mm: f32,
    pub draft_height_mm: f32,
    pub draft_quantity: u32,
    /// Which list is open. Not saved; it is only the open tab.
    pub list_group: CutListGroup,
    /// Which packed board is on screen when the estimate needs more than one.
    pub sheet_page: usize,
    pub estimate: Option<super::pack::Estimate>,
}

impl Default for CutJob {
    fn default() -> Self {
        Self {
            board: StockBoard::default(),
            parts: Vec::new(),
            draft_role: PartRole::Door,
            draft_name: String::new(),
            draft_width_mm: 600.0,
            draft_height_mm: 720.0,
            draft_quantity: 1,
            list_group: CutListGroup::Doors,
            sheet_page: 0,
            estimate: None,
        }
    }
}

impl CutJob {
    /// Rebuild the planner state from a saved project.
    pub fn from_file(file: &CutListFile) -> Self {
        let mut job = Self {
            board: StockBoard {
                width_mm: file.board_width_mm,
                height_mm: file.board_height_mm,
                thickness_mm: file.thickness_mm,
                kerf_mm: file.kerf_mm,
                trim_mm: file.trim_mm,
                allow_rotate: file.allow_rotate,
            },
            parts: file
                .parts
                .iter()
                .map(|part| CutPart {
                    name: part.name.clone(),
                    role: PartRole::from_label(&part.role),
                    width_mm: part.width_mm,
                    height_mm: part.height_mm,
                    quantity: part.quantity,
                })
                .collect(),
            ..Self::default()
        };
        job.draft_role = PartRole::from_label(&file.draft_role);
        job.draft_name = file.draft_name.clone();
        job.draft_width_mm = file.draft_width_mm;
        job.draft_height_mm = file.draft_height_mm;
        job.draft_quantity = file.draft_quantity;
        job
    }

    /// The saved form of this job, written into the project file.
    pub fn to_file(&self) -> CutListFile {
        CutListFile {
            board_width_mm: self.board.width_mm,
            board_height_mm: self.board.height_mm,
            thickness_mm: self.board.thickness_mm,
            kerf_mm: self.board.kerf_mm,
            trim_mm: self.board.trim_mm,
            allow_rotate: self.board.allow_rotate,
            draft_role: self.draft_role.label().to_owned(),
            draft_name: self.draft_name.clone(),
            draft_width_mm: self.draft_width_mm,
            draft_height_mm: self.draft_height_mm,
            draft_quantity: self.draft_quantity,
            parts: self
                .parts
                .iter()
                .map(|part| CutPartFile {
                    name: part.name.clone(),
                    role: part.role.label().to_owned(),
                    width_mm: part.width_mm,
                    height_mm: part.height_mm,
                    quantity: part.quantity,
                })
                .collect(),
        }
    }

    /// Adding or editing a part makes the last estimate out of date.
    pub fn mark_stale(&mut self) {
        if let Some(estimate) = &mut self.estimate {
            estimate.stale = true;
        }
    }

    /// Move the half-typed part at the bottom of the list into the list, and
    /// switch to the tab it landed in.
    pub fn add_draft(&mut self) {
        let quantity = self.draft_quantity.clamp(1, 100);
        let role = self.draft_role;
        self.parts.push(CutPart {
            name: self.draft_name.trim().to_owned(),
            role,
            width_mm: self.draft_width_mm,
            height_mm: self.draft_height_mm,
            quantity,
        });
        self.draft_name.clear();
        self.list_group = role.group();
        self.mark_stale();
    }

    /// Drop one material group and leave the others.
    pub fn clear_group(&mut self, group: CutListGroup) {
        let before = self.parts.len();
        self.parts.retain(|part| part.role.group() != group);
        if self.parts.len() != before {
            self.mark_stale();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(role: PartRole) -> CutPart {
        CutPart {
            name: role.label().to_owned(),
            role,
            width_mm: 100.0,
            height_mm: 100.0,
            quantity: 1,
        }
    }

    #[test]
    fn fronts_and_carcass_are_separate_groups() {
        assert!(PartRole::Door.is_front());
        assert!(PartRole::DrawerFront.is_front());
        assert!(!PartRole::Side.is_front());
        assert!(!PartRole::Back.is_front());
        assert!(!PartRole::Shelf.is_front());
        assert_eq!(PartRole::DrawerFront.group(), CutListGroup::Doors);
        assert_eq!(PartRole::Back.group(), CutListGroup::Backs);
        assert_eq!(PartRole::Top.group(), CutListGroup::Parts);
    }

    #[test]
    fn clearing_doors_keeps_carcass_parts() {
        let mut job = CutJob::default();
        job.parts = vec![
            part(PartRole::Door),
            part(PartRole::DrawerFront),
            part(PartRole::Back),
            part(PartRole::Side),
            part(PartRole::Shelf),
        ];
        job.clear_group(CutListGroup::Doors);
        let roles: Vec<_> = job.parts.iter().map(|part| part.role).collect();
        assert_eq!(roles, vec![PartRole::Back, PartRole::Side, PartRole::Shelf]);
        job.clear_group(CutListGroup::Backs);
        let roles: Vec<_> = job.parts.iter().map(|part| part.role).collect();
        assert_eq!(roles, vec![PartRole::Side, PartRole::Shelf]);
        job.clear_group(CutListGroup::Parts);
        assert!(job.parts.is_empty());
    }
}
