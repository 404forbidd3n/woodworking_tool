//! The Properties panel and the right-click menu.
//!
//! The panel always edits the primary selection. From top to bottom it shows:
//! name, size, the options for that kind of piece (drawer count, door count,
//! pipe direction, ...), door face, placement and legs, position, facing,
//! materials, and Duplicate / Delete.

use bevy::prelude::*;
use bevy_egui::egui;

use super::materials::{appliance_finish_row, finish_group, finish_row};
use super::spawn::place_at_center;
use super::widgets::mm_row;
use super::PropertiesUi;
use crate::layout::model::{
    jolly_width_mm, m_to_mm, mm_to_m, quarter_turns, rotation_from_turns, CornerStyle, DoorStyle,
    Furniture, Kind, LegStyle, PipeColor, Placement, Room, Selection, JOLLY_MIN_MM,
    PIPE_PAIR_GAP_MM,
};
use crate::theme;

// ---------------------------------------------------------------------------
// Panel frame and right-click menu
// ---------------------------------------------------------------------------

/// The docked editor. It follows the selection, so clicking another piece in
/// the room switches what the panel edits without reopening it.
pub(super) fn draw_properties_panel(
    ui: &mut egui::Ui,
    properties: &mut PropertiesUi,
    commands: &mut Commands,
    pieces: &mut Query<(Entity, &mut Furniture, &mut Transform)>,
    selection: &mut Selection,
    room: &Room,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Properties")
                .strong()
                .size(16.0)
                .color(theme::heading()),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Close").clicked() {
                properties.open = false;
            }
        });
    });
    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt("properties_scroll")
        .show(ui, |ui| {
            let Some(target) = selection
                .primary
                .filter(|entity| pieces.get(*entity).is_ok())
            else {
                ui.label("Select a piece to edit it here.");
                return;
            };
            if selection.entities.len() > 1 {
                ui.weak(format!(
                    "{} pieces selected. Editing the last one clicked.",
                    selection.entities.len()
                ));
                ui.separator();
            }
            draw_properties(
                ui,
                commands,
                pieces,
                selection,
                target,
                &mut properties.materials_detail,
                room,
            );
        });
}

/// Small menu at the cursor after a right-click on a piece. A press anywhere
/// outside it closes it.
pub(super) fn draw_context_menu(ctx: &egui::Context, properties: &mut PropertiesUi) {
    if properties.context_target.is_none() {
        properties.menu_rect = None;
        return;
    }
    let shown = egui::Area::new("piece_context_menu".into())
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(
            properties.context_position.x,
            properties.context_position.y,
        ))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .show(ui, |ui| ui.button("Properties").clicked())
                .inner
        });
    let rect = shown.response.rect;
    let pressed_outside = ctx.input(|input| {
        input.pointer.any_pressed()
            && input
                .pointer
                .interact_pos()
                .is_some_and(|position| !rect.contains(position))
    });
    if shown.inner {
        properties.open = true;
    }
    if shown.inner || pressed_outside {
        properties.context_target = None;
        properties.menu_rect = None;
    } else {
        properties.menu_rect = Some(Rect::new(rect.min.x, rect.min.y, rect.max.x, rect.max.y));
    }
}

// ---------------------------------------------------------------------------
// The editor for one piece
// ---------------------------------------------------------------------------

