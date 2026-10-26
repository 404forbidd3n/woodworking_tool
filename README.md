# Woodworking layout

A small 3D layout tool for kitchens, cabinets, and other rooms, plus a sheet
cutter and a 3D builder for custom furniture. Dimensions shown in the UI and
saved in project files are millimetres. The renderer works in metres.

Run it with:

```sh
cargo test
cargo run
```

`cargo test` runs the geometry, file-format, packing, and history checks.
`cargo run` opens the room fullscreen on the primary monitor. The toolbar
shows the open file and, in the center, the cut planner and the component
planner. The sidebar holds the room, the furniture, and the selection.

Product behaviour is written up in `PROJECT_SPECIFICATION.md`. This file is
the map of the code.

## Layout

This is the ordinary Rust layout: `src/main.rs` is the crate root, and a
folder with a `mod.rs` is a module. `layout/model/mod.rs` is the module
`layout::model`. Nothing about that was invented for this project. What
changed is that the three files that had grown past a thousand lines
(`model.rs`, `edit.rs`, `view.rs`) are now folders of smaller files. Every
folder's `mod.rs` starts with a table of its files, and every file starts
with a short note about what it is for.

```text
src/
├── main.rs            wires every system into the Bevy app
├── theme.rs           shared egui colors, frames, and collapsible sections
├── layout/            the main window: the 3D room
│   ├── model/         pure data and math (no drawing, no input)
│   │   └── geometry/  turns a piece into boxes, one file per furniture family
│   ├── view/          draws the room: camera, meshes, materials, grid
│   ├── edit/          changes the room: mouse, keyboard, sidebar, saving
│   ├── dimensions.rs  the "600 mm" labels under cabinets
│   ├── history.rs     undo / redo
│   └── project.rs     snapshots and the native file dialogs
├── cutting/           second window: packs panels onto plywood sheets
└── builder/           third window: builds a custom block from boards
```

Other modules still import the same paths as before, for example
`crate::layout::model::Furniture`. Each new folder re-exports its contents
from `mod.rs`, so a reader does not have to know which file a type lives in.

## How a frame works

