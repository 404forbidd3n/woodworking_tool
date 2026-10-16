//! Turn the cabinets already in the room into rectangles to cut.
//!
//! The sizes follow the same 18 mm panels the 3D view draws. This is a
//! starting list, not a joinery drawing: no grooves, no door gaps beyond the
//! small gap already used on the fronts, and no hardware.

use crate::layout::model::{
    equal_spans, m_to_mm, mm_to_m, Furniture, Kind, GAP_METERS, PANEL_METERS,
};

use super::job::{CutPart, PartRole};

/// Panels for one piece. Appliances are skipped: they are bought, not cut
/// from the sheet.
pub fn parts_from_furniture(furniture: &Furniture) -> Vec<CutPart> {
    if furniture.kind.is_appliance() {
        return Vec::new();
    }
    let name = furniture.name.trim();
    let name = if name.is_empty() {
        furniture.label()
    } else {
        name
    };
    let width = furniture.width_mm.max(1.0);
    let height = furniture.height_mm.max(1.0);
    let depth = furniture.depth_mm.max(1.0);
    let panel = m_to_mm(PANEL_METERS);

    match furniture.kind {
        Kind::Countertop | Kind::Tabletop => vec![named(name, PartRole::Top, width, depth, 1)],
        Kind::Table => {
            let mut parts = vec![named(name, PartRole::Top, width, depth, 1)];
            let leg = (height - 30.0).max(40.0);
            parts.push(named(name, PartRole::Custom, 50.0, leg, 4));
            parts.last_mut().unwrap().name = format!("{name} leg");
            parts
        }
        Kind::OpenShelves { shelves } => {
            let mut parts = carcass(name, width, height, depth, panel);
            parts.push(named(
                name,
                PartRole::Shelf,
                (width - panel * 2.0).max(1.0),
                depth,
                shelves.clamp(1, 10) as u32,
            ));
            parts
        }
        Kind::Drawers { count } => {
            cabinet(name, width, height, depth, panel, furniture.kind, count)
        }
        Kind::HingedDoors { doors } => {
            cabinet(name, width, height, depth, panel, furniture.kind, doors)
        }
        Kind::LiftDoor | Kind::SlidingDoors => {
            cabinet(name, width, height, depth, panel, furniture.kind, 1)
        }
        Kind::Refrigerator
        | Kind::Dishwasher
        | Kind::ElectricOven
        | Kind::CoffeeMachine
        | Kind::Microwave => Vec::new(),
    }
}

fn cabinet(
    name: &str,
    width: f32,
    height: f32,
    depth: f32,
    panel: f32,
    kind: Kind,
    count: u8,
) -> Vec<CutPart> {
    let mut parts = carcass(name, width, height, depth, panel);
    parts.extend(fronts(kind, name, width, height, count));
    parts
}

fn carcass(name: &str, width: f32, height: f32, depth: f32, panel: f32) -> Vec<CutPart> {
    let inner_width = (width - panel * 2.0).max(1.0);
    let inner_height = (height - panel * 2.0).max(1.0);
    vec![
        named(name, PartRole::Side, depth, height, 2),
        named(name, PartRole::Top, inner_width, depth, 1),
        named(name, PartRole::Bottom, inner_width, depth, 1),
        named(name, PartRole::Back, inner_width, inner_height, 1),
    ]
}

fn fronts(kind: Kind, name: &str, width: f32, height: f32, count: u8) -> Vec<CutPart> {
    match kind {
        Kind::Drawers { .. } => bands(height, count.max(1))
            .into_iter()
            .map(|(index, band)| CutPart {
                name: format!("{name} drawer {index}"),
                role: PartRole::DrawerFront,
                width_mm: (width - 6.0).max(1.0),
                height_mm: band,
                quantity: 1,
            })
            .collect(),
        Kind::HingedDoors { .. } => {
            let doors = count.clamp(1, 2);
            bands(width, doors)
                .into_iter()
                .map(|(index, band)| CutPart {
                    name: format!("{name} door {index}"),
                    role: PartRole::Door,
                    width_mm: band,
                    height_mm: (height - 6.0).max(1.0),
                    quantity: 1,
                })
                .collect()
        }
        Kind::LiftDoor => vec![named(
            name,
            PartRole::Door,
            (width - 6.0).max(1.0),
            (height - 6.0).max(1.0),
            1,
        )],
        Kind::SlidingDoors => vec![named(
            name,
            PartRole::Door,
            width * 0.55,
            (height - 6.0).max(1.0),
            2,
        )],
        _ => Vec::new(),
    }
}

/// Equal bands along `length`, in millimeters, numbered from 1.
fn bands(length: f32, count: u8) -> Vec<(usize, f32)> {
    equal_spans(mm_to_m(length), count, GAP_METERS)
        .into_iter()
        .enumerate()
        .map(|(index, (_, band))| (index + 1, m_to_mm(band).max(1.0)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::model::Kind;

    #[test]
    fn drawer_cabinet_lists_carcass_and_fronts() {
        let cabinet = Furniture::new(Kind::Drawers { count: 3 }, 600.0, 720.0, 560.0);
        let parts = parts_from_furniture(&cabinet);
        let roles: Vec<_> = parts.iter().map(|part| part.role).collect();
        assert_eq!(
            roles,
            vec![
                PartRole::Side,
                PartRole::Top,
                PartRole::Bottom,
                PartRole::Back,
                PartRole::DrawerFront,
                PartRole::DrawerFront,
                PartRole::DrawerFront,
            ]
        );
        assert_eq!(parts[0].quantity, 2);
        assert!(parts
            .iter()
            .all(|part| part.width_mm > 0.0 && part.height_mm > 0.0));
    }

    #[test]
    fn appliances_are_not_cut_from_the_sheet() {
        let fridge = Furniture::new(Kind::Refrigerator, 600.0, 1800.0, 600.0);
        assert!(parts_from_furniture(&fridge).is_empty());
    }

    #[test]
    fn open_cabinet_exports_its_shelves() {
        let cabinet = Furniture::new(Kind::OpenShelves { shelves: 4 }, 600.0, 720.0, 560.0);
        let parts = parts_from_furniture(&cabinet);
        let shelf = parts
            .iter()
            .find(|part| part.role == PartRole::Shelf)
            .unwrap();
        assert_eq!(shelf.quantity, 4);
        assert_eq!(shelf.width_mm, 564.0);
    }
}

fn named(owner: &str, role: PartRole, width: f32, height: f32, quantity: u32) -> CutPart {
    let name = if role == PartRole::Custom && owner.ends_with("leg") {
        owner.to_owned()
    } else {
        format!("{owner} {}", role.label().to_lowercase())
    };
    CutPart {
        name,
        role,
        width_mm: width,
        height_mm: height,
        quantity,
    }
}
