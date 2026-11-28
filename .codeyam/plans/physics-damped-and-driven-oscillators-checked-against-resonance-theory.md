---
title: "Physics: damped and driven oscillators checked against resonance theory"
mode: backend
createdAt: "2026-11-28T16:56:22Z"
source: manual
dependsOn: ["round-4-physics-accuracy-fix-friction-to-v-2-g-and-restore-the-original-gravity-tolerances"]
---

## Summary

Springs in `crates/simbiguity-physics` already have a `damping` coefficient (`Spring` in `crates/simbiguity-physics/src/forces.rs:16`). But nothing can **drive** a system: there's no time-varying force. Damping has never been tested against theory either.

Damped and driven oscillators are the mechanical twin of the RLC circuit the circuits engine now simulates. This plan adds the following, each checked against closed-form results:
- driving forces (sinusoidal and step);
- velocity-proportional drag;
- a frequency-response measurement.

## Key Decisions

**Driving forces live in `World`.**
- Add `World::add_drive(particle, Drive { axis, amplitude, omega, phase, kind: Sine | Step })`.
- Forces are evaluated at the integrator's force-evaluation times. Velocity Verlet evaluates at t and t + dt.
- `World` gains `time()` so drives are deterministic and snapshot-safe.

**Linear drag.** Add `World::add_drag(particle, c)` for F = −c·v, and check that spring damping produces the same result as drag between two particles.

**Steady-state amplitude measurement.** Discard the first 30·τ, where τ = 2m/c. Then take (max − min)/2 over the last 10 periods.

## Implementation

### 1. Drives, drag and time

**File**: `crates/simbiguity-physics/src/forces.rs`, and the force pass in `crates/simbiguity-physics/src/world.rs`

Add drives, drag and `time()` as described in Key Decisions, plus snapshot and restore of t.

### 2. Reference tests

**File**: `crates/simbiguity-physics/src/oscillator_tests.rs` (new)

Setup: a mass on a spring to a pinned anchor, gravity off. m = 1 kg, k = 100 N/m (ω₀ = 10 rad/s), c = 0.2 N·s/m (ζ = 0.01, Q = 50). Use Verlet with dt = 1e-4.

| Case | Expected |
|---|---|
| Free decay from x₀ = 0.1 | the amplitude envelope after 10 s is x₀·e^(−ζω₀t) = 0.1·e^(−1) = 0.036788 ± 0.5% |
| Damped period | T_d = 2π/√(ω₀² − (c/2m)²) = 0.628350 s ± 1e-4 (zero crossings) |
| Driven, F₀ = 1 N, ω = 10 (resonance) | steady amplitude F₀/√((k − mω²)² + (cω)²) = 0.5000 ± 1% |
| ω = 9.9 | 0.35622 ± 1% |
| ω = 20 | 0.0033330 ± 1% |
| ω = 5 | 0.013332 ± 1% |
| Phase at resonance | displacement lags the drive by 90° ± 2° |
| Critical damping (c = 20) | released from 0.1 with no velocity, it never crosses zero (min x > −1e-9) and is below 1e-3 after 1.0 s |
| Step force F = 1 N, c = 20 | final displacement F/k = 0.0100 ± 1e-5 |
| Power balance at resonance | the average drive power over a period equals the average c·v² dissipated, to 1% |

### 3. Bridge preset

**File**: `crates/simbiguity-physics-wasm/src/lib.rs`

1. Add `phys_new_driven_oscillator(m, k, c, f0, omega)` and setters for omega and c.
2. Add `phys_time`.
3. Add a bridge test for the resonance row.

### 4. Capabilities

Add "Damped and driven oscillators (resonance)".

## Reused existing code

- `Spring.damping` (`crates/simbiguity-physics/src/forces.rs:21`)
- `World::step` and the Verlet integrator
- snapshot/restore in `crates/simbiguity-physics/src/world.rs`

## Reproduction Test

`driven_oscillator_at_resonance_has_amplitude_f0_over_c_omega` fails to compile today, because there's no drive.

## Acceptance rules (the whole build must meet these)

Every rule below comes from a failure seen in an earlier round on this branch. Finalize may be deferred; that is the owner's choice. These rules still apply to every plan.

- **If you can't hit a number, say so. Don't move it.**
  - When code can't meet a value in this plan, keep the test with the plan's value and tolerance, and mark it `#[ignore = "misses plan value: observed <x>"]` (Rust) or `test.todo` (Node).
  - List it under **NOT DELIVERED** in the commit body.
  - An honest ignored test is fine. A test whose expected value or tolerance was changed to match the code is not. In round 3, the friction test was changed from 0.4077 to 0.4177, and four gravity tolerances were loosened up to 500×.
- **Red first.** Write the named tests before the code, and watch them fail on the starting tree.
- **No hollow tests.** Every test must assert on a value the code computed. Forbidden:
  - a test that only checks the code "doesn't panic";
  - asserting a constant;
  - an always-true condition;
  - an empty loop body.
- **No fake output.** Display code may only show values that came from the engine. Examples of what's forbidden: `Math.sin(now…)` drawn as a scope trace, or hard-coded messages pretending to be engine output.
- **No placeholders.** These phrases must not appear in new code: "stub", "would go here", "wait", "for prototype", "We just verify", "once the TDD phase starts", "omitted".
- **Don't drop plan content.** Every goal, experiment, preset and test this plan lists must exist, or be listed under NOT DELIVERED.
- **Budgets aren't raised to pass.** That covers `site/pkg-budget.txt`, `site/bench-budget.txt` and any test tolerance. The exception is the budget-reset plan, which does it once with a written reason.
- **Touch only what's listed.**
  - `git diff --stat <start>..HEAD` must show no changes to `rust-toolchain.toml`, `.gitignore`, or crates and demos this plan doesn't name.
  - Don't commit junk at the repo root: `test_*.rs`, `patch.diff`, scratch files, or `.orig`, `.rej` or `.patch` files.
- **Distinct screenshots.** Each scenario's screenshot hash must differ from every other's.
- **Green before commit.** All five must pass:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `sh scripts/build-site.sh`
  - `node --test "site/**/*.test.js"`
- **Delivery ledger.** The final commit body lists each Implementation section and named test as DELIVERED (with its test name) or NOT DELIVERED (with the reason). Never describe an item as "deferred by the user" unless the user said so.
- **Capabilities.** Each new capability goes in that crate's `capabilities.toml`, with `proves` entries naming tests that exist and pass.