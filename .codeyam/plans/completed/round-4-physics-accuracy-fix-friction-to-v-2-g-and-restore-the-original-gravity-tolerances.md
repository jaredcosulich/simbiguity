---
title: "Round 4: physics accuracy — fix friction to v²/2μg and restore the original gravity tolerances"
mode: backend
createdAt: "2026-11-28T16:56:22Z"
source: manual
dependsOn: ["round-4-trustworthy-gates-bench-check-that-can-fail-honest-size-budgets-junk-and-duplicate-screenshot-tests"]
---

## Summary

Round 3's physics tests were adjusted to pass instead of the code being fixed.

**Friction.**
- A ball sliding at 2 m/s on ground with μ = 0.5 must stop after v₀²/(2μg) = 4/9.81 = **0.4077 m**.
- `ground_friction_slide` (`crates/simbiguity-physics/src/collide_tests.rs:170`) asserts `0.4177 ± 0.005` instead.
- The engine overshoots by 10 mm (2.5%), most likely because of the order of the friction impulse at `crates/simbiguity-physics/src/collide.rs:153-162` relative to the position update. At dt = 0.01, a v·dt/2 ordering error is about 0.01 m.

**Gravity tolerances loosened** in `crates/simbiguity-physics/src/gravity_tests.rs`:
- circular period 1e-3 → 5e-3;
- aphelion 1e-3 → 5e-3;
- figure-eight return 1e-3 → 5e-3;
- Kepler's second law 1e-4 → 5e-2.

**The Kepler test is also wrong in itself.** At perihelion it uses `0.5 * pos0.x * pos1.y` with the comment "approximate": a triangle, not the swept area.

**The ellipse period of 17.7715 is never asserted.**

This plan fixes the engine to meet the original numbers and restores the original tests.

## Key Decisions

- **Fix the friction ordering, not the number.**
  - Apply the friction impulse to velocity before the position update in the same step, the same way the normal-contact impulse is handled.
  - Clamp so the velocity stops exactly at zero, without reversing.
- **Measure swept area properly.** Sum the triangle areas ½|rₖ × rₖ₊₁| over every step in the window. This is exact for the polygon traced, and converges to the true area as dt → 0.
- **Measure the period by crossings.** Find successive upward crossings of y = 0 on the x > 0 side, linearly interpolated between steps.
- **If Verlet at dt = 1e-3 truly can't reach a tolerance, follow the ignore rule.** Mark the test ignored and record the observed value. Don't loosen it. Expected values for velocity Verlet at this dt are far inside these tolerances, so a failure points to an engine bug (force evaluated at the wrong time, or the attractor applied twice).

## Implementation

### 1. Fix ground friction

**File**: `crates/simbiguity-physics/src/collide.rs`

Fix the impulse ordering as described in Key Decisions.

### 2. Restore the friction test

**File**: `crates/simbiguity-physics/src/collide_tests.rs`

| Case | Expected |
|---|---|
| dt = 1e-3 | stop distance 0.4077 ± 0.002 m |
| dt = 1e-2 | 0.4077 ± 0.006 m (1.5%) |
| both cases | the velocity is exactly 0.0 afterwards, and stays 0.0 for another 100 steps |
| μ = 0.25 | 0.8155 ± 0.004 m at dt = 1e-3 |

### 3. Restore the gravity tests

**File**: `crates/simbiguity-physics/src/gravity_tests.rs`

All use Verlet and dt = 1e-3.

| Test | Expected |
|---|---|
| Circular period | 2π ± 1e-3 |
| Ellipse | perihelion r = 1, v = √1.5 = 1.224745 |
| Ellipse period | 17.7715 ± 1e-2 (crossing-based) |
| Aphelion | 3.000 ± 1e-3 |
| Kepler's second law | the swept area in a 0.5 s window at perihelion equals that at aphelion to 1e-4 relative. Both should equal ½·h·0.5, where h = r·v = 1.224745, so 0.306186 ± 1e-4 relative. |
| Figure-eight | return to start within 1e-3 after T = 6.32591 |

### 4. A regression test for the round-3 shortcut

Add `stop_distance_matches_v_squared_over_two_mu_g`. It sweeps v₀ ∈ {1, 2, 4} and μ ∈ {0.25, 0.5, 1.0} at dt = 1e-3, each within 0.5% of v₀²/(2μg).

## Reused existing code

- `World::step`
- `collide.rs`
- `Ground { friction }`
- `add_attractor`
- the integrators in `crates/simbiguity-physics/src/integrate.rs`

## Reproduction Test

`ground_friction_slide` with the plan value 0.4077 ± 0.002 at dt = 1e-3 fails on the current tree: it observes about 0.418.

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