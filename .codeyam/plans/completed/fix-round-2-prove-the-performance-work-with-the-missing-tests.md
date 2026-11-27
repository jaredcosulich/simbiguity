---
title: "Fix round 2: prove the performance work with the missing tests"
mode: backend
createdAt: "2026-11-27T14:49:56Z"
source: manual
dependsOn: ["fix-round-2-trapezoidal-integrator-sign-error-and-hand-computed-transient-tests"]
---

## Summary

The performance fix (eaa1955) is the one change from the last round that is real. The stale-matrix bug, the budget gate, the grid benchmark and the frame-allocation clone are all fixed. What's missing is the proof: most of the tests the plan listed don't exist, and a new hand-written opcode encoder has no test tying it to the real encoder. This plan adds those tests and removes the duplicated encoding. Spec chain: `.codeyam/plans/completed/performance-benchmarks-wasm-size-budgets-and-an-incremental-circuit-solve.md` and `.codeyam/plans/completed/fix-performance-stale-lu-on-dt-change-budget-gate-that-cannot-fail-bogus-grid-bench.md`.

## Current behavior

- **Few cache tests.** `crates/simbiguity-circuits/src/sim.rs` has `cache_produces_bit_identical_results` (line 186) and `changing_dt_refactors_the_matrix` (line 221). Three tests are missing:
  - the seeded random-history equivalence test (cached sim vs always-re-solve sim, over many input/step sequences including capacitors and dt changes)
  - the Wheatstone EMF-only factorisation reuse
  - "a resistance change invalidates the LU"
- **Reuse test too short.** `frame_buf_reuses_allocations` (`crates/simbiguity-render/src/frame.rs:396`) encodes only 2 frames, not the plan's 100, and covers no instanced draw.
- **Duplicated opcode.** `push_draw_instanced` (`crates/simbiguity-render/src/commands.rs:227`) hand-writes opcode `9` (line 211 area) as well as the main `push(&Command::DrawInstanced { .. })` path. No test proves the two produce identical words.
- **Untested statistics.** `xtask/src/bench.rs` computes median/p95, but there's no test of the statistics or of the JSON shape.
- **Untested section and script.** `site/status/bench-section.js` has no test, and the budget check in `scripts/build-site.sh` has no test.

## Key Decisions

- **One encoder.** `push_draw_instanced` either delegates to the same code path as `push(&Command::DrawInstanced { .. })` or is removed. A test asserts the word-for-word equality of both forms.
- **The random-history equivalence test is the main guard** for the dirty flag and the LU cache. Use 500 seeded histories with `simbiguity_core::Rng`, mixing toggles, value changes, EMF changes, capacitors and dt changes, and compare bit-for-bit with a sim that refactors every step. The seed goes in the assert message.
- **The bench statistics test uses hand numbers:** samples [5, 1, 4, 2, 3] → median 3. For p95 over 1..=100, use nearest rank, which gives 95, and document the convention used.
- **The budget script test** runs `scripts/build-site.sh`'s check logic against fixture sizes. Factor the check into a small function or script that takes paths, so it can be tested without building the WASM.

## Implementation

### 1. Solver cache tests

**File**: `crates/simbiguity-circuits/src/sim.rs`

### 2. Encoder and frame tests

- **File**: `crates/simbiguity-render/src/commands.rs`
- **File**: `crates/simbiguity-render/src/frame.rs`: 100 frames of a scene with instances; buffer capacities unchanged after frame 1

### 3. Bench and budget tests

- **File**: `xtask/src/bench.rs`
- **File**: `scripts/build-site.sh` (factor out the check)
- **New file**: `site/status/bench-section.test.js`

## Reused existing code

- `CircuitSim` cache fields in `crates/simbiguity-circuits/src/sim.rs`
- `Rng` in `crates/simbiguity-core/src/rng.rs`
- `CommandBuffer::decode` in `crates/simbiguity-render/src/commands.rs`

## Reproduction Test

No known failing behaviour; this plan adds the missing proof. The first test written must be the seeded equivalence test, and it must be observed to *fail* when the dt-keyed cache check in `crates/simbiguity-circuits/src/mna.rs` is temporarily disabled. That shows the test can catch the bug it guards against. Record this in the commit body, then restore the check.

## Scenarios to Demonstrate

- Seeded equivalence test passing over 500 histories
- The same test failing (locally) with the dt cache key removed, shown in the commit body

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