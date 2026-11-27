---
title: "Fix round 2: make the branch green and keep it that way"
mode: backend
createdAt: "2026-11-27T14:49:55Z"
source: manual
---

## Summary

The branch still can't merge. The first cleanup commit (a0c5ca1) fixed clippy, added the toolchain pin and added ignore rules, but the next build (1f98951) reverted all three. The junk files were never untracked, and more have been added since. This plan redoes the cleanup and makes it stick. Every other round-2 plan depends on it.

## Current behavior

- `cargo clippy --workspace --all-targets -- -D warnings` fails with `needless_range_loop` at `crates/simbiguity-circuits/src/mna.rs:129` (`for node in 0..n`) and `crates/simbiguity-circuits/src/mna.rs:153` (`for root in 0..n`).
- The toolchain pin, `rust-toolchain.toml` (channel 1.99.0), is absent. It was added in a0c5ca1 and deleted in 1f98951. The source is `git show a0c5ca1:rust-toolchain.toml`.
- `.gitignore` has no patterns for scratch or leftover files. They were added in a0c5ca1 and removed in 1f98951.
- These files are tracked junk:
  - `scratch.py`, `scratch-tests.rs`, `scratch-node-fix.py`, `fix_lints.sh`
  - `summary.json`
  - `.codeyam/editor.json.orig`, `.codeyam/editor.json.rej`, `.codeyam/editor.json.patch`
  - `crates/simbiguity-physics/src/world.rs.patch`
  - `.gemini/hooks/__pycache__/editor-step-hook.cpython-310.pyc`

## Key Decisions

- **Rewrite the two loops with `iter().enumerate()`**, with no behaviour change and no `#[allow]`.
- **Restore the toolchain pin and ignore rules** from a0c5ca1 exactly. Add `*.patch`, `fix_*.sh` and `scratch*` patterns.
- **`git rm --cached` every file listed** (and delete it from disk). `.gemini/` is agent config: remove only the compiled `.pyc`, and never edit a hook source.
- **A regression guard so it can't silently come back.** Add a CI step (in `.github/workflows/ci.yml`) that fails if `rust-toolchain.toml` is missing or if `git ls-files` matches the junk patterns.

## Implementation

### 1. Clippy

**File**: `crates/simbiguity-circuits/src/mna.rs`: lines 129 and 153.

### 2. Pin and ignore rules

- **New file**: `rust-toolchain.toml` (new on this branch): from a0c5ca1
- **File**: `.gitignore`

### 3. Untrack the junk, and add the guard

Run `git rm --cached` on the files listed above.

**File**: `.github/workflows/ci.yml`: add a "repo hygiene" step to the `rust` job.

## Reused existing code

- The toolchain pin and ignore rules as written in commit a0c5ca1
- Existing MNA tests in `crates/simbiguity-circuits/src/mna_tests.rs` as the regression guard for the loop rewrite

## Reproduction Test

No unit repro. The red is `cargo clippy --workspace --all-targets -- -D warnings` failing at the two lines above. The green is it exiting 0, together with `git ls-files | grep -E 'scratch|fix_lints|\.patch$|\.orig$|\.rej$|\.pyc$|^summary.json$'` printing nothing.

## Scenarios to Demonstrate

- Clippy, fmt and tests green on the pinned 1.99.0 toolchain
- The CI hygiene step failing when a `scratch.py` is staged (shown locally, not committed)

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