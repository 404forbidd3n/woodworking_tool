//! Unit tests for the room model: geometry, file loading, and helpers.
//!
//! Run them with `cargo test`.

use bevy::prelude::*;

use super::*;

fn point_in_solid(part: &Solid, point: Vec3) -> bool {
    let local = part.rotation.inverse() * (point - part.translation);
    local.x.abs() <= part.size.x * 0.5
        && local.y.abs() <= part.size.y * 0.5
        && local.z.abs() <= part.size.z * 0.5
}

#[test]
fn drawers_split_the_front_into_equal_bands() {
    let furniture = Furniture::new(Kind::Drawers { count: 3 }, 600.0, 720.0, 560.0);
    let fronts: Vec<_> = solids(&furniture)
        .into_iter()
        .filter(|solid| matches!(solid.role, Surface::Front(_)))
        .collect();
    assert_eq!(fronts.len(), 3);
    let mut centers: Vec<_> = fronts.iter().map(|solid| solid.translation.y).collect();
    centers.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let first_gap = centers[1] - centers[0];
    let second_gap = centers[2] - centers[1];
    assert!((first_gap - second_gap).abs() < 1.0e-4);
    // The stack starts near the bottom and ends near the top, not bunched in the middle.
    let leg_height = furniture.leg_height_meters();
    assert!(centers[0] - leg_height < 200.0 / MM_PER_METER);
    assert!(centers[2] - leg_height > 500.0 / MM_PER_METER);
    let roles: Vec<_> = solids(&furniture)
        .into_iter()
        .filter_map(|solid| match solid.role {
            Surface::Front(index) => Some(index),
            _ => None,
        })
        .collect();
    assert_eq!(roles, vec![0, 1, 2]);
}

#[test]
fn jolly_cabinet_has_a_full_height_jolly_left_and_drawers_right() {
    let (furniture, y_mm) = Kind::JollyDrawers {
        drawers: 3,
        jolly_mm: JOLLY_DEFAULT_MM,
    }
    .preset(Placement::Floor);
    assert_eq!(
        (furniture.width_mm, furniture.height_mm, y_mm),
        (600.0, 780.0, 0.0)
    );
    assert_eq!(furniture.kind.front_count(), 4);
    assert_eq!(furniture.legs, LegStyle::Plastic100);

    let parts = solids(&furniture);
    let front = |index: u8| {
        parts
            .iter()
            .find(|solid| solid.role == Surface::Front(index))
            .copied()
            .unwrap()
    };
    let jolly = front(0);
    let height = furniture.height_mm / MM_PER_METER;
    assert!(jolly.size.y > height - 0.01, "jolly is not full height");
    assert!(jolly.translation.x < 0.0, "jolly is not on the left");
    assert!((jolly.size.x - 0.1955).abs() < 1.0e-4);

    let drawers: Vec<_> = (1..=3).map(front).collect();
    for drawer in &drawers {
        assert!(drawer.translation.x > 0.0, "drawer is not on the right");
        assert!(drawer.size.y < height / 3.0);
        // Drawers stop one gap short of the jolly: no overlap in X.
        let jolly_right = jolly.translation.x + jolly.size.x * 0.5;
        let drawer_left = drawer.translation.x - drawer.size.x * 0.5;
        assert!((drawer_left - jolly_right - GAP_METERS).abs() < 1.0e-4);
    }
    assert!(drawers[0].translation.y < drawers[1].translation.y);
    assert!(drawers[1].translation.y < drawers[2].translation.y);
}

#[test]
fn jolly_width_stays_inside_the_cabinet() {
    assert_eq!(jolly_width_mm(600.0, 200), 200.0);
    assert_eq!(jolly_width_mm(600.0, 20), JOLLY_MIN_MM);
    assert_eq!(jolly_width_mm(600.0, 900), 500.0);
    assert_eq!(jolly_width_mm(150.0, 200), 75.0);
}