/// Every control for one piece, top to bottom. Edits that change the shape
/// set `furniture.dirty`, which makes `view::sync_meshes` rebuild the mesh.
/// Position and facing go straight to the `Transform`.
fn draw_properties(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    pieces: &mut Query<(Entity, &mut Furniture, &mut Transform)>,
    selection: &mut Selection,
    entity: Entity,
    materials_detail: &mut bool,
    room: &Room,
) {
    let Ok((_, mut furniture, mut transform)) = pieces.get_mut(entity) else {
        return;
    };
    ui.strong(furniture.label());
    ui.horizontal(|ui| {
        ui.label("Name");
        ui.text_edit_singleline(&mut furniture.name);
    });

    if furniture.kind.is_pipe() {
        draw_pipe_size(ui, &mut furniture);
    } else {
        let mut width = furniture.width_mm;
        let mut height = furniture.height_mm;
        let mut depth = furniture.depth_mm;
        if mm_row(ui, "Width", &mut width, 50.0..=6000.0) {
            furniture.width_mm = width;
            furniture.dirty = true;
        }
        let min_height = furniture.kind.min_height_mm();
        if mm_row(ui, "Height", &mut height, min_height..=4000.0) {
            furniture.height_mm = height;
            furniture.dirty = true;
        }
        if mm_row(ui, "Depth", &mut depth, 50.0..=3000.0) {
            furniture.depth_mm = depth;
            furniture.dirty = true;
        }
    }

    kind_options(ui, &mut furniture);

    if furniture.kind.has_doors() {
        ui.horizontal(|ui| {
            ui.label("Door face");
            let solid = furniture.doors == DoorStyle::Solid;
            if ui.selectable_label(solid, "Solid").clicked() && !solid {
                furniture.doors = DoorStyle::Solid;
                furniture.dirty = true;
            }
            let glass = furniture.doors == DoorStyle::Glass;
            if ui.selectable_label(glass, "Glass").clicked() && !glass {
                furniture.doors = DoorStyle::Glass;
                furniture.dirty = true;
            }
        });
    }

    if furniture.kind.is_cabinet() {
        ui.horizontal(|ui| {
            ui.label("Placement");
            ui.strong(match furniture.placement {
                Placement::Floor => "Floor",
                Placement::Wall => "Wall mounted",
            });
        });
        if furniture.placement == Placement::Floor {
            ui.horizontal(|ui| {
                ui.label("Legs");
                egui::ComboBox::from_id_salt((entity, "legs"))
                    .selected_text(furniture.legs.label())
                    .show_ui(ui, |ui| {
                        for style in LegStyle::ALL {
                            if ui
                                .selectable_value(&mut furniture.legs, style, style.label())
                                .changed()
                            {
                                furniture.dirty = true;
                            }
                        }
                    });
            });
            ui.weak("Cabinet height is the carcass; legs add to installed height.");
        }
    }

    ui.add_space(4.0);
    let mut x_mm = m_to_mm(transform.translation.x);
    let mut y_mm = m_to_mm(transform.translation.y);
    let mut z_mm = m_to_mm(transform.translation.z);
    if mm_row(ui, "X right", &mut x_mm, -2000.0..=30000.0) {
        transform.translation.x = mm_to_m(x_mm);
    }
    if mm_row(ui, "Y up", &mut y_mm, 0.0..=6000.0) {
        transform.translation.y = mm_to_m(y_mm);
    }
    if mm_row(ui, "Z into room", &mut z_mm, -2000.0..=30000.0) {
        transform.translation.z = mm_to_m(z_mm);
    }

    ui.label("Facing. 0° looks into the room.");
    let turns = quarter_turns(transform.rotation);
    ui.horizontal(|ui| {
        for turn in 0..4u8 {
            // `turn` is a u8 so it can be compared with the stored facing.
            // Multiply in a wider type: 3 * 90 does not fit in a u8.
            let degrees = i32::from(turn) * 90;
            if ui
                .selectable_label(turns == turn, format!("{degrees}°"))
                .clicked()
            {
                transform.rotation = rotation_from_turns(turn);
            }
        }
    });

    if !furniture.kind.is_pipe() {
        materials_section(ui, &mut furniture, materials_detail);
    }

    let mut duplicate = None;
    let mut delete = false;
    ui.horizontal(|ui| {
        if ui.button("Duplicate").clicked() {
            let mut copy = furniture.clone();
            copy.dirty = true;
            let placed = place_at_center(
                room,
                m_to_mm(transform.translation.y),
                quarter_turns(transform.rotation),
            );
            duplicate = Some((copy, placed));
        }
        if ui.button("Delete").clicked() {
            delete = true;
        }
    });
    // The mutable piece borrow has to end before spawn or despawn.
    drop(furniture);
    drop(transform);
    if let Some((copy, placed)) = duplicate {
        selection.set_only(commands.spawn((copy, placed)).id());
    }
    if delete {
        commands.entity(entity).despawn();
        selection.entities.retain(|selected| *selected != entity);
        selection.primary = selection.entities.last().copied();
    }
}

