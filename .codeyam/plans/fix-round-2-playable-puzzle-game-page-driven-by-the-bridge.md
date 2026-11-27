---
title: "Fix round 2: playable puzzle game page driven by the bridge"
mode: ui
createdAt: "2026-11-27T14:49:57Z"
source: manual
dependsOn: ["fix-round-2-puzzle-engine-with-real-goals-verified-levels-undo-and-share"]
---

## Summary

The puzzle page doesn't play anything. `game.js` logs the selected level and contains a "for prototype" comment where the Rust check should be. This plan builds the playable game on the round-2 puzzle engine and the fixed 3D view. It adds bridge exports for the puzzle, a page where every decision comes from Rust, progress saving, sharing and the explanation card. Spec: `.codeyam/plans/completed/3d-circuit-puzzle-game-data-driven-levels-with-exact-verified-goals.md`, Implementation §3–4 and Scenarios.

## Current behavior

- **Nothing calls Rust.** `site/demos/circuit-puzzles/game.js:6` reads `// Rust checking happens here. For prototype, we will just advance to the next step.`, and the handler only `console.log`s the level id.
- **No puzzle exports.** None of the bridges exposes a puzzle export: no load, place, remove, undo, redo, check or share.
- **No status-page entry.** The status page has no puzzle capability embed.

## Key Decisions

- **The bridge owns all rules.** Add exports for:
  - listing levels
  - loading a level
  - placing, removing and toggling a part at a cell
  - undo and redo
  - checking (goal-by-goal pass/fail plus issue text)
  - encoding and decoding a share string

  JS only renders and forwards clicks. Place the exports in `crates/simbiguity-render-wasm/src/lib.rs`, next to the view they drive, or in a new `crates/simbiguity-circuits-puzzle-wasm` (new). Decide at execution and justify the choice in the commit.
- **The board is the 3D view.** Load the level's fixed parts and the learner's placements through `BoardLayout` into `CircuitView::from_layout`, so current flow and glow are live while building.
- **Progress is saved, sharing is real.** Progress goes in `localStorage` keyed by level id. A `#s=<share>` URL hash reloads the exact board, using the engine's checksummed decode, and an invalid hash shows a message.
- **Native tests for every export**, including bad level ids, out-of-range cells and corrupted share strings. There is also a `game-state.test.js` for the JS state module.

## Implementation

### 1. Bridge

`puzzle_*` exports in the crate chosen above, plus tests.

### 2. Page

- **File**: `site/demos/circuit-puzzles/game.js`: replace the placeholder with the bridge-driven loop
- **File**: `site/demos/circuit-puzzles/index.html`: tray, goal panel, hint, undo/redo/reset/share, explanation card
- **New file**: `site/demos/circuit-puzzles/game-state.js` (+ `game-state.test.js`)

### 3. Status page

**File**: `site/status/index.html`: embed the game under the puzzle capability.

## Reused existing code

- `bindLevelSelect` in `site/demos/circuit-puzzles/components/level-select.js`
- `CircuitView::from_layout` in `crates/simbiguity-circuits-view/src/lib.rs`
- Slot/status pattern in `crates/simbiguity-render-wasm/src/slot.rs`
- The puzzle engine from the round-2 puzzle-engine plan

## Reproduction Test

No unit repro of a page that plays nothing. Demonstrate through the scenarios below. The first native test written must be "checking level 1 with an empty board reports its goal as failed", observed red before the export exists.

## Scenarios to Demonstrate

- Level select with progress (some solved)
- Mid-build with live current flow
- Short-circuit feedback with the plain-language explanation
- Solved state with explanation card and a share link that reloads the exact board
- Opening a corrupted share link: a friendly "this link is damaged" message

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