#[test]
fn jolly_pull_is_a_long_vertical_bar() {
    let furniture = Kind::JollyDrawers {
        drawers: 2,
        jolly_mm: JOLLY_DEFAULT_MM,
    }
    .preset(Placement::Floor)
    .0;
    let parts = solids(&furniture);
    let handles: Vec<_> = parts
        .iter()
        .filter(|solid| solid.role == Surface::Handle)
        .collect();
    assert_eq!(handles.len(), 3);
    let jolly = handles
        .iter()
        .find(|solid| solid.translation.x < 0.0)
        .unwrap();
    assert!(
        jolly.size.y > jolly.size.x * 10.0,
        "jolly pull is not upright"
    );
    assert!(jolly.size.y >= 0.3);
    for drawer in handles.iter().filter(|solid| solid.translation.x > 0.0) {
        assert!(
            drawer.size.x > drawer.size.y,
            "drawer pull is not horizontal"
        );
    }
}

#[test]
fn every_front_gets_a_black_edge_band_behind_it() {
    for kind in [
        Kind::Drawers { count: 3 },
        Kind::JollyDrawers {
            drawers: 2,
            jolly_mm: 200,
        },
        Kind::HingedDoors { doors: 2 },
        Kind::LiftDoor,
        Kind::SlidingDoors,
        Kind::Corner {
            style: CornerStyle::DoubleDoor,
        },
    ] {
        let furniture = Furniture::new(kind, 600.0, 720.0, 600.0);
        let parts = solids(&furniture);
        let fronts: Vec<_> = parts
            .iter()
            .filter(|solid| matches!(solid.role, Surface::Front(_)))
            .collect();
        let edges: Vec<_> = parts
            .iter()
            .filter(|solid| solid.role == Surface::Edge)
            .collect();
        assert_eq!(fronts.len(), edges.len(), "{kind:?}");
        for (front, edge) in fronts.iter().zip(&edges) {
            assert_eq!(front.translation, edge.translation, "{kind:?}");
            assert_eq!(front.rotation, edge.rotation, "{kind:?}");
        }
    }
    let band = edge_band_size(Vec3::new(0.4, 0.2, 0.018));
    assert!((band.x - (0.4 + FRONT_EDGE_METERS * 2.0)).abs() < 1.0e-6);
    assert!((band.y - (0.2 + FRONT_EDGE_METERS * 2.0)).abs() < 1.0e-6);
    assert!((band.z - 0.017).abs() < 1.0e-6);
}

#[test]
fn induction_hob_is_a_thin_plate_on_the_worktop() {
    let (hob, y_mm) = Kind::InductionHob.preset(Placement::Floor);
    let worktop = Kind::Countertop.preset(Placement::Floor);
    assert_eq!(y_mm, worktop.1 + worktop.0.height_mm);
    assert!(hob.kind.is_appliance());
    assert_eq!(hob.legs, LegStyle::None);
    let size = hob.size_meters();
    assert!((size.y - 0.006).abs() < 1.0e-6, "hob height was clamped");
    let parts = solids(&hob);
    let zones = parts
        .iter()
        .filter(|solid| solid.role == Surface::ApplianceRound(1))
        .count();
    assert_eq!(zones, 4);
    let top = parts
        .iter()
        .map(|solid| solid.translation.y + solid.size.y * 0.5)
        .fold(0.0, f32::max);
    assert!(top < 0.01, "hob sticks up {top} m");
}

#[test]
fn thin_slabs_keep_their_thickness_when_loaded() {
    let piece = |kind: &str, height: f32| {
        format!(
            r#"{{"kind": {{"type": "{kind}"}}, "width_mm": 600.0, "height_mm": {height},
                "depth_mm": 520.0, "x_mm": 0.0, "y_mm": 720.0, "z_mm": 0.0, "turns": 0}}"#
        )
    };
    let json = format!(
        r#"{{"version": 3, "snap_enabled": true, "snap_mm": 50.0,
            "room": {{"width_mm": 4200.0, "depth_mm": 3000.0, "wall_height_mm": 2400.0, "show_walls": true}},
            "pieces": [{}, {}]}}"#,
        piece("countertop", 38.0),
        piece("induction_hob", 6.0),
    );
    let file: ProjectFile = serde_json::from_str(&json).unwrap();
    let file = file.sanitized();
    assert_eq!(file.pieces[0].height_mm, 38.0);
    assert_eq!(file.pieces[1].height_mm, 6.0);
}

