---
title: "Fix: time-series plots actually draw (Chart::draw stub, line geometry, tick loop)"
mode: ui
createdAt: "2026-11-26T22:14:08Z"
source: manual
dependsOn: ["fix-get-fast-sim-1-green-and-clean-clippy-toolchain-pin-committed-junk"]
---

## Summary

The time-series plot crate was marked complete, but it can't draw anything. `Chart::draw` is an empty stub, the line mesh only works for horizontal lines, and most of the plan was skipped. This fix finishes the original plan, `.codeyam/plans/completed/time-series-plots-simbiguity-plot-draws-values-over-time-with-the-existing-renderer.md`, which remains the spec. The capacitor and physics fixes both need working charts, so this must land first.

## Current behavior

- `crates/simbiguity-plot/src/chart.rs:12` — `Chart::draw(&self, _cmd: &mut CommandBuffer)` ignores the command buffer and returns `Vec::new()`. No series is ever drawn and no labels are produced.
- `crates/simbiguity-plot/src/line.rs:3` — `build_line_mesh` offsets every point only in y (`p.1 ± width/2`). A vertical segment has zero width, a steep one is hairline-thin, joints aren't mitred, and the width is in data units, not pixels. The "normal" written into each vertex is `(0, 0, 1)`, so it isn't the segment normal. It also computes `n_points - 1` after only checking `vertices.len()`.
- `crates/simbiguity-plot/src/ticks.rs:32` — `while t <= max + nice_step * 0.5 { ...; t += nice_step }` can loop forever when `t + nice_step == t`, for example min = 1e17 with a range of 16. Tick positions also build up floating-point error by repeated addition.
- `crates/simbiguity-plot/src/ticks.rs:51` — the only tick test asserts `!ticks.is_empty()` and `ticks[0] == 0.0`. It doesn't check the plan's worked examples.
- Missing entirely:
  - min/max-per-column downsampling
  - the scrolling time window
  - label formatting
  - an orthographic projection in `crates/simbiguity-render/src/camera.rs`
  - the `plot` feature in `crates/simbiguity/src/lib.rs`
  - `site/js/plot-labels.test.js`
- `crates/simbiguity-plot/capabilities.toml` doesn't use the `[[capability]]` / `proves = [...]` format that `xtask/src/capabilities.rs` parses, so the status page can't show proof for it. Compare `crates/simbiguity-circuits/capabilities.toml`.
- Public items have almost no doc comments, and `Cargo.toml` doesn't inherit the workspace package fields.

## Key Decisions

- **The original plan is the spec.** Every test listed in its "Tests" section must exist with the numbers it gives. Finalize must run, not be deferred, so clippy, fmt and the full test suite are green in the feature commit.
- **Ticks are computed by index**, `first + i * step` with a bounded count (`count * 4` maximum), not by repeated addition. This fixes both the possible infinite loop and the drift.
- **Line quads extrude along each segment's screen-space normal,** with mitre joins clamped to a limit, after data → pixel mapping. That way the width is in pixels, as the plan requires.

## Implementation

### 1. Make the chart draw

**File**: `crates/simbiguity-plot/src/chart.rs`

`draw` maps each series through `Axes`, downsamples it to at most 2 points per pixel column, builds the line mesh and appends the bind/draw commands to `cmd`. It returns the tick labels as `(text, x_px, y_px)`.

### 2. Correct line geometry

**File**: `crates/simbiguity-plot/src/line.rs`: screen-space normal extrusion with mitre joins; 0 or 1 points give an empty mesh.

### 3. Safe ticks and labels

**File**: `crates/simbiguity-plot/src/ticks.rs`: index-based generation, a guard against a degenerate range, and fixed-decimal label formatting.

### 4. Missing pieces

- **File**: `crates/simbiguity-plot/src/series.rs`: a "last N seconds" window query
- **File**: `crates/simbiguity-render/src/camera.rs`: orthographic projection, if it is still absent
- **File**: `crates/simbiguity/src/lib.rs`: a `plot` feature
- **File**: `crates/simbiguity-plot/capabilities.toml`: rewrite in the `[[capability]]` format with `proves` patterns
- **New file**: `site/js/plot-labels.test.js`
- Add doc comments that teach "data space → pixel space" on every public item.

## Reused existing code

- `Series` in `crates/simbiguity-plot/src/series.rs` and `Axes` in `crates/simbiguity-plot/src/axes.rs` (already correct, so keep them)
- `Mesh` from `crates/simbiguity-render/src/mesh.rs`; `CommandBuffer` from `crates/simbiguity-render/src/commands.rs`
- Capability format parsed by `xtask/src/capabilities.rs`

## Reproduction Test

`NiceTicks` doesn't produce the plan's worked example. It also hangs on large magnitudes.

**Target**: `crates/simbiguity-plot/src/ticks.rs` (its `tests` module). Run with `codeyam-editor editor refresh-tests --test ticks_for_zero_to_ten_step_by_two`.

```rust
/// The 1-2-5 rule: asking for about 5 ticks over [0, 10] gives a step of 2.
/// So the ticks are exactly 0, 2, 4, 6, 8, 10, a hand-checkable example of
/// choosing "nice" axis labels.
#[test]
fn ticks_for_zero_to_ten_step_by_two() {
    assert_eq!(NiceTicks::compute(0.0, 10.0, 5), vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
}

/// Near 1e17, adding a step of 2 no longer changes a float. Ticks must still
/// come back, with a bounded count, instead of looping forever.
#[test]
fn ticks_terminate_at_huge_magnitudes() {
    let ticks = NiceTicks::compute(1e17, 1e17 + 16.0, 5);
    assert!(ticks.len() <= 20);
}
```

Status: PROPOSED — confirm red at execution. Expected failure: the first test may fail on the exact list (confirm). The second hangs under the current `while` loop, so run it with a timeout.

## Scenarios to Demonstrate

- A sine wave plotted over a scrolling 5-second window
- A steep and a vertical segment drawn at the same pixel width as a flat one
- A millivolt range with sensible tick labels
- An empty chart (axes only)