---
title: "Hardening: randomized physics-law invariants, no panics, native-vs-WASM determinism"
mode: backend
createdAt: "2026-11-26T19:29:21Z"
source: manual
---

## Summary

simbiguity's promise is **exact, deterministic, never-crashing** simulation, the thing puzzle checking, undo and replay depend on. Today that promise is backed only by hand-picked examples. Library code still has panic paths that would trap the WASM module and freeze a learner's page:

- `unreachable!` at `crates/simbiguity-circuits/src/mna.rs:352`
- `expect` at `crates/simbiguity-circuits/src/mna.rs:231`

Nothing checks that native and browser builds agree bit-for-bit. This plan hardens the engines with **seeded randomized invariant tests** (physics laws that must hold for *every* circuit), removes the remaining panics, enforces "no panics in library code" with lints, and adds a native-vs-WASM determinism check in CI.

## Key Decisions

- **Invariants, not just examples.** A seeded generator (using simbiguity's own `Rng`, so failures reproduce from a printed seed) builds thousands of random circuits. Every solved one must satisfy:
  - **Kirchhoff's current law** at every node.
  - **Tellegen's theorem**: the power delivered by sources equals the power dissipated.
  - **Repeatability**: solving twice is bit-identical.
  - **Snapshot replay**: `CircuitSim` snapshot → restore → replay is bit-identical.
  - **A reason for every failure**: a circuit with no solution reports at least one fatal `CircuitIssue`.

  These also teach: each invariant's `///` comment explains the law.
- **Hand-rolled property testing.** No `proptest`/`quickcheck`. A tiny shrink-free loop over seeds, with the failing seed in the assert message, is enough and keeps zero dependencies.
- **Impossible states become reported issues, not panics.** The `unreachable!` and `expect` paths in MNA return a fatal internal issue (e.g. `CircuitIssue::SolverFailure` with a plain-language message) instead of aborting. A WASM trap is the worst possible learner experience.
- **Enforce it with lints.** Engine and bridge crates set `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable))]`, so CLAUDE.md's "no `unwrap()` in library code" is checked by `cargo clippy`, not by review.
- **Cross-target determinism.** A native test writes golden hashes (FNV-1a over the bits of every `f64` in a fixed scenario's states, written by hand) for the circuit engine and the 3D view's command buffer. A Node test loads the built `.wasm` from `site/pkg/`, runs the same scenario through the bridge, and must produce the same hashes. This proves `simbiguity_math::float` really makes native and browser identical.

## Implementation

### 1. Random circuit generator and invariants

**New file**: `crates/simbiguity-circuits/src/invariants_tests.rs`: seeded generator (batteries with and without internal resistance, resistors, bulbs, switches, wires; random topology including deliberately broken ones) plus the invariant tests above, over a fixed range of seeds.

**File**: `crates/simbiguity-circuits/src/lib.rs`: register the test module.

### 2. Remove panic paths

**File**: `crates/simbiguity-circuits/src/mna.rs`: replace the `unreachable!` at line 352 and the `expect` at line 231 with a returned fatal issue

**File**: `crates/simbiguity-circuits/src/diagnostics.rs`: add the internal-failure `CircuitIssue` variant with a learner-readable message

**File**: `crates/simbiguity-circuits-wasm/src/issues.rs`: map the new issue kind across the bridge

### 3. Lints

Add the `deny` attributes to each engine and bridge crate's `lib.rs` (`simbiguity-math`, `-core`, `-circuits`, `-circuits-wasm`, `-anim`, `-render`, `-render-wasm`, `-circuits-view`). Fix whatever they surface, and record the count found in the commit message.

### 4. Bridge robustness

**File**: `crates/simbiguity-circuits-wasm/src/tests.rs` and `crates/simbiguity-render-wasm/src/tests.rs`: a seeded "monkey" test that calls every export with random handles, ids, NaN/∞/negative values and freed handles. Every call must return a status code, never panic.

### 5. Native ↔ WASM determinism

**New file**: `crates/simbiguity-circuits-wasm/src/golden.rs` (test): writes or asserts `golden-hashes.json` for fixed scenarios
**New file**: `site/js/determinism.test.js`: loads `site/pkg/*.wasm`, replays the scenarios and compares the hashes
**File**: `.github/workflows/ci.yml`: the `site` job builds the WASM (`scripts/build-site.sh`, with the Rust toolchain) before `node --test`, so the comparison runs on every PR

## Reused existing code

- `Rng::new`, `Rng::range`, `Rng::range_u32`, `Rng::chance` from `crates/simbiguity-core/src/rng.rs`
- `Circuit`, `CircuitSim`, `Snapshot` from `crates/simbiguity-circuits/src/netlist.rs`, `crates/simbiguity-circuits/src/sim.rs` and `crates/simbiguity-core/src/simulation.rs`
- `CircuitIssue::is_fatal` from `crates/simbiguity-circuits/src/diagnostics.rs`
- WASM loader in `site/js/simbiguity.js`

## Tests (written first)

- KCL, Tellegen, repeatability, snapshot replay and "every failure has a reason" over 2,000 seeds each. The assert message names the seed.
- A crafted circuit reaching the former `unreachable!` path returns a fatal issue instead of panicking (confirm reachability at execution; if truly unreachable, the test documents why and the code still returns an issue).
- Bridge monkey test: 10,000 random calls per bridge with no panic.
- Golden hashes: native and WASM match for the hello circuit, a Wheatstone bridge and 600 frames of the 3D view.

## Scenarios to Demonstrate

- Status page showing the new invariant tests under the circuit engine's capabilities ("obeys Kirchhoff's laws for every circuit we tried")
- CI log showing the native/WASM hash comparison passing