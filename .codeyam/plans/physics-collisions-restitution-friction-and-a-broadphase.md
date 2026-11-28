---
title: "Physics: collisions, restitution, friction and a broadphase"
mode: backend
createdAt: "2026-11-28T02:43:18Z"
source: manual
dependsOn: ["round-3-site-build-under-budget-no-junk-and-a-determinism-test-that-can-fail"]
---

## Summary

Particles in `crates/simbiguity-physics` pass straight through each other.
- `Particle` (`crates/simbiguity-physics/src/world.rs:9`) has mass, position and velocity, but no size.
- The only contact is `Ground { y, restitution }` (`crates/simbiguity-physics/src/constraints.rs:9`). It has no friction, so a ball rolling on it never stops.

This plan adds:
- particle radius;
- sphere–sphere collisions with restitution;
- Coulomb friction on the ground;
- a uniform-grid broadphase, so hundreds of balls stay fast.

The tests are textbook collision problems with closed-form answers.

## Key Decisions

**Impulses, resolved after integration.**
- After each `step`, find overlapping pairs and push them apart along the contact normal, splitting the push by inverse mass.
- If they are approaching, apply the impulse j = −(1 + e)·v_rel·n / (1/m₁ + 1/m₂).
- Pinned particles have infinite mass.

**Iterate to settle stacks and chains.** Run up to 8 passes over the contacts each step, stopping early when no impulse exceeds 1e-12. This is what makes Newton's cradle hand momentum down the line. Confirm by experiment that 8 passes reproduce the cradle result below. If they don't, raise the pass count instead of loosening the test.

**Restitution per pair.** A pair uses min(e₁, e₂).

**Ground friction is Coulomb.**
- The tangential impulse is limited to μ·(normal impulse).
- While resting on the ground, apply kinetic friction μ·m·g opposite to the sliding velocity, and stop exactly at zero (no oscillating sign flips).

**Broadphase: a uniform spatial hash.**
- The cell size is 2 × the maximum radius.
- Iterate pairs in deterministic order: sort by (i, j).
- It must find exactly the same pairs as brute force.

## Implementation

### 1. Radius and material

**File**: `crates/simbiguity-physics/src/world.rs`

1. Add `radius` and `restitution` to `Particle`.
2. Keep `add_particle`'s signature, defaulting the radius to 0, which means "point: does not collide".
3. Add `add_ball(mass, pos, radius, restitution)`.
4. Add `friction` to `Ground`.

### 2. Narrowphase, broadphase and resolution

**New file**: `crates/simbiguity-physics/src/collide.rs`

This holds the contact search and the iterative impulse solver.

### 3. Reference tests

**File**: `crates/simbiguity-physics/src/collide_tests.rs` (new)

All 1-D cases are set up along x with gravity off:

| Case | Expected |
|---|---|
| Elastic (e = 1): m₁ = 1 at v = 2 hits m₂ = 3 at rest | v₁' = −1.000, v₂' = 1.000 (± 1e-9); momentum 2.0 conserved to 1e-12; kinetic energy 2.0 conserved to 1e-9 |
| Perfectly inelastic (e = 0), same masses | both 0.500 ± 1e-9 |
| Equal masses, elastic, head-on (+1 and −1) | the velocities swap exactly |
| Newton's cradle: 5 balls, radius 0.1, touching, e = 1; the first at v = 1 | after contact the last moves at 1.000 ± 1e-6 and the other four have \|v\| < 1e-6 |
| Drop from h = 1 m onto the ground, e = 0.5, g = 9.81, dt = 1/1000 | first rebound peak 0.250 m ± 1% (rebound speed = e·√(2gh)) |
| Slide on the ground at v₀ = 2 m/s, μ = 0.5 | stops after d = v₀²/(2μg) = 0.4077 m ± 1% and then stays exactly still |
| 400 balls dropped into a box made of 4 pinned walls of balls, 10 s | no pair overlaps by more than 1% of the radius at the end; total momentum in x and z stays < 1e-9 |

Broadphase equivalence: across 100 seeds of 300 random balls, the spatial hash returns exactly the pair set of the O(n²) search.

### 4. Bridge and bench

**Files**:
- `crates/simbiguity-physics-wasm/src/lib.rs`
- `xtask/src/bench_fixtures.rs`

1. Add `phys_new_cradle` and `phys_new_ball_pit(count)` presets.
2. Add the bench fixture `ball_pit_1000_one_second`, with a budget at the measured time × 1.5.
3. The physics WASM must stay within its 180,000-byte budget.

### 5. Capabilities

**File**: `crates/simbiguity-physics/capabilities.toml`

It currently has a single entry named "physics". Split it into real capabilities:
- integrators;
- springs and rods;
- collisions and restitution;
- friction.

Each one must be proven by named tests.

## Reused existing code

- `World::step`, `Ground` and the restitution handling at `crates/simbiguity-physics/src/world.rs:310-315`
- `Vec3` from `simbiguity-math`
- `compute_energy` (`crates/simbiguity-physics/src/energy.rs`)
- the seeded RNG in `simbiguity-core`

## Reproduction Test

`elastic_collision_one_kg_into_three_kg_bounces_back_at_minus_one` fails today: the balls pass through each other and v₁ stays 2.

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