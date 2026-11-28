---
title: "Round 4: Circuit Lab with a real oscilloscope and all six experiments"
mode: ui
createdAt: "2026-11-28T16:56:22Z"
source: manual
dependsOn: ["round-4-trustworthy-gates-bench-check-that-can-fail-honest-size-budgets-junk-and-duplicate-screenshot-tests"]
---

## Summary

Round 3 shipped a Circuit Lab page whose oscilloscope is fake.
- `site/demos/circuit-lab/lab.js:45` draws `scopeView.draw([0, Math.sin(now / 200), 1, Math.sin((now+100) / 200)])`. That's a made-up wave, with no engine data.
- `ScopeView` (`components/scope-view.js`) ignores its second channel and its trigger argument. It scales by a fixed `* 10`.

Five of the six planned experiments don't exist. `experiments.js` defines only `led`, and carries a "Wait, board_layout.validate checks wires!" comment.

The Node tests the plan named don't exist either:
- scope trigger;
- LED at 20 mA;
- RLC sweep peak;
- rectifier peak;
- hash round trip.

The engine side is real and tested: diodes, LEDs, `SignalSource`, probes, the sparse solver, and `Sim.addProbe` in `site/js/simbiguity.js:110`. The 3D board draws diodes, LEDs and sources (`crates/simbiguity-circuits-view/src/board.rs:93-100`). This plan connects them for real.

## Key Decisions

**The scope reads engine probes only.**
- Each experiment builds its circuit with `simbiguity.js` (`Circuit`/`Sim`) and attaches two probes.
- Each animation frame advances the sim by the frame's simulated time in fixed sub-steps of period/200.
- `ScopeView.draw(ch1, ch2, settings)` receives the probes' `Float64Array` samples.
- The 3D board is driven from the same sim's state, so the board and the scope always agree.

**A proper scope.**
- Two channels in distinct colours.
- Time/div and volts/div controls; a grid of 10×8 divisions.
- A rising-edge trigger on channel 1 at a configurable level. It holds the last good trace when no trigger is found.
- Pure functions `findTrigger(samples, level)` and `toPixels(samples, settings, w, h)` live in `site/demos/circuit-lab/scope.js` (new), so they can be unit-tested.

**Simulation speed.** Audio-frequency circuits run slowed down: a "time scale" control, defaulting to 1 simulated ms per real second for the RLC experiment. The on-screen readout states this.

## Implementation

### 1. Scope math and view

**Files**:
- `site/demos/circuit-lab/scope.js` (new)
- `components/scope-view.js`

Build the trigger, grid and two-channel drawing described in Key Decisions. Delete the fixed `* 10` scaling.

### 2. All six experiments

**File**: `site/demos/circuit-lab/experiments.js`

Each experiment returns `{ circuit, sim, probes, layout, readouts(sim) }`. All readout values come from the engine.

| Experiment | Circuit | What the readouts must show |
|---|---|---|
| LED + resistor | 9 V → R slider (47–1000 Ω) → red LED | LED current; Vf; blown state with the engine's issue text |
| Half-wave rectifier | 10 V-peak 50 Hz → Si diode → 1 kΩ | channel 1 input, channel 2 output; peak output |
| RLC resonance | 10 Ω, 10 mH, 1 µF, 1 V sine; frequency slider 200 Hz–10 kHz | current amplitude; a "Sweep" button plots amplitude against frequency (40 log-spaced points) |
| RC low-pass | 1 kΩ, 1 µF, 1 V sine; frequency slider | output amplitude and phase lag |
| Square wave into RC | 1 V, 10 Hz square | charge and discharge curves |
| Big ladder | 1000 rungs of 1 Ω | source current; solve time in ms |

Remove the "Wait…" comment.

### 3. Lab wiring and URL state

**Files**:
- `site/demos/circuit-lab/lab.js`
- `state.js`

1. Delete the `Math.sin` line.
2. Store the experiment, slider values and scope settings in the hash, with a round-trip test.

### 4. Node tests through the real WASM

**New files**:
- `site/demos/circuit-lab/scope.test.js`
- `experiments.test.js`

| Test | Expected |
|---|---|
| `findTrigger` on a synthetic sine | finds the rising zero crossing within one sample; returns −1 on a flat line |
| LED at R = 160 Ω | readout 20.0 mA ± 0.1 |
| LED at R = 47 Ω | the blown flag is set and the issue text is non-empty |
| Rectifier | output peak 9.29 ± 0.02 V |
| RLC sweep | maximum amplitude at 1591.5 Hz ± 2%; value 0.100 ± 0.002 A |
| RC at 159.15 Hz | amplitude 0.707 ± 0.01; lag 45° ± 2° |
| Ladder | 0.618034 ± 1e-6 A |

### 5. Scenarios

Capture six scenarios, one per experiment, reached through URL state. All six hashes must differ.

## Reused existing code

- `Circuit`, `Sim.addProbe` and `loadCircuits` in `site/js/simbiguity.js`
- the `BoardView` 3D board (`site/demos/circuit-lab/components/board-view.js`)
- `buildLayout` in `experiments.js`
- diode, LED and signal meshes in `crates/simbiguity-circuits-view/src/board.rs`
- `boot` from `site/js/boot.js`

## Reproduction Test

`rlc_sweep_peaks_at_1591_hz` (Node): it fails today, because there is no RLC experiment.

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