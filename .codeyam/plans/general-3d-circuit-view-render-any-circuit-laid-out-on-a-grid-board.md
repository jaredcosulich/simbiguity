---
title: "General 3D circuit view: render any circuit laid out on a grid board"
mode: ui
createdAt: "2026-11-26T19:29:21Z"
source: manual
---

## Summary

The 3D circuit view can only draw one circuit. `CircuitView::new(volts, closed)` builds a fixed battery–switch–bulb loop: `board::build_scene()` takes no arguments, `layout::current_loop()` is a single hard-coded path, and `c3d_pick` returns `1`/`2`/`3` for "battery/switch/bulb". This plan generalises the view so it renders **any circuit laid out on a grid board**: multiple bulbs and resistors, series and parallel branches, and current particles flowing along each wire at that wire's own solved current and direction. Picking returns the real `ComponentId`. This is the foundation that the queued 3D circuit puzzle game, a sandbox, and the RC demo all need.

## Key Decisions

- **Layout is data, separate from the netlist.** A `BoardLayout` assigns each component a grid cell and orientation, and each wire a route of grid points. The `Circuit` stays purely electrical, and the layout stays purely spatial. This matches the puzzle plan's snap-point board.
- **The hello circuit becomes a preset.** `CircuitView::new(volts, closed)` stays as a thin wrapper over `CircuitView::from_layout(hello_layout())`, so the existing demo, its tests and its pixel-stable captures keep working. This is the regression guard.
- **Per-wire current flow.** Particle speed and direction come from each wire's own current (already computed in `crates/simbiguity-circuits/src/wires.rs`). In a parallel circuit, learners see the current split and rejoin, which the single loop could never show.
- **Draw instanced, not one draw per part.** Particles and repeated parts use `DrawInstanced`, so a 30-part board stays one or two draw calls per mesh.
- **Bridge takes a packed layout buffer.** JS writes a flat `u32`/`f64` layout description into shared memory (with the format documented in the bridge crate). Rust parses and validates it, and returns a status code and message on bad input. This keeps the JS side dumb.

## Implementation

### 1. Layout model

**New file**: `crates/simbiguity-circuits-view/src/board_layout.rs`: `BoardLayout`, `PartPlacement { component, cell, rotation }`, `WireRoute { component, points }`, validation (every component placed, wires start and end on terminals, no two parts in one cell), and `hello_layout()`.

### 2. Scene from layout

**File**: `crates/simbiguity-circuits-view/src/board.rs`: `build_scene` takes a `&Circuit` and `&BoardLayout` and emits the meshes and transforms for each part kind.

**File**: `crates/simbiguity-circuits-view/src/layout.rs`: replace the single `current_loop()` with per-wire paths built from `WireRoute`s, and keep `particle_offsets` per path.

**File**: `crates/simbiguity-circuits-view/src/lib.rs`: add `CircuitView::from_layout(circuit, layout)`. Make springs per bulb and per switch, and picking return `ComponentId`. `CircuitView::new` becomes a wrapper.

### 3. Bridge

**File**: `crates/simbiguity-render-wasm/src/lib.rs`: add `c3d_new_from_layout(ptr, len)` and `c3d_toggle_part(h, id)`. `c3d_pick` returns `component_id + 1` (0 = nothing), keeping the old codes for the hello preset during a transition. Add `c3d_part_count` and `c3d_part_kind` for readouts.

### 4. Demo

**New file**: `site/demos/circuit-3d-parallel/index.html` (+ `demo.js`): two bulbs in parallel plus one in series, with a switch on one branch, showing current splitting.

**File**: `crates/simbiguity-circuits-view/capabilities.toml`: "draws any circuit on a board".

## Reused existing code

- `Circuit`, `ComponentId`, `CircuitSim` from `crates/simbiguity-circuits/src/netlist.rs` and `crates/simbiguity-circuits/src/sim.rs`
- Wire currents from `crates/simbiguity-circuits/src/wires.rs`
- `build_scene`, `filament_paths` from `crates/simbiguity-circuits-view/src/board.rs`; `particle_offsets`, `lever_transform`, `knob_transform`, `wrap` from `crates/simbiguity-circuits-view/src/layout.rs`; `bulb_look`, `charge_look` from `crates/simbiguity-circuits-view/src/look.rs`
- `Spring` from `simbiguity-anim`; `Scene`, `Command::DrawInstanced`, picking from `simbiguity-render`
- Slot/status pattern in `crates/simbiguity-render-wasm/src/slot.rs`

## Tests (written first)

- `hello_layout()` through `from_layout` produces a command buffer bit-identical to today's `CircuitView::new`. This guards the existing demo.
- Parallel bulbs (two identical bulbs across a 6 V battery): each branch's particles move at the same speed, and the battery wire's particles move at twice that.
- A branch with an open switch has stationary particles, while the other branch flows.
- Reversing a battery reverses particle direction.
- Layout validation errors: an unplaced component, a wire ending off a terminal, two parts in one cell. Each returns a named error, not a panic.
- Picking a ray through the second bulb returns that bulb's `ComponentId`.
- Bridge: a malformed layout buffer returns a negative status with a readable message (tested natively).

## Scenarios to Demonstrate

- Hello circuit unchanged (regression)
- Parallel branches with current visibly splitting
- One branch switched off
- Series string of three bulbs, each dimmer than a single bulb
- A blown bulb in one branch while the other stays lit