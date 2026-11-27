---
title: "Fix round 2: trapezoidal integrator sign error and hand-computed transient tests"
mode: ui
createdAt: "2026-11-27T14:49:55Z"
source: manual
dependsOn: ["fix-round-2-make-the-branch-green-and-keep-it-that-way"]
---

## Summary

**The trapezoidal integrator is still wrong.** It gives 0.476 V where the answer is 6.3249 V, and the demo now offers it to learners as a toggle. The first fix (1f98951) didn't change the sign code at all, didn't add the test the plan specified, left the 0.5 V tolerances in place, and deleted two diagnostics tests. This plan fixes the sign convention, adds the hand-computed tests and restores what was lost. Spec chain: `.codeyam/plans/completed/circuits-that-change-over-time-capacitors-inductors-and-transient-analysis.md` and `.codeyam/plans/completed/fix-transient-circuits-trapezoidal-sign-error-dt-0-dc-solve-hand-computed-tests.md`.

## Current behavior

- **Sign mismatch (unchanged since the last review).**
  - `component_current` at `crates/simbiguity-circuits/src/mna.rs:301` returns `current - g * d` for a Norton stamp (`crates/simbiguity-circuits/src/mna.rs:304`). That is the source-convention current, out of `+`.
  - That value is stored as the element's `amps`, and the trapezoidal companions then add it as if it were the passive current: `crates/simbiguity-circuits/src/component.rs:176` (`g * volts + amps`) and `crates/simbiguity-circuits/src/component.rs:193` (`amps + g * volts`).
  - Measured on the current tree:

    | Case | Got | Should be |
    |---|---|---|
    | RC, 1 kΩ / 1 mF / 10 V, dt = 0.1, 10 steps, trapezoidal | 0.476190 V | 6.3249 V |
    | Same RC case, backward Euler | 6.144567 V | 6.1446 V (correct) |
    | RL inductor current, backward Euler | −0.6145 A | +0.6145 A |

- **dt guard missing.** `analyze_step` (`crates/simbiguity-circuits/src/mna.rs:541`) has no guard. `CircuitIssue::InvalidTimeStep` (`crates/simbiguity-circuits/src/diagnostics.rs:64`) and `CircuitIssue::Inrush` (`crates/simbiguity-circuits/src/diagnostics.rs:58`) are declared but never emitted. `circuit_step(h, 0.0)` and `circuit_step(h, NaN)` still return a `FloatingNode` issue.
- **Ambiguous issue codes.** `crates/simbiguity-circuits-wasm/src/issues.rs:10` maps `SolverFailure` to 7, and `crates/simbiguity-circuits-wasm/src/issues.rs:17` maps `Inrush` to 7 as well. `InvalidTimeStep` → 8 (`issues.rs:18`) has no name in `site/js/simbiguity.js`'s `IssueKind`.
- **Weak tests remain.** `crates/simbiguity-circuits/tests/analysis.rs:410`, `:420`, `:451` and `:460` still use `< 0.5` tolerances. Line 451 compares `sim_i.abs()`, which hides the sign. No test uses `Integrator::Trapezoidal`.
- **Deleted tests.** 1f98951 deleted `fatal_issues_are_exactly_the_unsolvable_ones` and `messages_name_the_part` from `crates/simbiguity-circuits/src/diagnostics.rs`. Restore them from `git show 1f98951^:crates/simbiguity-circuits/src/diagnostics.rs`.
- **Phantom proofs.** `crates/simbiguity-circuits/capabilities.toml` lists `proves` entries for tests that don't exist.
- **Placeholder test.** `site/demos/rc-charging/state.test.js` is a placeholder reading "Tests are deferred until Extract TDD step."
- **Unescaped JSON.** `circuit_plot_draw` (`crates/simbiguity-circuits-wasm/src/lib.rs:291`) hand-builds JSON with `format!`, so a NaN or a `"` in a label breaks the parse.

## Key Decisions

- **One convention: store the passive current.** Convert from the Norton read-back in exactly one place, and document the trapezoidal companion derivation in a `///` comment.
- **Guard dt at the entry.** `analyze_step` returns a fatal `InvalidTimeStep` when `dt` isn't finite and positive. Emit `Inrush` when an ideal battery sits directly across an uncharged capacitor.
- **Every issue gets a distinct bridge code, named in JS,** with a test that walks every `CircuitIssue` variant.
- **Replace the plot JSON with a packed numeric buffer** (`f64`s plus string-table indices), or escape it properly. NaN or inf samples are dropped, with a test.

## Implementation

### 1. Sign fix

**File**: `crates/simbiguity-circuits/src/mna.rs`, **File**: `crates/simbiguity-circuits/src/component.rs`

