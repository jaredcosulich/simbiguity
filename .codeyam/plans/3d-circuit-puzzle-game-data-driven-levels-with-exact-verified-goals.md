---
title: "3D circuit puzzle game: data-driven levels with exact, verified goals"
mode: ui
createdAt: "2026-11-26T12:42:15Z"
source: manual
dependsOn: ["3d-rendering-and-animation-simbiguity-anim-simbiguity-render-a-webgl2-executor-and-the-3d-circuit-view", "engine-status-page-public-demos-of-what-simbiguity-can-do-and-every-test-that-proves-it"]
---

## Summary

Turn the 3D circuit view into the flagship educational experience: a **3D circuit puzzle game**. Each level poses a goal ("light both bulbs fully", "light A but not B", "make the bulb exactly half as bright", "build it with no more than 2 resistors"), gives a parts tray and a board of snap points, and checks the solution exactly using the deterministic circuit engine. Levels teach one idea at a time (a complete circuit, series vs parallel, voltage dividers, shorts, the Wheatstone bridge) with hints, an explanation after solving, undo, and a shareable solution link. The puzzle engine is its own importable crate, so another project could build different puzzle content on it.

## Key Decisions

- **Levels are data.** A level is a JSON/TOML file: board size, fixed components, parts tray with counts, goal predicates, hint text, and a post-solve explanation. Adding a level needs no code, so educators can contribute levels.
- **Goals are predicates over the `Solution`.** `BulbBrightness { bulb, min, max }`, `NoIssue(ShortCircuit)`, `PartBudget { kind, max }`, `CurrentThrough { component, approx }`, composable with `All` / `Any` / `Not`. Checking is pure and deterministic, so a solution either passes or it does not, and a test can prove every shipped level is solvable.
- **Every shipped level ships with a reference solution, verified in CI.** This prevents an unsolvable level from ever reaching learners.
- **Undo/redo and sharing come from the core contracts.** Edits are inputs, the state is snapshot-able, and a solution serializes to a compact URL hash. Sharing a link reproduces the exact board.
- **Feedback is animated, not just a checkmark.** Live current flow while building, a wire flashing red on a short with the diagnostic in plain words, a celebratory animation on success. All of it is driven by `simbiguity-anim` from solver state.
- **Placement is snap-to-grid** on the board, with picking via `simbiguity-render`. No free-form wiring in v1: it keeps puzzles about circuit ideas, not dexterity.

## Implementation

### 1. Puzzle engine crate

**New file**: `crates/simbiguity-puzzle/src/lib.rs`: generic `Level`, `Goal`, `Attempt`, `check`, and undo stack over any `Simulation + Snapshot`
**New file**: `crates/simbiguity-circuits-puzzle/src/lib.rs`: circuit goals and board/snap-point model mapped onto `Circuit`
**New file**: `crates/simbiguity-circuits-puzzle/src/level_format.rs`: parse/validate level files (hand-written, zero deps)
**New file**: `crates/simbiguity-circuits-puzzle/src/share.rs`: compact solution encoding for URL hashes

### 2. Level pack

**New file**: `site/demos/circuit-puzzles/levels/` with 8–10 levels: complete the circuit, the switch, series dims, parallel stays bright, don't short it, voltage divider, exact brightness, Wheatstone balance. Each has a reference solution.

### 3. Game page

**New file**: `site/demos/circuit-puzzles/index.html`: level select, 3D board, parts tray, goal panel, hint, undo/redo, reset, share; explanation card on solve; progress kept in `localStorage`
**New file**: `site/demos/circuit-puzzles/game.js`

### 4. Status page

**File**: `site/status/index.html`: puzzle capabilities ("every level verified solvable") with the game embedded

## Reused existing code

- `simbiguity-circuits-view`, `simbiguity-render`, `simbiguity-anim`, the WebGL2 executor (plan: 3D renderer + animation)
- `Simulation` / `Snapshot` (plan: foundations); `CircuitSim` / `CircuitIssue` (plan: circuits DC)
- The status-page capability manifest (plan: engine status page)

## Tests (TDD — written first)

- Goal predicates: each one against hand-built solutions (pass, fail, boundary values); `All` / `Any` / `Not` composition.
- Every shipped level parses, and its reference solution passes `check`; a deliberately broken solution fails (proves the check is not vacuous).
- A level whose tray cannot satisfy its budget is rejected by validation.
- Undo/redo across place/remove/toggle restores identical states.
- Share encoding round-trips, rejects corrupted hashes with an error, and stays short (size bound).
- codeyam scenarios per level state: unsolved, short-circuit feedback, solved with explanation card (seeded via a URL hash solution).

## Scenarios to Demonstrate

- Level select with progress (some solved)
- Mid-build with live current flow
- Short-circuit feedback with the plain-language explanation
- Solved state with explanation card and share link
- Exact-brightness level showing the readout against the target band