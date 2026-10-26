//! The main egui pass: the toolbar on top, the sidebar on the left, and the
//! docked Properties panel next to it.
//!
//! [`sidebar`] is the Bevy system. It reads the undo/redo/save shortcuts,
//! lays out the panels, and then records how much of the window they cover
//! in [`SidebarWidth`] and [`ToolbarHeight`], so the 3D view knows which
//! clicks are its own.
//!
//! The sidebar is a column of collapsible sections. Each section has its own
//! function below, in the same order they appear on screen:
//!
//! 1. [`view_section`] — camera presets and the snap grid
//! 2. [`room_section`] — room size, walls, doors and windows, utility pipes
//! 3. [`furniture_section`] — floor and wall cabinets
//! 4. [`customs_section`] — blocks saved from the component planner
//! 5. [`surfaces_section`] — worktops and tabletops
//! 6. [`appliances_section`] — fridge, oven, sinks, ...
//! 7. [`in_room_section`] — a list of every piece in the room
//! 8. [`selected_section`] — what is selected, and a button to edit it

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use super::files::{perform_history, save_layout};
use super::library::library_button;
use super::properties::{draw_context_menu, draw_properties_panel};
use super::spawn::{add_custom, add_piece, despawn_all};
use super::toolbar::draw_toolbar;
use super::widgets::mm_row;
use super::{PropertiesUi, SidebarWidth, ToolbarHeight};
use crate::builder::{ComponentWindow, CustomLibrary};
use crate::cutting::{CutJob, CutWindow};
use crate::layout::history::History;
use crate::layout::model::{
    example_layout, sanitize_openings, CornerStyle, Editor, Furniture, Kind, Opening, OpeningKind,
    PipeColor, Placement, Room, Selection, WallSide,
};
use crate::layout::project::NativeDialog;
use crate::layout::view::OrbitCamera;
use crate::theme::{self, dropdown, toolbar_frame};

/// The query every sidebar section reads pieces through.
type Pieces<'w, 's> = Query<'w, 's, (Entity, &'static mut Furniture, &'static mut Transform)>;

// ---------------------------------------------------------------------------
// The system
// ---------------------------------------------------------------------------

