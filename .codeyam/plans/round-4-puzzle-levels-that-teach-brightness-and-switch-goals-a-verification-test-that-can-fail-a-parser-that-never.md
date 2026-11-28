---
title: "Round 4: puzzle levels that teach — brightness and switch goals, a verification test that can fail, a parser that never panics"
mode: backend
createdAt: "2026-11-28T16:56:22Z"
source: manual
dependsOn: ["round-4-trustworthy-gates-bench-check-that-can-fail-honest-size-budgets-junk-and-duplicate-screenshot-tests"]
---

## Summary

The puzzle levels lost their teaching, and the level check is hollow.

**Brightness goals were dropped.** Every level file in `crates/simbiguity-circuits-puzzle/levels/` uses only `lit_bulbs`, `no_issue` or `part_budget`. The round-3 plan's brightness goals are gone:
- series bulbs at 25% ± 2% (level 3);
- parallel at ≥ 95% (level 4);
- the dimmer at 40–60% (level 7);
- ≥ 80% when not blown (level 8).

So level 7, "Dimmer", accepts any resistor.

**The verification test checks nothing.** `every_level_reference_solution_meets_all_its_goals` (`crates/simbiguity-circuits-puzzle/src/lib.rs:346`) ends with `let _ = sim.state().solution();` and the comment "We just verify it simulates without panicking".

**The parser can crash.** `parse_level` (`crates/simbiguity-circuits-puzzle/src/level_format.rs:143`) indexes `parts[1]` without checking it exists, so a line like `id` with no value crashes instead of returning `LevelError`.

**Level 2's switch goal isn't checked in both states.**

**`prototypeSimulate()`** is still in `site/demos/circuit-puzzles/game-state.js:55`. It's dead fake-message code.

## Key Decisions

**A `brightness` goal keyword, using power ratio.**
- Write it as `goal brightness <bulb-index> <min> <max>`, with min and max as fractions of rated power.
- It maps to `CircuitGoal::BulbBrightness` using P/P_rated, not `calculate_brightness`. That keeps the numbers hand-checkable.
- `<bulb-index>` is the bulb's order among the level's bulbs: fixed bulbs first, then placed ones.

**A `switch_states` goal for level 2.** Write it as `goal switch_states <switch-index> open:0 closed:1`. Checking it solves the board with the switch in each state and counts the lit bulbs.

**The verification test must test both directions.**
- Every level's reference solution passes all goals.
- The empty board fails at least one goal.
- At least one wrong solution per level fails. For example:
  - level 7 with the 27 Ω resistor gives 25% brightness, which fails;
  - level 3 with the bulbs in parallel fails;
  - level 5 with the short wire fails;
  - level 8 without a resistor blows the bulb.

## Implementation

### 1. Brightness and switch-state goals

**Files**:
- `crates/simbiguity-circuits-puzzle/src/level_format.rs`
- `src/lib.rs`

Bulbs are 9 V / 3 W, so R = 27 Ω. These numbers come from Ohm's law. If the engine bulb's resistance isn't V²/P, say so and recompute.

| Level | Goal | Values |
|---|---|---|
| 3 | `brightness 0 0.23 0.27` and `brightness 1 0.23 0.27` | series: each 4.5 V, so 0.25 |
| 4 | `brightness 0 0.95 1.05` and `brightness 1 0.95 1.05` | parallel: 1.00 each |
| 7 | `brightness 0 0.40 0.60` | 10 Ω gives (9·27/37)²/27/3 = 0.5325, which passes; 27 Ω gives 0.25 and 47 Ω gives 0.1331, which both fail |
| 8 | `brightness 0 0.80 1.05` and `no_issue bulb_blown` | 18 V with 27 Ω gives 9 V, so 1.00 |
| 2 | `switch_states 0 open:0 closed:1` | |

### 2. Update the level files

**Files**: `levels/level_2.txt`, `level_3.txt`, `level_4.txt`, `level_7.txt`, `level_8.txt`

Add the goals above. Fix any reference solution that doesn't actually meet them.

### 3. A real verification test

Rewrite `every_level_reference_solution_meets_all_its_goals` as described in Key Decisions. Add `each_level_rejects_its_named_wrong_solution`, a table of (level, wrong edits, expected failing goal).

### 4. A parser that never crashes

1. Replace every `parts[i]` with `parts.get(i).ok_or(LevelError::MissingValue { line })`.
2. Add a test feeding every keyword with no value, and each must return an error naming its line.
3. Add a fuzz-style test: 2,000 seeded random strings built from level keywords and numbers. `parse_level` never panics.

### 5. Remove `prototypeSimulate`

**File**: `site/demos/circuit-puzzles/game-state.js`

Delete the method and anything that calls it.

## Reused existing code

- `CircuitGoal::BulbBrightness`
- `build_circuit`
- `BoardEdits`
- `load_level`
- the seeded RNG in `simbiguity-core`

## Reproduction Test

`level_7_rejects_27_ohm_resistor`: placing 27 Ω instead of 10 Ω must fail level 7's goals. Today it passes, because the level only checks `lit_bulbs 1`.

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