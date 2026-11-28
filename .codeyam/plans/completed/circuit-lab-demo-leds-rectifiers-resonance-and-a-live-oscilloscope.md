---
title: "Circuit Lab demo: LEDs, rectifiers, resonance and a live oscilloscope"
mode: ui
createdAt: "2026-11-28T02:43:18Z"
source: manual
dependsOn: ["circuits-sparse-solver-for-thousand-node-circuits", "round-3-puzzle-game-page-that-plays-the-real-levels-through-the-bridge"]
---

## Summary

The circuit engine will soon have diodes, LEDs, AC sources, oscilloscope probes and a sparse solver. None of that shows on the site yet:
- `hello-circuit` is a single DC loop;
- `rc-charging` plots one capacitor voltage;
- the 3D view (`crates/simbiguity-circuits-view`) only knows batteries, switches, bulbs and wires.

This plan builds a **Circuit Lab** page: a set of preset experiments, each with live sliders and a two-channel oscilloscope. It also teaches the 3D board to draw diodes, LEDs (glowing in their own colour) and signal sources.

## Key Decisions

**Experiments.** Each maps to a tested engine result, so what a learner sees matches a passing test:

| Experiment | Controls and readouts |
|---|---|
| LED + resistor | a resistor slider from 47–1000 Ω; a readout of LED current and Vf; the LED glows with brightness ∝ current. Below about 100 Ω, at 9 V, it exceeds 1.5× rated current and blows, with the engine's issue text. |
| Half-wave rectifier | the scope shows the input sine and the output, with the ~0.71 V drop visible at the peak |
| Series RLC resonance | a frequency slider from 200 Hz to 10 kHz; a live current-amplitude readout that peaks at 0.1 A at 1591.5 Hz; a "sweep" button that plots amplitude against frequency |
| RC low-pass | a frequency slider; the scope shows the attenuation and the 45° lag at 159 Hz |
| Square wave into RC | shows charging and discharging curves |
| Big ladder | 1000 rungs; reads source current → 0.618 A and shows the solve time in ms, which demonstrates the sparse solver |

**Scope rendering.**
- Use `simbiguity-plot` with two series, a time/div control and a volts/div control.
- Use a trigger on a rising zero crossing of channel 1, so the trace stands still.
- The samples come straight from `Sim.addProbe()`, with no JS-side physics.

**3D parts.** Add diode, LED and signal-source meshes to `crates/simbiguity-circuits-view/src/board.rs`.
- The LED's emissive colour is its `color`, scaled by light output.
- The diode shows a band on its cathode end.
- The signal source is a circle with a sine glyph.
- `board_layout.rs` gains record kinds for each. Use the strict per-kind unpack added in round 2; do not reuse a fixed offset.

## Implementation

### 1. 3D view parts

**Files**:
- `crates/simbiguity-circuits-view/src/board.rs`
- `board_layout.rs`
- `src/lib.rs`

Tests:
- LED glow is 0 when off, about 1 at 20 mA, and the colour matches.
- The diode's band is at the cathode end. Assert a vertex-position property, not just a mesh count.
- Layout round-trips each new kind. A record with the wrong length is rejected.

### 2. Bridge

**File**: `crates/simbiguity-render-wasm/src/lib.rs`

1. Expose `c3d_new_from_layout` support for the new kinds.
2. Expose probe access for the lab's scope.
3. Keep the render WASM within its 230,000-byte budget.

### 3. Page

**New files**: `site/demos/circuit-lab/index.html`, `lab.js`, `scope.js`, `experiments.js`, `state.js`

1. An experiment picker, sliders and readouts.
2. The scope canvas, plus the 3D board for the current experiment.
3. The URL-hash state: experiment and slider values.
4. Errors from the engine are shown as text.

### 4. Node tests

**New files**: `site/demos/circuit-lab/scope.test.js`, `experiments.test.js`

The **trigger** finds a rising zero crossing within one sample on a synthetic sine.

Run these through the real WASM:
- **LED experiment:** at R = 160 Ω, the readout shows 20.0 mA.
- **RLC sweep:** it peaks within ±2% of 1591.5 Hz.
- **Rectifier:** the output peak is 9.29 V ± 0.02.
- **Hash state** round-trips.

### 5. Scenarios

Capture six distinct screenshots, one per experiment, each in a meaningful state:
- the LED blown at 47 Ω;
- RLC at resonance;
- and so on for the other four.

All hashes must differ.

## Reused existing code

- `Sim.addProbe`, `addDiode` and `addLed` from the AC and diode plans (`site/js/simbiguity.js`)
- the board meshes and the per-bulb glow logic in `crates/simbiguity-circuits-view` (fixed in `a941c59`)
- `renderLabels` and `simbiguity-plot`
- `WebGL2Executor`
- `bindCanvasInput` from `site/demos/circuit-3d/canvas-input.js`

## Reproduction Test

`rlc_sweep_peaks_at_resonance` (Node) fails today, because the page and the signal-source bridge don't exist yet.

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