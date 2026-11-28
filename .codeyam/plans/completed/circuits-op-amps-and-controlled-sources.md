---
title: "Circuits: op-amps and controlled sources"
mode: backend
createdAt: "2026-11-28T16:56:22Z"
source: manual
dependsOn: ["circuits-bipolar-transistors-ebers-moll-as-amplifiers-and-switches"]
---

## Summary

Op-amps are the other building block learners meet after transistors: amplifiers, comparators, followers, active filters. This plan adds an op-amp component to the circuits engine, modelled as a high-gain voltage-controlled voltage source with smooth rail saturation. It goes through the existing Newton loop, so a saturated output converges instead of jumping.

## Key Decisions

**Model.**
- V_out = V_sat·tanh(A·(V₊ − V₋)/V_sat).
- Defaults: A = 1e5, V_sat = 12 V, infinite input impedance, zero output impedance.
- The output is an ideal voltage source to ground, stamped as an MNA voltage-source row.
- Its value and derivative, A·sech²(·), go into the Newton Jacobian. So it uses the voltage-source branch the battery already has, plus controlling terms on the V₊ and V₋ columns.

**A general VCVS first.** Add `Component::Vcvs { gain }`, a linear voltage-controlled voltage source, as its own part. The op-amp is the VCVS with tanh saturation. Users and tests get both.

## Implementation

### 1. VCVS

**File**: `crates/simbiguity-circuits/src/component.rs`

Add `Component::Vcvs { gain }`, a 4-terminal part: out+, out−, ctrl+ and ctrl−. Stamp it in `mna.rs`.

Test: gain 10, with 0.3 V on ctrl and an open output, gives out = 3.000000 V exactly (1e-12).

### 2. Op-amp

Add `Component::OpAmp { gain, v_sat }`, a 3-terminal part (+, −, out) with the output referenced to ground.

### 3. Reference tests

**File**: `crates/simbiguity-circuits/tests/opamp.rs` (new)

These values come from an independent bisection solve of the exact tanh model.

| Circuit | Expected |
|---|---|
| Inverting amp, R_in = 1 kΩ, R_f = 10 kΩ, V_in = 0.5 V | V_out = −4.999414 V ± 1e-5 (ideal −5; the 0.012% error comes from finite gain and tanh) |
| Same, V_in = 2.0 V (would need −20 V) | V_out = −12.000 ± 1e-3, i.e. saturated at the rail; Newton converges in ≤ 100 iterations |
| Voltage follower, V_in = 3 V | V_out = 2.999969 V ± 1e-6 |
| Comparator, V₊ = 1 V, V₋ = 0.5 V, open loop | V_out = 12.000 ± 1e-3 |
| Non-inverting amp, R_g = 1 kΩ, R_f = 9 kΩ, V_in = 0.2 V | V_out = 2.000 ± 1e-3 (gain 10) |
| Integrator, R = 10 kΩ, C = 1 µF, 1 V step in, trapezoidal, dt = 1e-5 | V_out ramps at −100 V/s; at t = 50 ms, −5.000 ± 0.01 V; rails at −12 V after 120 ms |
| Active low-pass (inverting, R_f ∥ C_f with R_f = 10 kΩ, C_f = 15.915 nF, so fc = 1 kHz; R_in = 10 kΩ), 1 V sine at 1 kHz | output amplitude 0.7071 ± 1% |

### 4. Bridge, JS and 3D

Add `circuit_add_opamp`, `Circuit.addOpAmp`, a triangle-shaped 3D mesh with +, − and out labels, and a layout record kind. Add a bridge test for the inverting amp.

### 5. Capabilities

Add "Op-amps and controlled sources".

## Reused existing code

- the voltage-source rows in `crates/simbiguity-circuits/src/mna.rs`
- the Newton loop from the diode plan
- `SignalSource` and probes from the AC plan, for the filter test
- `simbiguity_math::float` (`tanh` if present; otherwise add a deterministic `tanh` there, with tests against known values)

## Reproduction Test

`inverting_amp_gain_of_minus_ten` fails to compile today, because there's no `OpAmp`.

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