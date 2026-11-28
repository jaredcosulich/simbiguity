---
title: "Physics: rigid bodies that spin, roll and tip"
mode: backend
createdAt: "2026-11-28T16:56:22Z"
source: manual
dependsOn: ["physics-damped-and-driven-oscillators-checked-against-resonance-theory"]
---

## Summary

The physics engine simulates point particles only. `Particle` (`crates/simbiguity-physics/src/world.rs:9`) has position and velocity, but no orientation, angular velocity or inertia. So it can't show anything that rolls, spins or tips.

This plan adds **3D rigid bodies** (spheres and boxes) with full rotational dynamics, and contacts against the ground with friction. Tests check classic results:
- rolling without slipping;
- the tennis-racket (intermediate-axis) instability;
- conservation of angular momentum.

## Key Decisions

**A separate `RigidBody` type.**
- Fields: mass, inverse inertia tensor (body frame), position, orientation `Quat`, linear and angular velocity.
- It lives in `World::bodies`, so particle code stays simple.
- Shapes: `Sphere { r }` and `Box { half_extents }`.

**Integration.**
- Symplectic Euler for linear motion.
- For rotation, integrate angular momentum in the world frame, and update orientation with q ← normalize(q + ½·dt·ω⊗q).
- Add the gyroscopic term so torque-free precession is right.
- Renormalise the quaternion every step.

**Contacts.**
- Sphere–ground and box–ground: test the box's 8 corners.
- Resolve with sequential impulses, including friction, at the contact point. Use r × J for the angular part.
- Body–body contacts are out of scope here. Leave a clear `NotSupported` error, not a silent pass-through.

## Implementation

### 1. Rigid body type and inertia

**File**: `crates/simbiguity-physics/src/rigid.rs` (new)

Inertia formulas:
- solid sphere: (2/5)·m·r²
- box: (m/12)·(b² + c²) and so on

Test: a box 1×2×3 m with m = 1 has I = diag(1.08333, 0.83333, 0.41667) ± 1e-5.

### 2. Integrator and ground contacts

**Files**:
- `crates/simbiguity-physics/src/rigid.rs`
- `world.rs`

Add the integrator, gyroscopic term and contact resolution described in Key Decisions.

### 3. Reference tests

**File**: `crates/simbiguity-physics/src/rigid_tests.rs` (new)

| Case | Expected |
|---|---|
| Torque-free box spinning about a random axis, 10 s, dt = 1e-3 | world angular momentum conserved to 1e-9 relative; kinetic energy to 1e-6 |
| Spin about the largest-inertia axis (+ 1e-3 perturbation) | stays within 1° of the axis for 20 s |
| Spin about the smallest-inertia axis | stays within 1° of the axis for 20 s |
| Spin about the intermediate axis (+ 1e-3 perturbation) | the axis flips (> 150° from start) at least once within 20 s: the tennis-racket effect |
| Solid sphere rolling down a 30° ramp, μ = 0.5 (the ramp is a tilted ground plane, or the ground with gravity tilted) | acceleration along the slope (5/7)·g·sin θ = 3.5036 m/s² ± 1%; contact-point slip speed < 1e-3 m/s |
| Same with μ = 0.05 (less than (2/7)·tan θ = 0.165) | it slips; acceleration g·(sin θ − μ cos θ) = 4.4802 m/s² ± 1% |
| Box resting on the ground, 10 s | rests still: \|v\| < 1e-6, no drift, no jitter > 1e-6 m |
| Box dropped corner-first from 1 m, e = 0.3 | comes to rest flat on a face (one axis within 1° of vertical) within 5 s |

### 4. View and bridge

**Files**:
- `crates/simbiguity-physics-view/src/lib.rs`
- `crates/simbiguity-physics-wasm/src/lib.rs`

1. Draw rigid bodies with their orientation (box and sphere meshes), plus an axis gizmo option.
2. Add presets `phys_new_tennis_racket`, `phys_new_rolling_ramp` and `phys_new_box_drop`, each with a bridge test.
3. Watch the physics WASM budget.

### 5. Capabilities

Add "Rigid bodies: rotation, rolling, tipping".

## Reused existing code

- `Quat`, `Mat4` and `Vec3` from `simbiguity-math`
- `Ground { restitution, friction }` and the impulse solver in `crates/simbiguity-physics/src/collide.rs`
- `PhysicsView` meshes (`Mesh::sphere`, `Mesh::cylinder`; add `Mesh::cuboid` in `simbiguity-render` if it doesn't exist)

## Reproduction Test

`box_inertia_tensor_matches_formula` fails to compile today, because there's no `RigidBody`.

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