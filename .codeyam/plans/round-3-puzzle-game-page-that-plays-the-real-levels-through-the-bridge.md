---
title: "Round 3: puzzle game page that plays the real levels through the bridge"
mode: ui
createdAt: "2026-11-28T02:43:18Z"
source: manual
dependsOn: ["round-3-puzzle-engine-with-eight-authored-levels-a-real-parser-goals-and-share-codes"]
---

## Summary

The puzzle game page fakes most of what it shows:

- `site/demos/circuit-puzzles/game-state.js`:
  - `undo()`, `redo()` and `reset()` only `console.log`.
  - `share()` writes a hard-coded `#s=123` link into `innerHTML`.
  - `prototypeSimulate()` paints made-up "Short Circuit!" and "link is damaged" messages.
- `game.js:52-58` triggers those fake states from `localStorage.getItem('prototype')` so the scenario screenshots have something to show.
- Two of the five committed screenshots (`level-select-with-progress` and `mid-build-with-live-current-flow`) are byte-identical.

This plan wires the page to the real puzzle engine from the previous plan. Every message and number on screen comes from the bridge.

## Key Decisions

- **The page holds no game logic.** This is the same rule as `site/demos/hello-circuit/demo.js`. Placing, removing, undo/redo, goal checks, issue messages and share codes all go through new `puzzle_*` exports in `crates/simbiguity-render-wasm/src/lib.rs`.
- **Scenarios reach their states by real input.** Each scenario arrives at its state through URL state: a `#level=…&s=<share code>` hash that the page decodes through the bridge. Delete the `localStorage` "prototype" hook entirely.
- **Progress is persisted.** Store solved level ids in `localStorage['simbiguity.puzzles.solved']`. The level select shows a check mark on each solved level and "3 of 8 solved".

## Implementation

### 1. Bridge exports

**File**: `crates/simbiguity-render-wasm/src/lib.rs`

Add these exports, each with a Rust test:
- `puzzle_place(h, kind, x, z, rot)`, `puzzle_remove(h, x, z)`
- `puzzle_undo`, `puzzle_redo`, `puzzle_reset`
- `puzzle_goal_count`, `puzzle_goal_status(h, i)`, `puzzle_goal_text(h, i)`
- `puzzle_issue_text`
- `puzzle_share` / `puzzle_load_share` (UTF-8 out through `sb_last_*`-style ptr/len)
- `puzzle_level_count`, `puzzle_level_title`

Watch the WASM budget (230,000 bytes). Keep the text in the engine crate, and avoid `format!` in the bridge.

### 2. Game page

**Files**:
- `site/demos/circuit-puzzles/game.js`
- `game-state.js`
- `components/*.js`

1. Clicking an empty cell with a palette part selected places it. Clicking a placed part removes it. Picking comes from `Circuit3D` ray picking.
2. Show goal checklist rows from `puzzle_goal_text`, ticked live from `puzzle_goal_status` after every edit.
3. Show issue text from `puzzle_issue_text`.
4. When every goal passes:
   - show the explanation panel with the share link from `puzzle_share`;
   - mark the level solved.
5. A bad `#s=` code shows the engine's `ShareError` message.
6. Remove all `innerHTML` writes of fixed strings. Use `textContent`.

### 3. Node tests for the page logic

**File**: `site/demos/circuit-puzzles/game-state.test.js` (new)

Load the real WASM in Node, the way `site/js/determinism.test.js` will after the first round-3 plan. Then:
1. Load level 1, place its reference solution through `GameState`, and assert every goal is ticked and a share code is produced.
2. Feed that code back and assert the same board results.
3. Flip one character and assert the error text is non-empty and the board is unchanged.
4. Undo/redo through `GameState` changes the part count as expected.

### 4. Scenarios with distinct screenshots

Recapture five scenarios through URL state only:
- level select with 3 solved;
- level 4 mid-build with one bulb lit;
- level 5 with the short-circuit wire placed;
- level 1 solved, showing the share link;
- a corrupted share link.

All five screenshot hashes must differ.

## Reused existing code

- `PuzzleSession`, `load_level` and `share` from `simbiguity-circuits-puzzle` (previous plan)
- `Circuit3D` picking in `site/js/simbiguity-render.js`
- `WebGL2Executor`
- the `hashFromState` / `stateFromHash` pattern from `site/demos/hello-circuit/state.js`

## Reproduction Test

`game_state_solving_level_1_produces_a_share_code`: in Node, placing level 1's reference solution must tick every goal and return a non-empty share code that `puzzle_load_share` accepts. It fails today because `share()` only writes `#s=123`.

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