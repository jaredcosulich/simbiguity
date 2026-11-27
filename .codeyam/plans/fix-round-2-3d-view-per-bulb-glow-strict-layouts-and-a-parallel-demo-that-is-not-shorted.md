---
title: "Fix round 2: 3D view per-bulb glow, strict layouts and a parallel demo that is not shorted"
mode: ui
createdAt: "2026-11-27T14:49:56Z"
source: manual
dependsOn: ["fix-round-2-make-the-branch-green-and-keep-it-that-way"]
---

## Summary

The first 3D view fix (c229af7) delivered layout-driven placement, wire routes and per-wire particle paths. Four things remain:
- **Every bulb still glows with the first bulb's brightness.**
- The `first_*` helpers still drive behaviour.
- The parallel demo's wiring shorts out both bulbs.
- The bridge reads a raw pointer with no checks, and none of its new exports are tested.

Several tests pass vacuously. Spec chain: `.codeyam/plans/completed/general-3d-circuit-view-render-any-circuit-laid-out-on-a-grid-board.md` and `.codeyam/plans/completed/fix-general-3d-circuit-view-is-still-stubbed-and-hard-coded-to-the-first-part.md`.

## Current behavior

- **Shared brightness.** `self.readouts().brightness` is computed once and assigned to every glow: in the input path (`crates/simbiguity-circuits-view/src/lib.rs:269`–`272`) and in `seek` (`crates/simbiguity-circuits-view/src/lib.rs:409`–`413`). On a blow, every flash fires together (`for f in &mut self.flashes`, around line 259).
- **"First part" helpers remain.** `first_battery`, `first_switch` and `first_bulb` (`crates/simbiguity-circuits-view/src/lib.rs:190`, `:203`, `:216`) still feed readouts and particle colour. `charge_look(current)` uses one battery current for every wire (around line 463).
- **Blanket allow.** `#![allow(missing_docs, dead_code, unused_imports, unused_variables)]` remains at `crates/simbiguity-circuits-view/src/lib.rs:27`. It hides the dead `layout::wires()`.
- **Misleading comment.** The leftover `// Initial setup omitted for brevity in minimal prototype` at `crates/simbiguity-circuits-view/src/lib.rs:156` sits right before the real setup call.
- **Bad ids accepted.** `toggle_part` (`crates/simbiguity-circuits-view/src/lib.rs:295`) sends `ToggleSwitch` to any id, wires and bulbs included.
- **No validation in the view.** `from_layout` (`crates/simbiguity-circuits-view/src/lib.rs:130`) never calls `BoardLayout::validate` and returns `CircuitView`, not `Result`.
- **`unpack` weaknesses** (`crates/simbiguity-circuits-view/src/board_layout.rs:88`):
  - The rotation is read from `pos + 5` for every record kind, wires included (line 124).
  - `u32 as i32` cell coordinates wrap silently (line 182).
  - Trailing words aren't rejected.
  - `validate` (line 57) checks only that a wire's endpoints land on part cells, not that the wire's nodes match those parts' terminals.
  - `LayoutError` (line 5) is undocumented and has no tests.
- **Unchecked raw pointer.** `c3d_new_from_layout` (`crates/simbiguity-render-wasm/src/lib.rs:68`) calls `core::slice::from_raw_parts(ptr, len)` (line 69) with no null or length check and no SAFETY comment. The module doc at line 25 and the `c3d_pick` doc around line 209 still describe the old `1`/`2`/`3` tags. None of `c3d_new_from_layout`, `c3d_toggle_part`, `c3d_part_count` or `c3d_part_kind` is tested.
- **Vacuous tests.**
  - `parts_point_at_the_animated_nodes` asserts `halo_mat.emissive > -1.0` (`crates/simbiguity-circuits-view/src/board.rs:292`), which is always true.
  - Tests still build self-loop wires `circuit.connect(plus, plus)` and similar (`crates/simbiguity-circuits-view/src/board.rs:258`–`260`, `:280`).
  - `parallel_bulbs_are_drawn_in_their_own_cells` (`crates/simbiguity-circuits-view/src/view_tests.rs:146`) uses a fixture with a wire to a dangling node and both bulbs on the same nodes.
  - Its helper `part_position` (line 197) returns `ZERO` for anything that isn't a bulb, and has a stray "wait, glasses is just in order of bulbs" comment (line 202).
