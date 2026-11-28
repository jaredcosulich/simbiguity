---
title: "Physics Lab demo: every engine capability on one page"
mode: ui
createdAt: "2026-11-28T02:43:18Z"
source: manual
dependsOn: ["physics-inverse-square-gravity-and-kepler-checked-orbits"]
---

## Summary

`site/demos/physics/` has four presets: pendulum, spring–mass, projectile and chain. It has an integrator dropdown and an energy chart. Once the collision and gravity plans land, the engine can do far more than the page shows. This plan turns the page into a **Physics Lab** that demonstrates every engine capability, and it lets a learner *see* the integrator story the tests prove.

## Key Decisions

**Presets.** Each preset demonstrates one tested capability:

| Preset | What it shows |
|---|---|
| Pendulum (existing) | energy conservation |
| Double pendulum | chaos; see below |
| Newton's cradle | momentum transfer |
| Bouncing balls | restitution slider 0–1 |
| Ball pit (300 balls) | broadphase speed; the FPS readout must stay ≥ 55 on a laptop |
| Sliding block | friction slider |
| Orbit | integrator choice: explicit Euler visibly spirals out, Verlet doesn't |
| Binary star | two-body gravity |
| Figure-eight three-body | a striking three-body orbit |
| Cloth | a 20×20 spring grid pinned at two corners, with Verlet |

**Chaos made visible.** The double pendulum runs two copies whose starting angles differ by 1e-9 rad. They are drawn in two colours and visibly diverge after a few seconds. A readout shows their angle difference on a log scale.

**Numbers on screen come from the engine.** These readouts are read from `phys_*` exports, never computed in JS:
- energy (kinetic, potential, total)
- momentum
- simulated time
- step time in ms

**Controls.**
- play/pause, single step, and time scale (0.1×–4×)
- a "Reset" button and a "Share" button
- Shareable state in the URL hash, as in `site/demos/hello-circuit/state.js`: preset, integrator, sliders and seed.
- Loading a shared hash reproduces the run bit-for-bit, because the engine is deterministic.

## Implementation

### 1. Bridge

**File**: `crates/simbiguity-physics-wasm/src/lib.rs`

Add:
- `phys_new_double_pendulum(offset)` and `phys_new_cloth(n)`, plus a sliding-block preset;
- setters for restitution, friction and time scale;
- `phys_momentum`;
- `phys_particle_count` and `phys_positions_ptr`, used for the chaos overlay.

Each export gets a Rust test, for example:
- The double pendulum with offset 0 vs 1e-9 differs by < 1e-6 rad at t = 1 s and by > 0.1 rad at t = 20 s. Confirm both thresholds empirically, and record the observed values.
- The cloth's corner pins hold.

### 2. Rendering

**File**: `crates/simbiguity-physics-view/src/lib.rs`

1. Draw balls at their real radius, in a colour per preset group.
2. Draw the cloth as a triangle mesh, with normals recomputed each frame, not as cylinders.
3. Draw orbit trails as a polyline of the last 500 positions.

### 3. Page

**Files**:
- `site/demos/physics/index.html`
- `demo.js`
- `state.js`
- `state.test.js`

Wire the presets, sliders, readouts, energy chart and the share hash described above. The energy chart uses the existing `#chart` and `simbiguity-plot`.

### 4. Node tests

**File**: `site/demos/physics/state.test.js`

1. Hash round trip for every preset.
2. Load the WASM in Node, run the orbit preset for 10 orbits with explicit Euler and with Verlet, and assert:
   - Euler's total energy rose by > 1%;
   - Verlet's drifted by < 1e-6.

### 5. Scenarios

Capture six distinct scenarios through URL state:
- the cradle mid-swing;
- the ball pit settled;
- the orbit with the Euler spiral;
- the double pendulum diverged;
- the cloth draped;
- the figure-eight.

All six screenshot hashes must differ.

## Reused existing code

- `PhysicsView` (`crates/simbiguity-physics-view/src/lib.rs`)
- `WebGL2Executor` (`site/js/webgl2-executor.js`)
- `renderLabels` (`site/js/plot-labels.js`)
- the hash state pattern from `site/demos/hello-circuit/state.js`
- the presets from the collisions and gravity plans

## Reproduction Test

`orbit_preset_energy_drift_euler_vs_verlet` (Node) fails today, because there is no orbit preset in the bridge.

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