// ---------------------------------------------------------------------------
// Sections of the editor
// ---------------------------------------------------------------------------

/// Pipes are sized by length and diameter instead of width/height/depth.
/// Which axis is the "length" depends on whether the pipe is horizontal.
fn draw_pipe_size(ui: &mut egui::Ui, furniture: &mut Furniture) {
    let Kind::Pipe {
        horizontal, count, ..
    } = furniture.kind
    else {
        return;
    };
    let mut length = if horizontal {
        furniture.width_mm
    } else {
        furniture.height_mm
    };
    let mut diameter = furniture.depth_mm;
    if mm_row(ui, "Length", &mut length, 100.0..=10000.0) {
        if horizontal {
            furniture.width_mm = length;
        } else {
            furniture.height_mm = length;
        }
        furniture.dirty = true;
    }
    if mm_row(ui, "Diameter", &mut diameter, 8.0..=200.0) {
        furniture.depth_mm = diameter;
        if count < 2 {
            if horizontal {
                furniture.height_mm = diameter;
            } else {
                furniture.width_mm = diameter;
            }
        } else if horizontal {
            furniture.height_mm = furniture.height_mm.max(diameter + PIPE_PAIR_GAP_MM);
        } else {
            furniture.width_mm = furniture.width_mm.max(diameter + PIPE_PAIR_GAP_MM);
        }
        furniture.dirty = true;
    }
}

