//! Lay rectangles on a sheet so a person can cut them with a track saw.
//!
//! The layout is a row packer, not a free puzzle. Parts sit in horizontal
//! bands. When a band is full, the next band starts underneath it. That wastes
//! a little more than a perfect nest, and it matches a cut you can actually
//! make: one rip across the sheet, then crosscuts along the strip.
//!
//! The saw kerf is a gap after each part and under each band. A trim, when
//! set, is kept clear on every edge of the sheet.

use super::job::{CutPart, StockBoard};

/// One rectangle that landed on a sheet. Sizes are the finished part, not
/// the kerf around it.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedPart {
    pub part_index: usize,
    pub x_mm: f32,
    pub y_mm: f32,
    pub width_mm: f32,
    pub height_mm: f32,
    pub rotated: bool,
}

/// Everything placed on one sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct PackedSheet {
    pub parts: Vec<PlacedPart>,
}

/// The result of pressing Estimate.
#[derive(Clone, Debug, PartialEq)]
pub struct Estimate {
    pub sheets: Vec<PackedSheet>,
    /// Parts that cannot fit on the sheet even alone.
    pub too_big: Vec<usize>,
    pub used_area_mm2: f32,
    pub sheet_area_mm2: f32,
    /// True after the list changes, until Estimate is pressed again.
    pub stale: bool,
}

impl Estimate {
    /// How many sheets to buy.
    pub fn boards_to_order(&self) -> usize {
        self.sheets.len()
    }

    /// Share of the bought sheets that ends up as offcut, 0.0 to 1.0.
    pub fn waste_ratio(&self) -> f32 {
        if self.sheet_area_mm2 <= f32::EPSILON {
            0.0
        } else {
            (1.0 - self.used_area_mm2 / self.sheet_area_mm2).clamp(0.0, 1.0)
        }
    }
}

/// One horizontal band on a sheet while packing. `cursor_mm` is where the
/// next part in this band would start.
struct OpenShelf {
    y_mm: f32,
    height_mm: f32,
    cursor_mm: f32,
}

/// A sheet while packing: its bands and the parts placed so far.
struct OpenSheet {
    shelves: Vec<OpenShelf>,
    parts: Vec<PlacedPart>,
}

