---
title: "Circuits: bipolar transistors (Ebers-Moll) as amplifiers and switches"
mode: backend
createdAt: "2026-11-28T16:56:22Z"
source: manual
dependsOn: ["round-4-trustworthy-gates-bench-check-that-can-fail-honest-size-budgets-junk-and-duplicate-screenshot-tests"]
---

## Summary

With Newton–Raphson and diodes in place (`crates/simbiguity-circuits/tests/nonlinear.rs`), the circuits engine can take on the part that makes electronics interesting: the **transistor**.

This plan adds an NPN/PNP bipolar junction transistor, using the transport form of the Ebers–Moll model. Tests check two classic textbook operating points: forward-active amplification, and saturation as a switch.

## Key Decisions

**Model: Ebers–Moll transport form.**
- I_C = Is·(e^(V_BE/Vt) − e^(V_BC/Vt)) − (Is/β_R)·(e^(V_BC/Vt) − 1)
- I_B = (Is/β_F)·(e^(V_BE/Vt) − 1) + (Is/β_R)·(e^(V_BC/Vt) − 1)
- Vt = 0.025852 V. Defaults: Is = 1e-14, β_F = 100, β_R = 1.
- For PNP, use the same equations with every voltage and current negated.

**Stamping: linearise each step into a 3-terminal Norton companion.**
- That is, the 2×2 conductance Jacobian between the base–emitter and base–collector junctions, plus equivalent current sources.
- Reuse `pnjlim` voltage limiting on both junctions.

**Diagnostics.** Each transistor reports its region: cutoff, active, saturation or reverse. The current flow and the explain panel use it.

## Implementation

### 1. Component

**File**: `crates/simbiguity-circuits/src/component.rs`

1. Add `Component::Bjt { polarity, saturation_amps, beta_f, beta_r }`, with three terminals.
2. `Circuit::add_bjt(c, b, e, …)` adds one.
3. Extend the netlist to support a 3-terminal part. Check how `Part { a, b }` is stored and add an optional third node, rather than faking it with two parts.

### 2. Stamps and the Newton loop

**File**: `crates/simbiguity-circuits/src/mna.rs`

Add the Jacobian stamp, limiting, and convergence on both junction voltages.

### 3. Reference tests

**File**: `crates/simbiguity-circuits/tests/bjt.rs` (new)

These values come from an independent Newton solve of the exact Ebers–Moll equations. Each circuit is: Vcc = 9 V, R_B from Vcc to the base, R_C = 1 kΩ from Vcc to the collector, emitter grounded.

| Case | Expected |
|---|---|
| R_B = 470 kΩ (forward active) | V_BE = 0.669587 V ± 1e-5; I_B = 17.7243 µA ± 1e-9 A; I_C = 1.772428 mA ± 1e-8 A; V_CE = 7.227572 V ± 1e-5; region Active; I_C/I_B = 100.0 ± 0.01 |
| R_B = 47 kΩ (saturated) | V_BE = 0.711740 V ± 1e-5; V_CE = 0.120425 V ± 1e-5; I_C = 8.879575 mA ± 1e-8 A; I_B = 176.346 µA ± 1e-8 A; region Saturation; I_C/I_B ≈ 50.4, which is below β_F |
| Base open | I_C < 1e-12 A; region Cutoff |
| PNP mirror of the 470 kΩ case (Vcc = −9 V) | every voltage and current is negated, to within the same tolerances |
| KCL | I_E = I_B + I_C to 1e-12, at every Newton-converged solution in the cases above |
| LED driven by a transistor switch | 9 V → 330 Ω → red LED → collector; base fed through 10 kΩ from a switch to 9 V. Switch open: LED off (light < 1e-3). Switch closed: LED lit (light > 0.9). |

### 4. Bridge, JS and 3D

**Files**:
- `crates/simbiguity-circuits-wasm/src/lib.rs`
- `site/js/simbiguity.js`
- `crates/simbiguity-circuits-view/src/board.rs`
- `board_layout.rs`

1. Add `circuit_add_bjt` and `Circuit.addBjt`.
2. Add a 3D mesh: a TO-92 body with three legs, labelled C, B and E.
3. Add a layout record kind with a strict, length-checked unpack.
4. Add a bridge test for the 470 kΩ case.

### 5. Capabilities

Add "Transistors (BJT, Ebers–Moll)", proven by the table's tests.

## Reused existing code

- the Newton loop and `pnjlim` from the diode plan, in `crates/simbiguity-circuits/src/mna.rs`
- the `Stamp::Norton` companion
- `crates/simbiguity-circuits/tests/nonlinear.rs` as a pattern
- part meshes in `crates/simbiguity-circuits-view/src/board.rs`

## Reproduction Test

`bjt_forward_active_470k_gives_beta_100` fails to compile today, because there's no `Bjt`.

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