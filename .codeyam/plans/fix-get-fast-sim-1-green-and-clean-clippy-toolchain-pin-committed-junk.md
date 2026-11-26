---
title: "Fix: get fast-sim-1 green and clean (clippy, toolchain pin, committed junk)"
mode: backend
createdAt: "2026-11-26T22:13:44Z"
source: manual
---

## Summary

`fast-sim-1` can't merge to `main` right now. CI's clippy step fails, and the last round of builds committed scratch and merge-leftover files. This plan makes the branch green and clean so every later fix plan starts from a passing tree. It is deliberately tiny, and every other fix plan depends on it.

## Current behavior

- `cargo clippy --workspace --all-targets -- -D warnings` fails with two `needless_range_loop` errors. Both come from the Performance build's rewrite of the MNA solver:
  - `crates/simbiguity-circuits/src/mna.rs:129` — `for node in 0..n`, whose body (lines 130–131) indexes `touched`, `reported` and `t.island` by `node`.
  - `crates/simbiguity-circuits/src/mna.rs:153` — `for root in 0..n`, whose body (lines 154–155) indexes `voltage` and `t.group` by `root`.
- The Rust toolchain pin (`rust-toolchain.toml` on `main`, channel 1.99.0, which adds wasm32 + clippy + llvm-tools) exists on `main` (commit `ace704f`) but was never brought onto this branch. The branch builds with whatever toolchain each machine happens to have.
- Junk is tracked in git:
  - `scratch.py` (added in 4971487)
  - `scratch-tests.rs` and `scratch-node-fix.py` (added in cc0b50c)
  - `summary.json` (added in 203f311)
  - `.codeyam/editor.json.orig` and `.codeyam/editor.json.rej` (a failed patch, added in 16cb5cd)
  - `.gemini/hooks/__pycache__/editor-step-hook.cpython-310.pyc`

## Key Decisions

- **Fix the two loops by iterating, not by `#[allow]`.** For example, `for (node, &isl) in t.island.iter().enumerate()`. Silencing the lint would contradict the "deny warnings" gate.
- **Bring the toolchain pin over verbatim** from `main` (`git show main:rust-toolchain.toml`) rather than merging all of `main`. That keeps this plan's diff to exactly what it claims.
- **Untrack the junk and ignore its patterns** (`scratch*`, `*.orig`, `*.rej`, `__pycache__/`, `summary.json` at the root) so a future build can't recommit it. The `.pyc` lives under `.gemini/`, an agent-config directory. Only `git rm --cached` the compiled cache file; do not edit any hook source.

## Implementation

### 1. Clippy

**File**: `crates/simbiguity-circuits/src/mna.rs`: rewrite the loops at lines 129 and 153 with `iter().enumerate()`, with no behaviour change. The existing MNA tests in `crates/simbiguity-circuits/src/mna_tests.rs` must still pass.

### 2. Toolchain pin

**New file**: `rust-toolchain.toml` (new on this branch): copied from `main`.

### 3. Junk

**File**: `.gitignore`: add the patterns above. Then `git rm --cached` the seven files listed.

## Reused existing code

- The toolchain pin's content from `main` (`git show main:rust-toolchain.toml`)
- Existing MNA tests in `crates/simbiguity-circuits/src/mna_tests.rs` act as the regression guard

## Reproduction Test

No unit-level repro: the failure is a lint, and its proof is `cargo clippy --workspace --all-targets -- -D warnings` exiting 0 with the pinned 1.99.0 toolchain.

## Scenarios to Demonstrate

- Clippy, fmt and the whole test suite green on the pinned toolchain
- `git ls-files | grep -E 'scratch|\.orig$|\.rej$|\.pyc$|^summary.json$'` prints nothing