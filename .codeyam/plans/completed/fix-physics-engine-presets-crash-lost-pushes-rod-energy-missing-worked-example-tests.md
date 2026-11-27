---
title: "Fix: physics engine presets crash, lost pushes, rod energy, missing worked-example tests"
mode: ui
createdAt: "2026-11-26T22:14:09Z"
source: manual
dependsOn: ["fix-time-series-plots-actually-draw-chart-draw-stub-line-geometry-tick-loop"]
---

## Summary

The physics engine's integrators are correct. But both browser presets crash on their first step, user pushes are silently lost, rods change the energy, and none of the plan's hand-computed tests exist. The demo page is a single comment. This fix makes the engine safe and correct, adds the plan's worked-example tests, and finishes `.codeyam/plans/completed/physics-engine-simbiguity-physics-with-particles-springs-rods-and-comparable-integrators.md`, which remains the spec.

## Current behavior

- **Both presets panic.**
  - `crates/simbiguity-physics-wasm/src/lib.rs:58` (`phys_new_pendulum`) and the spring preset (`phys_new_spring_mass`) add the pinned anchor with `mass = 0.0`.
  - `World::add_particle` rejects that (`crates/simbiguity-physics/src/world.rs:57`, `NonPositiveMass`), and `let _ =` swallows the error. The world ends up with one particle.
  - `add_spring` (`crates/simbiguity-physics/src/world.rs:71`) and `add_rod` (`:92`) never bounds-check their indices, so the spring or rod still points at index 1.
  - The first `phys_advance` panics indexing `self.particles[s.p2]` (`crates/simbiguity-physics/src/world.rs:110` / `:129`). So does `phys_encode`, via `crates/simbiguity-physics-view/src/lib.rs:69`.
- **Push is lost.** `step` adds `Push` to `particles[index].force` (`crates/simbiguity-physics/src/world.rs:215`). It then calls `apply_forces()` (`:229`), which overwrites `p.force = p.mass * gravity` (`:122`).
- **Rods don't correct velocity.** `satisfy_constraints` (`crates/simbiguity-physics/src/world.rs:150`) projects positions only, so a pendulum's velocity drifts off the tangent and the rod injects or removes energy, which undermines the energy lesson. The ground clamp (`:158`) flips `velocity.y` even for a particle already moving up.
- **No validation** of negative or NaN `k`, `length`, `rest_length` or `dt`. `phys_advance` (`crates/simbiguity-physics-wasm/src/lib.rs:86`) passes `dt` straight through.
- **CLAUDE.md violations.**
  - `static mut SLOTS` global state plus `unsafe fn slots()` with `.unwrap()` (`crates/simbiguity-physics-wasm/src/lib.rs:13`, `:24`, `:28`), instead of the slot/status pattern in `crates/simbiguity-render-wasm/src/slot.rs`.
  - `axis.unwrap()` in `crates/simbiguity-physics-view/src/lib.rs:99`.
  - `simbiguity-physics-view` isn't `#![no_std]`.
  - Test comments use `//` instead of `///`, and public items lack doc comments.
- **Tests:** only construction smoke tests. There is no projectile, spring-period, pendulum, energy-drift, rod, bounce, determinism or error test.
- **Skipped:**
  - projectile and chain presets
  - linear drag
  - integrator switching and inputs in the bridge
  - chart exports
  - `site/demos/physics/demo.js` is the single line `// basic demo logic to hook up Wasm`, and `state.js` / `state.test.js` are missing

## Key Decisions

- **Errors propagate; nothing is swallowed.** Presets use `?`-style construction and a bridge status code. `add_spring` and `add_rod` validate indices and parameters and return `PhysicsError`. A pinned anchor gets a positive nominal mass, because "pinned" is what fixes it, not "massless".
- **Forces are accumulated, then integrated.** `apply_forces` resets each force to gravity first. Pending pushes are added after the reset, so a push lasts exactly one step, as its comment claims.
- **Rods correct velocity too.** After position projection, remove the velocity component along the rod (relative velocity projected out). The ground only reflects velocity that is moving into the ground. Energy tests prove the pendulum conserves energy under Verlet.
- **The bridge is rebuilt on the shared slot/status pattern** used by the render bridge: no `static mut`, no `unsafe`.
- **The tests are the plan's numbers:** projectile range 20.387 m, spring period π s, pendulum 2.006 s, explicit Euler energy growth over 10% vs Verlet under 0.1%, rod length held to 1e-6, a bounce to 0.25 m. Finalize must run.

## Implementation

### 1. Engine correctness

**File**: `crates/simbiguity-physics/src/world.rs`: validation, index checks, push ordering, rod velocity correction, ground fix, drag, a bounded input queue

**File**: `crates/simbiguity-physics/src/error.rs`: variants for a bad index, a bad parameter and a bad time step

### 2. Bridge and view

**File**: `crates/simbiguity-physics-wasm/src/lib.rs`: slot/status pattern, four presets (pendulum, spring-mass, projectile, chain), integrator selection, push/pin inputs, energy chart exports via `simbiguity-plot`

**File**: `crates/simbiguity-physics-view/src/lib.rs`: `#![no_std]`, no `unwrap`, bounds-safe rendering

### 3. Tests

The plan's full test list goes in `crates/simbiguity-physics/src/world.rs` tests (or a `world_tests.rs` beside it), each with a learner-facing `///`. Add a bridge test that every preset steps 600 frames without panicking.

### 4. Demo

**File**: `site/demos/physics/demo.js`: preset picker, sliders, integrator toggle, energy chart, drag-to-push

**New file**: `site/demos/physics/state.js` (+ `state.test.js`)

## Reused existing code

- Integrators in `crates/simbiguity-physics/src/world.rs` (correct; keep them)
- Slot/status pattern: `crates/simbiguity-render-wasm/src/slot.rs`, `crates/simbiguity-render-wasm/src/status.rs`
- `Chart` / `Series` from `crates/simbiguity-plot` (after the plot fix)

## Reproduction Test

The pendulum preset panics on its first step, because the anchor's mass-0 particle is rejected and the rod points past the end of the particle list.

**Target**: `crates/simbiguity-physics-wasm/src/lib.rs` (its `tests` module). Run with `codeyam-editor editor refresh-tests --test pendulum_preset_steps_without_panicking`.

```rust
/// The pendulum preset is what a learner sees first. It must build two
/// particles (a pinned anchor and a bob) and keep swinging for 10 seconds
/// of frames without crashing the page.
#[test]
fn pendulum_preset_steps_without_panicking() {
    let h = phys_new_pendulum();
    assert_ne!(h, 0);
    for _ in 0..600 {
        assert!(phys_advance(h, 1.0 / 60.0) >= 0);
    }
}
```

Status: PROPOSED — confirm red at execution. Expected failure: a panic, "index out of bounds: the len is 1 but the index is 1", inside the rod projection on the first `phys_advance`.

## Scenarios to Demonstrate

- Pendulum swinging with a flat energy chart (Verlet)
- The same pendulum under explicit Euler, with energy climbing
- Spring-mass oscillation with a period readout of π s
- Projectile arc with a range readout of 20.39 m
- Chain draped under gravity; dragging a link pushes it