#[test]
fn each_kind_has_the_expected_number_of_boxes() {
    // Every front also gets one black edge band.
    let cases = [
        (Kind::Drawers { count: 3 }, 18),
        (
            Kind::JollyDrawers {
                drawers: 2,
                jolly_mm: 200,
            },
            19,
        ),
        (Kind::HingedDoors { doors: 1 }, 12),
        (Kind::HingedDoors { doors: 2 }, 15),
        (Kind::LiftDoor, 8),
        (Kind::SlidingDoors, 15),
        (Kind::Table, 5),
        (Kind::Countertop, 1),
        (Kind::Tabletop, 1),
        (Kind::OpenShelves { shelves: 3 }, 12),
        (Kind::Refrigerator, 4),
        (Kind::Dishwasher, 3),
        (Kind::ElectricOven, 4),
        (Kind::CoffeeMachine, 6),
        (Kind::Microwave, 4),
        (Kind::InductionHob, 10),
        (Kind::SquareSink, 11),
        (Kind::RoundSink, 27),
        (Kind::DoubleSink, 14),
        (Kind::SinkFaucet, 4),
        (Kind::CentralHeatingBoiler, 5),
        (Kind::AirConditioner, 7),
    ];
    for (kind, expected) in cases {
        let furniture = Furniture::new(kind, 600.0, 720.0, 560.0);
        assert_eq!(solids(&furniture).len(), expected, "{kind:?}");
    }
}

#[test]
fn kitchen_sinks_are_open_wash_basins() {
    for kind in [Kind::SquareSink, Kind::RoundSink, Kind::DoubleSink] {
        let furniture = Furniture::new(kind, 600.0, 180.0, 500.0);
        let parts = solids(&furniture);
        let top = parts
            .iter()
            .map(|part| part.translation.y + part.size.y * 0.5)
            .fold(0.0, f32::max);
        let sample = Vec3::new(0.12, top - 0.03, 0.0);
        assert!(
            parts.iter().all(|part| !point_in_solid(part, sample)),
            "{kind:?} covers the top of its basin"
        );
        assert!(
            parts.iter().any(|part| part.translation.y < 0.02),
            "{kind:?} has no basin floor"
        );
    }
}

#[test]
fn ray_hits_a_box_in_front_and_misses_one_beside_it() {
    let min = Vec3::new(-0.3, 0.0, -0.3);
    let max = Vec3::new(0.3, 0.7, 0.3);
    let hit = ray_aabb(
        Vec3::new(0.0, 0.3, 2.0),
        Vec3::new(0.0, 0.0, -1.0),
        min,
        max,
    );
    assert!(hit.is_some());
    let miss = ray_aabb(
        Vec3::new(2.0, 0.3, 2.0),
        Vec3::new(0.0, 0.0, -1.0),
        min,
        max,
    );
    assert!(miss.is_none());
}

#[test]
fn layout_file_is_in_the_project_folder() {
    let path = resolve_layout_path("layout.json");
    assert!(path.is_absolute());
    assert_eq!(path.parent(), Some(project_dir()));
    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("layout.json")
    );
    let nested = resolve_layout_path("jobs/client.json");
    assert_eq!(
        nested.parent().and_then(|dir| dir.file_name()),
        Some(std::ffi::OsStr::new("jobs"))
    );
    assert!(nested.starts_with(project_dir()));
}

#[test]
fn snap_and_quarter_turns_stay_on_the_grid() {
    assert!((snap_to_step(0.126, 0.05) - 0.15).abs() < 1.0e-5);
    assert!((snap_to_step(0.126, 0.0) - 0.126).abs() < 1.0e-5);
    let rotation = rotation_from_turns(3);
    assert_eq!(quarter_turns(rotation), 3);
    assert_eq!(quarter_turns(rotation_from_turns(4)), 0);
}

