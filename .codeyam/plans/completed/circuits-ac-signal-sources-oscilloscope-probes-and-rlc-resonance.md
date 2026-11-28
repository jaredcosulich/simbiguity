---
title: "Circuits: AC signal sources, oscilloscope probes and RLC resonance"
mode: backend
createdAt: "2026-11-28T02:43:18Z"
source: manual
dependsOn: ["circuits-newton-raphson-solver-with-diodes-and-leds"]
---

## Summary

Every source in the circuits engine is DC: `Component::Battery` holds one EMF. `CircuitSim` (`crates/simbiguity-circuits/src/sim.rs`) steps capacitors and inductors, but it doesn't track elapsed time, so nothing can vary with it. This plan adds:

- **A signal source**: a sine, square or triangle voltage with amplitude, frequency, phase and DC offset. It powers lessons on AC, filters and resonance.
- **An oscilloscope probe**: a fixed-size ring buffer of (t, value) samples for a node voltage or a part current. Demos draw it with `simbiguity-plot`.

The tests below check the integrator against textbook AC results: RLC resonance, the RC low-pass −3 dB point, and a half-wave rectifier, which needs the diode from the previous plan.

## Key Decisions

- **Time lives in `CircuitSim`.** Add `CircuitSim::time() -> Seconds`. `step(dt)` evaluates each source at t + dt, which is the time the backward-Euler/trapezoidal companions solve for. Then it advances t. Snapshots include t, so replay stays deterministic.
- **Waveforms use `simbiguity_math::float::sin`, not `libm`.** That keeps every platform bit-identical.
  - The square wave is +A for the first half period and −A for the second.
  - The triangle wave is piecewise linear.
- **Plain DC solves treat a signal source as its value at t = 0.** So `Circuit::solve()` keeps working.
- **Probes are fixed capacity.**
  - `CircuitSim::add_probe(Probe::NodeVoltage(n) | Probe::Current(id), capacity)` returns a `ProbeId`, and `probe_samples(id)` returns them oldest first.
  - Once full, the oldest sample is overwritten. There is no unbounded growth.

## Implementation

### 1. `Component::SignalSource`

**File**: `crates/simbiguity-circuits/src/component.rs`

Add `Component::SignalSource { waveform, amplitude, frequency, phase, offset }`, stamped like `Battery` with the time-dependent EMF. Reject frequency ≤ 0 and non-finite values with `InvalidValue`.

### 2. Time and probes in `CircuitSim`

**File**: `crates/simbiguity-circuits/src/sim.rs`

Add the time tracking and probes described above. Test that `snapshot`/`restore` resumes with identical probe output.

### 3. Reference tests

**File**: `crates/simbiguity-circuits/tests/ac.rs` (new)

All runs use the trapezoidal integrator with dt = period/200 unless stated. Measure amplitude as (max − min)/2 of a probe over the last 5 periods, after at least 20 periods of settling.

| Circuit | Expected |
|---|---|
| Series RLC: R = 10 Ω, L = 10 mH, C = 1 µF, 1 V sine at f0 = 1/(2π√LC) = 1591.549 Hz | current amplitude 0.1000 A ± 1%. At resonance the impedance is just R. |
| Same, driven at 2·f0 = 3183.099 Hz | current 6.652 mA ± 2%. \|Z\| = √(R² + (ωL − 1/ωC)²) |
| Same, driven at f0/2 = 795.775 Hz | 6.652 mA ± 2%. This is symmetric in this case, a good check on sign errors. |
| RC low-pass: R = 1 kΩ, C = 1 µF, 1 V sine at fc = 1/(2πRC) = 159.155 Hz | capacitor amplitude 0.7071 V ± 1%; phase lag 45° ± 1° |
| Same at 10·fc | amplitude 0.0995 V ± 2% (1/√101) |
| Half-wave rectifier: 10 V-peak 50 Hz sine → Si diode (Is = 1e-14, n = 1) → 1 kΩ, dt = 20 ms/2000 | output peak 9.2876 V ± 0.01 V (diode drop at the peak is 0.7124 V); output never below −1e-9 V |
| Square wave, 1 V, 10 Hz into the RC above | capacitor ends each half period within 1e-6 of the analytic value. The time constant is 1 ms, so it settles fully. |
| Backward Euler at dt = period/20 on the resonance case | the amplitude is visibly damped below 0.1 A. Assert < 0.095 A. This documents why trapezoidal is the default for AC. |

### 4. Bridge and JS

**Files**:
- `crates/simbiguity-circuits-wasm/src/lib.rs`
- `site/js/simbiguity.js`

1. Add `circuit_add_signal`, `sim_time`, `sim_add_probe` and `sim_probe_samples_ptr/len`.
2. Add a JS `Sim.addProbe()` that returns a `Float64Array` view.
3. Add a Node test that runs the RLC-at-f0 row through the bridge and gets 0.1 A ± 1%.

### 5. Capabilities

Add "AC signal sources and resonance" and "Oscilloscope probes" to `crates/simbiguity-circuits/capabilities.toml`.

## Reused existing code

- `Stamp` and the battery stamp in `crates/simbiguity-circuits/src/mna.rs`
- `Integrator::Trapezoidal`, whose sign was fixed in `3713ad0`
- `simbiguity_math::float::{sin, round}`
- snapshot support in `simbiguity-core`
- the `Diode` component from the "Newton–Raphson … diodes and LEDs" plan

## Reproduction Test

`series_rlc_at_resonance_draws_r_limited_current` must fail to compile today, because there is no `SignalSource`.

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