/// Pack every copy of every part. Sheets are added until the list is placed
/// or a part is reported as too big to ever fit.
pub fn estimate(board: &StockBoard, parts: &[CutPart]) -> Estimate {
    let usable_width = (board.width_mm - board.trim_mm * 2.0).max(0.0);
    let usable_height = (board.height_mm - board.trim_mm * 2.0).max(0.0);
    let kerf = board.kerf_mm.max(0.0);
    let mut copies: Vec<(usize, f32, f32)> = Vec::new();
    for (index, part) in parts.iter().enumerate() {
        let count = part.quantity.clamp(1, 100);
        for _ in 0..count {
            copies.push((index, part.width_mm.max(1.0), part.height_mm.max(1.0)));
        }
    }
    // Tall parts first, so a band is not opened by a short piece and then
    // left with a gap no tall piece can use.
    copies.sort_by(|left, right| {
        right
            .2
            .max(right.1)
            .partial_cmp(&left.2.max(left.1))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut sheets = Vec::new();
    let mut too_big = Vec::new();
    let mut used_area = 0.0;
    for (part_index, width, height) in copies {
        let orientations = orientations(width, height, board.allow_rotate);
        if !orientations
            .iter()
            .any(|(w, h, _)| *w <= usable_width + 0.01 && *h <= usable_height + 0.01)
        {
            if !too_big.contains(&part_index) {
                too_big.push(part_index);
            }
            continue;
        }
        if !sheets.iter_mut().any(|sheet: &mut OpenSheet| {
            place_on_sheet(
                sheet,
                part_index,
                &orientations,
                usable_width,
                usable_height,
                kerf,
                board.trim_mm,
            )
        }) {
            let mut sheet = OpenSheet {
                shelves: Vec::new(),
                parts: Vec::new(),
            };
            let placed = place_on_sheet(
                &mut sheet,
                part_index,
                &orientations,
                usable_width,
                usable_height,
                kerf,
                board.trim_mm,
            );
            if placed {
                used_area += width * height;
                sheets.push(sheet);
            }
        } else {
            used_area += width * height;
        }
    }

    let sheet_area = board.width_mm.max(0.0) * board.height_mm.max(0.0) * sheets.len() as f32;
    Estimate {
        sheets: sheets
            .into_iter()
            .map(|sheet| PackedSheet { parts: sheet.parts })
            .collect(),
        too_big,
        used_area_mm2: used_area,
        sheet_area_mm2: sheet_area,
        stale: false,
    }
}

/// The ways a part may lie on the sheet: as drawn, and turned 90° when
/// rotation is allowed. Returned as (width, height, rotated).
fn orientations(width: f32, height: f32, allow_rotate: bool) -> Vec<(f32, f32, bool)> {
    if allow_rotate && (width - height).abs() > 0.1 {
        // Prefer the orientation that is shorter in height. A shorter band
        // leaves more of the sheet for the next row.
        let mut both = vec![(width, height, false), (height, width, true)];
        both.sort_by(|left, right| {
            left.1
                .partial_cmp(&right.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        both
    } else {
        vec![(width, height, false)]
    }
}

/// Try to place one part on a sheet: first in an existing band, then in a
/// new band underneath. Returns `false` if the sheet is full.
fn place_on_sheet(
    sheet: &mut OpenSheet,
    part_index: usize,
    orientations: &[(f32, f32, bool)],
    usable_width: f32,
    usable_height: f32,
    kerf: f32,
    trim: f32,
) -> bool {
    for shelf in &mut sheet.shelves {
        for &(width, height, rotated) in orientations {
            if height > shelf.height_mm + 0.01 || shelf.cursor_mm + width > usable_width + 0.01 {
                continue;
            }
            sheet.parts.push(PlacedPart {
                part_index,
                x_mm: trim + shelf.cursor_mm,
                y_mm: trim + shelf.y_mm,
                width_mm: width,
                height_mm: height,
                rotated,
            });
            shelf.cursor_mm += width + kerf;
            return true;
        }
    }

    // No open band can take it. Open a new band at the bottom of the used area.
    let used_height = sheet
        .shelves
        .last()
        .map(|shelf| shelf.y_mm + shelf.height_mm + kerf)
        .unwrap_or(0.0);
    for &(width, height, rotated) in orientations {
        if width > usable_width + 0.01 || used_height + height > usable_height + 0.01 {
            continue;
        }
        sheet.shelves.push(OpenShelf {
            y_mm: used_height,
            height_mm: height,
            cursor_mm: width + kerf,
        });
        sheet.parts.push(PlacedPart {
            part_index,
            x_mm: trim,
            y_mm: trim + used_height,
            width_mm: width,
            height_mm: height,
            rotated,
        });
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board() -> StockBoard {
        StockBoard {
            width_mm: 1000.0,
            height_mm: 500.0,
            thickness_mm: 18.0,
            kerf_mm: 0.0,
            trim_mm: 0.0,
            allow_rotate: true,
        }
    }

    fn part(width: f32, height: f32, quantity: u32) -> CutPart {
        CutPart {
            name: String::new(),
            role: super::super::job::PartRole::Custom,
            width_mm: width,
            height_mm: height,
            quantity,
        }
    }

    #[test]
    fn two_parts_share_one_sheet_side_by_side() {
        let result = estimate(&board(), &[part(400.0, 200.0, 2)]);
        assert_eq!(result.boards_to_order(), 1);
        assert_eq!(result.sheets[0].parts.len(), 2);
        assert!(result.too_big.is_empty());
    }

    #[test]
    fn a_part_taller_than_the_row_opens_the_next_sheet() {
        let result = estimate(&board(), &[part(600.0, 400.0, 2)]);
        assert_eq!(result.boards_to_order(), 2);
    }

    #[test]
    fn kerf_is_reserved_between_parts() {
        let mut sheet = board();
        sheet.kerf_mm = 20.0;
        // 490 + 20 kerf + 490 = 1000, so the second part still fits.
        let fitted = estimate(&sheet, &[part(490.0, 100.0, 2)]);
        assert_eq!(fitted.boards_to_order(), 1);
        // 500 + 20 kerf + 500 does not fit one row or a second row of height 100
        // on a 500 sheet... the second row does fit. Use full height instead.
        let split = estimate(&sheet, &[part(500.0, 500.0, 2)]);
        assert_eq!(split.boards_to_order(), 2);
    }

    #[test]
    fn a_part_larger_than_the_sheet_is_reported() {
        let result = estimate(&board(), &[part(1200.0, 800.0, 1), part(100.0, 100.0, 1)]);
        assert_eq!(result.too_big, vec![0]);
        assert_eq!(result.sheets[0].parts.len(), 1);
    }

    #[test]
    fn rotation_lets_a_long_part_use_the_long_edge() {
        let mut fixed = board();
        fixed.allow_rotate = false;
        let locked = estimate(&fixed, &[part(200.0, 800.0, 1)]);
        assert_eq!(locked.too_big, vec![0]);

        let turned = estimate(&board(), &[part(200.0, 800.0, 1)]);
        assert!(turned.too_big.is_empty());
        assert!(turned.sheets[0].parts[0].rotated);
    }
}
