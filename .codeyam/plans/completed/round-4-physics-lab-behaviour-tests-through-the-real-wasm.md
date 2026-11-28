---
title: "Round 4: Physics Lab behaviour tests through the real WASM"
mode: ui
createdAt: "2026-11-28T16:56:22Z"
source: manual
dependsOn: ["round-4-physics-accuracy-fix-friction-to-v-2-g-and-restore-the-original-gravity-tolerances"]
---

## Summary

The Physics Lab (`site/demos/physics/`) wires all 12 presets to real engine presets. But its only tests are two URL-parsing checks in `site/demos/physics/state.test.js`.

The round-3 plan's key behavioural test is missing: on the orbit preset, explicit Euler gains energy and Verlet doesn't. Also missing:
- double-pendulum divergence through the bridge;
- share-hash replay.

This plan adds Node tests that run the real physics WASM, plus the readouts those tests rely on.

## Key Decisions

- **Test through the same API the page uses.** Factor the `Physics` class out of `demo.js` into `site/demos/physics/physics.js` (new). Then `demo.js` and the tests share it, and the tests load the `.wasm` bytes with `fs.readFile`.
- **Replay is bit-exact.** Two `Physics` instances built from the same hash and stepped the same way produce identical `Float64Array`s of positions. This uses `Object.is` on every element, not a tolerance.

## Implementation

### 1. Extract `physics.js`

**Files**:
- `site/demos/physics/physics.js` (new)
- `demo.js`

Move the class out of `demo.js`. Make the loader accept bytes.

### 2. Node tests

**File**: `site/demos/physics/physics.test.js` (new)

| Test | Expected |
|---|---|
| Orbit, explicit Euler, 10 orbits (dt = 1e-3) | total energy rose > 1% |
| Orbit, Verlet, same | relative drift < 1e-6 |
| Double pendulum, offset 0 vs 1e-9 | angle difference < 1e-6 rad at 1 s, > 0.1 rad at 20 s (dt = 1/240) |
| Cradle | after the first impact, the last ball's speed is 1.000 ± 1e-3 |
| Cloth 20×20 | the two pinned corners haven't moved after 5 s (exact equality) |
| Replay | the same hash gives a bit-identical position buffer after 1000 steps |
| Every preset | builds and steps 100 times without an error status |

### 3. Readouts

Shown on the page: energy (KE, PE and total), momentum, and simulated time, all from `phys_*` exports. Add any missing bridge exports, each with a Rust test.

### 4. Scenarios

Capture six distinct scenarios through URL state:
- the cradle mid-swing;
- the ball pit settled;
- the Euler orbit spiral;
- the double pendulum diverged;
- the cloth;
- the figure-eight.

All six hashes must differ.

## Reused existing code

- the `Physics` class in `site/demos/physics/demo.js`
- `parseState` in `site/demos/physics/state.js`
- the `phys_new_*` presets in `crates/simbiguity-physics-wasm/src/lib.rs`

## Reproduction Test

`orbit_explicit_euler_gains_energy_verlet_does_not` (Node) fails today, because `physics.js` doesn't exist.

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