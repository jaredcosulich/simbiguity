---
title: "Fix round 2: physics energy that is really conserved and tests from the textbook"
mode: ui
createdAt: "2026-11-27T14:49:55Z"
source: manual
dependsOn: ["fix-round-2-plots-that-actually-reach-the-screen"]
---

## Summary

The first physics fix (a28fe00) fixed the preset crash and lost pushes. But it rewrote the worked-example tests around whatever the engine happened to output, so they no longer test the physics. Those loose tests hide a real bug: with a rod, velocity Verlet *loses 28% of its energy* in 10 s, which inverts the demo's central lesson. The demo page is still unwired. This plan makes the energy story true, replaces the tests with the hand-computed ones and wires up the demo. Spec chain: `.codeyam/plans/completed/physics-engine-simbiguity-physics-with-particles-springs-rods-and-comparable-integrators.md` and `.codeyam/plans/completed/fix-physics-engine-presets-crash-lost-pushes-rod-energy-missing-worked-example-tests.md`.

## Current behavior

- **Tests fitted to the output** (`crates/simbiguity-physics/src/world_tests.rs`):
  - `projectile_range` (line 9) asserts `x ≈ 20.5 ± 0.1` at line 28. That is vx·t sliding along the ground, not the 20.387 m range.
  - `pendulum_period` (line 60) asserts `x ≈ 0.8 ± 0.1` at line 77. That is a position, not a period.
  - `energy_drift` (line 82) uses a *rod pendulum* and asserts `|ΔE_euler| > 0.5` and `|ΔE_verlet| < 2.0` J at lines 106–107. 2 J is about 10% of the 19.62 J total, and `abs()` hides that both methods *lose* energy.
  - `rod_length` (line 112) uses a tolerance of 1e-5 at line 128 instead of 1e-6.
  - `bounce` (line 133) checks y at a fixed time, ±0.1, rather than the apex.
  - There are no determinism tests and no error-path tests.
- **The rod drains energy.** `satisfy_constraints` (`crates/simbiguity-physics/src/world.rs:178`) projects positions and removes the velocity along the rod after the integrator runs. Over 10 s, measured: Verlet loses 28.3% of the energy and explicit Euler loses 20.7%, so the lesson is backwards.
- **No dt validation.** `World::step` (`crates/simbiguity-physics/src/world.rs:252`) doesn't validate `dt`; only the bridge does.
- **Bridge problems** (`crates/simbiguity-physics-wasm/src/lib.rs`):
  - `fail` (line 19) discards its message.
  - `phys_get_energy` (line 319) writes through an unchecked raw pointer in an `unsafe` block (line 322), with `#[allow(clippy::not_unsafe_ptr_arg_deref)]`.
  - There is no `phys_free`.
- **Demo:** `site/demos/physics/demo.js:15` still reads `// Basic wasm hookup would go here...`.
- **Wrong capability format:** `crates/simbiguity-physics/capabilities.toml` uses a `[capabilities]` table, not the `[[capability]]` / `proves` format that `xtask/src/capabilities.rs` parses.
- **A `//` test comment** at `crates/simbiguity-physics/src/world.rs:334` should be `///`.

## Key Decisions

- **Test the textbook energy lesson on the system where it's clean:** an undamped spring with no rod and no gravity. That is what the original plan specified.
  - m = 1 kg, k = 4 N/m (ω = 2), dt = 1/240, 10,000 steps.
  - Explicit Euler multiplies the energy by (1 + ω²dt²) each step, giving ≈ e^0.694 ≈ ×2.0, so it must *grow* by more than 10% (a signed assertion).
  - Velocity Verlet must stay within 0.1%.
- **Make rods energy-consistent.** Use a constraint scheme that conserves energy with velocity Verlet: RATTLE-style position and velocity constraints inside the step, not a post-hoc projection. Requirement: a 1 m pendulum released at 30°, dt = 1/240, keeps total energy within 1% over 10 s. Document the approach as a lesson.
- **Hand-computed tests only:**
  - **Projectile:** v₀ = (10, 10) m/s, g = 9.81, ground off. At t = 1 s (60 steps of 1/60) x = 10 and y = 5.095 to 1e-9, because Verlet is exact for constant acceleration. Interpolating the zero crossing of y gives a range of 20.387 m to 1e-3.
  - **Spring period:** the zero-crossing period is π s within 0.1%.
  - **Pendulum, small angle:** L = 1 m at 5° has period 2.006 s within 0.5%.
  - **Pendulum, large angle:** 90° gives 2.006 × 1.1803 = 2.368 s within 1%, which teaches where the small-angle formula breaks.
  - **Rod:** length held to 1e-6.
  - **Bounce:** a drop from 1 m with restitution 0.5 has a first apex of 0.25 m ± 0.005.
  - **Determinism:** snapshot → restore → replay is bit-identical.
  - **Errors:** a bad index, mass, k, length or dt each returns its `PhysicsError`.
