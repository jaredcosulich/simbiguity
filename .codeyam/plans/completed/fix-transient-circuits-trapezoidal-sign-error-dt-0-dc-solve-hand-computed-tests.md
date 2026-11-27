---
title: "Fix: transient circuits trapezoidal sign error, dt=0 DC solve, hand-computed tests"
mode: ui
createdAt: "2026-11-26T22:14:09Z"
source: manual
dependsOn: ["fix-time-series-plots-actually-draw-chart-draw-stub-line-geometry-tick-loop"]
---

## Summary

Capacitors and inductors exist, but the trapezoidal method gives badly wrong answers, which is a sign error. Any circuit with a capacitor fails in the browser, and the tests are too loose to notice either problem. This fix corrects the solver and puts in the plan's hand-computed tests. It also finishes the skipped parts of `.codeyam/plans/completed/circuits-that-change-over-time-capacitors-inductors-and-transient-analysis.md`, which remains the spec: the inrush issue, chart exports, the integrator toggle and the LC preset.

## Current behavior

- **Trapezoidal sign error.**
  - `crates/simbiguity-circuits/src/mna.rs:300` — `component_current` returns `current - g * d` for a `Stamp::Norton`. That is the current delivered *out of* the `+` terminal, the source convention.
  - The value written back as each element's `amps` therefore has the opposite sign to the passive current the trapezoidal companion model expects.
  - `crates/simbiguity-circuits/src/component.rs:176` (capacitor: `current: g * volts + amps`) and `crates/simbiguity-circuits/src/component.rs:193` (inductor: `amps + g * volts`) then add a current with the wrong sign.
  - Measured on the plan's RC case (1 kΩ, 1 mF, 10 V, dt = 0.1 s, 10 steps): backward Euler gives 6.1446 V (correct), but trapezoidal gives about 0.476 V instead of 6.3249 V. RL under trapezoidal gives -0.0476 A.
  - The backward Euler companions at `crates/simbiguity-circuits/src/component.rs:165` and `:185` are correct.
- **The browser's DC solve is a dt = 0 step.**
  - `crates/simbiguity-circuits-wasm/src/slot.rs:134` — `Slot::solve` calls `self.step(0.0)`.
  - With dt = 0 the capacitor conductance `farads / dt` is infinite, so any circuit with a capacitor returns no solution plus a `FloatingNode` issue.
  - An inductor's `dt / henries` becomes 0, so it acts as an open circuit, the opposite of its DC limit (a wire).
  - `analyze_step` in `crates/simbiguity-circuits/src/mna.rs` has no guard for `dt <= 0`.
- **Weak tests in `crates/simbiguity-circuits/tests/analysis.rs`.**
  - `rc_circuit_charging_matches_exact_solution` (line 389) uses 5 Ω / 0.2 F with a tolerance of 0.5 V (line 410). The real backward Euler error is 0.18 V, so the test can't tell the two methods apart.
  - Its energy check (line 420) recomputes ½CV² from the same simulated V, so it is circular.
  - `rl_circuit_matches_exact_solution` (line 431) compares `sim_i.abs()` (line 451), which hides the sign bug.
  - Nothing exercises `Integrator::Trapezoidal` or `CircuitSim::with_integrator` (`crates/simbiguity-circuits/src/sim.rs:55`).
  - There are no tests for LC energy, the DC limit, or snapshot determinism.
- **Demo.**
  - `site/demos/rc-charging/demo.js:37` draws the chart with canvas 2D in JS, which goes against "logic out of JS" and doesn't use `simbiguity-plot`.
  - The `#reset` button in `site/demos/rc-charging/index.html:18` has no handler.
  - `site/demos/rc-charging/state.test.js` is the single comment `// state test`.
  - There is no bulb, no LC preset and no integrator toggle.
- The new capability in `crates/simbiguity-circuits/capabilities.toml` proves nothing (`proves = []`).

## Key Decisions

- **One sign convention, documented once.** Store each energy-storage element's *passive* current (into its `a` terminal), and convert at the single place the Norton current is read back. Do not flip signs at the two companion sites. Add a doc comment that works through the trapezoidal companion derivation as a lesson.
- **DC means DC, not a step.** The bridge's `circuit_solve` calls `Circuit::solve()`, where a capacitor is open and an inductor is a wire, as the plan specified. `circuit_step(h, dt)` is the only transient entry point. `analyze_step` rejects `dt <= 0` or a non-finite `dt` with a fatal, named issue instead of producing infinities.
- **The tests are the plan's numbers, exactly.** Assert 6.1446 V (backward Euler) and 6.3249 V (trapezoidal) to 1e-4, the textbook answer 6.3212 V, and the gap between them. Tighten the existing tests rather than adding parallel ones.
- Finalize must run (no `Codeyam-Finalize: deferred`).