/// The egui system for the main window. See the top of this file.
pub fn sidebar(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut pieces: Pieces,
    mut selection: ResMut<Selection>,
    mut room: ResMut<Room>,
    mut editor: ResMut<Editor>,
    mut properties: ResMut<PropertiesUi>,
    mut camera: Query<&mut OrbitCamera>,
    mut sidebar_width: ResMut<SidebarWidth>,
    mut toolbar_height: ResMut<ToolbarHeight>,
    mut cut_job: ResMut<CutJob>,
    mut cut_window: ResMut<CutWindow>,
    mut component_window: ResMut<ComponentWindow>,
    library: Res<CustomLibrary>,
    mut history: ResMut<History>,
    mut dialogs: NonSendMut<NativeDialog>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    theme::apply_theme(ctx);

    // --- Keyboard shortcuts egui would otherwise swallow -------------------

    // Cmd+S is read here so it still saves when Project is collapsed.
    let save_shortcut =
        ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::S));
    let redo_modifiers = egui::Modifiers {
        command: true,
        shift: true,
        ..default()
    };
    let redo_shortcut = ctx.input_mut(|input| {
        input.consume_key(redo_modifiers, egui::Key::Z)
            || input.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)
    });
    let undo_shortcut = !redo_shortcut
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::Z));
    if save_shortcut {
        save_layout(&mut editor, &room, &pieces, &cut_job);
    }
    if undo_shortcut {
        perform_history(
            false,
            &mut history,
            &mut commands,
            &mut room,
            &mut editor,
            &mut selection,
            &pieces,
        );
    } else if redo_shortcut {
        perform_history(
            true,
            &mut history,
            &mut commands,
            &mut room,
            &mut editor,
            &mut selection,
            &pieces,
        );
    }

    // --- Toolbar across the top --------------------------------------------

    // egui 0.36 draws panels into a root Ui instead of straight onto the context.
    // The toolbar is added first, so it spans the full width above the sidebar.
    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    let toolbar = egui::Panel::top("toolbar")
        .frame(toolbar_frame())
        .show(&mut viewport_ui, |ui| {
            draw_toolbar(
                ui,
                &mut commands,
                &mut room,
                &mut editor,
                &mut selection,
                &pieces,
                &mut cut_job,
                &mut cut_window,
                &mut component_window,
                &mut history,
                &mut dialogs,
            );
        });
    toolbar_height.0 = toolbar.response.rect.height();

    // --- Sidebar on the left -----------------------------------------------

    let panel = egui::Panel::left("tools")
        .resizable(true)
        .default_size(300.0)
        .frame(theme::sidebar_frame())
        .show(&mut viewport_ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                view_section(ui, &mut camera, &mut editor, &room, &selection, &pieces);
                room_section(ui, &mut commands, &mut room, &mut selection);
                furniture_section(ui, &mut commands, &room, &mut selection);
                customs_section(ui, &mut commands, &room, &mut selection, &library);
                surfaces_section(ui, &mut commands, &room, &mut selection);
                appliances_section(ui, &mut commands, &room, &mut selection);
                in_room_section(
                    ui,
                    &mut commands,
                    &room,
                    &mut editor,
                    &mut selection,
                    &mut camera,
                    &pieces,
                );
                selected_section(ui, &selection, &pieces, &mut properties);
            });
        });

    // --- Properties panel, docked to the right of the sidebar --------------

    // Properties docks beside the sidebar, so the 3D view treats both as one strip.
    let mut chrome_right = panel.response.rect.right();
    if properties.open {
        let docked = egui::Panel::left("properties")
            .resizable(true)
            .default_size(680.0)
            .frame(theme::sidebar_frame())
            .show(&mut viewport_ui, |ui| {
                draw_properties_panel(
                    ui,
                    &mut properties,
                    &mut commands,
                    &mut pieces,
                    &mut selection,
                    &room,
                );
            });
        chrome_right = docked.response.rect.right();
    }
    sidebar_width.0 = chrome_right;
    draw_context_menu(ctx, &mut properties);
}

// ---------------------------------------------------------------------------
// Sidebar sections, top to bottom
// ---------------------------------------------------------------------------

/// Camera presets ("Corner", "Front", "Top", framing) and the snap grid.
fn view_section(
    ui: &mut egui::Ui,
    camera: &mut Query<&mut OrbitCamera>,
    editor: &mut Editor,
    room: &Room,
    selection: &Selection,
    pieces: &Pieces,
) {
    dropdown(ui, "View", false, |ui| {
        ui.horizontal(|ui| {
            if ui.button("Corner").clicked() {
                if let Ok(mut orbit) = camera.single_mut() {
                    orbit.corner_view();
                }
            }
            if ui.button("Front").clicked() {
                if let Ok(mut orbit) = camera.single_mut() {
                    orbit.front_view();
                }
            }
            if ui.button("Top").clicked() {
                if let Ok(mut orbit) = camera.single_mut() {
                    orbit.top_view();
                }
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Frame room").clicked() {
                if let Ok(mut orbit) = camera.single_mut() {
                    orbit.frame_room(room);
                }
            }
            if ui.button("Frame piece").clicked() {
                if let Some(entity) = selection.primary {
                    if let Ok((_, furniture, transform)) = pieces.get(entity) {
                        let base = transform.translation;
                        let size = furniture.size_meters();
                        if let Ok(mut orbit) = camera.single_mut() {
                            orbit.frame_piece(base, size);
                        }
                    }
                }
            }
        });
        ui.separator();
        ui.checkbox(&mut editor.snap_enabled, "Snap moves to a grid");
        mm_row(ui, "Step", &mut editor.snap_mm, 1.0..=500.0);
    });
}

