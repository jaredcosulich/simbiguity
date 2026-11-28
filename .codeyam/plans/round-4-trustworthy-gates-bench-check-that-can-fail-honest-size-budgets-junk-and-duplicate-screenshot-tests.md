---
title: "Round 4: trustworthy gates — bench check that can fail, honest size budgets, junk and duplicate-screenshot tests"
mode: backend
createdAt: "2026-11-28T16:56:22Z"
source: manual
---

## Summary

Round 3 left the quality gates themselves broken or loosened:

- **`scripts/check-budgets.sh:44` errors and still passes.** It prints `[: 100017106951829432100100879510554: integer expression expected`, because `grep -A 1 … | tr -dc '0-9'` glues several numbers together. So no benchmark budget in `site/bench-budget.txt` has ever been enforced, yet the script exits 0.
- **The WASM size budgets were raised without a written reason.** In `site/pkg-budget.txt`, render went 230,000 → 253,000 (in `c85abd5` and `e643982`) and physics went 180,000 → 210,000.
- **`test_x.rs` was committed at the repo root.** It's scratch code with a "wait, I want to see the panic" comment.
- **Two pairs of scenario screenshots are byte-identical** under `.codeyam/scenarios/screenshots/`.

This plan makes the gates trustworthy again, so later plans can't quietly pass.

## Key Decisions

**Parse bench JSON properly.** Have `xtask` print the check itself: add `cargo xtask check-bench site/bench-budget.txt`.
- It reads its own bench output as structured data and compares each fixture's `medianUs`.
- It exits non-zero on a missing fixture, a missing budget, or anything over budget.
- The shell script just calls it. No more `grep`/`tr` on JSON.

**Re-baseline size budgets once, honestly.** Diodes, AC, sparse and the puzzle exports are real growth.
- Set each budget to the current measured size + 2%.
- Above each line, add a `# reason:` comment naming the features that grew it.
- Add a test that every budget line has a `# reason:` comment above it and that no budget exceeds the measured size by more than 5%. That stops future silent bumps.

**Junk guard.** Add a test (`xtask/src/repo_hygiene_tests.rs`, new) that fails if `git ls-files` lists any of:
- root-level `*.rs`;
- `patch.diff`, `*.orig`, `*.rej` or `*.patch`;
- `scratch*`.

**Screenshot uniqueness.** Add a test that fails if any two `*.png.hash` files under `.codeyam/scenarios/screenshots/` hold the same hash.
1. Find the duplicate pairs.
2. Recapture them so each shows its own intended state, or delete a scenario that has no distinct state.

## Implementation

### 1. `cargo xtask check-bench`

**Files**:
- `xtask/src/main.rs`
- `xtask/src/bench.rs`
- `scripts/check-budgets.sh`

Tests cover: over budget fails; missing fixture fails; within budget passes. Include a JSON fixture with several fixtures, mirroring today's output, as a regression test for the concatenation bug.

### 2. Re-baselined `site/pkg-budget.txt` with reasons

Add the reasons and the budget-file test described above.

### 3. Repo hygiene test, and delete `test_x.rs`

Add the junk-guard test, then `git rm test_x.rs`.

### 4. Screenshot uniqueness test, and fix the duplicates

Add the uniqueness test, then fix the duplicate pairs it finds.

### 5. Wire into CI

**File**: `.github/workflows/ci.yml`

Make sure `check-budgets.sh` runs in the `rust` job and exits non-zero on failure.

## Reused existing code

- `xtask/src/bench.rs`
- `xtask/src/bench_fixtures.rs`
- `xtask/src/check_budgets_tests.rs`
- `scripts/check-budgets.sh`

## Reproduction Test

`check_bench_rejects_a_fixture_over_budget`: given JSON with `ladder_1000_dc` at 50,000 µs and a budget of 35,586, it must fail. Today the shell version prints an integer error and exits 0.

## Acceptance rules (the whole build must meet these)

Every rule below comes from a failure seen in an earlier round on this branch. Finalize may be deferred; that is the owner's choice. These rules still apply to every plan.

- **If you can't hit a number, say so. Don't move it.**
  - When code can't meet a value in this plan, keep the test with the plan's value and tolerance, and mark it `#[ignore = "misses plan value: observed <x>"]` (Rust) or `test.todo` (Node).
  - List it under **NOT DELIVERED** in the commit body.
  - An honest ignored test is fine. A test whose expected value or tolerance was changed to match the code is not. In round 3, the friction test was changed from 0.4077 to 0.4177, and four gravity tolerances were loosened up to 500×.
- **Red first.** Write the named tests before the code, and watch them fail on the starting tree.
- **No hollow tests.** Every test must assert on a value the code computed. Forbidden:
  - a test that only checks the code "doesn't panic";
  - asserting a constant;
  - an always-true condition;
  - an empty loop body.
- **No fake output.** Display code may only show values that came from the engine. Examples of what's forbidden: `Math.sin(now…)` drawn as a scope trace, or hard-coded messages pretending to be engine output.
- **No placeholders.** These phrases must not appear in new code: "stub", "would go here", "wait", "for prototype", "We just verify", "once the TDD phase starts", "omitted".
- **Don't drop plan content.** Every goal, experiment, preset and test this plan lists must exist, or be listed under NOT DELIVERED.
- **Budgets aren't raised to pass.** That covers `site/pkg-budget.txt`, `site/bench-budget.txt` and any test tolerance. The exception is the budget-reset plan, which does it once with a written reason.
- **Touch only what's listed.**
  - `git diff --stat <start>..HEAD` must show no changes to `rust-toolchain.toml`, `.gitignore`, or crates and demos this plan doesn't name.
  - Don't commit junk at the repo root: `test_*.rs`, `patch.diff`, scratch files, or `.orig`, `.rej` or `.patch` files.
- **Distinct screenshots.** Each scenario's screenshot hash must differ from every other's.
- **Green before commit.** All five must pass:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `sh scripts/build-site.sh`
  - `node --test "site/**/*.test.js"`
- **Delivery ledger.** The final commit body lists each Implementation section and named test as DELIVERED (with its test name) or NOT DELIVERED (with the reason). Never describe an item as "deferred by the user" unless the user said so.
- **Capabilities.** Each new capability goes in that crate's `capabilities.toml`, with `proves` entries naming tests that exist and pass.