## Implementation

### 1. Sign fix

- **File**: `crates/simbiguity-circuits/src/mna.rs`: where transient element state is written back, store passive current
- **File**: `crates/simbiguity-circuits/src/component.rs`: trapezoidal companions consume the passive current; document the derivation

### 2. DC and dt guards

- **File**: `crates/simbiguity-circuits-wasm/src/slot.rs`: `solve` uses the DC analysis; add a separate `step(dt)`
- **File**: `crates/simbiguity-circuits/src/mna.rs`: `analyze_step` rejects a dt that isn't positive and finite
- **File**: `crates/simbiguity-circuits/src/diagnostics.rs`: add the inrush issue (an ideal battery directly across a capacitor) and the invalid-time-step issue, in plain words

### 3. Tests

**File**: `crates/simbiguity-circuits/tests/analysis.rs`: replace the 0.5 V tolerances with the plan's hand numbers. Use signed currents, compute energy from the analytic V, and add:
- LC energy within 1e-9 over 1000 trapezoidal steps, and decaying under backward Euler
- a DC limit after 10τ
- snapshot → restore → replay giving bit-identical results
- an inductor DC-as-wire test through the bridge's solve

### 4. Demo and capability

- **File**: `site/demos/rc-charging/demo.js`: replace canvas drawing with `simbiguity-plot` chart exports from the bridge; wire up `#reset`; add a bulb, an LC preset and an integrator toggle
- **File**: `site/demos/rc-charging/state.test.js`: real tests of the demo state logic
- **File**: `crates/simbiguity-circuits-wasm/src/lib.rs`: integrator selection and chart exports
- **File**: `crates/simbiguity-circuits/capabilities.toml`: `proves` patterns naming the new tests

## Reused existing code

- `Stamp::Norton` in `crates/simbiguity-circuits/src/component.rs`; `component_current` in `crates/simbiguity-circuits/src/mna.rs`
- `CircuitSim::with_integrator` in `crates/simbiguity-circuits/src/sim.rs`; `Circuit::add_capacitor` in `crates/simbiguity-circuits/src/netlist.rs`
- `Chart` / `Series` from `crates/simbiguity-plot` (after the plot fix)

## Reproduction Test

Trapezoidal RC charging is badly wrong: about 0.48 V instead of 6.32 V after one time constant.

**Target**: `crates/simbiguity-circuits/tests/analysis.rs`. Run with `codeyam-editor editor refresh-tests --test trapezoidal_rc_matches_hand_computed_discrete_answer`.

```rust
/// RC charging, R = 1 kΩ and C = 1 mF (so τ = 1 s), from a 10 V battery:
/// ten trapezoidal steps of dt = 0.1 s. Each step multiplies the remaining
/// gap by (1 - dt/2τ)/(1 + dt/2τ) = 0.95/1.05, so after ten steps
/// V = 10·(1 - (0.95/1.05)^10) = 6.3249 V. That is very close to the
/// textbook answer 10·(1 - e⁻¹) = 6.3212 V, which is why trapezoidal is
/// called second-order accurate.
#[test]
fn trapezoidal_rc_matches_hand_computed_discrete_answer() {
    let mut c = Circuit::new();
    let (p, mid, m) = (c.node(), c.node(), c.node());
    c.add_battery(p, m, 10.0);
    c.add_resistor(p, mid, 1000.0);
    let cap = c.add_capacitor(mid, m, 0.001);
    let mut sim = CircuitSim::new(c).with_integrator(Integrator::Trapezoidal);
    for _ in 0..10 {
        sim.step(Seconds(0.1));
    }
    let v = sim.state().solution().unwrap().voltage_across(cap).unwrap();
    let expected = 10.0 * (1.0 - (0.95_f64 / 1.05).powi(10));
    assert!((v - expected).abs() < 1e-4, "got {v}, expected {expected}");
}
```

Status: PROPOSED — confirm red at execution. Expected failure: `v` is about 0.476, so the assertion fails with "got 0.47…, expected 6.3249…". Confirm the import path for `Integrator` at execution.

## Scenarios to Demonstrate

- RC mid-charge with the plot-crate chart approaching 10 V
- Switch opened: capacitor discharging through the bulb, which fades
- LC ringing under trapezoidal (sustained) vs backward Euler (decaying)
- Inductor-only circuit solved as DC (acts as a wire, no error)