/// Wall visibility, and (behind the "Enable room editing" lock) the room
/// size, doors and windows, and utility pipes.
fn room_section(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    room: &mut Room,
    selection: &mut Selection,
) {
    dropdown(ui, "Room", true, |ui| {
        // Which walls are drawn. Always available.
        let mut show_walls = room.show_walls;
        if ui.checkbox(&mut show_walls, "Show walls").changed() {
            room.show_walls = show_walls;
            room.dirty = true;
        }
        ui.checkbox(&mut room.edit_enabled, "Enable room editing");
        ui.checkbox(&mut room.show_measurements, "Show room measurements");
        ui.add_enabled_ui(room.show_walls, |ui| {
            for side in WallSide::ALL {
                let mut visible = room.walls.visible(side);
                if ui.checkbox(&mut visible, side.label()).changed() {
                    room.walls.set(side, visible);
                    room.dirty = true;
                }
            }
        });
        ui.weak("Hide a wall to look through that side.");

        // Everything below changes the room itself, so it is locked by default.
        ui.add_enabled_ui(room.edit_enabled, |ui| {
            let mut width = room.width_mm;
            let mut depth = room.depth_mm;
            let mut height = room.wall_height_mm;
            if mm_row(ui, "Width", &mut width, 500.0..=30000.0)
                | mm_row(ui, "Depth", &mut depth, 500.0..=30000.0)
                | mm_row(ui, "Wall height", &mut height, 500.0..=6000.0)
            {
                room.width_mm = width;
                room.depth_mm = depth;
                room.wall_height_mm = height;
                room.dirty = true;
            }

            // Doors and windows: one framed group per opening.
            ui.separator();
            ui.strong("Doors & windows");
            let mut remove = None;
            let mut openings_changed = false;
            for (index, opening) in room.openings.iter_mut().enumerate() {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt(("opening_kind", index))
                            .selected_text(match opening.kind {
                                OpeningKind::Door => "Door",
                                OpeningKind::Window => "Window",
                            })
                            .show_ui(ui, |ui| {
                                openings_changed |= ui
                                    .selectable_value(&mut opening.kind, OpeningKind::Door, "Door")
                                    .changed();
                                openings_changed |= ui
                                    .selectable_value(
                                        &mut opening.kind,
                                        OpeningKind::Window,
                                        "Window",
                                    )
                                    .changed();
                            });
                        egui::ComboBox::from_id_salt(("opening_wall", index))
                            .selected_text(opening.wall.label())
                            .show_ui(ui, |ui| {
                                for wall in WallSide::ALL {
                                    openings_changed |= ui
                                        .selectable_value(&mut opening.wall, wall, wall.label())
                                        .changed();
                                }
                            });
                        if ui.button("Remove").clicked() {
                            remove = Some(index);
                        }
                    });
                    openings_changed |=
                        mm_row(ui, "Position", &mut opening.offset_mm, 0.0..=30000.0);
                    openings_changed |= mm_row(ui, "Width", &mut opening.width_mm, 100.0..=10000.0);
                    openings_changed |=
                        mm_row(ui, "Height", &mut opening.height_mm, 100.0..=6000.0);
                    if opening.kind == OpeningKind::Window {
                        openings_changed |=
                            mm_row(ui, "Sill height", &mut opening.sill_mm, 0.0..=5000.0);
                    } else {
                        opening.sill_mm = 0.0;
                    }
                });
            }
            room.dirty |= openings_changed;
            if let Some(index) = remove {
                room.openings.remove(index);
                room.dirty = true;
            }
            ui.horizontal(|ui| {
                if ui.button("Add door").clicked() {
                    room.openings.push(Opening::door());
                    room.dirty = true;
                }
                if ui.button("Add window").clicked() {
                    room.openings.push(Opening::window());
                    room.dirty = true;
                }
            });
            // Keep every opening inside its wall after any edit above.
            if room.dirty {
                sanitize_openings(
                    room.width_mm,
                    room.depth_mm,
                    room.wall_height_mm,
                    &mut room.openings,
                );
            }

            // Pipes are pieces like any other, but they belong to the room.
            ui.separator();
            ui.strong("Utility pipes");
            ui.weak("Drag pipes like cabinets while room editing is on.");
            ui.horizontal(|ui| {
                for (label, horizontal) in
                    [("Add vertical pipe", false), ("Add horizontal pipe", true)]
                {
                    if ui.button(label).clicked() {
                        let kind = Kind::Pipe {
                            horizontal,
                            count: 1,
                            color: PipeColor::Gray,
                        };
                        let entity = add_piece(commands, room, kind, Placement::Floor);
                        selection.set_only(entity);
                    }
                }
            });
        });
    });
}

