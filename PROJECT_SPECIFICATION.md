# Woodworking Layout Tool — Project Specification

## 1. Purpose

The Woodworking Layout Tool is a practical desktop application for planning
kitchens, cabinets, furniture, and simple rooms. It combines:

- a three-dimensional room editor;
- configurable cabinet and appliance models;
- project files that can be shared and reopened;
- a separate two-dimensional sheet-cut planner.

The application is written in Rust with Bevy and egui. Dimensions shown to the
user are millimetres; the renderer uses metres internally.

This document is the living specification for the implemented application.
`new_tasks.md` is the working list of tasks and bugfixes.

## 2. Coordinate and measurement conventions

- The room origin is the back-left corner of the floor.
- X points right along the back wall.
- Y points upward.
- Z points from the back wall into the room.
- A furniture transform is located at the centre of its footprint.
- A piece facing 0° points toward local +Z.
- Furniture dimensions are width × height × depth.
- Cabinet height describes the carcass. Adjustable legs add to installed
  height.
- Saved positions, furniture sizes, room sizes, openings, and cutting sizes
  are millimetres.

## 3. Main window

### 3.1 Top toolbar

The toolbar displays the active project filename and provides:

- **Load** — select a JSON project through the native operating-system dialog;
- **Save** — overwrite the active project;
- **Save As** — select a new JSON filename and location;
- **Cut planner** — open the independent sheet-layout window;
- **Undo** and **Redo** — restore recent layout edits.

Cmd/Ctrl+S performs a quick save.

### 3.2 Sidebar

The sidebar uses collapsible sections with bold black headings. Room is open by
default; other sections start closed.

Sections include:

1. Project
2. View
3. Room
4. Add Furniture
5. Add Surfaces
6. Add Appliances
7. In the room
8. Selected
9. Snap
10. Help

Properties opens as a docked panel beside the sidebar. It never floats over the
3D stage.

## 4. Furniture catalog

### 4.1 Floor furniture

- Drawer Base Cabinet — editable drawer count, equally divided fronts.
- Door Base Cabinet — one or two hinged doors.
- Sliding-Door Cabinet — two overlapping leaves.
- Open Shelf Cabinet — editable count of evenly spaced internal shelves.
- Work Table — top and four structural legs.

New floor cabinets have plastic 100 mm adjustable legs by default.

### 4.2 Wall-mounted furniture

- Wall Door Cabinet — one or two hinged doors.
- Wall Lift-Up Cabinet — one top-hinged front.
- Wall Drawer Cabinet — editable drawer count.

Wall cabinets default to 350 mm depth, have no floor legs, and are initially
placed 1400 mm above the floor.

### 4.3 Surfaces

- Kitchen Worktop — independent 38 mm thick worktop preset.
- Tabletop — independent 30 mm thick top preset.

### 4.4 Appliances

- Refrigerator
- Dishwasher
- Electric oven
- Coffee machine
- Microwave

Appliances use lightweight built-in geometry and require no external model
assets.

## 5. Cabinet construction and appearance

Cabinets are generated from simple solids:

- left side;
- right side;
- bottom;
- top;
- back;
- fronts, handles, and shelves as required.

Available adjustable leg choices are:

- none;
- plastic 100 mm;
- plastic 150 mm;
- metal 100 mm;
- metal 150 mm;
- wood 100 mm.

Four legs are positioned under a floor cabinet. Legs are purchased hardware
and are excluded from plywood cutting estimates.

Finishes can be edited independently for carcass parts, each front, handles,
tops, and appliance bodies. Presets are oak, walnut, white, charcoal, stainless
steel, and custom RGB colour.

## 6. Selection and editing

### 6.1 Pointer controls

- Left-drag a piece — move it across the floor.
- Shift+left-drag a piece — move it vertically.
- Left-drag empty space — orbit.
- Middle-drag or Alt+left-drag — orbit.
- Right-drag, Shift+middle-drag, or Space+left-drag — pan.
- Scroll — zoom.
- Ctrl/Cmd+click — add or remove a piece from the selection.
- Right-click a piece — open its Properties command.

Dragging any member of a multi-selection moves the whole group while preserving
relative positions.

### 6.2 Keyboard controls

- R — rotate selected pieces by 90°.
- D — duplicate selected pieces.
- Delete/Backspace — remove selected pieces.
- Arrow keys — nudge selected pieces.
- Page Up/Page Down — move selected pieces vertically.
- F — frame the room or selected piece.
- Cmd/Ctrl+Z — undo.
- Shift+Cmd/Ctrl+Z or Cmd/Ctrl+Y — redo.

Snap is enabled by default with a 50 mm step and can be changed or disabled.

### 6.3 Properties

Properties exposes:

- piece name;
- width, carcass height, and depth;
- drawer, door, or shelf count where relevant;
- floor/wall placement;
- adjustable leg style for floor cabinets;
- X, Y, and Z position;
- facing direction;
- all applicable material and colour controls;
- duplicate and delete actions.

## 7. Undo and redo

