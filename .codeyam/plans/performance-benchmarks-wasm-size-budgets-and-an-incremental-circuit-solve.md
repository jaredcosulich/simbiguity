---
title: "Performance: benchmarks, WASM size budgets and an incremental circuit solve"
mode: backend
createdAt: "2026-11-26T19:29:21Z"
source: manual
---

## Summary

simbiguity has no way to know whether it is fast. There are no benchmarks, no WASM size budget and no frame-time numbers. It already does avoidable work every frame: `CircuitSim::step` re-runs the full analysis (`Topology::build`, assemble, dense LU, bulb-blow loop) on every call even when no input changed (`crates/simbiguity-circuits/src/sim.rs`). The 3D view steps at 60 Hz. This plan adds **measurement first, then the cheap wins it justifies**: an `xtask bench` harness with fixed fixtures, results published on the status page, a WASM size budget enforced in CI, and an incremental fast path in the circuit solver.

## Key Decisions

- **Measure before optimising.** The bench harness and baseline numbers land first in the same plan, so every speedup below is justified by a before/after number in the commit.
- **No external benchmark crates.** `xtask` is tooling, not an engine crate, but it stays dependency-free in spirit. It is a small hand-written harness: warm-up, N timed iterations with `std::time::Instant`, and reporting the median and p95. Benchmarks call the engines' public API natively.
- **Skip re-solving when nothing changed.** `CircuitSim` keeps a "dirty" flag set by `apply`. A DC `step` with a clean flag returns the cached `Analysis`. This is deterministic by construction, because the result is identical to re-solving, and a test proves it bit-for-bit. Once the transient-circuits plan lands, a circuit with capacitors or inductors is always dirty.
- **Reuse the factorisation when only source values change.** A battery EMF slider changes only the right-hand side, so the stored `Lu` (`crates/simbiguity-math/src/linsolve.rs` already exposes `Lu::decompose` / `Lu::solve`) can be reused. Changes to resistance or switch state re-stamp and refactor.
- **No sparse solver yet.** Dense LU is O(n³), but puzzle-sized circuits are tens of nodes. The bench includes a 500-resistor ladder to show where dense LU stops being fine. Writing a sparse solver is a future plan if, and only if, the numbers demand it.
- **Budgets are measured, then pinned.** The WASM size budget and per-frame encode budget are set at execution time from the measured baseline plus headroom. They are not guessed here.

## Implementation

### 1. Bench harness

**File**: `xtask/src/main.rs`: add a `bench` subcommand alongside `site-report`
**New file**: `xtask/src/bench.rs`: harness (warm-up, iterations, median/p95, JSON output)
**New file**: `xtask/src/bench_fixtures.rs`: fixed workloads:
- hello circuit solve
- 10/100/500-resistor ladders
- a 10×10 resistor grid
- `CircuitView` advance + encode, 600 frames
- `Lu::decompose` at n = 10/50/200

### 2. Publish and gate

**File**: `.github/workflows/pages.yml`: run `cargo run -p xtask --release -- bench --out site/status/bench.json` and publish it
**New file**: `site/status/bench-section.js` (+ test): renders the bench table on the status page
**File**: `scripts/build-site.sh`: compare each `.wasm` size to a budget file and fail if it is over
**New file**: `site/pkg-budget.txt`: per-bridge byte budgets (baseline + headroom, set at execution)

### 3. Solver fast paths

**File**: `crates/simbiguity-circuits/src/sim.rs`: dirty flag; `step` returns the cached analysis when clean
**File**: `crates/simbiguity-circuits/src/mna.rs`: separate "assemble + factor" from "solve for this RHS", so an EMF-only change reuses the factorisation; reuse matrix storage between solves

### 4. Frame encode allocations

**File**: `crates/simbiguity-render/src/frame.rs`

Check `encode_frame` / `CommandBuffer` for per-frame heap allocation. If it allocates, let callers pass a reusable buffer that is cleared and refilled, so `c3d_encode` allocates nothing in steady state. Add a test that counts the buffer's capacity across frames.

## Reused existing code

- `Lu::decompose`, `Lu::solve` from `crates/simbiguity-math/src/linsolve.rs`
- `CircuitSim` in `crates/simbiguity-circuits/src/sim.rs`; `analyze` in `crates/simbiguity-circuits/src/mna.rs`
- `CircuitView` from `crates/simbiguity-circuits-view/src/lib.rs`
- `site-report` command structure in `xtask/src/main.rs` and its report JSON in `xtask/src/report.rs`
- Status page modules under `site/status/`

## Tests (written first)

- Dirty-flag equivalence: for a seeded random sequence of inputs and steps, the cached `CircuitSim` gives bit-identical `Analysis` values to one that always re-solves.
- EMF-only change: reusing the factorisation gives bit-identical voltages to a fresh solve (Wheatstone bridge example).
- A resistance change invalidates the cached factorisation (the result matches a fresh solve, not the stale one).
- Bench harness: median/p95 of a known synthetic sample list are computed correctly, and the JSON output is stable in shape.
- The budget check fails on an over-budget file and passes under budget (script tested on fixture sizes).
- Frame buffer reuse: encoding 100 frames does not grow the reused buffer's capacity after the first.

## Scenarios to Demonstrate

- Status page bench table with baseline numbers
- Before/after numbers for the dirty-flag fast path in the commit message
- CI failing a deliberately over-budget WASM file