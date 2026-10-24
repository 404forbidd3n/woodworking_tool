# Woodworking Layout Tool — Project Specification

## 1. Purpose

The Woodworking Layout Tool is a practical desktop application for planning
kitchens, cabinets, furniture, and simple rooms. It combines:

- a three-dimensional room editor;
- configurable cabinet and appliance models;
- project files that can be shared and reopened;
- a separate two-dimensional sheet-cut planner;
- a 3D component planner for building a custom furniture type from loose
  boards and saving it into the room.

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

The main window opens fullscreen on the primary monitor.

### 3.1 Top toolbar

The toolbar displays the active project filename and provides:

- **Load** — select a JSON project through the native operating-system dialog;
- **Save** — overwrite the active project;
- **Save As** — select a new JSON filename and location;
- **Cut planner** and **Component planner** — two large buttons centered on the
  toolbar. Cut planner opens the sheet-layout window. Component planner opens
  the 3D block builder;
- **Undo** and **Redo** — restore recent layout edits. Load, Save, and Save As
  stay on the right.

Cmd/Ctrl+S performs a quick save.

### 3.2 Sidebar

The panels, toolbar, cut planner, and component planner use a Visual Studio
Code–style dark theme: near-black panels, white button labels, a blue outline
on buttons, a solid blue fill while a button is pressed, and light-blue
section titles. The 3D room itself keeps its existing floor, walls, and
lighting. Measurement labels on the room stay black on a light backing.

The sidebar uses collapsible sections with bold light headings. Room is open by
default; other sections start closed.

Sections include:

1. View — camera buttons, plus the snap checkbox and step
2. Room
3. Add Furniture
4. Customs — saved custom furniture types, separate from the floor and wall lists
5. Add Surfaces
6. Add Appliances
7. In the room
8. Selected

Load, Save, and Save As live only on the toolbar. Properties opens as a docked
panel beside the sidebar, at about 680 logical pixels so the paint and wood
swatches fit on one line. It can still be resized. Clicks and scrolling on the
toolbar, the sidebar, and the properties panel stay on those controls. The 3D
view ignores the pointer there, so it does not select or move pieces.

## 4. Furniture catalog

### 4.1 Floor furniture

- Drawer Base Cabinet — editable drawer count, equally divided fronts.
- Door Base Cabinet — one or two hinged doors.
- Sliding-Door Cabinet — two overlapping leaves.
- Open Shelf Cabinet — editable count of evenly spaced internal shelves.
- Corner Base, Shelves — an L that runs 600 mm along each wall. Two small doors
  close the straight arms, and two shelves sit in the 350 mm corner bay.
- Corner Base, Double Door — the same L, closed by two hinged doors across the
  diagonal opening.
- Work Table — top and four structural legs.

New floor cabinets have plastic 100 mm adjustable legs by default.

### 4.2 Wall-mounted furniture

- Wall Door Cabinet — one or two hinged doors.
- Wall Lift-Up Cabinet — one top-hinged front.
- Wall Drawer Cabinet — editable drawer count.
- Corner Wall, Shelves — the same shelf-and-two-door corner cabinet, hung at
  1400 mm with no legs.
- Corner Wall, Double Door — the diagonal-door corner cabinet, hung at 1400 mm.

Width on a corner cabinet is the run along the back wall. Depth is the run
along the left wall. Both default to 600 mm. The arm depth stays 350 mm, which
is deeper along the diagonal than on either straight margin. A 560 mm base
depth on a 600 mm plan would leave almost no door opening, so floor corners
use the same 600 × 600 plan as the wall ones and sit on the usual 100 mm legs.

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
- Square kitchen sink
- Round kitchen sink
- Large double-bowl square kitchen sink
- Kitchen faucet / water pipe
- Central heating boiler (`centrală termică`)
- Air conditioner

Appliances use lightweight built-in geometry and require no external model
assets. Sink rims align with the default worktop height. The faucet is a
separate movable appliance so it can be positioned with any sink. The central
heating boiler and air conditioner start wall-mounted.

Each appliance has a more recognizable silhouette: the coffee machine has a
hopper, drip tray, and twin spouts; the microwave has a wide door, control
strip, and handle; kitchen sinks are open wash basins with a rim, walls, a
visible floor, and a drain; the boiler has a control panel and service pipes;
and the air conditioner has a vent and louvers. Appliance body color is
limited to silver metal, black, or dark gray.

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

Any cabinet that has doors (hinged, lift-up, sliding, and both corner types)
can use a solid front or a glass front. Glass is a frame of the front finish
with a clear pane in the middle. The default is solid, so older project files
still open with solid doors. Drawer fronts stay solid.