/// Floor and wall-mounted cabinets. Each button adds one piece in the middle
/// of the room and selects it.
fn furniture_section(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    room: &Room,
    selection: &mut Selection,
) {
    dropdown(ui, "Add Furniture", false, |ui| {
        ui.strong("Floor");
        for (label, kind) in [
            ("Drawer Base Cabinet", Kind::Drawers { count: 3 }),
            ("Door Base Cabinet", Kind::HingedDoors { doors: 2 }),
            ("Sliding-Door Cabinet", Kind::SlidingDoors),
            ("Open Shelf Cabinet", Kind::OpenShelves { shelves: 3 }),
            (
                "Corner Base, Shelves",
                Kind::Corner {
                    style: CornerStyle::ShelfDoors,
                },
            ),
            (
                "Corner Base, Double Door",
                Kind::Corner {
                    style: CornerStyle::DoubleDoor,
                },
            ),
            ("Work Table", Kind::Table),
        ] {
            if library_button(ui, label, kind).clicked() {
                let entity = add_piece(commands, room, kind, Placement::Floor);
                selection.set_only(entity);
            }
        }
        ui.add_space(6.0);
        ui.strong("Wall mounted");
        for (label, kind) in [
            ("Wall Door Cabinet", Kind::HingedDoors { doors: 2 }),
            ("Wall Lift-Up Cabinet", Kind::LiftDoor),
            ("Wall Drawer Cabinet", Kind::Drawers { count: 3 }),
            (
                "Corner Wall, Shelves",
                Kind::Corner {
                    style: CornerStyle::ShelfDoors,
                },
            ),
            (
                "Corner Wall, Double Door",
                Kind::Corner {
                    style: CornerStyle::DoubleDoor,
                },
            ),
        ] {
            if library_button(ui, label, kind).clicked() {
                let entity = add_piece(commands, room, kind, Placement::Wall);
                selection.set_only(entity);
            }
        }
    });
}

/// Blocks saved in the component planner.
fn customs_section(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    room: &Room,
    selection: &mut Selection,
    library: &CustomLibrary,
) {
    dropdown(ui, "Customs", false, |ui| {
        if library.types.is_empty() {
            ui.label("Save a block in the Component planner and it will be listed here.");
        } else {
            for block in &library.types {
                if library_button(ui, block.name.as_str(), Kind::Custom).clicked() {
                    let entity = add_custom(commands, room, block);
                    selection.set_only(entity);
                }
            }
        }
    });
}

/// Loose worktops and tabletops.
fn surfaces_section(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    room: &Room,
    selection: &mut Selection,
) {
    dropdown(ui, "Add Surfaces", false, |ui| {
        for (label, kind) in [
            ("Kitchen Worktop", Kind::Countertop),
            ("Tabletop", Kind::Tabletop),
        ] {
            if library_button(ui, label, kind).clicked() {
                let entity = add_piece(commands, room, kind, Placement::Floor);
                selection.set_only(entity);
            }
        }
    });
}

