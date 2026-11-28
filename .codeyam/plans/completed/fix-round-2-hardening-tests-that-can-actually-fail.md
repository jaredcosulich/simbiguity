---
title: "Fix round 2: hardening tests that can actually fail"
mode: backend
createdAt: "2026-11-27T14:49:56Z"
source: manual
dependsOn: ["fix-round-2-prove-the-performance-work-with-the-missing-tests", "fix-round-2-physics-energy-that-is-really-conserved-and-tests-from-the-textbook", "fix-round-2-3d-view-per-bulb-glow-strict-layouts-and-a-parallel-demo-that-is-not-shorted"]
---

## Summary

The hardening fix (757fe5a) replaced the missing tests with **tests that cannot fail**:
- the determinism checks assert `1 == 1`
- the "KCL invariant" loops over 2,000 circuits and checks nothing
- the bridge stress test calls new/solve/free on an empty circuit

Their `///` descriptions claim real checks, and those descriptions are published on the public status page. This plan writes the invariants, stress tests and native-vs-WASM determinism check for real. Spec chain: `.codeyam/plans/completed/hardening-randomized-physics-law-invariants-no-panics-native-vs-wasm-determinism.md` and `.codeyam/plans/completed/fix-hardening-deliver-the-invariants-stress-tests-and-native-vs-wasm-check.md`.

## Current behavior

- **Empty invariant.** `kcl_invariant` (`crates/simbiguity-circuits/src/invariants_tests.rs:42`) generates circuits with `generate_circuit` (line 7) and then does `if let Some(_sol) = analysis.solution() { // simple check }` (line 48). It asserts nothing. There are no Tellegen, repeatability, snapshot-replay or "every failure has a reason" invariants.
- **Constant golden test.** `golden_determinism_check` (`crates/simbiguity-circuits-wasm/src/golden.rs:6`) is `assert_eq!(1, 1)`. Its `///` says it "produces deterministic outputs matching golden hashes".
- **Constant JS test.** `site/js/determinism.test.js` is `assert.strictEqual(1, 1)`.
- **Trivial stress test.** `bridge_stress_test` (`crates/simbiguity-circuits-wasm/src/tests.rs:280`) loops `circuit_new` / `circuit_solve` / `circuit_free` 10,000 times on an empty circuit. Its comment says "10000 seeded random calls", but there is no RNG, no invalid handles, no NaN, and no render or physics bridge coverage.
- **Unused WASM build.** The CI `site` job builds the WASM (`.github/workflows/ci.yml`), but no test consumes it.

## Key Decisions

- **Real invariants over 2,000 seeds each**, with the seed in every message. Each `///` explains the law for a learner.
  - **KCL:** the net current at every node is below 1e-9 × the largest current magnitude in that circuit (a relative tolerance, so big and small circuits are judged alike).
  - **Tellegen:** |Σ V·I| over all elements is below 1e-9 × Σ |V·I|.
  - **Repeatability:** two solves are bit-identical.
  - **Snapshot replay:** bit-identical.
  - **Every failure has a reason:** a missing solution always comes with at least one fatal `CircuitIssue`.
  - **Coverage:** the generator includes capacitors and inductors, stepped with both integrators, plus deliberately broken topologies.
- **Prove each invariant can fail.** For each one, record in the commit body that it went red when the property was broken on purpose, e.g. negating one wire current. Do not commit the sabotage.
- **Real stress tests.** 10,000 seeded calls per bridge (circuits, render, physics) choosing random exports and arguments: freed, zero and huge handles; out-of-range ids; NaN, ±∞ and negative values. Every call returns a status, and nothing panics.
- **Real determinism.**
  - A native test computes an FNV-1a hash (hand-written) over the bits of every `f64` in a fixed scenario's states, and over the 3D view's command buffer after 600 frames. It compares them with committed values in `crates/simbiguity-circuits-wasm/golden-hashes.json` (new).
  - `site/js/determinism.test.js` loads `site/pkg/*.wasm`, runs the same scenarios through the bridge and asserts the same hashes.
  - CI's `site` job already builds the WASM, so this runs on every PR.
  - Scenarios: the hello circuit, a Wheatstone bridge, 100 trapezoidal RC steps, and 600 frames of the 3D view.

## Implementation

### 1. Invariants

**File**: `crates/simbiguity-circuits/src/invariants_tests.rs`

### 2. Stress tests

- **File**: `crates/simbiguity-circuits-wasm/src/tests.rs`
- **File**: `crates/simbiguity-render-wasm/src/tests.rs`
- physics bridge tests in `crates/simbiguity-physics-wasm/src/lib.rs`

### 3. Determinism

- **File**: `crates/simbiguity-circuits-wasm/src/golden.rs`
- **New file**: `crates/simbiguity-circuits-wasm/golden-hashes.json`
- **File**: `site/js/determinism.test.js`

## Reused existing code

- `generate_circuit` in `crates/simbiguity-circuits/src/invariants_tests.rs` (extend it; don't replace it)
- `Rng` in `crates/simbiguity-core/src/rng.rs`
- WASM loader in `site/js/simbiguity.js`
- CI `site` job in `.github/workflows/ci.yml`

## Reproduction Test

The determinism test asserts a constant, so it passes no matter what the engine does.

**Target**: `crates/simbiguity-circuits-wasm/src/golden.rs`. Run with `codeyam-editor editor refresh-tests --test golden_determinism_check`.

Change the existing test: replace `assert_eq!(1, 1)` with a hash computed from the hello-circuit and Wheatstone scenarios, compared with `golden-hashes.json`. Prove it is not vacuous by changing one resistor value in the scenario locally and observing it go red (record this in the commit body).

Status: PROPOSED — confirm red at execution. Expected first red: there is no `golden-hashes.json` yet, so the test fails to load it until the hashes are generated once and committed.

## Scenarios to Demonstrate

- Status page showing the invariant tests with learner-facing descriptions
- CI log: native and WASM hashes compared and equal
- Commit body listing each invariant's deliberate-break red

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