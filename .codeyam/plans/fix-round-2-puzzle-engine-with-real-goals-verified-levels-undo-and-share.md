---
title: "Fix round 2: puzzle engine with real goals, verified levels, undo and share"
mode: backend
createdAt: "2026-11-27T14:49:57Z"
source: manual
dependsOn: ["fix-round-2-3d-view-per-bulb-glow-strict-layouts-and-a-parallel-demo-that-is-not-shorted"]
---

## Summary

The puzzle crates are still placeholders after two builds. The only goal always returns `true`, `load_level` returns a hard-coded level for any id, sharing is the identity function, and no levels or reference solutions exist. This plan builds the **Rust puzzle engine only**: goal predicates, the level format, a verified level pack, undo, and share encoding, all with hand-checkable tests. The game page is a separate follow-up plan that builds on it. Spec: `.codeyam/plans/completed/3d-circuit-puzzle-game-data-driven-levels-with-exact-verified-goals.md`, Key Decisions and Implementation §1–2.

## Current behavior

- **The goal is always true.** `CircuitGoal` (`crates/simbiguity-circuits-puzzle/src/lib.rs:20`) has the single variant `LitBulbs(usize)`, and its `check` (line 25) ignores its state and returns `true` (line 27). The test `goal_lit_bulbs` (line 39) therefore passes vacuously.
- **Any id loads a level.** `load_level` (`crates/simbiguity-circuits-puzzle/src/level_format.rs:8`) returns `Some(Level { .. })` (line 9) for any id. There is no schema, parser or validation.
- **Sharing is the identity.** `encode_solution` and `decode_solution` (`crates/simbiguity-circuits-puzzle/src/share.rs`) return their input, and "corruption" means "contains `!`".
- **No levels.** No level files or reference solutions exist.
- **Undo is minimal and untested.** `UndoStack` (`crates/simbiguity-puzzle/src/lib.rs:30`) has `push` (line 43) and `undo` (line 48), with no redo, docs or tests. `Level<G>` (line 20) has `id`, `hint` and `goals`, but no board, tray or explanation.

## Key Decisions

- **Goals are real predicates over the solved `Analysis`.**
  - `BulbBrightness { bulb, min, max }`
  - `NoIssue(kind)`
  - `PartBudget { kind, max }`
  - `CurrentThrough { component, amps, tolerance }`
  - `All`, `Any` and `Not` to combine them
- **Every goal gets a pass, a fail and a boundary test** against hand-built circuits. For example, a 6 V battery across a 6 V / 3 W bulb gives brightness 1.0, and two such bulbs in series give 0.25 brightness each (a quarter of the power).
- **The level format is hand-written and dependency-free.** Use a small line-based or TOML-subset format, with a parser returning a named `LevelError`. A level holds:
  - board size
  - fixed parts
  - the parts tray with counts
  - goals
  - hint and explanation text
  - **a reference solution**
- **Every shipped level is verified by a test.**
  - It parses.
  - Its reference solution satisfies its goals.
  - A deliberately broken variant (for example, the reference with one part removed) fails, which shows the check is meaningful.
  - Validation rejects a tray that can't meet its `PartBudget`.
- **Levels:** 8, in the order of the original plan:
  1. complete the circuit
  2. the switch
  3. series dims
  4. parallel stays bright
  5. don't short it
  6. voltage divider
  7. exact brightness
  8. Wheatstone balance
- **Undo and redo are snapshot stacks.** A test runs place → place → undo → redo → undo and checks the states are bit-identical to direct construction.
- **Share is a compact, checksummed encoding.** Use base64url over a packed attempt plus a CRC-8 or similar checksum, all hand-written.
  - A round trip is exact.
  - Any single-character change is rejected.
  - Length is bounded: the full Wheatstone solution is at most 64 characters.

## Implementation

### 1. Generic engine

**File**: `crates/simbiguity-puzzle/src/lib.rs`: `Level`, `Goal`, `Attempt`, `check`, `UndoStack` with redo, plus docs.

### 2. Circuit goals and levels

- **File**: `crates/simbiguity-circuits-puzzle/src/lib.rs`: goals and combinators; board and snap-point mapping onto `Circuit` and `simbiguity_circuits_view::board_layout::BoardLayout`
- **File**: `crates/simbiguity-circuits-puzzle/src/level_format.rs`: schema, parser, `LevelError`, validation
- **New file**: `crates/simbiguity-circuits-puzzle/levels/` with the 8 level files, embedded with `include_str!`

### 3. Share

**File**: `crates/simbiguity-circuits-puzzle/src/share.rs`

### 4. Capability

**New file**: `crates/simbiguity-circuits-puzzle/capabilities.toml`: "every level is verified solvable", proving the level-pack test.

## Reused existing code

- `Simulation` and `Snapshot` in `crates/simbiguity-core/src/simulation.rs`
- `CircuitSim`, `Analysis`, `CircuitIssue` in `crates/simbiguity-circuits/src/sim.rs` and `crates/simbiguity-circuits/src/diagnostics.rs`
- `BoardLayout` in `crates/simbiguity-circuits-view/src/board_layout.rs`

## Reproduction Test

The only goal always passes, even for a circuit with no bulb lit.

**Target**: `crates/simbiguity-circuits-puzzle/src/lib.rs` (its `tests` module). Run with `codeyam-editor editor refresh-tests --test lit_bulb_goal_fails_when_the_bulb_is_dark`.

```rust
/// A goal must be able to say "no". A bulb across an open switch carries no
/// current and stays dark, so "light this bulb" has to fail. A goal that
/// always passes would let every learner "solve" every level.
#[test]
fn lit_bulb_goal_fails_when_the_bulb_is_dark() {
    let mut c = Circuit::new();
    let (p, mid, m) = (c.node(), c.node(), c.node());
    c.add_battery(p, m, 6.0);
    c.add_switch(p, mid, false);
    let bulb = c.add_bulb(mid, m, 6.0, 3.0);
    let analysis = c.solve();
    let goal = CircuitGoal::BulbBrightness { bulb, min: 0.5, max: 1.0 };
    assert!(!goal.check(&analysis));
}
```

Status: PROPOSED — confirm red at execution. Expected failure: today's only variant is `LitBulbs`, whose `check` returns `true`. Expect a compile error first (the variant doesn't exist yet), then red against a `true`-returning stub before the real implementation.

## Scenarios to Demonstrate

- Level-pack test output: 8 levels, each "reference solves; broken variant fails"
- A share string for the Wheatstone solution, and a corrupted one rejected

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