#[test]
fn stage_one_json_loads_with_stage_two_defaults() {
    let json = r#"{
        "version": 1,
        "snap_enabled": true,
        "snap_mm": 50.0,
        "room": {
            "width_mm": 4200.0,
            "depth_mm": 3000.0,
            "wall_height_mm": 2400.0,
            "show_walls": true
        },
        "pieces": [{
            "kind": {"type": "drawers", "count": 3},
            "width_mm": 600.0,
            "height_mm": 720.0,
            "depth_mm": 560.0,
            "x_mm": 300.0,
            "y_mm": 0.0,
            "z_mm": 280.0,
            "turns": 0
        }]
    }"#;
    let file: ProjectFile = serde_json::from_str(json).unwrap();
    let file = file.sanitized();
    assert_eq!(file.version, 1);
    assert_eq!(file.pieces[0].name, "Drawer Base Cabinet");
    assert_eq!(file.pieces[0].finishes.fronts.len(), 3);
    assert_eq!(file.pieces[0].placement, Placement::Floor);
    assert_eq!(file.pieces[0].legs, LegStyle::Plastic100);
    assert_eq!(file.cut_list.board_width_mm, 2440.0);
    assert!(file.cut_list.parts.is_empty());
}

#[test]
fn finish_and_selection_helpers_round_trip() {
    let mut finish = Finish::default();
    finish.preset = FinishPreset::Custom;
    finish.color = [12, 34, 56];
    let encoded = serde_json::to_string(&finish).unwrap();
    let decoded: Finish = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.rgb(), [12, 34, 56]);

    let mut world = World::new();
    let first = world.spawn_empty().id();
    let second = world.spawn_empty().id();
    let mut selection = Selection::default();
    selection.set_only(first);
    selection.toggle(second);
    assert!(selection.contains(first) && selection.contains(second));
    assert_eq!(selection.primary, Some(second));
    selection.toggle(second);
    assert_eq!(selection.primary, Some(first));
}

