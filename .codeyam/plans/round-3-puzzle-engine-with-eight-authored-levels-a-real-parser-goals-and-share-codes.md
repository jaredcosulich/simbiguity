---
title: "Round 3: puzzle engine with eight authored levels, a real parser, goals and share codes"
mode: backend
createdAt: "2026-11-28T02:43:18Z"
source: manual
dependsOn: ["round-3-site-build-under-budget-no-junk-and-a-determinism-test-that-can-fail"]
---

## Summary

The circuit puzzle engine is still a shell after two rounds, even though its plan was titled "real goals, verified levels, undo and share":

- All eight files in `crates/simbiguity-circuits-puzzle/levels/level_1.txt` … `level_8.txt` are **0 bytes**.
- `parse_level` (`src/level_format.rs:31`) ignores its input and returns the same hard-coded level for any non-empty string.
- `load_level` (`:21`) carries the comment "A real parser would go here".
- `share.rs` says "Stub implementation". It copies bytes into a `String` and "detects corruption" only by looking for `'!'`.
- `CircuitGoal::LitBulbs(_) => false` (`crates/simbiguity-circuits-puzzle/src/lib.rs:127`) went from always-true to always-false.

This plan builds the engine for real: a level text format, eight authored levels with real content, a parser, goal checks, undo/redo, and tamper-evident share codes.

## Key Decisions

**Level format: line-based text, embedded with `include_str!`.**
- It is `no_std`-friendly and easy to read in review.
- Lines starting with `#` are comments. Each record sits on its own line:

```
id 1
title First light
hint A bulb only lights when current can flow out of the battery, through the bulb, and back.
grid 6 4
fixed battery 0 1 rot 0 emf 9
fixed bulb 4 1 rot 0 volts 9 watts 3
palette wire 6
goal lit_bulbs 1
goal no_issue short_circuit
solution wire 1 1 2 1
solution wire 2 1 3 1
...
```

- Any unknown keyword, wrong arity, unparsable number, or out-of-grid cell is a `LevelError` naming the line number.

**Every level ships a reference solution, and a test proves it.** For each of the eight levels, the test:
1. parses the level;
2. places the reference solution on the board;
3. solves it with `simbiguity-circuits`;
4. asserts **every** goal passes;
5. asserts the *empty* board fails at least one goal.

This is what "verified levels" means.

**"Lit" is defined once.** A bulb counts as lit when its power is at least 5% of its rated power. `LitBulbs(n)` is true when at least n bulbs are lit.

**Share codes: version byte, level id, placed parts, then a CRC-32, base64url encoded.**
- Use CRC-32 IEEE (polynomial 0xEDB88320). Its check value: `crc32(b"123456789") == 0xCBF43926`.
- Decode returns a typed error for each failure: bad character, too short, unknown version, CRC mismatch, or level mismatch.

## Implementation

### 1. Level format, parser, and the eight levels

**Files**:
- `crates/simbiguity-circuits-puzzle/src/level_format.rs`
- `levels/level_1.txt` … `level_8.txt`

The bulbs are 9 V / 3 W, so R = V²/P = 27 Ω, and batteries are ideal 9 V unless stated. Confirm that the engine's `Bulb` has a fixed resistance of rated V²/P. If it does not, recompute the numbers below from the engine's model, and say so in the commit.

| Level | Concept | Goals |
|---|---|---|
| 1 First light | close a loop | `lit_bulbs 1`, `no_issue short_circuit` |
| 2 Switch it | a switch controls the loop | `lit_bulbs 1` with the fixed switch closed; with it open, 0 lit (check both states) |
| 3 Two in a row | series | two bulbs lit; each at 25% ± 2% of rated power (each gets 4.5 V) |
| 4 Side by side | parallel | two bulbs lit; each at ≥ 95% of rated power |
| 5 Don't short it | a tempting wire shorts the battery | `lit_bulbs 1`, `no_issue short_circuit`; the palette includes one extra wire |
| 6 Budget | economy | three bulbs lit using ≤ 4 wires (`part_budget wire 4`) |
| 7 Dimmer | a series resistor | bulb at 40–60% of rated power; palette offers 10 Ω, 27 Ω and 47 Ω resistors |
| 8 Don't blow it | 18 V battery, 9 V bulb | `no_issue bulb_blown`, bulb ≥ 80% of rated power; palette offers a 27 Ω resistor and a second bulb |

How the level 7 numbers work:
- With 10 Ω in series, the bulb gets 9·27/37 = 6.568 V, which is 1.598 W, or 53% of rated. That passes.
- 27 Ω gives 25%, and 47 Ω gives about 13%. Both fail.