/// Options that only make sense for one kind of piece: drawer count, door
/// count, shelf count, pipe direction and color, or a short description.
fn kind_options(ui: &mut egui::Ui, furniture: &mut Furniture) {
    match furniture.kind {
        Kind::Drawers { count } => {
            let mut drawers = count as i32;
            if ui
                .add(egui::Slider::new(&mut drawers, 1..=10).text("Drawers"))
                .changed()
            {
                furniture.kind = Kind::Drawers {
                    count: drawers as u8,
                };
                furniture.ensure_front_finishes();
                furniture.dirty = true;
            }
            ui.label("Drawers split the height into equal fronts.");
        }
        Kind::JollyDrawers { drawers, jolly_mm } => {
            let mut count = drawers as i32;
            if ui
                .add(egui::Slider::new(&mut count, 1..=10).text("Drawers"))
                .changed()
            {
                furniture.kind = Kind::JollyDrawers {
                    drawers: count as u8,
                    jolly_mm,
                };
                furniture.ensure_front_finishes();
                furniture.dirty = true;
            }
            let max = (furniture.width_mm - JOLLY_MIN_MM).max(JOLLY_MIN_MM);
            let mut jolly = jolly_width_mm(furniture.width_mm, jolly_mm);
            if ui
                .add(
                    egui::Slider::new(&mut jolly, JOLLY_MIN_MM..=max)
                        .step_by(1.0)
                        .suffix(" mm")
                        .text("Jolly width"),
                )
                .changed()
            {
                furniture.kind = Kind::JollyDrawers {
                    drawers,
                    jolly_mm: jolly.round() as u16,
                };
                furniture.dirty = true;
            }
            ui.label("A full-height pull-out on the left, drawers stacked on the right.");
        }
        Kind::HingedDoors { doors } => {
            ui.horizontal(|ui| {
                if ui.selectable_label(doors == 1, "1 door").clicked() && doors != 1 {
                    furniture.kind = Kind::HingedDoors { doors: 1 };
                    furniture.ensure_front_finishes();
                    furniture.dirty = true;
                }
                if ui.selectable_label(doors != 1, "2 doors").clicked() && doors == 1 {
                    furniture.kind = Kind::HingedDoors { doors: 2 };
                    furniture.ensure_front_finishes();
                    furniture.dirty = true;
                }
            });
        }
        Kind::LiftDoor => {
            ui.label("Hinge is on the top edge. Drawn open so it reads as a lift-up.");
        }
        Kind::SlidingDoors => {
            ui.label("Two leaves, one in front of the other.");
        }
        Kind::OpenShelves { shelves } => {
            let mut count = shelves as i32;
            if ui
                .add(egui::Slider::new(&mut count, 1..=10).text("Shelves"))
                .changed()
            {
                furniture.kind = Kind::OpenShelves {
                    shelves: count as u8,
                };
                furniture.dirty = true;
            }
            ui.label("Open-front cabinet with evenly spaced shelves.");
        }
        Kind::Table => {
            ui.label("Top and four legs. Height is the top surface.");
        }
        Kind::Countertop => {
            ui.label("Standalone thick worktop. Height is its thickness.");
        }
        Kind::Tabletop => {
            ui.label("Standalone tabletop. Height is its thickness.");
        }
        Kind::Refrigerator
        | Kind::Dishwasher
        | Kind::ElectricOven
        | Kind::CoffeeMachine
        | Kind::Microwave
        | Kind::CentralHeatingBoiler
        | Kind::AirConditioner => {
            ui.label("Stylized appliance model with editable overall dimensions.");
        }
        Kind::InductionHob => {
            ui.label("Black glass plate with four cooking zones. Height is the glass thickness.");
        }
        Kind::SquareSink => {
            ui.label("Open rectangular basin for washing dishes. The rim sits at worktop height.");
        }
        Kind::RoundSink => {
            ui.label("Open round basin for washing dishes. The rim sits at worktop height.");
        }
        Kind::DoubleSink => {
            ui.label("Large open sink with two rectangular basins.");
        }
        Kind::SinkFaucet => {
            ui.label("Movable water pipe / faucet for a kitchen sink.");
        }
        Kind::Pipe {
            horizontal,
            count,
            color,
        } => {
            let mut next = (horizontal, count, color);
            ui.horizontal(|ui| {
                if ui.selectable_label(!horizontal, "Vertical").clicked() {
                    next.0 = false;
                }
                if ui.selectable_label(horizontal, "Horizontal").clicked() {
                    next.0 = true;
                }
            });
            ui.horizontal(|ui| {
                if ui.selectable_label(count < 2, "1 pipe").clicked() {
                    next.1 = 1;
                }
                if ui.selectable_label(count >= 2, "2 pipes").clicked() {
                    next.1 = 2;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Color");
                for option in PipeColor::ALL {
                    if ui
                        .selectable_label(color == option, option.label())
                        .clicked()
                    {
                        next.2 = option;
                    }
                }
            });
            if next != (horizontal, count, color) {
                if next.0 != horizontal {
                    std::mem::swap(&mut furniture.width_mm, &mut furniture.height_mm);
                }
                if next.1 != count {
                    let span = if next.1 >= 2 {
                        furniture.depth_mm + PIPE_PAIR_GAP_MM
                    } else {
                        furniture.depth_mm
                    };
                    if next.0 {
                        furniture.height_mm = span;
                    } else {
                        furniture.width_mm = span;
                    }
                }
                furniture.kind = Kind::Pipe {
                    horizontal: next.0,
                    count: next.1,
                    color: next.2,
                };
                if furniture.name.ends_with("Pipe") {
                    furniture.name = furniture.label().to_owned();
                }
                furniture.dirty = true;
            }
        }
        Kind::Corner { style } => {
            ui.label(match style {
                CornerStyle::ShelfDoors => {
                    "L-shaped corner cabinet. Shelves sit in the corner, with a small door on each wall."
                }
                CornerStyle::DoubleDoor => {
                    "L-shaped corner cabinet. Two hinged doors close the opening."
                }
            });
            ui.label(
                "Width runs along the back wall and depth along the left wall. Each arm is 350 mm deep, so the corner itself is deeper on the diagonal.",
            );
        }
        Kind::Custom => {
            ui.label(
                "Custom block. Width, height, and depth scale every board together. Each board keeps its own size.",
            );
        }
    }
}

/// The "Materials & colors" section. Cabinets can switch between one color
/// for the body and one for the fronts, or one color per board.
fn materials_section(ui: &mut egui::Ui, furniture: &mut Furniture, materials_detail: &mut bool) {
    egui::CollapsingHeader::new(
        egui::RichText::new("Materials & colors")
            .strong()
            .size(15.0)
            .color(theme::heading()),
    )
    .default_open(true)
    .show(ui, |ui| {
        if furniture.kind == Kind::Custom {
            ui.label(
                egui::RichText::new(
                    "Body colours the carcass. Fronts colour doors and drawers. Legs keep their own material.",
                )
                    .size(12.0)
                    .color(theme::muted()),
            );
            if finish_group(ui, "Body", &mut furniture.finishes.carcass)
                | finish_group(ui, "Fronts", &mut furniture.finishes.fronts)
            {
                furniture.dirty = true;
            }
        } else if furniture.kind.is_cabinet() {
            ui.horizontal(|ui| {
                ui.selectable_value(materials_detail, false, "Body & fronts");
                ui.selectable_value(materials_detail, true, "Each board");
            });
            if *materials_detail {
                ui.label(
                    egui::RichText::new("One colour for each board.")
                        .size(12.0)
                        .color(theme::muted()),
                );
                let wall_names = ["Left side", "Right side", "Bottom", "Top", "Back"];
                for (index, label) in wall_names.into_iter().enumerate() {
                    if finish_row(ui, label, &mut furniture.finishes.carcass[index]) {
                        furniture.dirty = true;
                    }
                }
                let front_word = if matches!(furniture.kind, Kind::Drawers { .. }) {
                    "Drawer"
                } else {
                    "Door"
                };
                let jolly = matches!(furniture.kind, Kind::JollyDrawers { .. });
                for index in 0..furniture.finishes.fronts.len() {
                    let label = match (jolly, index) {
                        (true, 0) => "Jolly".to_owned(),
                        (true, _) => format!("Drawer {index}"),
                        _ => format!("{front_word} {}", index + 1),
                    };
                    if finish_row(
                        ui,
                        &label,
                        &mut furniture.finishes.fronts[index],
                    ) {
                        furniture.dirty = true;
                    }
                }
            } else {
                ui.label(
                    egui::RichText::new(
                        "Body is the carcass. Fronts are the doors and drawers.",
                    )
                    .size(12.0)
                    .color(theme::muted()),
                );
                if finish_group(ui, "Body", &mut furniture.finishes.carcass)
                    | finish_group(ui, "Fronts", &mut furniture.finishes.fronts)
                {
                    furniture.dirty = true;
                }
            }
            if finish_row(ui, "Handles", &mut furniture.finishes.handles) {
                furniture.dirty = true;
            }
        }
        if matches!(furniture.kind, Kind::Table)
            && finish_row(ui, "Frame / legs", &mut furniture.finishes.carcass[0])
        {
            furniture.dirty = true;
        }
        if matches!(
            furniture.kind,
            Kind::Table | Kind::Countertop | Kind::Tabletop
        ) && finish_row(ui, "Top", &mut furniture.finishes.top)
        {
            furniture.dirty = true;
        }
        if furniture.kind.is_appliance() {
            if appliance_finish_row(ui, &mut furniture.finishes.appliance) {
                furniture.dirty = true;
            }
        }
    });
}
