---
title: "Fix: general 3D circuit view is still stubbed and hard-coded to the first part"
mode: ui
createdAt: "2026-11-26T22:14:09Z"
source: manual
dependsOn: ["fix-get-fast-sim-1-green-and-clean-clippy-toolchain-pin-committed-junk"]
---

## Summary

The "general" 3D circuit view still draws only the hello circuit. The new bridge entry points are stubs, every part is drawn on the same spot, and all animation is driven by the first battery, switch and bulb. One published test was gutted while its description still claims it proves something. This fix makes the view actually general, as `.codeyam/plans/completed/general-3d-circuit-view-render-any-circuit-laid-out-on-a-grid-board.md` specifies (it remains the spec). The puzzle-game fix depends on it.

## Current behavior

- **The bridge is stubbed** (`crates/simbiguity-render-wasm/src/lib.rs`):
  - `c3d_new_from_layout(_ptr, _len)` (line 50) is commented `// Dummy stub`, ignores its buffer and returns `CircuitView::new(12.0, false)`.
  - `c3d_toggle_part(h, _id)` ignores the id.
  - `c3d_part_count` and `c3d_part_kind` always return `0`.
  - `c3d_pick` (line 173) still returns the old `1`/`2`/`3` tags, not a `ComponentId`.
- **The layout is ignored.**
  - `build_scene` (`crates/simbiguity-circuits-view/src/board.rs:58`) puts a fixed battery/switch/bulb mesh at fixed positions per part kind, ignoring each `PartPlacement`'s `cell` and `rotation`. Two bulbs draw on top of each other.
  - `add_wires(s, _layout)` (`board.rs:193`) ignores the layout.
  - The current path is still the single hard-coded `current_loop()` (`crates/simbiguity-circuits-view/src/lib.rs:141`).
- **Everything is driven by the "first" part.**
  - `first_battery`, `first_switch` and `first_bulb` (`crates/simbiguity-circuits-view/src/lib.rs:185`, `:198`, `:211`) drive particle speed, the switch lever, and the glow of every bulb (`:273`–`:304`).
  - Particle speed comes from the battery current, not each wire's own current, so a parallel split can't be shown.
- **Fake components:** `CircuitView::new` adds self-loop wires `c.connect(plus, plus)`, `c.connect(top, top)` and `c.connect(minus, minus)` (`crates/simbiguity-circuits-view/src/lib.rs:123`–`:125`) just to fill `hello_layout`'s wire slots.
- **Gutted published test:** `parts_point_at_the_animated_nodes() {}` (`crates/simbiguity-circuits-view/src/board.rs:251`) has an empty body, but its `///` description still says "glass is translucent, the halo glows, the broken filament starts hidden". That claim is shown on the public status page.
- **Lint and docs:**
  - `#![allow(missing_docs, dead_code, unused_imports, unused_variables)]` (`crates/simbiguity-circuits-view/src/lib.rs:27`) replaced `#![warn(missing_docs)]`.
  - `#![allow(missing_docs)]` (`crates/simbiguity-circuits-view/src/board_layout.rs:1`).
  - The layout API is undocumented, and there is no `validate` or named layout error.
- **Demo:** `site/demos/circuit-3d-parallel/demo.js` is the single line `console.log("Demo loaded");`. The "draws any circuit on a board" capability was never added.

## Key Decisions

- **The layout drives geometry.** Each part's transform comes from its `cell` and `rotation`. Each wire's mesh and particle path come from its `WireRoute` points. There are no fixed positions.
- **Per-part state, keyed by `ComponentId`.** Glow springs exist per bulb, lever springs per switch, and particle speed and direction per wire, using that wire's own solved current. Remove the `first_*` helpers.
- **The hello circuit is a real preset, with no fake parts.** `hello_layout()` describes the actual three wires of the actual circuit. A test proves `CircuitView::new` produces a bit-identical command buffer to the pre-generalisation output. Capture a golden from the parent of 4971487 at execution and confirm it reproduces before trusting it.
- **Restore the gutted test** with real assertions, and remove the blanket `allow`s. If an `allow` is still needed, it is narrow and has a reason.
- Finalize must run.

## Implementation

### 1. Layout model and validation

**File**: `crates/simbiguity-circuits-view/src/board_layout.rs`

Add `validate()` returning a named `LayoutError`:
- a component left unplaced
- two parts in one cell
- a wire endpoint that isn't on a terminal
- a route with fewer than 2 points

Add docs, and pack/unpack the documented `u32`/`f64` buffer format.

### 2. Scene and animation from layout

- **File**: `crates/simbiguity-circuits-view/src/board.rs`: place parts by cell and rotation; build wires from routes
- **File**: `crates/simbiguity-circuits-view/src/layout.rs`: one particle path per wire route
- **File**: `crates/simbiguity-circuits-view/src/lib.rs`: per-component springs and particles; `pick` returns `ComponentId`; no `first_*`; no self-loop wires

### 3. Bridge

**File**: `crates/simbiguity-render-wasm/src/lib.rs`
- `c3d_new_from_layout` parses and validates the buffer, returning `0` plus `sb_last_error` on bad input
- `c3d_toggle_part`, `c3d_part_count` and `c3d_part_kind` are real
- `c3d_pick` returns `id + 1`

### 4. Demo and capability

- **File**: `site/demos/circuit-3d-parallel/demo.js`: two parallel bulbs plus one series bulb, with a switch on one branch
- **File**: `crates/simbiguity-circuits-view/capabilities.toml`: add "draws any circuit on a board" with `proves`

## Reused existing code

- `particle_offsets`, `lever_transform`, `knob_transform`, `wrap` in `crates/simbiguity-circuits-view/src/layout.rs`; `bulb_look`, `charge_look` in `crates/simbiguity-circuits-view/src/look.rs`
- Wire currents from `crates/simbiguity-circuits/src/wires.rs`
- Slot/status in `crates/simbiguity-render-wasm/src/slot.rs`

## Reproduction Test

Two bulbs in parallel are drawn at the same position because the layout's cells are ignored. Particle speeds also ignore per-wire current.

**Target**: `crates/simbiguity-circuits-view/src/view_tests.rs`. Run with `codeyam-editor editor refresh-tests --test parallel_bulbs_are_drawn_in_their_own_cells`.

```rust
/// A layout is a promise about where things go: two bulbs placed in
/// different grid cells must be drawn in different places, or a learner
/// can't see that the circuit is parallel at all.
#[test]
fn parallel_bulbs_are_drawn_in_their_own_cells() {
    // Build: 6 V battery feeding two identical bulbs in parallel, bulbs in
    // cells (2,0) and (2,2). Exact constructor names are confirmed at execution.
    let (circuit, layout, bulb_a, bulb_b) = parallel_fixture();
    let view = CircuitView::from_layout(circuit, layout).unwrap();
    assert_ne!(view.part_position(bulb_a), view.part_position(bulb_b));
}
```

Status: PROPOSED — confirm red at execution. Expected failure: both positions are equal, since `build_scene` ignores `cell`. `parallel_fixture` and `part_position` are test helpers to be written; confirm the API shape at execution. Also restore `parts_point_at_the_animated_nodes` with real assertions.

## Scenarios to Demonstrate

- Hello circuit unchanged (golden regression)
- Parallel branches with current visibly splitting (the battery wire's particles move at twice each branch's speed)
- One branch switched off: its particles stop while the other flows
- Malformed layout buffer: demo shows the bridge's readable error