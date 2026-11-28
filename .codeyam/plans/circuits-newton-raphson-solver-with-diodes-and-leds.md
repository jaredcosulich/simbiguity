---
title: "Circuits: Newton-Raphson solver with diodes and LEDs"
mode: backend
createdAt: "2026-11-28T02:43:18Z"
source: manual
dependsOn: ["round-3-site-build-under-budget-no-junk-and-a-determinism-test-that-can-fail"]
---

## Summary

The circuits engine handles only linear parts: battery, resistor, bulb, switch, wire, capacitor and inductor. `solve_once` in `crates/simbiguity-circuits/src/mna.rs:412` does a single linear solve. Diodes and LEDs are the next thing every learner meets: rectifiers, indicator lights, "why does the LED need a resistor?". They need a nonlinear solver.

This plan adds:
- Newton–Raphson iteration to MNA;
- a `Diode` component (Shockley model);
- an `Led` component (a diode with a colour, a rated current, and a light output the 3D view can glow with).

## Key Decisions

**The diode model is Shockley: I = Is·(e^(V/(n·Vt)) − 1).**
- Vt = 0.025852 V (300 K).
- Each Newton iteration linearises the diode into the existing `Stamp::Norton` companion:
  - g = (Is/(n·Vt))·e^(V/(n·Vt))
  - I_eq = I(V) − g·V
- No new stamp kind is needed.

**Voltage limiting stops exp() overflow.** Use SPICE's `pnjlim`:
- Above the critical voltage Vcrit = n·Vt·ln(n·Vt/(√2·Is)), limit each iteration's change in diode voltage logarithmically.
- Without this, a 9 V source across a diode overflows on iteration one.

**Convergence rule.**
- Every node voltage must satisfy |ΔV| ≤ 1e-9 + 1e-6·|V|, and every diode current must agree with its model within 1e-12 + 1e-6·|I|.
- Allow at most 100 iterations. Otherwise emit `CircuitIssue::SolverFailure` with a message saying Newton did not converge.

**Linear circuits stay one-shot.** A circuit with no nonlinear parts must converge in exactly 1 iteration and give bit-identical results to today's solver. The golden hashes must not change. Expose `Solution::newton_iterations()` so a test can check this.

**The LED is the same model with fitted parameters.**
- Use n = 2 and Is = 1.5195e-17 A. These give Vf = 1.800 V at 20 mA.
- Light output = current / rated current (20 mA), clamped to [0, 1.5].
- Above 1.5× rated current the LED "blows". Reuse the existing blow-out mechanism for bulbs.

## Implementation

### 1. Components

**File**: `crates/simbiguity-circuits/src/component.rs`

Add:
- `Component::Diode { saturation_amps, emission }`
- `Component::Led { color, rated_amps, saturation_amps, emission }`

Builders on `Circuit` follow the existing `add_bulb` style. Invalid parameters (Is ≤ 0, n ≤ 0) give `CircuitIssue::InvalidValue`.

### 2. Newton loop in MNA

**File**: `crates/simbiguity-circuits/src/mna.rs`

1. Wrap `solve_once` in an iteration. The previous iterate's voltages feed the diode companions.
2. Initial guess: 0.6 V across each diode.
3. In transient analysis, each time step runs its own Newton loop, starting from the previous step's solution.

### 3. Reference tests

**File**: `crates/simbiguity-circuits/tests/nonlinear.rs` (new)

These values were computed independently by bisection on the exact Shockley equation:

| Circuit | Expected |
|---|---|
| 5 V → 1 kΩ → Si diode (Is = 1e-14, n = 1) | Vd = 0.692544 V ± 1e-5; I = 4.307456 mA ± 1e-8 A |
| 5 V → 1 kΩ → two Si diodes in series | each Vd = 0.688076 V ± 1e-5; I = 3.623848 mA ± 1e-8 A |
| Si diode reversed, 5 V, 1 kΩ | I = −1e-14 A ± 1e-15 A |
| 5 V → 160 Ω → red LED | Vf = 1.80000 V ± 1e-4; I = 20.000 mA ± 1e-6 A; light = 1.000 ± 1e-3 |
| 9 V → 330 Ω → red LED | Vf = 1.80447 V ± 1e-4; I = 21.805 mA ± 1e-6 A |
| 9 V directly → LED (no resistor) | the LED blows (issue emitted); the solver still converges |
| any circuit from `invariants_tests.rs::generate_circuit` (2000 seeds) | `newton_iterations() == 1`; KCL still holds |

Add KCL over 500 random circuits that each include 1–3 diodes: net current ≤ 1e-9·max|I| at every node.

### 4. Bridge and JS

**Files**:
- `crates/simbiguity-circuits-wasm/src/lib.rs`
- `site/js/simbiguity.js`

Add `circuit_add_diode` and `circuit_add_led`, and a JS `Circuit.addDiode` / `addLed`. Add a test through the bridge for the 160 Ω LED row.

### 5. Capabilities

**File**: `crates/simbiguity-circuits/capabilities.toml`

Add "Diodes and LEDs (nonlinear solving)", proven by the tests above.

## Reused existing code

- `Stamp::Norton` and `component_current` in `crates/simbiguity-circuits/src/mna.rs`
- `solve_once` (`mna.rs:412`)
- the bulb blow-out logic in `crates/simbiguity-circuits/src/diagnostics.rs`
- `generate_circuit` in `crates/simbiguity-circuits/src/invariants_tests.rs`
- `Lu` / `lu_solve` in `crates/simbiguity-math/src/linsolve.rs`

## Reproduction Test

`diode_with_1k_from_5v_drops_0_6925_volts`: build the first row of the table. It fails to compile today, because there is no `Diode` component.

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