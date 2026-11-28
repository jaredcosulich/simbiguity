---
title: "Physics: inverse-square gravity and Kepler-checked orbits"
mode: backend
createdAt: "2026-11-28T02:43:18Z"
source: manual
dependsOn: ["physics-collisions-restitution-friction-and-a-broadphase"]
---

## Summary

The physics engine's only gravity is a uniform field: `Gravity` in `crates/simbiguity-physics/src/forces.rs:4`. This plan adds inverse-square gravity, both as attraction to a fixed point mass and as mutual N-body attraction. With it the engine can simulate orbits, slingshots and a solar system. Orbits are also the sharpest test of an integrator: Kepler's laws give exact answers, and explicit Euler visibly spirals outward.

## Key Decisions

**Force law.**
- F = G·m₁·m₂·r / (|r|² + ε²)^(3/2), with softening ε configurable (default 0) so close passes don't blow up.
- Use `World::add_attractor(position, gm)` for a fixed central body.
- Use `World::mutual_gravity = Some(G)` for N-body.

**Direct O(n²) summation over pairs.**
- It is deterministic and exact. A Barnes–Hut tree is out of scope.
- Cap the particle count at 2,000 and return `PhysicsError` above it.
- Sum in index order so results are bit-reproducible.

**Energy includes gravitational potential.** `compute_energy` gains −G·m₁·m₂/r terms, so the conservation tests are meaningful.

## Implementation

### 1. Forces

**File**: `crates/simbiguity-physics/src/forces.rs`

Add the attractor and mutual gravity. Accumulate them in the existing force pass in `World::step`, so every integrator picks them up.

### 2. Energy

**File**: `crates/simbiguity-physics/src/energy.rs`

Add the potential terms. Test it on two unit masses 2 m apart with G = 1: the potential energy is −0.5.

### 3. Reference tests

**File**: `crates/simbiguity-physics/src/gravity_tests.rs` (new)

All use GM = 1 unless stated:

| Case | Expected |
|---|---|
| Circular orbit, r = 1, v = 1, Verlet, dt = 1e-3 | period (measured between successive upward y crossings) = 2π = 6.28319 ± 1e-3; radius stays within 1e-4 of 1 for 10 orbits |
| Same, energy | relative drift < 1e-6 after 10 orbits (symplectic, bounded) |
| Same with explicit Euler | energy increases by > 1% after 10 orbits, and the radius grows. This documents the failure. |
| Ellipse: perihelion r = 1, v = √1.5 = 1.224745 (so a = 2) | period 2π·a^1.5 = 17.7715 ± 1e-2; aphelion 3.000 ± 1e-3 |
| Kepler's 2nd law on that ellipse | the area swept in equal 0.5 s windows is equal to 1e-4 relative at perihelion and aphelion |
| Two-body, mutual G = 1, masses 1 and 1 on a circular binary of separation 2 (each at v = 0.5) | centre of mass fixed to 1e-12; total momentum zero to 1e-12; separation 2 ± 1e-4 over 10 orbits |
| Figure-eight three-body (Chenciner–Montgomery initial conditions, G = 1, m = 1) | after one period T = 6.32591 the positions return to the start within 1e-3. Take the exact initial values from the published paper, and cite it in a comment. |

### 4. Bridge presets

**File**: `crates/simbiguity-physics-wasm/src/lib.rs`

Add `phys_new_orbit`, `phys_new_binary` and `phys_new_figure_eight`. Each gets a bridge test that runs one period and checks that the first particle returns near its start.

### 5. Capabilities

Add "Gravity and orbits (Kepler-checked)" to `crates/simbiguity-physics/capabilities.toml`.

## Reused existing code

- `World::step` and the integrators in `crates/simbiguity-physics/src/integrate.rs` (Verlet, symplectic Euler, explicit Euler)
- `compute_energy`
- `Vec3`
- the preset pattern in `crates/simbiguity-physics-wasm/src/lib.rs` (`phys_new_pendulum` at line 94)

## Reproduction Test

`circular_orbit_period_is_two_pi` fails to compile today, because there is no `add_attractor`.

## Acceptance rules (the whole build must meet these)

Three earlier builds of this work passed their own checks while shipping stubs. Each rule below comes from a real failure:

- **Red first.** Write the tests named in this plan before the code. Run them on the starting tree and see them fail. Put the failure message in the commit body.
- **No hollow assertions.** All of these are forbidden:
  - asserting a constant (`assert_eq!(1, 1)`, `assert.strictEqual(typeof x, "string")` standing in for the real check)
  - asserting something that is always true (`x > -1.0`)
  - empty `if let` or loop bodies that check nothing
  - a goal or check that returns a hard-coded `true` or `false`
  - loosening a tolerance until the test passes
- **No stubs that only change shape.** Replacing `return true` with `return false`, or one constant with another, is not an implementation. So is a parser that ignores its input, or an encoder that copies its input. Placeholder comments are forbidden too: "stub", "would go here", "a real X would…", "once the TDD phase starts", "for prototype", "wait…", "omitted for brevity", "deferred".
- **Numbers come from this plan, not from the code.** The reference values here were worked out by hand or with an independent script. If the code disagrees with one, the code is wrong. Do not take the expected value from the code's own output.
- **No empty files, no fake screenshots.**
  - Every file this plan creates must have real content.
  - Each scenario must show a different state. Its screenshot must differ from every other scenario's (no identical hashes).
- **Budgets are not raised to fit.** Don't edit `site/pkg-budget.txt` or any bench budget upward to make a check pass. Make the code smaller or faster instead. If a budget truly must move, say why in the commit body and cap the increase at 5%.
- **Touch only what's listed.** Before committing, `git diff --stat <start>..HEAD` must show no changes to:
  - `rust-toolchain.toml`
  - `.gitignore`
  - crates or demos this plan doesn't name

  If anything like that shows up, restore it from the starting commit.
- **Everything green, and finalize runs.** No `Codeyam-Finalize: deferred`. All five must pass:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `sh scripts/build-site.sh` (it must exit 0)
  - `node --test "site/**/*.test.js"`
- **No junk.** Don't commit `patch.diff`, scratch files, or `.patch`, `.orig` or `.rej` files.
- **Every `proves` entry exists.** In each touched `capabilities.toml`, every entry must name a test that exists and passes. Every new capability must be listed there, so it appears on the status page.
- **Done means demonstrable.** The last commit's body must list each Implementation section and the test(s) that prove it, one line per section. If a section is not done, say so plainly. Do not mark the plan complete.