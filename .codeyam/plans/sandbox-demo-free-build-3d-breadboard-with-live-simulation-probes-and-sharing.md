---
title: "Sandbox demo: free-build 3D breadboard with live simulation, probes and sharing"
mode: ui
createdAt: "2026-11-28T16:56:22Z"
source: manual
dependsOn: ["round-4-puzzle-levels-that-teach-brightness-and-switch-goals-a-verification-test-that-can-fail-a-parser-that-never", "round-4-circuit-lab-with-a-real-oscilloscope-and-all-six-experiments", "circuits-op-amps-and-controlled-sources"]
---

## Summary

Every circuit on the site is preset: the hello-circuit loop, the 3D demos, the puzzle levels and the Circuit Lab experiments. The puzzle page already has real placement on a grid (`puzzle_place` / `puzzle_remove` in `crates/simbiguity-render-wasm/src/lib.rs`, plus `BoardEdits` in `simbiguity-circuits-puzzle`), but only inside a level's palette.

This plan builds a **Sandbox**: a free-build 3D breadboard. Learners place any part, wire it, edit its values, and watch current flow, bulbs glow, LEDs light and the scope trace. They can also share what they built.

## Key Decisions

**Reuse the puzzle board engine, not a new one.** Add a `SandboxSession` next to `PuzzleSession` in `simbiguity-circuits-puzzle`.
- It has no level, no goals, an unlimited palette, and a grid of 16×12 cells.
- Board edits, undo/redo, `build_circuit` and share codes all carry over.
- Share codes gain a version 2 that records part values.

**Part set: everything the engine supports.**
- battery, signal source, resistor, bulb, switch, wire
- capacitor, inductor, diode, LED
- and, once their plans land, BJT and op-amp

Each kind's values (EMF, ohms, farads and so on) are edited in a side panel. Values come from typed inputs with engine-side validation: an invalid value shows the engine's `InvalidValue` text.

**Live simulation.**
- The board runs `CircuitSim` continuously.
- Any edit rebuilds the circuit and keeps capacitor and inductor state where parts are unchanged (match by cell and kind).
- Diagnostics (short circuit, floating node, blown bulb) appear as engine text attached to the offending part.

**Probing.** Clicking a node with the probe tool attaches a scope channel (2 max). The scope is the `ScopeView` from the Circuit Lab fix plan.

## Implementation

### 1. Sandbox session

**File**: `crates/simbiguity-circuits-puzzle/src/lib.rs`

1. Add `SandboxSession` with place, remove, set_value, undo, redo, clear and `share_v2`.
2. Tests:
   - a round trip of a 12-part build through `share_v2`;
   - v1 codes still decode;
   - `set_value` then undo restores the old value;
   - the state-preserving rebuild: a charged capacitor keeps its voltage when an unrelated wire is added.

### 2. Bridge

**File**: `crates/simbiguity-render-wasm/src/lib.rs`

1. Add `sandbox_*` exports mirroring the session.
2. Add a part-info readout: kind, values, voltage across, current through.
3. Each export gets a Rust test. Keep within the render WASM budget.

### 3. Page

**New files**: `site/demos/sandbox/index.html`, `sandbox.js`, `components/palette.js`, `components/inspector.js`, `components/toolbar.js`

1. Palette, then click to place, R to rotate, click a placed part to select it.
2. The inspector edits values.
3. A probe tool feeds the scope.
4. The toolbar has undo/redo/clear/share.
5. The URL hash holds the share code.

### 4. Node tests

**File**: `site/demos/sandbox/sandbox.test.js` (new)

These run through the real WASM:
- Build battery (9 V) + bulb (9 V/3 W) + 2 wires by placement calls, and read the bulb at 1/3 A ± 1e-6.
- Add a wire that shorts the battery. The issue text is non-empty and names the short circuit.
- Undo. The bulb is back at 1/3 A.
- Share, then load in a new session. The part list is identical.

### 5. Manifest, gallery and scenarios

1. Add the demo to `site/demos/manifest.json`.
2. Capture four distinct scenarios through share codes:
   - an empty board;
   - a lit series circuit;
   - an LED with its resistor probed on the scope;
   - a short-circuit diagnostic.

## Reused existing code

- `PuzzleSession`, `BoardEdits`, `build_circuit` and `crates/simbiguity-circuits-puzzle/src/share.rs`
- `puzzle_place` and related exports
- `Circuit3D` picking
- `ScopeView` and `site/demos/circuit-lab/scope.js` from the Circuit Lab fix plan
- `WebGL2Executor`
- `site/js/boot.js`

## Reproduction Test

`sandbox_build_battery_and_bulb_reads_one_third_amp` (Node) fails today, because `SandboxSession` doesn't exist.

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