#[test]
fn stage_four_presets_and_legacy_box_migrate() {
    let legacy: Kind = serde_json::from_str(r#"{"type":"box"}"#).unwrap();
    assert_eq!(legacy, Kind::OpenShelves { shelves: 3 });

    let (floor, floor_y) = Kind::Drawers { count: 3 }.preset(Placement::Floor);
    assert_eq!(floor_y, 0.0);
    assert_eq!(floor.legs, LegStyle::Plastic100);
    assert_eq!(floor.depth_mm, 560.0);

    let (wall, wall_y) = Kind::Drawers { count: 3 }.preset(Placement::Wall);
    assert_eq!(wall_y, 1400.0);
    assert_eq!(wall.legs, LegStyle::None);
    assert_eq!(wall.depth_mm, 350.0);
}

#[test]
fn room_openings_are_clamped_to_their_wall() {
    let mut openings = vec![Opening {
        kind: OpeningKind::Window,
        wall: WallSide::Left,
        offset_mm: 9000.0,
        width_mm: 4000.0,
        height_mm: 2000.0,
        sill_mm: 1800.0,
    }];
    sanitize_openings(4200.0, 3000.0, 2400.0, &mut openings);
    assert_eq!(openings[0].width_mm, 3000.0);
    assert_eq!(openings[0].offset_mm, 0.0);
    assert_eq!(openings[0].sill_mm, 400.0);
}

#[test]
fn pipes_are_round_pieces_with_one_or_two_members() {
    let vertical = Kind::Pipe {
        horizontal: false,
        count: 2,
        color: PipeColor::Gray,
    };
    let (furniture, y) = vertical.preset(Placement::Floor);
    assert_eq!(y, 0.0);
    assert_eq!(furniture.legs, LegStyle::None);
    let parts = solids(&furniture);
    assert_eq!(parts.len(), 2);
    assert!(parts.iter().all(|part| part.role == Surface::Pipe));
    assert!(parts.iter().all(|part| part.size.y > part.size.x));

    let horizontal = Kind::Pipe {
        horizontal: true,
        count: 1,
        color: PipeColor::Blue,
    };
    let (furniture, y) = horizontal.preset(Placement::Floor);
    assert_eq!(y, 500.0);
    let parts = solids(&furniture);
    assert_eq!(parts.len(), 1);
    assert!(parts[0].size.x > parts[0].size.y);
    assert!((parts[0].size.y - 0.02).abs() < 0.001);

    let saved: RoomFile = serde_json::from_str(
        r#"{
            "width_mm": 4200.0,
            "depth_mm": 3000.0,
            "wall_height_mm": 2400.0,
            "show_walls": true
        }"#,
    )
    .unwrap();
    let walls = saved.resolved_walls();
    assert!(walls.back && walls.left && walls.right);
    assert!(!walls.front);

    let minimal: Kind = serde_json::from_str(r#"{"type":"pipe"}"#).unwrap();
    assert_eq!(
        minimal,
        Kind::Pipe {
            horizontal: false,
            count: 1,
            color: PipeColor::Gray
        }
    );
}

#[test]
fn corner_shelf_cabinet_has_two_doors_and_shelves() {
    let furniture = Furniture::new(
        Kind::Corner {
            style: CornerStyle::ShelfDoors,
        },
        600.0,
        720.0,
        600.0,
    );
    let parts = solids(&furniture);
    let fronts = parts
        .iter()
        .filter(|solid| matches!(solid.role, Surface::Front(_)))
        .count();
    assert_eq!(fronts, 2);
    let shelves = parts
        .iter()
        .filter(|solid| {
            solid.role == Surface::Carcass(2) && solid.size.x < 0.4 && solid.size.z < 0.4
        })
        .count();
    assert!(shelves >= 2, "{shelves}");
}

#[test]
fn corner_double_door_spans_the_opening() {
    let furniture = Furniture::new(
        Kind::Corner {
            style: CornerStyle::DoubleDoor,
        },
        600.0,
        720.0,
        600.0,
    );
    let parts = solids(&furniture);
    let fronts: Vec<_> = parts
        .into_iter()
        .filter(|solid| matches!(solid.role, Surface::Front(_)))
        .collect();
    assert_eq!(fronts.len(), 2);
    let delta = fronts[1].translation - fronts[0].translation;
    assert!(delta.x.abs() > 0.05);
    assert!(delta.z.abs() > 0.05);
    assert_ne!(fronts[0].rotation, Quat::IDENTITY);
}

#[test]
fn glass_door_adds_a_pane_inside_a_frame() {
    let mut furniture = Furniture::new(Kind::HingedDoors { doors: 1 }, 600.0, 720.0, 560.0);
    furniture.doors = DoorStyle::Glass;
    let parts = solids(&furniture);
    assert!(parts.iter().any(|solid| solid.role == Surface::Glass));
    assert!(
        parts
            .iter()
            .filter(|solid| matches!(solid.role, Surface::Front(_)))
            .count()
            >= 4
    );
}

#[test]
fn older_pieces_load_with_solid_doors() {
    let piece: PieceFile = serde_json::from_str(
        r#"{
            "kind": {"type": "hinged_doors", "doors": 2},
            "width_mm": 600.0,
            "height_mm": 720.0,
            "depth_mm": 560.0,
            "x_mm": 0.0,
            "y_mm": 0.0,
            "z_mm": 0.0,
            "turns": 0
        }"#,
    )
    .unwrap();
    assert_eq!(piece.doors, DoorStyle::Solid);
}

