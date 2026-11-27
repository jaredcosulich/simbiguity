---
title: "Fix: circuit puzzle game is a placeholder (real levels, goals, undo, share)"
mode: ui
createdAt: "2026-11-26T22:14:09Z"
source: manual
dependsOn: ["fix-general-3d-circuit-view-is-still-stubbed-and-hard-coded-to-the-first-part"]
---

## Summary

The 3D circuit puzzle game is a placeholder. The puzzle crates define their own empty traits, a level is just an id and a hint, the share encoder returns its input, there are no levels or reference solutions, and the page fakes a solved state in JavaScript. This fix builds the game that `.codeyam/plans/completed/3d-circuit-puzzle-game-data-driven-levels-with-exact-verified-goals.md` describes (still the spec) on top of the fixed general 3D view.

## Current behavior

- **Its own empty traits.** `crates/simbiguity-puzzle/src/lib.rs` defines its own `Simulation { type State; type Input; }` and `Snapshot { type Data; }`, with no methods, instead of using `simbiguity_core::{Simulation, Snapshot}`. It isn't `#![no_std]` and has no undo stack.
- **Levels are empty.** `Level` in `crates/simbiguity-circuits-puzzle/src/level_format.rs` is only `{ id, hint }`. There is no board, tray, goals or explanation. There are no level files, parser, validation or reference solutions.
- **No goal predicates.** There is no `BulbBrightness`, `NoIssue`, `PartBudget`, `CurrentThrough`, or `All` / `Any` / `Not`.
- **Share does nothing.** `encode_solution` in `crates/simbiguity-circuits-puzzle/src/share.rs` returns `sol.to_string()`. Its only test asserts `encode_solution("A") == "A"`.
- **The game is faked in JS.** `site/demos/circuit-puzzles/game.js` contains `// Mock solved state for level 1` and adds the `solved` class to level 1 in JavaScript. That is logic in JS and a fake result.
- **No lints or docs.** Neither puzzle crate has the clippy deny block or doc comments. The circuits-puzzle crate has 1 test.

## Key Decisions

- **Build on the real contracts.** Delete the local trait copies. `simbiguity-puzzle` is generic over `simbiguity_core::Simulation + Snapshot`, and undo/redo is a stack of snapshots, so it is exact by construction.
- **Levels are data with verified reference solutions.** Levels live in a hand-written, zero-dependency format with a parser and validator. Each shipped level includes its reference solution. A test proves every level parses, its solution passes, and a deliberately broken solution fails, which shows the check isn't vacuous.
- **Checking happens only in Rust.** The page asks the bridge `check(level, attempt)` and renders the result. Remove the JS fake.
- **Share is a real compact encoding**, with round-trip, corruption-rejection and size-bound tests.
- **Uses the fixed general 3D view** for the board, placement and picking by `ComponentId`.
- Finalize must run.

## Implementation

### 1. Generic puzzle engine

**File**: `crates/simbiguity-puzzle/src/lib.rs`: `Level`, `Goal`, `Attempt`, `check`, and `UndoStack<S: Snapshot>` over `simbiguity-core`; `#![no_std]`; lints; docs.

### 2. Circuit puzzles

- **File**: `crates/simbiguity-circuits-puzzle/src/lib.rs`: board and snap-point model mapped onto `Circuit` and the view's `BoardLayout`; goal predicates with composition
- **File**: `crates/simbiguity-circuits-puzzle/src/level_format.rs`: full level schema, parser and validation (including "the tray can't meet the budget")
- **File**: `crates/simbiguity-circuits-puzzle/src/share.rs`: compact encoding plus decoding with an error type

### 3. Level pack and bridge

- **New file**: `site/demos/circuit-puzzles/levels/` with 8–10 levels plus reference solutions, per the original plan
- Add bridge exports for load, place, remove, toggle, undo, redo, check and share. They go in the render bridge (`crates/simbiguity-render-wasm/src/lib.rs`) or a new `crates/simbiguity-circuits-puzzle-wasm` (new); decide at execution.

### 4. Page

**File**: `site/demos/circuit-puzzles/game.js`: remove the mock; drive everything through the bridge; keep progress in `localStorage`.

## Reused existing code

- `Simulation` / `Snapshot` in `crates/simbiguity-core/src/simulation.rs`
- `CircuitSim`, `CircuitIssue` in `crates/simbiguity-circuits/src/sim.rs` and `crates/simbiguity-circuits/src/diagnostics.rs`
- `BoardLayout` in `crates/simbiguity-circuits-view/src/board_layout.rs` (after the view fix)
- Level select in `site/demos/circuit-puzzles/components/level-select.js`

## Reproduction Test

Sharing a solution doesn't encode anything. It also accepts any corrupted string as a "solution".

**Target**: `crates/simbiguity-circuits-puzzle/src/share.rs`. Run with `codeyam-editor editor refresh-tests --test corrupted_share_link_is_rejected`.

```rust
/// A share link must reproduce the exact board, and a mangled link must be
/// refused with an error rather than loading a board nobody built.
#[test]
fn corrupted_share_link_is_rejected() {
    let encoded = encode_solution(&sample_attempt());
    let mut broken = encoded.clone();
    broken.push('!');
    assert_eq!(decode_solution(&encoded), Ok(sample_attempt()));
    assert!(decode_solution(&broken).is_err());
}
```

Status: PROPOSED — confirm red at execution. Expected failure: this won't compile today, because there is no `decode_solution`, `encode_solution` takes `&str`, and there is no `sample_attempt` fixture. A compile failure is the red; the types are settled at execution.

## Scenarios to Demonstrate

- Level select with progress (some solved), all driven by Rust's `check`
- Mid-build with live current flow on the general board
- Short-circuit feedback with the plain-language explanation
- Solved state with explanation card and a share link that reloads the exact board