/// Appliances. Most stand on the floor; the boiler and air conditioner hang
/// on the wall.
fn appliances_section(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    room: &Room,
    selection: &mut Selection,
) {
    dropdown(ui, "Add Appliances", false, |ui| {
        for (label, kind, placement) in [
            ("Refrigerator", Kind::Refrigerator, Placement::Floor),
            ("Dishwasher", Kind::Dishwasher, Placement::Floor),
            ("Electric Oven", Kind::ElectricOven, Placement::Floor),
            ("Coffee Machine", Kind::CoffeeMachine, Placement::Floor),
            ("Microwave", Kind::Microwave, Placement::Floor),
            ("Square Kitchen Sink", Kind::SquareSink, Placement::Floor),
            ("Round Kitchen Sink", Kind::RoundSink, Placement::Floor),
            (
                "Double-Bowl Kitchen Sink",
                Kind::DoubleSink,
                Placement::Floor,
            ),
            (
                "Kitchen Faucet / Water Pipe",
                Kind::SinkFaucet,
                Placement::Floor,
            ),
            (
                "Central Heating Boiler",
                Kind::CentralHeatingBoiler,
                Placement::Wall,
            ),
            ("Air Conditioner", Kind::AirConditioner, Placement::Wall),
        ] {
            if library_button(ui, label, kind).clicked() {
                let entity = add_piece(commands, room, kind, placement);
                selection.set_only(entity);
            }
        }
    });
}

/// A clickable list of every piece in the room, plus "Example kitchen" and
/// "Clear". Pipes are listed but only selectable while room editing is on.
fn in_room_section(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    room: &Room,
    editor: &mut Editor,
    selection: &mut Selection,
    camera: &mut Query<&mut OrbitCamera>,
    pieces: &Pieces,
) {
    dropdown(ui, "In the room", false, |ui| {
        // (entity, row label, is it a pipe)
        let rows: Vec<(Entity, String, bool)> = pieces
            .iter()
            .map(|(entity, furniture, _)| {
                (
                    entity,
                    format!(
                        "{}   {:.0} × {:.0} × {:.0}",
                        furniture.label(),
                        furniture.width_mm,
                        furniture.height_mm,
                        furniture.depth_mm
                    ),
                    furniture.kind.is_pipe(),
                )
            })
            .collect();
        if rows.is_empty() {
            ui.label("Nothing yet.");
        }
        for (entity, label, is_pipe) in &rows {
            let selected = selection.contains(*entity);
            let row = ui.add_enabled(
                room.edit_enabled || !is_pipe,
                egui::Button::selectable(selected, label),
            );
            if row.clicked() {
                selection.set_only(*entity);
            }
        }
        ui.horizontal(|ui| {
            if ui.button("Example kitchen").clicked() {
                despawn_all(commands, &rows);
                for (furniture, transform) in example_layout() {
                    commands.spawn((furniture, transform));
                }
                selection.clear();
                if let Ok(mut orbit) = camera.single_mut() {
                    orbit.frame_room(room);
                }
                editor.status = "Example kitchen placed.".into();
            }
            if ui.button("Clear").clicked() {
                despawn_all(commands, &rows);
                selection.clear();
                editor.status = "Room cleared.".into();
            }
        });
    });
}

/// The name of the primary selection and a button that opens Properties.
fn selected_section(
    ui: &mut egui::Ui,
    selection: &Selection,
    pieces: &Pieces,
    properties: &mut PropertiesUi,
) {
    dropdown(ui, "Selected", false, |ui| {
        let primary = selection.primary.and_then(|entity| pieces.get(entity).ok());
        if let Some((_, furniture, _)) = primary {
            ui.label(furniture.name.as_str());
            if selection.entities.len() > 1 {
                ui.weak(format!("{} pieces selected", selection.entities.len()));
            }
        } else {
            ui.label("Click a piece, or add one.");
        }
        if ui.button("Open Properties").clicked() {
            properties.open = true;
        }
    });
}