#[test]
fn custom_block_scales_from_the_designed_size() {
    let mut block = CustomType::empty("Wide");
    block.boards.push(CustomBoard {
        role: CustomRole::Wall,
        width_mm: 18.0,
        height_mm: 100.0,
        depth_mm: 40.0,
        x_mm: -50.0,
        y_mm: 50.0,
        z_mm: 0.0,
        yaw_deg: 0.0,
        pitch_deg: 0.0,
        roll_deg: 0.0,
        turns: 0,
        ..CustomRole::Wall.default_board()
    });
    block.boards.push(CustomBoard {
        role: CustomRole::Wall,
        width_mm: 18.0,
        height_mm: 100.0,
        depth_mm: 40.0,
        x_mm: 50.0,
        y_mm: 50.0,
        z_mm: 0.0,
        yaw_deg: 0.0,
        pitch_deg: 0.0,
        roll_deg: 0.0,
        turns: 0,
        ..CustomRole::Wall.default_board()
    });
    block.recompute_nominal();
    let designed = block.scaled_solids(block.width_mm, block.height_mm, block.depth_mm);
    let wider = block.scaled_solids(block.width_mm * 2.0, block.height_mm, block.depth_mm);
    let designed_gap = (designed[1].translation.x - designed[0].translation.x).abs();
    let wider_gap = (wider[1].translation.x - wider[0].translation.x).abs();
    assert!((wider_gap - designed_gap * 2.0).abs() < 0.001);
    assert!((wider[0].size.y - designed[0].size.y).abs() < 0.001);
    // The bottoms of both walls stay on the floor when only the width changes.
    let designed_bottom = designed[0].translation.y - designed[0].size.y * 0.5;
    let wider_bottom = wider[0].translation.y - wider[0].size.y * 0.5;
    assert!(designed_bottom.abs() < 0.001);
    assert!(wider_bottom.abs() < 0.001);
}

#[test]
fn a_quarter_turn_swaps_a_boards_width_and_depth() {
    let mut block = CustomType::empty("Turned");
    block.boards.push(CustomBoard {
        role: CustomRole::Plank,
        width_mm: 200.0,
        height_mm: 18.0,
        depth_mm: 40.0,
        x_mm: 0.0,
        y_mm: 9.0,
        z_mm: 0.0,
        yaw_deg: 0.0,
        pitch_deg: 0.0,
        roll_deg: 0.0,
        turns: 1,
        ..CustomRole::Plank.default_board()
    });
    block.recompute_nominal();
    assert!((block.width_mm - 40.0).abs() < 0.1);
    assert!((block.depth_mm - 200.0).abs() < 0.1);
    block.sanitize();
    assert_eq!(block.boards[0].turns, 0);
    assert!((block.boards[0].yaw_deg - 90.0).abs() < 0.01);
    let solid = &block.scaled_solids(block.width_mm, block.height_mm, block.depth_mm)[0];
    assert_ne!(solid.rotation, Quat::IDENTITY);
}

#[test]
fn a_pitched_board_stands_up_like_a_roof() {
    let board = CustomBoard {
        role: CustomRole::Plank,
        width_mm: 80.0,
        height_mm: 18.0,
        depth_mm: 200.0,
        x_mm: 0.0,
        y_mm: 0.0,
        z_mm: 0.0,
        yaw_deg: 0.0,
        pitch_deg: 90.0,
        roll_deg: 0.0,
        turns: 0,
        ..CustomRole::Plank.default_board()
    };
    let half = board.world_half_mm();
    assert!((half.x - 40.0).abs() < 0.2);
    assert!((half.y - 100.0).abs() < 0.2);
    assert!((half.z - 9.0).abs() < 0.2);
}

#[test]
fn a_hole_splits_a_board_into_a_frame() {
    let mut board = CustomRole::Door.default_board();
    board.hole_width_mm = 200.0;
    board.hole_height_mm = 300.0;
    assert!(board.pieces_mm().len() >= 4);
    let mut block = CustomType::empty("Open");
    block.boards.push(board);
    block.recompute_nominal();
    let solids = block.scaled_solids(block.width_mm, block.height_mm, block.depth_mm);
    assert!(solids.len() >= 4);
    assert!(solids.iter().all(|solid| solid.shape == BoardShape::Box));
}

#[test]
fn a_triangle_wall_keeps_its_shape() {
    let mut board = CustomRole::Wall.default_board();
    board.shape = BoardShape::Triangle;
    let mut block = CustomType::empty("Gable");
    block.boards.push(board);
    block.recompute_nominal();
    let solids = block.scaled_solids(block.width_mm, block.height_mm, block.depth_mm);
    assert_eq!(solids.len(), 1);
    assert_eq!(solids[0].shape, BoardShape::Triangle);
}