- The editor stores up to ten layout snapshots.
- Continuous dragging and continuous numeric controls produce one undo step.
- A new edit clears the redo stack.
- Furniture, transforms, room settings, openings, and snap settings participate
  in history.
- The cut-planner job has independent state and is not changed by layout undo.
- Loading another project starts a new history.

## 8. Room

The default room is 4200 × 3000 × 2400 mm. It has a floor, back wall, left
wall, and right wall. The front stays open for camera access.

Editable room fields are:

- width;
- depth;
- wall height;
- wall visibility.

### 8.1 Doors and windows

Any number of openings can be added to the back, left, or right wall.

Each opening stores:

- type: door or window;
- wall;
- horizontal position from the wall's left/back end;
- width;
- height;
- sill height for windows.

Doors always begin at floor level. Walls are divided into solid panels around
the opening, creating a real hole. Out-of-range values are clamped to the room;
overlapping openings form the union of their holes.

## 9. Visual guides

- The floor grid uses 100 mm spacing in normal rooms and 500 mm spacing in very
  large rooms.
- Corner axes identify X, Y, and Z.
- Every cabinet, drawer cabinet, and open shelf cabinet displays a compact
  black width annotation beneath its front edge.
- Width annotations include a left-to-right dimension line, arrowheads, and a
  bold `NNN mm` label.
- Selected pieces receive a wire outline; the primary selection is distinct
  from secondary selected pieces.

## 10. Project files

Projects are human-readable JSON files. The active schema version is 3.

A project stores:

- room dimensions, wall visibility, and openings;
- snap settings;
- furniture kind, name, dimensions, position, facing, placement, legs, and
  finishes;
- cut-planner board settings, draft values, and parts.

The application reads versions 1, 2, and 3.

Migration rules:

- missing Stage 2 finish/name fields receive defaults;
- legacy `box` furniture becomes an open shelf cabinet with three shelves;
- legacy lift-up cabinets become wall-mounted;
- other legacy cabinets become floor cabinets with plastic 100 mm legs;
- missing room openings become an empty list;
- missing cut-list data receives a 2440 × 1220 × 18 mm sheet.

Relative paths resolve inside the project directory. Native dialogs may select
files elsewhere. Load and Save As open those dialogs without stopping the
window. Save As appends `.json` when no extension is supplied.

## 11. Cut planner

The Cut planner opens in a separate operating-system window so its pointer
input cannot move the 3D stage.

### 11.1 Supplied board

Editable board fields are:

- width;
- height;
- thickness;
- saw kerf;
- edge trim;
- whether parts may rotate 90°.

Defaults are 2440 × 1220 × 18 mm, 3 mm kerf, no edge trim, and rotation
enabled.

### 11.2 Parts

Each part has:

- role: door, drawer front, shelf, top, bottom, side, back, divider, or custom;
- optional name;
- width and height;
- quantity.

Parts can be entered manually or imported from furniture in the room.
Appliances and adjustable cabinet legs are excluded from sheet stock.

### 11.3 Estimate and canvas

Estimate uses a deterministic shelf/row packing strategy suitable for rip cuts
followed by crosscuts. It:

- respects kerf and edge trim;
- optionally rotates parts;
- opens additional sheets as needed;
- reports parts that cannot fit;
- shows board count and estimated offcut percentage.

Every board is drawn on a responsive 2D canvas. The full sheet remains visible,
board dimensions are labelled in millimetres, and parts use distinct colours
with names and dimensions where space permits.

## 12. Code organization

- `src/main.rs` — application resources, plugins, and schedules.
- `src/layout/model.rs` — domain data, furniture geometry, serialization, and
  compatibility.
- `src/layout/view.rs` — 3D camera, room/furniture rendering, and guides.
- `src/layout/edit.rs` — input, toolbar, sidebar, Properties, and editor
  actions.
- `src/layout/history.rs` — bounded layout undo/redo.
- `src/layout/dimensions.rs` — projected cabinet width annotations.
- `src/layout/project.rs` — native project file selection.
- `src/cutting/job.rs` — supplied board and part-list state.
- `src/cutting/pack.rs` — packing and board estimation.
- `src/cutting/from_room.rs` — cabinet-to-cut-part conversion.
- `src/cutting/ui.rs` — cut-planner interface and canvas.
- `src/cutting/mod.rs` — second-window lifecycle and Egui schedule.
- `src/theme.rs` — shared visual styling.

The domain model does not depend on the cutting module. Conversion between
saved cut-list data and planner runtime state stays inside `cutting`.

## 13. Acceptance criteria

A release is acceptable when:

- the project builds without compiler warnings;
- all unit tests pass;
- v1, v2, and v3 projects load safely;
- the main window and cut-planner window open without panic or Egui pass errors;
- save, Save As, and Load preserve room, furniture, materials, openings, and
  cut-list data;
- undo/redo restore complete layout edits;
- floor and wall catalogs create their documented defaults;
- room openings produce visible wall holes;
- sheet estimates respect quantity, rotation, trim, and kerf.
