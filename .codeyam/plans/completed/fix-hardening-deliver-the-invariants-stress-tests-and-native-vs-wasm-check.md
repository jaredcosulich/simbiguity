---
title: "Fix: hardening deliver the invariants, stress tests and native-vs-WASM check"
mode: backend
createdAt: "2026-11-26T22:14:09Z"
source: manual
dependsOn: ["fix-performance-stale-lu-on-dt-change-budget-gate-that-cannot-fail-bogus-grid-bench", "fix-physics-engine-presets-crash-lost-pushes-rod-energy-missing-worked-example-tests"]
---

## Summary

The Hardening build did the panic removal and added lints to eight crates. It skipped what the plan was mainly for:
- the randomized physics-law invariants
- the bridge stress tests
- the native-vs-WASM determinism check

Its new tests also pin the wrong behaviour: an empty circuit now reports "solver failure". This fix delivers the skipped half of `.codeyam/plans/completed/hardening-randomized-physics-law-invariants-no-panics-native-vs-wasm-determinism.md` (still the spec), extends the lints to the six crates added since, and maps the new issue in the browser. It runs after the capacitor and performance fixes, so the invariants exercise the final solver, including capacitors.

## Current behavior

- **Invariants missing.** There is no `crates/simbiguity-circuits/src/invariants_tests.rs`: no seeded random circuits and no checks for KCL, Tellegen, repeatability, snapshot replay, or "every failure has a reason".
- **Bridge stress tests missing.** `crates/simbiguity-circuits-wasm/src/issues.rs` calls a single mapping test a "monkey test". No test calls exports with random handles, ids or NaN values.
- **Determinism check missing.** There is no `crates/simbiguity-circuits-wasm/src/golden.rs` and no `site/js/determinism.test.js`. The `site` job in `.github/workflows/ci.yml` still runs only `node --test` and builds no WASM.
- **Tests that pin a bug.**
  - `solve_once_returns_failure_on_missing_topology` (`crates/simbiguity-circuits/src/mna_tests.rs:188`) solves an *empty* circuit with `needs_refactor = false` and asserts a `SolverFailure`.
  - Because `solve_once` builds the topology itself, the failure really comes from the `"Missing LU decomp"` branch. So the test proves "an empty circuit is a solver failure", which is wrong (an empty circuit is trivially solved), and its name describes a different path.
  - `solve_once_returns_failure_on_missing_unknowns` (`:199`) has the same problem.
  - Their `///` descriptions are terse and not written for learners.
- **Browser can't name the new issue.** `circuit_issue_kind` returns `7` for `SolverFailure` (`crates/simbiguity-circuits-wasm/src/issues.rs:10`), but `IssueKind` in `site/js/simbiguity.js` stops at `6: "bulb-blown"`, so the browser shows `kind: undefined`.
- **Lints don't cover newer crates.** The `deny(clippy::unwrap_used, …)` attributes are absent from `simbiguity-puzzle`, `simbiguity-circuits-puzzle`, `simbiguity-physics`, `simbiguity-physics-wasm`, `simbiguity-physics-view` and `simbiguity-plot`. Their `lib.rs` files contain no `deny(clippy`.
- **Registry out of date.** The Hardening commit dropped about 1,181 node-test entries from `.codeyam/test-registry.json` while the tests still exist (e.g. `site/status/report-view.test.js`).

## Key Decisions

- **The invariants are the point.** Use 2,000 seeds per invariant, with the seed in every assert message. The generator includes capacitors and inductors, stepped with both integrators, plus deliberately broken topologies.
- **Delete or rewrite tests that pin wrong behaviour.** An empty circuit solves with no issues. Internal cache-miss paths are tested through `CircuitSim` histories, not by poking `solve_once` with impossible arguments.
- **The determinism check runs in CI on every PR.** The `site` job installs the pinned Rust toolchain and runs `scripts/build-site.sh` before `node --test`.
- **Regenerate the registry** through codeyam's own refresh, not by hand.
- Finalize must run.

## Implementation

### 1. Invariants

**New file**: `crates/simbiguity-circuits/src/invariants_tests.rs`, registered in `crates/simbiguity-circuits/src/lib.rs`.

### 2. Fix the misleading tests

**File**: `crates/simbiguity-circuits/src/mna_tests.rs`: replace the two tests at lines 188 and 199 with "an empty circuit solves cleanly" and history-driven cache tests.

### 3. Bridge stress tests and the issue mapping

- **File**: `crates/simbiguity-circuits-wasm/src/tests.rs`, `crates/simbiguity-render-wasm/src/tests.rs`, and the physics bridge tests: 10,000 seeded random calls each, with no panics
- **File**: `site/js/simbiguity.js`: `7: "solver-failure"`, plus a test

### 4. Determinism

- **New file**: `crates/simbiguity-circuits-wasm/src/golden.rs`
- **New file**: `site/js/determinism.test.js`
- **File**: `.github/workflows/ci.yml`: build the WASM in the `site` job

### 5. Lints

Add the `cfg_attr(not(test), deny(...))` block to the six crates above and fix what it surfaces.

## Reused existing code

- `Rng` in `crates/simbiguity-core/src/rng.rs`
- `CircuitIssue::is_fatal` in `crates/simbiguity-circuits/src/diagnostics.rs`
- The existing deny block in `crates/simbiguity-circuits/src/lib.rs`, as the template
- WASM loader in `site/js/simbiguity.js`

## Reproduction Test

An empty circuit is reported as a solver failure instead of solving trivially.

**Target**: `crates/simbiguity-circuits/src/mna_tests.rs`. Run with `codeyam-editor editor refresh-tests --test empty_circuit_solves_with_no_issues`.

```rust
/// A circuit with nothing in it has nothing to solve: no voltages, no
/// currents, and nothing wrong with it. Reporting a "solver failure" here
/// would tell a learner who has just cleared the board that something broke.
#[test]
fn empty_circuit_solves_with_no_issues() {
    let analysis = Circuit::new().solve();
    assert!(analysis.issues().is_empty(), "{:?}", analysis.issues());
}
```

Status: PROPOSED — confirm red at execution. Expected failure: if the public DC path shares the cached `solve_once` path, `issues` contains `SolverFailure { message: "Missing LU decomp" }`. If `Circuit::solve` happens to pass, the bug is confined to `CircuitSim`, so repeat the check through `CircuitSim::new(Circuit::new()).step(…)` and keep whichever goes red.

## Scenarios to Demonstrate

- Status page lists the invariant tests under the circuit engine ("obeys Kirchhoff's laws for every circuit we tried")
- CI log showing the native/WASM hash comparison
- A seeded invariant failure message naming its seed (shown by temporarily breaking KCL locally, not committed)