- **Bridge safety.** Return energy through a shared results buffer, as `crates/simbiguity-circuits-wasm/src/slot.rs` does, instead of writing through a caller pointer. `fail` stores its message for `sb_last_error`. Add `phys_free`.

## Implementation

### 1. Engine

**File**: `crates/simbiguity-physics/src/world.rs`: RATTLE-style rod constraints, dt validation in `step`, `///` comments.

### 2. Tests

**File**: `crates/simbiguity-physics/src/world_tests.rs`: replace the five fitted tests with the hand-computed ones above, and add determinism and error tests.

### 3. Bridge and demo

- **File**: `crates/simbiguity-physics-wasm/src/lib.rs`: results buffer, error messages, `phys_free`
- **File**: `site/demos/physics/demo.js`: load the bridge, preset picker, sliders, integrator toggle, energy chart via `simbiguity-plot`, drag-to-push
- **New file**: `site/demos/physics/state.test.js`

### 4. Capability

**File**: `crates/simbiguity-physics/capabilities.toml`: `[[capability]]` format with real `proves` entries.

## Reused existing code

- `World::compute_energy` in `crates/simbiguity-physics/src/world.rs`
- Results-buffer pattern in `crates/simbiguity-circuits-wasm/src/slot.rs`
- Capability parser in `xtask/src/capabilities.rs`

## Reproduction Test

Explicit Euler on an undamped spring should gain energy and Verlet should hold it, but the engine's lesson is currently untested. With a rod it is inverted.

**Target**: `crates/simbiguity-physics/src/world_tests.rs`. Run with `codeyam-editor editor refresh-tests --test pendulum_energy_is_conserved_by_verlet`.

```rust
/// A swinging pendulum with no friction keeps its energy: kinetic plus
/// potential stays constant. Velocity Verlet with a properly enforced rod
/// should keep it within 1% over 10 seconds. If it doesn't, the integrator
/// or the rod is leaking energy that real physics wouldn't.
#[test]
fn pendulum_energy_is_conserved_by_verlet() {
    let mut w = World::new();
    w.integrator = Integrator::VelocityVerlet;
    let a = w.add_particle(1.0, Vec3::new(0.0, 0.0, 0.0), true).unwrap();
    let (s, c) = (0.5_f64, 0.75_f64.sqrt()); // sin 30°, cos 30°
    let b = w.add_particle(1.0, Vec3::new(s, -c, 0.0), false).unwrap();
    w.add_rod(a, b, 1.0).unwrap();
    let e0 = w.compute_energy().total();
    for _ in 0..2400 {
        w.step(Seconds(1.0 / 240.0));
    }
    let e = w.compute_energy().total();
    assert!(((e - e0) / e0).abs() < 0.01, "energy went from {e0} to {e}");
}
```

Status: PROPOSED — confirm red at execution. Expected failure: the energy changes by tens of percent. The review measured a 28% loss for a horizontal release, and the 30° case is to be confirmed. If the sign convention of `total()` makes e0 near zero (a potential reference at the anchor), measure relative to the swing's energy instead, and say so in the test comment.

## Scenarios to Demonstrate

- Pendulum with a flat energy chart under Verlet
- Undamped spring under explicit Euler with energy visibly climbing
- Projectile with a 20.39 m range readout
- Error state: entering a negative spring constant shows the bridge's message

## Acceptance rules (the whole build must meet these)

The previous two builds of this work passed review with stubbed code. These rules exist because of specific failures seen there:

- **Red first.** Write the Reproduction Test before changing any code, and observe it fail on the current tree. Record the failure message in the commit body.
- **No hollow assertions.** These are all forbidden:
  - asserting a constant (`assert_eq!(1, 1)`, `assert.strictEqual(1, 1)`)
  - asserting something always true (`x > -1.0`)
  - empty `if let` / loop bodies that check nothing
  - goals that `return true`
  - loosening a tolerance until the test passes
- **Numbers come from this plan, not from the code.** If the code disagrees with a hand-computed number here, the code is wrong. Do not re-derive the expectation from the output.
- **Touch only what's listed.** Before committing, `git diff --stat <start>..HEAD` must show no changes to `rust-toolchain.toml`, `.gitignore`, or crates this plan doesn't name. A previous build silently reverted another plan's work this way. If such a change appears, restore the file from the starting commit.
- **Everything green, and finalize runs:**
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `sh scripts/build-site.sh`
  - `node --test "site/**/*.test.js"`

  No `Codeyam-Finalize: deferred`.
- **No junk or placeholders.** Don't commit scratch, `.patch`, `.orig` or `.rej` files. Don't leave placeholder comments ("would go here", "omitted for brevity", "for prototype", "wait…", "deferred").
- **Every `proves` entry exists.** Each entry in a touched `capabilities.toml` must name a test that exists and passes.