### 2. `load_level` uses the embedded files

Add `load_level(id) -> Result<Level, LevelError>`. An unknown id is an error, not a default level.

### 3. Real goal checks

**File**: `crates/simbiguity-circuits-puzzle/src/lib.rs`

- `LitBulbs` follows the 5% rule.
- Tests use a single 9 V battery and bulb:
  - The open circuit gives `LitBulbs(1)` = false.
  - Closed, `LitBulbs(1)` = true and `LitBulbs(2)` = false.
- Check every other `CircuitGoal` variant against a hand-built circuit where it passes and one where it fails.

### 4. Undo and redo over board edits

Use `UndoStack` from `crates/simbiguity-puzzle/src/lib.rs:36`, and expose `place`, `remove`, `undo`, `redo` and `reset` on a `PuzzleSession`.

Test:
1. Place three wires, then undo twice. One wire remains.
2. Redo. Two remain.
3. Place a new wire. Redo is now empty.
4. Reset. The board is back to the level's fixed parts.

### 5. Share codes

**File**: `crates/simbiguity-circuits-puzzle/src/share.rs`

Implement `encode_solution(&PuzzleSession) -> String` and `decode_solution(&str) -> Result<Attempt, ShareError>`. Tests:

- The CRC check value above is matched.
- Round trip across 500 seeded random attempts (`simbiguity_core` RNG).
- Each of the five `ShareError` cases has its own test:
  - flip one character;
  - drop the last 4 characters;
  - set the version to 99;
  - insert a `'*'`;
  - decode a level-3 code while expecting level 5.

## Reused existing code

- `UndoStack` in `crates/simbiguity-puzzle/src/lib.rs`
- `IssueKind`, `PartKind` and `calculate_brightness` in `crates/simbiguity-circuits-puzzle/src/lib.rs`
- `BoardLayout` in `crates/simbiguity-circuits-view/src/board_layout.rs`
- the seeded RNG in `simbiguity-core`

## Reproduction Test

`every_level_reference_solution_meets_all_its_goals` must fail on the current tree, because the level files are empty and `parse_level` ignores its input.

## Acceptance rules (the whole build must meet these)

Three earlier builds of this work passed their own checks while shipping stubs. Each rule below comes from a real failure:

- **Red first.** Write the tests named in this plan before the code. Run them on the starting tree and see them fail. Put the failure message in the commit body.
- **No hollow assertions.** All of these are forbidden:
  - asserting a constant (`assert_eq!(1, 1)`, `assert.strictEqual(typeof x, "string")` standing in for the real check)
  - asserting something that is always true (`x > -1.0`)
  - empty `if let` or loop bodies that check nothing
  - a goal or check that returns a hard-coded `true` or `false`
  - loosening a tolerance until the test passes
- **No stubs that only change shape.** Replacing `return true` with `return false`, or one constant with another, is not an implementation. So is a parser that ignores its input, or an encoder that copies its input. Placeholder comments are forbidden too: "stub", "would go here", "a real X would…", "once the TDD phase starts", "for prototype", "wait…", "omitted for brevity", "deferred".
- **Numbers come from this plan, not from the code.** The reference values here were worked out by hand or with an independent script. If the code disagrees with one, the code is wrong. Do not take the expected value from the code's own output.
- **No empty files, no fake screenshots.**
  - Every file this plan creates must have real content.
  - Each scenario must show a different state. Its screenshot must differ from every other scenario's (no identical hashes).
- **Budgets are not raised to fit.** Don't edit `site/pkg-budget.txt` or any bench budget upward to make a check pass. Make the code smaller or faster instead. If a budget truly must move, say why in the commit body and cap the increase at 5%.
- **Touch only what's listed.** Before committing, `git diff --stat <start>..HEAD` must show no changes to:
  - `rust-toolchain.toml`
  - `.gitignore`
  - crates or demos this plan doesn't name

  If anything like that shows up, restore it from the starting commit.
- **Everything green, and finalize runs.** No `Codeyam-Finalize: deferred`. All five must pass:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `sh scripts/build-site.sh` (it must exit 0)
  - `node --test "site/**/*.test.js"`
- **No junk.** Don't commit `patch.diff`, scratch files, or `.patch`, `.orig` or `.rej` files.
- **Every `proves` entry exists.** In each touched `capabilities.toml`, every entry must name a test that exists and passes. Every new capability must be listed there, so it appears on the status page.
- **Done means demonstrable.** The last commit's body must list each Implementation section and the test(s) that prove it, one line per section. If a section is not done, say so plainly. Do not mark the plan complete.