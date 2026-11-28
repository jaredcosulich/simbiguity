---
title: "Analogies demo: an RLC circuit and a mass on a spring resonate identically"
mode: ui
createdAt: "2026-11-28T16:56:22Z"
source: manual
dependsOn: ["round-4-physics-lab-behaviour-tests-through-the-real-wasm", "physics-damped-and-driven-oscillators-checked-against-resonance-theory"]
---

## Summary

The two engines now each simulate a resonator: the series RLC circuit (`crates/simbiguity-circuits/tests/ac.rs`) and the damped driven mass–spring (from the oscillator plan). The two systems obey the same equation:

| Mechanical | Electrical |
|---|---|
| m·ẍ + c·ẋ + k·x = F | L·q̈ + R·q̇ + q/C = V |
| mass m | inductance L |
| damping c | resistance R |
| stiffness k | 1/C |
| displacement x | charge q |
| velocity v | current i |

This is one of the most illuminating ideas in physics, and a perfect showcase for having two engines that share deterministic infrastructure.

This plan builds an **"Analogies" demo**:
- the 3D mass–spring and the 3D RLC board sit side by side;
- one set of sliders drives both;
- one plot overlays their responses, which are numerically identical.

## Key Decisions

**One slider set, mapped by the analogy.**
- Mass m ↔ L, damping c ↔ R, stiffness k ↔ 1/C, drive amplitude F₀ ↔ V₀, drive frequency ω.
- Each engine runs its own simulation.
- The page plots x(t) from physics and q(t) = ∫i dt from circuits, by adding a charge probe on the capacitor (q = C·V_C) on the same axes.

**The match is a test, not an assertion of faith.**
- With m = L = 1, c = R = 0.2, k = 1/C = 100 and F₀ = V₀ = 1 at ω = 10, both engines run at dt = 1e-4.
- Physics uses Verlet; circuits uses trapezoidal.
- At steady state the two amplitudes match within 1%, and each matches the analytic 0.5.

**Teaching overlay.** A table shows the mapping and the live values: ω₀ = 1/√(m/k) = 1/√(LC), and Q = √(mk)/c = √(L/C)/R. All values come from the engines' parameters, never hard-coded.

## Implementation

### 1. Capacitor charge probe

**File**: `crates/simbiguity-circuits/src/sim.rs`

1. Add `Probe::Charge(capacitor)` (q = C·V_C).
2. Add a test: a 1 µF capacitor at 5 V reads 5e-6 C.
3. Expose it in the bridge and in `simbiguity.js`.

### 2. Page

**New files**: `site/demos/analogies/index.html`, `analogies.js`, `mapping.js`, `state.js`

1. Put the two canvases side by side: `WebGL2Executor` for each, or one canvas with two viewports.
2. Add the shared sliders.
3. Plot x(t) and q(t) overlaid with `simbiguity-plot`.
4. Add a resonance sweep that plots amplitude against ω for both engines.
5. Show the mapping table, and keep state in the URL hash.

### 3. Node tests through both WASMs

**File**: `site/demos/analogies/analogies.test.js` (new)

| Test | Expected |
|---|---|
| Resonance case above | both engines' steady amplitudes are 0.500 ± 1%, and they agree with each other within 1% |
| ω = 20 | both 0.003333 ± 2% |
| Sweep | both peak at ω = 10 ± 1% |
| `mapping.js` | maps the slider values to both parameter sets exactly, and is invertible (round trip) |

### 4. Manifest and gallery

**File**: `site/demos/manifest.json`

Add the demo with its capabilities: "AC signal sources and resonance" and "Damped and driven oscillators". Add a thumbnail.

### 5. Scenarios

Capture three distinct scenarios: at resonance, off-resonance (ω = 20), and heavily damped (c = R = 5).

## Reused existing code

- `Sim.addProbe` and `loadCircuits` (`site/js/simbiguity.js`)
- the `Physics` class (`site/demos/physics/physics.js`, from the Physics Lab tests plan)
- the driven-oscillator preset
- `WebGL2Executor`
- `renderLabels` and the `simbiguity-plot` chart
- `site/js/boot.js`

## Reproduction Test

`rlc_and_mass_spring_resonate_identically` (Node) fails today, because the demo and the charge probe don't exist.

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