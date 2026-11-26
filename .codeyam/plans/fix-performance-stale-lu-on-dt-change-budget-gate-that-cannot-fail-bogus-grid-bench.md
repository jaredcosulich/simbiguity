---
title: "Fix: performance stale LU on dt change, budget gate that cannot fail, bogus grid bench"
mode: backend
createdAt: "2026-11-26T22:14:09Z"
source: manual
dependsOn: ["fix-transient-circuits-trapezoidal-sign-error-dt-0-dc-solve-hand-computed-tests"]
---

## Summary

The Performance build added the right structure but has four real defects:
- The solver reuses a stale matrix factorisation when the time step changes, which gives wrong voltages.
- The WASM size budget recreates itself on every CI run, so it can never fail.
- The "10×10 grid" benchmark measures 100 self-loops on one node.
- Frame encoding still allocates every frame.

This fix corrects all four and adds the tests `.codeyam/plans/completed/performance-benchmarks-wasm-size-budgets-and-an-incremental-circuit-solve.md` asked for. That plan remains the spec.

## Current behavior

- **Stale LU on a dt change.**
  - `CircuitSim::step` (`crates/simbiguity-circuits/src/sim.rs:100`) always re-solves a circuit with capacitors or inductors, but passes `self.needs_refactor`, which stays `false`.
  - `solve_once` (`crates/simbiguity-circuits/src/mna.rs:395`) then skips `assemble_matrix` and `Lu::decompose` (the `if needs_refactor || cache.matrix.rows() != u.count` branch).
  - A companion conductance depends on dt (`farads / dt`, `dt / henries` in `crates/simbiguity-circuits/src/component.rs`), so stepping with a different `dt` solves with the old matrix.
- **Cache poisoned after a failure.** `cache.t` and `cache.u` are updated before factorisation, and `needs_refactor` is cleared even when factorisation failed. A later EMF-only change then hits the `"Missing LU decomp"` `SolverFailure`, or reuses an LU from a different topology.
- **A silent failure.** `lu.solve` failing returns `(None, issues)` with no fatal issue in it (the `Err(_) => return (None, issues)` arm in `solve_once`).
- **The budget can't fail.** `scripts/build-site.sh` (lines 25–31) writes `site/pkg-budget.txt` from the *current* sizes plus 5000 bytes whenever the file is missing. The file is untracked, so every fresh CI checkout regenerates a passing budget. A bridge missing from the file is skipped (`[ -n "$budget" ]`).
- **Bogus grid benchmark.** `xtask/src/bench_fixtures.rs:48` uses `vec![vec![c.node(); 10]; 10]`, which calls `c.node()` once and clones it. All 100 entries are the same node, and every resistor is a self-loop.
- **Frame allocation.** `crates/simbiguity-render/src/frame.rs:130` does `offsets: buf.offsets.clone()` on every instanced draw. `frame_buf_reuses_allocations` (`:399`) checks 2 frames, not the plan's 100, and doesn't cover this clone.
- **Missing tests:**
  - seeded dirty-flag equivalence
  - Wheatstone EMF-only factorisation reuse
  - a resistance change invalidating the LU
  - bench median/p95 and JSON shape
  - the budget script
  - `site/status/bench-section.js`

## Key Decisions

- **Key the cache on what the matrix depends on.** Store the `dt` and integrator the factorisation was built with, and refactor when either changes or when `needs_refactor` is set. Commit to the cache (`t`, `u`, `lu`) only after a successful factorisation, and clear it on failure.
- **Every `None` solution carries a fatal reason**, including an LU solve failure.
- **The budget is a committed file.** Commit `site/pkg-budget.txt` with the measured sizes plus headroom. `build-site.sh` *fails* when the file or a bridge's entry is missing, and never writes it. Updating the budget is a deliberate, reviewed edit.
- **Fix the benchmark,** and keep the old broken number out of any comparison.
- Finalize must run.

## Implementation

### 1. Solver cache

- **File**: `crates/simbiguity-circuits/src/mna.rs`: cache key includes `dt` and integrator; transactional cache update; a fatal issue on solve failure
- **File**: `crates/simbiguity-circuits/src/sim.rs`: stop clearing `needs_refactor` when the solve failed

### 2. Budget gate

- **File**: `scripts/build-site.sh`: remove the auto-create block; fail on a missing file or entry
- **New file**: `site/pkg-budget.txt`: committed. Sizes are measured at execution (current build: circuits about 132 KB, render about 205 KB, physics about 155 KB).

### 3. Benchmark and frame

- **File**: `xtask/src/bench_fixtures.rs`: build 100 distinct nodes with real grid edges
- **File**: `crates/simbiguity-render/src/frame.rs`: reuse the offsets storage instead of cloning; extend the reuse test to 100 frames covering instanced draws

### 4. Missing tests

Add the plan's list, written for a learner:
- `crates/simbiguity-circuits/src/sim.rs`: seeded equivalence of the cached sim against an always-re-solve sim, including dt changes and capacitors
- `xtask/src/bench.rs`: median/p95
- a JSON shape test
- `site/status/bench-section.test.js`

## Reused existing code

- `Lu::decompose` / `Lu::solve` in `crates/simbiguity-math/src/linsolve.rs`
- `Rng` in `crates/simbiguity-core/src/rng.rs` for the seeded equivalence test
- `FrameBuf` in `crates/simbiguity-render/src/frame.rs`

## Reproduction Test

Changing `dt` between steps reuses a matrix built for the old `dt`, so the capacitor voltage doesn't match a fresh solve.

**Target**: `crates/simbiguity-circuits/src/sim.rs` (its `tests` module). Run with `codeyam-editor editor refresh-tests --test changing_dt_refactors_the_matrix`.

```rust
/// A capacitor's step model is a conductance C/dt, so the matrix depends on
/// dt. If dt changes, a cached simulation must give exactly what a fresh
/// simulation (no cache) gives for the same history.
#[test]
fn changing_dt_refactors_the_matrix() {
    let build = || {
        let mut c = Circuit::new();
        let (p, mid, m) = (c.node(), c.node(), c.node());
        c.add_battery(p, m, 10.0);
        c.add_resistor(p, mid, 1000.0);
        let cap = c.add_capacitor(mid, m, 0.001);
        (c, cap)
    };
    let (c, cap) = build();
    let mut cached = CircuitSim::new(c);
    cached.step(Seconds(0.1));
    cached.step(Seconds(0.01));
    let v_cached = cached.state().solution().unwrap().voltage_across(cap).unwrap();

    let (c2, _) = build();
    let mut fresh = CircuitSim::new(c2);
    fresh.step(Seconds(0.1));
    fresh.force_refactor(); // test-only hook, or rebuild from snapshot; decide at execution
    fresh.step(Seconds(0.01));
    let v_fresh = fresh.state().solution().unwrap().voltage_across(cap).unwrap();
    assert_eq!(v_cached.to_bits(), v_fresh.to_bits());
}
```

Status: PROPOSED — confirm red at execution. Expected failure: the bit comparison fails, because the cached second step uses the C/0.1 matrix. The "fresh" side's refactor hook is to be settled at execution.

## Scenarios to Demonstrate

- Status-page bench table with a correct 10×10 grid number
- CI failing on a deliberately over-budget `.wasm` and on a missing budget entry