- **No golden test.** Nothing checks the hello circuit against the original look.
- **The parallel demo shorts the battery.** In `site/demos/circuit-3d-parallel/demo.js`, both wire records are `kind=4, a=2, b=0` (around lines 47–53), so node 2 is tied to node 0 and both bulbs are bypassed. It has no series bulb, frees the buffer only on success, and has "Wait…" comments at lines 22 and 47.
- **Missing capability.** `crates/simbiguity-circuits-view/capabilities.toml` has no "draws any circuit on a board" capability.

## Key Decisions

- **Per-bulb state by `ComponentId`.**
  - Each glow targets its own bulb's solved brightness, and each flash fires only for the bulb that blew.
  - Particle colour and speed come from each wire's own current.
  - Remove `first_*` entirely; the hello-circuit readouts look parts up by id.
- **Validate everywhere.** `from_layout` returns `Result<CircuitView, LayoutError>` and always validates. `unpack` is strict: per-kind record lengths, range-checked coordinates, no trailing words. `validate` checks that wire nodes match terminals. `toggle_part` on a non-switch returns `false`.
- **Bridge safety.** Reject a null pointer or a `len` over a documented maximum before building the slice, and add a SAFETY comment. Native tests cover each export, including malformed buffers.
- **Golden from history.** Capture a command-buffer golden for `CircuitView::new(9.0, true)` at a fixed seek from commit `16cb5cd` (before generalisation). If the geometry change was intentional (straight vs diagonal wires), document the difference and record a new golden explicitly. Don't skip it.
- **Correct demo wiring:** a battery feeding two parallel bulbs, with a switch on one branch and one bulb in series, built from a validated layout.

## Implementation

### 1. Per-component animation

**File**: `crates/simbiguity-circuits-view/src/lib.rs`

### 2. Strict layouts

**File**: `crates/simbiguity-circuits-view/src/board_layout.rs`

### 3. Tests

- **File**: `crates/simbiguity-circuits-view/src/board.rs`: real emissive assertion, no self-loops
- **File**: `crates/simbiguity-circuits-view/src/view_tests.rs`: a valid parallel fixture; per-bulb glow; an open branch stops only its own particles; a reversed battery reverses direction; golden
- **File**: `crates/simbiguity-render-wasm/src/tests.rs`: one test per new export

### 4. Bridge, demo, capability

- **File**: `crates/simbiguity-render-wasm/src/lib.rs`
- **File**: `site/demos/circuit-3d-parallel/demo.js`
- **File**: `crates/simbiguity-circuits-view/capabilities.toml`

## Reused existing code

- `BoardLayout::validate` / `BoardLayout::unpack` in `crates/simbiguity-circuits-view/src/board_layout.rs`
- Wire currents from `crates/simbiguity-circuits/src/wires.rs`
- `bulb_look` / `charge_look` in `crates/simbiguity-circuits-view/src/look.rs`

## Reproduction Test

Two bulbs with different brightness glow identically, because every glow copies the first bulb's brightness.

**Target**: `crates/simbiguity-circuits-view/src/view_tests.rs`. Run with `codeyam-editor editor refresh-tests --test each_bulb_glows_at_its_own_brightness`.

```rust
/// Put a bulb on its own across the battery, and two more bulbs in series on
/// a parallel branch. The lone bulb gets the full voltage and the series pair
/// get half each, so they're much dimmer. Their glows must differ, or the 3D
/// view is lying about the circuit.
#[test]
fn each_bulb_glows_at_its_own_brightness() {
    let (view, lone, series_a) = one_bright_two_dim_fixture(); // validated layout; built at execution
    let view = settle(view); // seek far enough for the springs to reach target
    assert!(view.glow_of(lone) > 2.0 * view.glow_of(series_a));
}
```

Status: PROPOSED — confirm red at execution. Expected failure: both glows are equal (the first bulb's brightness). The helper names (`one_bright_two_dim_fixture`, `settle`, `glow_of`) are test-side and are written at execution.

## Scenarios to Demonstrate

- Parallel demo: two bright parallel bulbs, one dimmer series bulb, current splitting
- One branch switched off: its bulb dark and its particles still; the other branch unchanged
- Malformed layout buffer: the demo shows the bridge's error text

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