Cabinet colours open on Body & fronts. Body paints every carcass board at
once. Fronts paints every door and every drawer front at once. Handles stay
one choice for the whole piece. Each board is a second tab for the left side,
right side, bottom, top, back, and each door or drawer on its own. A mixed
group, where those boards already differ, shows no swatch selected until you
pick one.

The picker is two rows of swatches. Paints are blue, nordic white (a cream
white), military green, and black. Woods are pine, oak, walnut, white pine,
poplar, cherry, and birch. Wood is drawn with a generated grain image so it
reads as timber rather than flat paint. A custom colour already stored in an
older file can still be edited; new choices come from the swatches. Appliance
bodies instead use the focused silver metal, black, and dark-gray choices
documented above. Tables and worktops keep a single frame colour and a single
top colour.

## 6. Selection and editing

Adding a piece from the sidebar, or duplicating one with D or Properties, puts
its footprint at the centre of the floor. Wall cabinets stay at their mounting
height. Horizontal pipes stay 500 mm up. A duplicate keeps its height and facing.

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
- Arrow keys — nudge selected pieces. With nothing selected, or while Ctrl is
  held, the same keys pan the camera left, right, up, and down, the same way a
  right-drag does.
- Page Up/Page Down — move selected pieces vertically.
- F — frame the room or selected piece.
- Cmd/Ctrl+Z — undo.
- Shift+Cmd/Ctrl+Z or Cmd/Ctrl+Y — redo.

Snap is enabled by default with a 50 mm step. The checkbox and step live in
the View section.

### 6.3 Properties

Properties exposes:

- piece name;
- width, carcass height, and depth;
- drawer, door, or shelf count where relevant;
- door face, solid or glass, on cabinets that have doors;
- floor/wall placement;
- adjustable leg style for floor cabinets;
- X, Y, and Z position;
- facing direction;
- body and front colours, with a second tab for each board;
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

The default room is 4200 × 3000 × 2400 mm. It has a floor and four walls:
back, left, right, and front. The front wall starts hidden so the camera can
still see into the room.

The Room section starts with these checkboxes, one after another:

- **Show walls** — hides or shows every wall at once;
- **Enable room editing**;
- **Show room measurements**;
- **Back wall**, **Left wall**, **Right wall**, and **Front wall**.

The four wall checkboxes show or hide that wall on its own. Hide the wall you
are looking through when furniture sits on every side. They stay available
while the room is locked, and each wall's visibility is saved. While **Show
walls** is off, the four wall checkboxes are disabled and no wall is drawn.

Room editing is locked by default. **Enable room editing** must be checked
before wall dimensions, openings, or utility pipes can be changed, added,
selected, or dragged. This avoids accidental room changes while furniture is
being arranged.

**Show room measurements** toggles the wall, door, and window dimension lines.
It is off by default, is not saved in the project, and works whether the room
is locked or not. A hidden wall does not draw its measurements.

### 8.1 Doors and windows

Any number of openings can be added to the back, left, right, or front wall.

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

### 8.2 Utility pipes

Pipes only mark obstacles in the space, so they are kept simple. The Room
section offers **Add vertical pipe** and **Add horizontal pipe**; a new pipe is
placed at the centre of the floor.

Pipes are pieces like furniture: while room editing is enabled they can be
clicked, dragged (Shift-drag to raise or lower), rotated with R, duplicated
with D, deleted, undone, and edited in Properties. While room editing is locked
they cannot be selected or moved, and Clear / Example kitchen leave them in
place.

Properties for a pipe:

- vertical or horizontal;
- one pipe or two parallel pipes;
- color: gray (default), black, or blue;
- length;
- diameter, from 8 mm to 200 mm.

Defaults are 1000 mm long and 20 mm in diameter. A pair is 100 mm across.
Vertical pipes start on the floor; horizontal pipes start 500 mm up. Pipes
render as round members and are excluded from sheet stock.

## 9. Visual guides

- The floor grid uses 100 mm spacing in normal rooms and 500 mm spacing in very
  large rooms.
- Corner axes identify X, Y, and Z.
- Every cabinet, drawer cabinet, and open shelf cabinet displays a compact
  black width annotation beneath its front edge.
- Width annotations include a left-to-right dimension line, arrowheads, and a
  bold `NNN mm` label.
- When **Show room measurements** is on, each visible wall displays its full
  width above the wall, and every door and window on a visible wall displays
  both width and height dimension lines.
- Selected pieces receive a wire outline; the primary selection is distinct
  from secondary selected pieces.

## 10. Project files

Projects are human-readable JSON files. The active schema version is 3.

A project stores:

- room dimensions, the wall master switch, each wall's visibility, the edit
  lock, and openings;