### 2. dt guard, Inrush, issue codes

- **File**: `crates/simbiguity-circuits/src/mna.rs`
- **File**: `crates/simbiguity-circuits/src/diagnostics.rs`
- **File**: `crates/simbiguity-circuits-wasm/src/issues.rs`
- **File**: `site/js/simbiguity.js`

### 3. Tests

**File**: `crates/simbiguity-circuits/tests/analysis.rs`: replace the `< 0.5` checks with the numbers below, signed currents and analytic energy.

| Case | Expected |
|---|---|
| RC, backward Euler | 6.1446 V |
| RC, trapezoidal | 6.3249 V |
| RL (10 V, R = 10 Ω, L = 1 H, so τ = 0.1 s; dt = 0.01, 10 steps), backward Euler | +0.6145 A |
| LC (L = 1 H, C = 1 F, V₀ = 1 V, dt = 0.01), trapezoidal | energy within 1e-9 of 0.5 J after 1000 steps |
| Same LC, backward Euler | each step scales energy by 1/(1 + ω²dt²) = 1/1.0001, so after 1000 steps 0.5·1.0001⁻¹⁰⁰⁰ ≈ 0.4524 J; assert below 0.46 J |
| DC limit after 10τ | equals `Circuit::solve()` |
| Snapshot → restore → replay | bit-identical |

Restore the two deleted diagnostics tests.

### 4. Demo and capability

- **File**: `site/demos/rc-charging/state.test.js`: real tests of `state.js`
- **File**: `crates/simbiguity-circuits-wasm/src/lib.rs`: plot export fix
- **File**: `crates/simbiguity-circuits/capabilities.toml`: only real test names

## Reused existing code

- `CircuitSim::with_integrator` in `crates/simbiguity-circuits/src/sim.rs`
- `Circuit::add_capacitor` in `crates/simbiguity-circuits/src/netlist.rs`
- The two diagnostics tests as they existed at `1f98951^`

## Reproduction Test

Trapezoidal RC charging gives about 0.48 V after one time constant, instead of 6.3249 V.

**Target**: `crates/simbiguity-circuits/tests/analysis.rs`. Run with `codeyam-editor editor refresh-tests --test trapezoidal_rc_matches_hand_computed_discrete_answer`.

```rust
/// RC charging, R = 1 kΩ and C = 1 mF (so τ = 1 s), from 10 V, using ten
/// trapezoidal steps of dt = 0.1 s. Each step multiplies the remaining gap
/// by (1 - dt/2τ)/(1 + dt/2τ) = 0.95/1.05, so V = 10·(1 - (0.95/1.05)^10)
/// = 6.3249 V. That is very close to the textbook 10·(1 - e⁻¹) = 6.3212 V,
/// which is why trapezoidal is called second-order accurate.
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

Status: PROPOSED — confirm red at execution. Expected failure: "got 0.476…, expected 6.3249…". This number was measured on the current tree during review.

## Scenarios to Demonstrate

- The RC demo with the trapezoidal toggle on, charging to 6.32 V at t = τ
- LC ringing: sustained under trapezoidal, decaying under backward Euler
- `circuit_step(h, 0)` showing the plain-language "time step must be positive" issue

## Acceptance rules (the whole build must meet these)

The previous two builds of this work passed review with stubbed code. These rules exist because of specific failures seen there:

- **Red first.** Write the Reproduction Test before changing any code, and observe it fail on the current tree. Record the failure message in the commit body.
- **No hollow assertions.** These are all forbidden:
  - asserting a constant (`assert_eq!(1, 1)`, `assert.strictEqual(1, 1)`)
  - asserting something always true (`x > -1.0`)
  - empty `if let` / loop bodies that check nothing
  - goals that `return true`
  - loosening a tolerance until the test passes
- **Numbers come from this plan, not from the code.** If the code disagrees with a hand-computed number here, the code is wrong. Do not re-derive the expectation from the output.
- **Touch only what's listed.** Before committing, `git diff --stat <start>..HEAD` must show no changes to `rust-toolchain.toml`, `.gitignore`, or crates this plan doesn't name. A previous build silently reverted another plan's work this way. If such a change appears, restore the file from the starting commit.
- **Everything green, and finalize runs:**
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `sh scripts/build-site.sh`
  - `node --test "site/**/*.test.js"`

  No `Codeyam-Finalize: deferred`.
- **No junk or placeholders.** Don't commit scratch, `.patch`, `.orig` or `.rej` files. Don't leave placeholder comments ("would go here", "omitted for brevity", "for prototype", "wait…", "deferred").
- **Every `proves` entry exists.** Each entry in a touched `capabilities.toml` must name a test that exists and passes.