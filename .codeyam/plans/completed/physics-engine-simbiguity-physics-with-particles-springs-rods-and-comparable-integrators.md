---
title: "Physics engine: simbiguity-physics with particles, springs, rods and comparable integrators"
mode: ui
createdAt: "2026-11-26T19:29:20Z"
source: manual
dependsOn: ["time-series-plots-simbiguity-plot-draws-values-over-time-with-the-existing-renderer"]
---

## Summary

Add simbiguity's second engine, **`simbiguity-physics`**: point masses, springs, gravity, rods (distance constraints) and a ground plane, stepped on the fixed clock. Physics was on the original roadmap and dropped. It is the best second engine because it reuses nearly everything already built: the `Simulation`/`Snapshot` contract, `FixedClock`, `Lockstep`, `Vec3`, the 3D renderer, and `simbiguity-plot` for energy graphs. The demo page shows a pendulum, a spring-mass system and a projectile in 3D, with a live energy chart. The tests are textbook mechanics with hand-computed answers.

## Key Decisions

- **Integrators are first-class and compared.** Ship explicit Euler (to show what goes wrong), semi-implicit (symplectic) Euler, and velocity Verlet. Default to velocity Verlet. Tests prove the classic lesson: explicit Euler's energy drifts upward, while symplectic methods keep energy bounded.
- **Particles before rigid bodies.** v1 is point masses with springs and distance constraints (rods, pendulums, cloth-like chains). That is enough for the canonical high-school experiments. Rigid-body rotation and shape-shape collisions come later.
- **Constraints by iterative projection** (position-based: correct positions after each step, a fixed number of iterations, fixed order). This keeps results deterministic. The fixed iteration count is documented as the accuracy knob.
- **Engine and view are separate crates**, per "one concern per crate": `simbiguity-physics` (the simulation, no rendering), `simbiguity-physics-view` (turns physics state into a `Scene`), and `simbiguity-physics-wasm` (the bridge). The view depends on physics, never the reverse.
- **All float math through `simbiguity_math::float`** (sin, cos, sqrt), so native and WASM runs agree bit-for-bit.

## Implementation

### 1. Physics engine

**New file**: `crates/simbiguity-physics/src/lib.rs`: crate docs teaching F = ma → integration
**New file**: `crates/simbiguity-physics/src/world.rs`: `World` holds particles (mass, position, velocity, pinned flag) and implements `Simulation` + `Snapshot`, with `PhysicsInput` (push, pin/unpin, set gravity)
**New file**: `crates/simbiguity-physics/src/forces.rs`: gravity, Hooke springs with damping, linear drag
**New file**: `crates/simbiguity-physics/src/integrate.rs`: `Integrator::{ExplicitEuler, SymplecticEuler, VelocityVerlet}`
**New file**: `crates/simbiguity-physics/src/constraints.rs`: distance constraints (rods) and a ground plane with restitution
**New file**: `crates/simbiguity-physics/src/energy.rs`: kinetic, gravitational and spring potential energy, for readouts and tests
**New file**: `crates/simbiguity-physics/src/error.rs`: `PhysicsError` (non-positive mass, a spring joining a particle to itself, and so on)
**New file**: `crates/simbiguity-physics/capabilities.toml`

### 2. View and bridge

**New file**: `crates/simbiguity-physics-view/src/lib.rs`: spheres for particles, cylinders for springs and rods, a ground plane, and a scene + camera
**New file**: `crates/simbiguity-physics-wasm/src/lib.rs`: `phys_*` exports in the same handle/status style as `crates/simbiguity-render-wasm/src/lib.rs`, plus preset constructors (pendulum, spring-mass, projectile, chain) and chart exports via `simbiguity-plot`

**File**: `scripts/build-site.sh`: add `simbiguity-physics-wasm` to `bridges`

**File**: `crates/simbiguity/src/lib.rs`: a `physics` feature

### 3. Demo page

**New file**: `site/demos/physics/index.html` (+ `demo.js`, `state.js`, `state.test.js`): pick a preset, drag sliders (mass, spring k, rod length, gravity), switch integrators, and watch the energy chart. Picking a particle and dragging it applies a `push` input.

**File**: `site/index.html`: link the demo.

## Reused existing code

- `Simulation`, `Snapshot`, `Seconds` from `crates/simbiguity-core/src/simulation.rs`
- `FixedClock` from `crates/simbiguity-core/src/clock.rs`; `Lockstep` from `crates/simbiguity-core/src/lockstep.rs` (a future circuit-drives-motor demo)
- `Vec3` from `crates/simbiguity-math/src/vec.rs`; deterministic float functions from `crates/simbiguity-math/src/float.rs`
- `Mesh::sphere`, `Mesh::cylinder`, `Scene`, `OrbitCamera`, `encode_frame`, picking from `crates/simbiguity-render`
- Handle/slot/status pattern from `crates/simbiguity-render-wasm/src/slot.rs` and `crates/simbiguity-render-wasm/src/status.rs`
- `simbiguity-plot` from the plot plan

## Tests (written first, hand-computed)

- **Projectile:** v₀ = (10, 10) m/s, g = 9.81 m/s². The range is 2·10·10/9.81 = 20.387 m and the flight time is 2.0387 s. Velocity Verlet is exact for constant acceleration, so the test asserts this to 1e-9 with dt = 1/60.
- **Spring-mass:** m = 1 kg, k = 4 N/m. The period is 2π√(m/k) = π s, and the measured period (between zero crossings) is within 0.1% at dt = 1/240.
- **Pendulum:** small-angle period 2π√(L/g) at L = 1 m is 2.006 s. A large-angle (90°) swing is measurably slower, which teaches where the small-angle approximation breaks.
- **Energy:** after 10,000 steps of an undamped spring, explicit Euler's energy grows more than 10%, while velocity Verlet stays within 0.1%.
- **Rod constraint:** the length is held to 1e-6 after projection.
- **Ground:** with restitution 0.5, a ball dropped from 1 m bounces to 0.25 m.
- **Determinism:** snapshot/restore/replay is bit-identical, and two identically seeded worlds stay identical.
- **Errors:** invalid parameters return `PhysicsError`, never a panic.

## Scenarios to Demonstrate

- Pendulum swinging with the energy chart flat (Verlet)
- The same pendulum under explicit Euler, the energy chart climbing and the swing growing
- Spring-mass oscillation with a period readout
- Projectile arc with a range readout matching the formula
- Chain of rods draped under gravity