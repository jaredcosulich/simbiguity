---
title: "Foundations: simbiguity-math and simbiguity-core (zero-dependency math, Simulation contract, fixed clock, seeded RNG, snapshots)"
mode: backend
createdAt: "2026-11-26T12:42:15Z"
source: manual
---

## Summary

Lay the two foundation crates every simbiguity engine builds on: **`simbiguity-math`** (vectors, matrices, quaternions, and a dense linear solver) and **`simbiguity-core`** (the shared contracts that let engines compose: the `Simulation` trait, a fixed-timestep clock, a deterministic seeded RNG, and snapshot/restore). Both have zero external dependencies, are `#![no_std]` + `alloc`, and are independently importable. The circuit solver, physics, markets and animation crates all depend on these. Getting the contracts right here is what makes "a circuit drives a motor that spins a rigid body, stepped in lockstep" possible later.

## Key Decisions

- **Zero dependencies, `no_std` + `alloc`.** Engines must run in the browser (WASM), natively, and on small devices. `std` is only enabled behind an opt-in `std` feature (for `Error` impls and float intrinsics). Without `std`, float functions (`sqrt`, `sin`, …) need a source. Provide them in `simbiguity-math::float` as small, documented, tested implementations (or `core::intrinsics`-free polynomial approximations), rather than pulling `libm`.
- **`f64` by default, generic later only if needed.** Simulation accuracy matters more than GPU-friendliness at this layer. The renderer converts to `f32` at its boundary. No premature `Scalar` trait.
- **The `Simulation` contract.** `trait Simulation { type State; type Input; fn step(&mut self, dt: Seconds); fn apply(&mut self, input: Self::Input); fn state(&self) -> &Self::State; }`, plus `Snapshot: Simulation` with `snapshot()` / `restore()`. Deterministic: the same inputs at the same steps give bit-identical states. This is what makes puzzle checking, undo, and replay exact.
- **Fixed-timestep clock with an accumulator.** `FixedClock::advance(real_dt) -> StepCount` yields whole fixed steps plus an interpolation alpha for rendering. Several simulations share one clock to run in lockstep.
- **RNG: a documented PCG32 (or SplitMix64 seeding xoshiro256\*\*), seeded explicitly.** There is no global or thread RNG: every random draw is reproducible from the seed.
- **Dense LU with partial pivoting** is the first solver: it is correct, readable, and plenty for puzzle-sized circuits (tens of nodes). Report singular systems as a typed error, because the circuit crate turns that into "short circuit" or "floating node" messages. Sparse solvers come later if a demo needs them.
- **Readable source is a feature.** Each public item carries a doc comment that teaches (what a quaternion is for, why pivoting matters), since learners read this code.

## Implementation

### 1. `simbiguity-math`

**New file**: `crates/simbiguity-math/Cargo.toml`
**New file**: `crates/simbiguity-math/src/lib.rs`
**New file**: `crates/simbiguity-math/src/vec.rs`: `Vec2`, `Vec3` (add/sub/scale, dot, cross, length, normalize, lerp)
**New file**: `crates/simbiguity-math/src/quat.rs`: `Quat` (axis-angle, multiply, rotate vector, slerp, normalize)
**New file**: `crates/simbiguity-math/src/mat.rs`: `Mat4` (identity, multiply, translate/rotate/scale, perspective, look_at, inverse), plus a heap `DenseMatrix` for solvers
**New file**: `crates/simbiguity-math/src/linsolve.rs`: `lu_solve(a, b) -> Result<Vec<f64>, SolveError>` with partial pivoting; `SolveError::Singular { pivot_row }`
**New file**: `crates/simbiguity-math/src/float.rs`: `no_std` float functions with documented error bounds

### 2. `simbiguity-core`

**New file**: `crates/simbiguity-core/Cargo.toml`
**New file**: `crates/simbiguity-core/src/lib.rs`
**New file**: `crates/simbiguity-core/src/simulation.rs`: `Simulation`, `Snapshot`, `Seconds` newtype
**New file**: `crates/simbiguity-core/src/clock.rs`: `FixedClock` (accumulator, max-steps-per-frame guard against the "spiral of death", interpolation alpha)
**New file**: `crates/simbiguity-core/src/rng.rs`: seeded RNG (`next_u32`, `next_f64` in [0,1), `range`, `normal` via Box–Muller)
**New file**: `crates/simbiguity-core/src/lockstep.rs`: a tiny `Lockstep` runner that steps several `Simulation`s on one clock, showing composition

### 3. Wire into the umbrella crate

**File**: `crates/simbiguity/src/lib.rs`

Re-export `math` and `core` behind default-on features (`math`, `core`), so `simbiguity = { default-features = false, features = ["math"] }` works.

### 4. Register the new crates with codeyam

Re-run `codeyam-editor editor configure-test-commands` so each crate gets a test partition with `dependsOn` from the manifest graph, then `bootstrap-registry`.

## Reused existing code

- Greenfield: no engine code exists yet. Uses the workspace settings in `Cargo.toml` (`workspace.package`, the `crates/*` member glob) and the umbrella crate `crates/simbiguity/src/lib.rs`.

## Tests (TDD — written first)

- Vector/quaternion identities: `a × b ⟂ a`, `|q·v| = |v|` for unit `q`, slerp endpoints, `Mat4 · inverse ≈ I`.
- `lu_solve`: a known 3×3 system; a system that needs pivoting (zero on the diagonal); a singular matrix → `Singular`; a 1×1; an empty system.
- `float`: `sqrt`/`sin`/`cos`/`exp` within the documented error bound across ranges, including edge values (0, negative, large).
- `FixedClock`: exact step counts for uneven frame times; the alpha is in [0,1); a long stall is capped at max steps.
- RNG: the same seed gives the same sequence; different seeds diverge; the `range` bounds hold; the mean/variance of `normal` are near (0,1) over N draws (deterministic, so the test is exact).
- `Snapshot`: step → snapshot → step → restore → step gives the identical state to stepping straight through.
- `Lockstep`: two toy simulations stay in step across uneven frames.

## Scenarios to Demonstrate

- Library-only plan, no visual surface yet. The public test-results page (a later plan) is where these tests become visible.