The app is built on [Bevy](https://bevyengine.org) and
[egui](https://github.com/emilk/egui).

- An **entity** is an ID. Every placed cabinet is one entity.
- A **component** is data attached to an entity. A cabinet has a `Furniture`
  (size, kind, colors) and a `Transform` (where it stands and which way it
  faces).
- A **resource** is one global value: `Room`, `Selection`, `Editor`.
- A **system** is a plain function Bevy calls every frame. Its parameters say
  what it reads: `Res<Room>` reads the room, `ResMut<Room>` may change it,
  `Query<&Furniture>` loops over every piece.

Data flows one way. Input systems in `layout::edit` change components and
resources, and set a `dirty` flag when a shape changed. `layout::view` then
rebuilds the meshes for whatever is dirty. `layout::model` knows nothing
about input or drawing.

One frame, in order:

1. **Update.** Open any window that was requested. Handle input in the
   component planner and rebuild its preview. Apply a finished file dialog.
   Then handle the room's mouse and keyboard and move its camera. Windows
   the user closed are cleaned up last.
2. **Egui pass, main window.** Draw the width labels, then the toolbar, the
   sidebar, and the Properties panel.
3. **Egui pass, other windows.** Draw the cut planner and the component
   planner, if those windows are open.
4. **PostUpdate.** Rebuild changed meshes, record an undo step, and draw the
   floor grid and the selection outline.

The schedule itself is `main`.

## The room (`layout`)

The room is split into data, drawing, and input.

### `model/` — data and math

| File | What is in it |
|---|---|
| `units.rs` | millimetres ↔ metres, grid snap, quarter-turn facing |
| `kind.rs` | `Kind` (what a piece is), plus door, corner, leg, and pipe options |
| `furniture.rs` | `Furniture`, the component on every placed piece |
| `custom.rs` | custom blocks from the component planner |
| `finish.rs` | materials and colors |
| `room.rs` | `Room`, walls, doors, and windows |
| `editor.rs` | snap settings, the open file, and the selection |
| `geometry/` | turning a piece into boxes for the renderer |
| `picking.rs` | ray tests for clicking and dragging |
| `example.rs` | the example kitchen |
| `file.rs` | the JSON save format, loading and saving |
| `tests.rs` | geometry, file loading, and helper tests |

`geometry/` has one file per family of furniture: `cabinet`, `corner`,
`appliance`, `sink`, `table`, and `pipe`. `geometry/mod.rs` is the function
`solids`, which picks the right family for a piece and returns plain boxes
(or triangular prisms) with a size, a position, and a surface tag. The view
turns those into meshes. All of that math is in metres, in the piece's local
space: the origin is the centre of the base, +Y is up, and +Z is the front.

The room origin is the back-left corner of the floor. X runs right along the
back wall, Y runs up, and Z runs out into the room. A piece is positioned by
the centre of its base. Facing 0 keeps the front pointing into the room.

### `view/` — drawing

| File | What is in it |
|---|---|
| `mod.rs` | startup: lights, camera, and the first load |
| `camera.rs` | orbit-camera angles and the Corner / Front / Top presets |
| `meshes.rs` | rebuilds meshes for pieces marked dirty |
| `walls.rs` | floor and wall boxes, with holes for doors and windows |
| `materials.rs` | colors, wood grain, and the material cache |
| `guides.rs` | floor grid, axis marker, selection outline |
| `convert.rs` | live pieces ↔ saved file records |

`sync_meshes` runs every frame. Any piece marked `dirty`, and the room when
it is dirty, has its child meshes thrown away and rebuilt. Rebuilding is
cheap because a cabinet is a handful of boxes.

### `edit/` — input and panels

| File | What is in it |
|---|---|
| `pointer.rs` | mouse clicks and drags in the 3D view |
| `camera.rs` | zoom, orbit, and pan |
| `keyboard.rs` | Delete, R, D, F, and the arrow keys |
| `sidebar.rs` | the egui pass that lays out every panel |
| `toolbar.rs` | the bar across the top |
| `library.rs` | the "Add …" buttons and their small icons |
| `properties.rs` | the Properties panel and the right-click menu |
| `materials.rs` | paint and wood swatch pickers |
| `widgets.rs` | the millimetre number field |
| `spawn.rs` | adding and removing pieces |
| `files.rs` | save, load, file dialogs, undo and redo |

Left drag does the thing under the cursor: a piece moves, empty space
orbits. Hold Shift during a move to raise or lower the piece. Exact
millimetres, door counts, and saving live in the sidebar.

`sidebar.rs` is one function per section, in the order they appear on
screen: View, Room, Add Furniture, Customs, Add Surfaces, Add Appliances,
In the room, Selected. `properties.rs` edits the last piece clicked:
`kind_options` draws the controls that depend on the kind of piece, and
`materials_section` draws the colors.

### The three helpers

- `dimensions.rs` projects a width label under the front of every cabinet,
  and wall and opening sizes when "Show room measurements" is on.
- `history.rs` keeps the last ten room snapshots. A drag that is still held
  counts as one undo step.
- `project.rs` turns the live room into a snapshot and back, and owns the
  native Save As and Open dialogs. The dialogs run on another thread so the
  3D window keeps drawing.

## The other two windows

`cutting/` is a separate window. The room decides where a cabinet stands;
this module decides how its panels fit on the plywood you order.

| File | What is in it |
|---|---|
| `mod.rs` | opens and closes the window |
| `job.rs` | the sheet, the parts list, and the three tabs (doors, backs, carcass) |
| `pack.rs` | lays parts in horizontal bands, one rip per band |
| `from_room.rs` | reads panels off the cabinets already in the room |
| `ui.rs` | the sidebar and the sheet drawing |

`builder/` is the component planner. A saved type is written to
`customs.json` and listed under Customs in the room. Width, height, and
depth on a placed piece scale every board together.

| File | What is in it |
|---|---|
| `mod.rs` | the plan being edited, the saved library, and opening the window |
| `scene.rs` | orbit, picking, dragging boards, and the preview meshes |
| `ui.rs` | the sidebar: add a board, edit the selected one, save the type |

`theme.rs` is the shared dark theme. Both extra windows call it, so a new
window picks up the same panels, buttons, and section headers.

## Saved files

Projects are JSON, versions 1 through 3, written next to the program (the
folder this crate was compiled from), not into whatever directory the
terminal happened to be in. `layout.json` is the default name. `customs.json`
is the library of custom blocks and is separate from the room. Loading
clamps every number, so a hand-edited or older file cannot put the app into
a broken state.