- snap settings;
- furniture and pipe kind, name, dimensions, position, facing, placement, legs,
  door face, finishes, and, for a custom block, the boards it was built from
  (a pipe's orientation, count, and color are part of its kind);
- cut-planner board settings, draft values, and parts.

The application reads versions 1, 2, and 3.

Migration rules:

- missing Stage 2 finish/name fields receive defaults;
- legacy `box` furniture becomes an open shelf cabinet with three shelves;
- legacy lift-up cabinets become wall-mounted;
- other legacy cabinets become floor cabinets with plastic 100 mm legs;
- missing room openings become an empty list;
- a room saved before per-wall flags keeps its back, left, and right walls,
  with the front wall hidden;
- missing cut-list data receives a 2440 × 1220 × 18 mm sheet;
- a missing door face is solid;
- pine, white pine, poplar, cherry, birch, blue, nordic white, military green,
  and black are extra finish names on the same file version. Older files keep
  the finishes they already stored.

A custom piece stores its boards in the project file, so the room still opens
if `customs.json` is missing. The Customs list itself comes from `customs.json`.

Relative paths resolve inside the project directory. Native dialogs may select
files elsewhere. Load and Save As open those dialogs without stopping the
window. Save As appends `.json` when no extension is supplied.

Project files are personal work, not source code. `.gitignore` excludes every
`*.json` file, so saved projects stay on the local machine and are never
committed or pushed.

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

The list is split into three tabs:

- **Doors** — doors and drawer fronts. Each row starts with a Door or Drawer tag. These are often a different sheet, such as MDF.
- **Backs** — cabinet backs. Each row starts with a Back tag. These are often a thinner board, not the carcass sheet and not the door face.
- **Parts** — sides, tops, bottoms, shelves, dividers, and other carcass pieces. Each row starts with its role tag.

Each row shows the tag, the quantity and name, and the size in millimetres. A row can be removed on its own. **Clear doors**, **Clear backs**, or **Clear parts** drops only the open tab. **Clear list** drops all three. A back can be added by hand with kind Back, or removed on its own, the same way as a door. Importing the room still adds every cut piece from the cabinets into the matching tab. Estimate packs whatever is still in any tab, so clearing Doors and Backs leaves only the carcass on the sheet.

Parts can be entered manually or imported from furniture in the room.
Appliances, pipes, and adjustable cabinet legs are excluded from sheet stock.
A corner cabinet contributes an approximate list: sides, top, bottom, back,
two doors, and, for the shelf style, two shelves. A glass door is still listed
as a door; the pane is bought, not cut from the sheet. A custom block
contributes one rectangle for each piece of each board, after the piece is
scaled. A door or drawer front goes to the Doors tab and a back goes to the
Backs tab. A wall is a side, a plank is a shelf, and a bottom or top keeps
that role. A leg is listed under Parts as a custom piece. A rectangular hole
is cut as the frame around the opening, one rectangle for each side of that
frame. A triangle is cut as the rectangle around it. The cabinet's purchased
adjustable legs stay off the sheet.

### 11.3 Estimate and canvas

Estimate uses a deterministic shelf/row packing strategy suitable for rip cuts
followed by crosscuts. It:

- respects kerf and edge trim;
- optionally rotates parts;
- opens additional sheets as needed;
- reports parts that cannot fit;
- shows board count and estimated offcut percentage.

One board is drawn at a time and fills the canvas beside the parts list.
**Previous** and **Next** step through extra sheets. The drawn sheet stays a
light board with a dark outline so the cuts stay readable on the dark planner.
Board dimensions are labelled in millimetres, and parts use distinct colours
with names and dimensions. The parts list has a maximum width, so it cannot
grow across the window and shrink that diagram.

## 11.4 Component planner

The Component planner is a separate window, opened from the center of the
toolbar. It builds a furniture type from loose boards in 3D.

- Add a plank, wall, bottom, top, door, back, drawer, or leg. Each board has
  its own width, height, depth, and position, so one wall can be taller than
  the other and a top can be smaller than the bottom.
- The stage is a 2.4 m floor centered on the block. It uses the room's sky,
  floor colour, 100 mm grid, red/green/blue axes, and lighting. The selected
  board gets the same orange wire outline as a selected piece in the room.
- Drag a board to move it across the floor. The move snaps to 10 mm.
  Shift-drag raises or lowers it. The camera matches the room: drag empty
  space, middle-drag, or Alt-drag to look around; right-drag, Shift+middle-drag,
  or Space+left-drag to pan; scroll to zoom. With nothing selected, or while
  Ctrl is held, the arrow keys pan the camera too.
- Each part is listed under **In this block** with its own **Delete**. Delete
  or Backspace also removes the selected part, including when the pointer is
  over the sidebar, as long as a text field does not have the keyboard.
- A part can be a box, an isosceles triangle, or a right triangle. **Triangle
  wall** and **Right triangle** add a gable on the back of the block. The
  triangle is a prism: its width is the base and its height is the peak. A
  right triangle has the right angle at the lower left.
- Legs can be plastic or metal at 100 mm or 150 mm, or wood at 100 mm. Each
  click places the next leg at a footprint corner, then steps outward.
- **Add drawer** places a drawer front and the box behind it: two sides, a
  bottom, and a back. Another click stacks the next drawer above the last.
- Hole width and height cut a rectangular opening through the depth of a box.
  Hole X and Hole Y shift that opening from the centre of the face. Leave
  either size at 0 mm for a solid board. Triangles ignore holes.
- Arrow keys nudge the selected board the same way as a piece in the room:
  left and right move it sideways, up moves it toward the back wall, and down
  moves it toward the front. Page Up and Page Down change its height. The step
  is the room snap, or 10 mm when snap is off. These keys act only while the
  component planner is the focused window, and a text or number field keeps
  them for itself.
- **Yaw**, **Pitch**, and **Roll** set a board's angle in degrees. Pitch lifts
  the depth edge, which is how a roof slopes. Roll tilts the other way. **R**
  adds 90° of yaw. Tilting lifts the board so its lowest corner stays on the
  floor.
- **Save type** writes the block to `customs.json` beside the project the
  program was compiled from. The file is a list of named types. Each type
  stores its boards (role, shape, leg, hole, size, position, yaw, pitch, and
  roll) and the outer width, height, and depth they were designed at. Saving
  a name that already exists updates that type. The file is gitignored with
  the other JSON files.
  The next launch loads it into **Customs**.
- Choosing a type in Customs places a floor cabinet and copies the boards onto
  that piece. Editing the library later does not change pieces already in the
  room. The copy is stored in the project file, so the piece remains if
  `customs.json` is removed.
- A placed custom piece uses the same properties as the other cabinets:
  name, width, height, depth, placement, legs, position, and facing. Width,
  height, and depth scale every board from the designed size and keep each
  board's angle. Body colour paints every board except doors and drawers.
  Fronts colour the doors and drawers. Legs keep their own plastic, metal, or
  wood look. A custom piece has no per-board colour tab and no glass
  door face, because its doors are plain boards.

Joining boards into a manufactured carcass is still to be decided.

## 12. Code organization

- `src/main.rs` — application resources, plugins, and schedules.
- `src/layout/model.rs` — domain data, furniture geometry, serialization, and
  compatibility.
- `src/layout/view.rs` — 3D camera, room/furniture rendering, and guides.
- `src/layout/edit.rs` — input, toolbar, sidebar, Properties, and editor
  actions.
- `src/layout/history.rs` — bounded layout undo/redo.
- `src/layout/dimensions.rs` — projected cabinet width and optional room
  measurement annotations.
- `src/layout/project.rs` — native project file selection.
- `src/cutting/job.rs` — supplied board and part-list state.
- `src/cutting/pack.rs` — packing and board estimation.
- `src/cutting/from_room.rs` — cabinet-to-cut-part conversion.
- `src/cutting/ui.rs` — cut-planner interface and canvas.
- `src/cutting/mod.rs` — second-window lifecycle and Egui schedule.
- `src/builder/mod.rs` — component-planner window, saved custom types.
- `src/builder/scene.rs` — 3D boards, orbit, and dragging.
- `src/builder/ui.rs` — the board list and save controls.
- `src/theme.rs` — shared visual styling.

The domain model does not depend on the cutting module. Conversion between
saved cut-list data and planner runtime state stays inside `cutting`.

## 13. Acceptance criteria

A release is acceptable when:

- the project builds without compiler warnings;
- all unit tests pass;
- v1, v2, and v3 projects load safely;
- the main window opens fullscreen on the primary monitor, and the cut planner
  and the component planner open without panic or Egui pass errors;
- a newly added or duplicated piece appears at the centre of the floor;
- the cut list keeps backs on their own tab, separate from doors and carcass
  parts;
- save, Save As, and Load preserve room, furniture, materials, openings,
  utility pipes, and cut-list data;
- undo/redo restore complete layout edits;
- floor and wall catalogs create their documented defaults, including both
  corner subtypes;
- a cabinet door face can be switched between solid and glass, and a missing
  door face in an older file stays solid;
- body and front colours can be set together, and each board can be set on
  its own tab;
- a custom type can be saved, placed from Customs, scaled, and keeps each
  board's yaw, pitch, and roll; the selected board in the component planner
  is outlined, and a part can be deleted, shaped, holed, or given a leg or
  drawer;
- room openings produce visible wall holes;
- each of the four walls can be shown or hidden on its own;
- a new pipe is 20 mm in diameter and its diameter can be edited;
- wall, door, and window measurements are hidden by default and remain
  readable while orbiting when shown;
- pipes drag like furniture while room editing is on;
- the room edit lock prevents accidental opening, pipe, and wall changes;
- sheet estimates respect quantity, rotation, trim, and kerf.
