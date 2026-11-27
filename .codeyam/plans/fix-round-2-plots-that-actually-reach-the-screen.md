---
title: "Fix round 2: plots that actually reach the screen"
mode: ui
createdAt: "2026-11-27T14:49:55Z"
source: manual
dependsOn: ["fix-round-2-make-the-branch-green-and-keep-it-that-way"]
---

## Summary

`Chart::draw` now emits draw commands, but **a chart still can't appear on screen**. It never sets the view-projection or model matrices, so the shader multiplies every vertex by zero. The RC demo's label code also wipes the chart's canvas every frame. The plot crate has only two tests, both for ticks, and none for chart, line, series or downsampling. This plan makes charts visible and proves it with tests. The capacitor and physics demos both depend on it. Spec chain: `.codeyam/plans/completed/time-series-plots-simbiguity-plot-draws-values-over-time-with-the-existing-renderer.md` and `.codeyam/plans/completed/fix-time-series-plots-actually-draw-chart-draw-stub-line-geometry-tick-loop.md`.

## Current behavior

- **No matrices.** `Chart::draw` (`crates/simbiguity-plot/src/chart.rs:20`) pushes `SetProgram(Program::Unlit)` (lines 77 and 135) and `SetUniformVec4(Uniform::Color, …)` (line 136), then binds and draws. It never pushes `SetUniformMat4(Uniform::ViewProj, …)` or `SetUniformMat4(Uniform::Model, …)`, so the unlit vertex shader multiplies by zeroed matrices and nothing renders.
- **Ortho projection unused.** `orthographic_projection` (`crates/simbiguity-render/src/camera.rs:8`) exists, with a test, but nothing calls it.
- **No clear or viewport.** Nothing emits `Command::Clear` or a viewport for the plot area.
- **Labels wipe the canvas.** `renderLabels` (`site/js/plot-labels.js:2`) does `container.innerHTML = ""`. The RC demo passes the container that also holds its canvas (`site/demos/rc-charging/demo.js:103`), so the chart canvas is deleted every frame.
- **Tick drift remains.** Ticks still accumulate by addition (`crates/simbiguity-plot/src/ticks.rs:37`, `let next_t = t + nice_step`), so [0, 1] gives `0.19999999999999996, …, 0.9999999999999998`.
- **Almost no tests.** `crates/simbiguity-plot/src/chart.rs`, `line.rs`, `axes.rs` and `series.rs` contain zero `#[test]`s, and `ticks.rs` has 2.
- **Docs and proofs.** `Series` and `NiceTicks` lack doc comments, and `crates/simbiguity-plot/capabilities.toml` proves only `ticks::tests`.
- **Fake demo.** `site/demos/plot-demo/index.html` shows three hard-coded labels instead of a real chart.

## Key Decisions

- **The chart owns its camera.** `draw` emits a viewport for the plot rectangle and `SetUniformMat4(ViewProj, orthographic_projection(…))` mapping pixel space to clip space. It also emits `Model = identity` before the line meshes. A test decodes the emitted `CommandBuffer` and asserts these commands come before the first `DrawIndexed`.
- **Prove a vertex lands where it should.** A test takes one series point and pushes it through `Axes` → the line mesh → the emitted ViewProj. It asserts the clip-space position equals the hand-computed value. For example, with the data rectangle [0, 10] × [0, 1] on a 200 × 100 pixel area, (5, 0.5) maps to clip (0, 0).
- **Labels get their own layer.** `renderLabels` writes into a dedicated overlay element and never clears siblings. The demo markup gets a separate label layer.
- **Ticks by index:** `first + i * step`, then rounded to the step's decimal places, so [0, 1] gives exactly 0, 0.2, 0.4, 0.6, 0.8, 1.
- **Every module gets tests:**
  - **Line:** a horizontal, vertical and diagonal segment each have a pixel width equal to the requested width.
  - **Mitre:** the mitre is clamped at a sharp turn.
  - **Series:** pushing capacity + 3 keeps the newest values in order; `window`.
  - **Downsampling:** one spike in 10,000 flat samples survives, with at most 2 × width points.
  - **Axes:** the corners map exactly.

## Implementation

### 1. Chart camera

**File**: `crates/simbiguity-plot/src/chart.rs`

### 2. Ticks

**File**: `crates/simbiguity-plot/src/ticks.rs`

### 3. Tests and docs

- **File**: `crates/simbiguity-plot/src/line.rs`
- **File**: `crates/simbiguity-plot/src/series.rs`
- **File**: `crates/simbiguity-plot/src/axes.rs`
- **File**: `crates/simbiguity-plot/capabilities.toml`

### 4. Labels and demo

- **File**: `site/js/plot-labels.js`, with `site/js/plot-labels.test.js` (asserting siblings survive)
- **File**: `site/demos/rc-charging/index.html`: a separate label layer
- **File**: `site/demos/plot-demo/index.html`: a real chart of a sine wave through the bridge, or delete the page and its link if no bridge exposes a generic chart

## Reused existing code

- `orthographic_projection` in `crates/simbiguity-render/src/camera.rs`
- `Command::SetUniformMat4`, `Uniform::ViewProj`, `Uniform::Model`, `Command::Clear` in `crates/simbiguity-render/src/commands.rs`
- `CommandBuffer::decode` for asserting emitted commands

## Reproduction Test

The chart's draw commands never set a view-projection matrix, so nothing reaches the screen.

**Target**: `crates/simbiguity-plot/src/chart.rs` (a new `tests` module). Run with `codeyam-editor editor refresh-tests --test chart_sets_view_projection_before_drawing`.

```rust
/// The GPU draws a triangle only after it's told how to map positions to the
/// screen (the view-projection matrix). A chart that draws its lines without
/// first setting that matrix produces nothing visible, however correct its
/// data is.
#[test]
fn chart_sets_view_projection_before_drawing() {
    let mut s = Series::new(8);
    s.push(0.0, 0.0);
    s.push(1.0, 1.0);
    let chart = sample_chart(&s); // [0,1]×[0,1] on 200×100 px; settle the constructor at execution
    let mut cmd = CommandBuffer::new();
    chart.draw(&mut cmd /* , size args as the API requires */);
    let cmds = CommandBuffer::decode(cmd.words()).unwrap();
    let first_draw = cmds.iter().position(|c| matches!(c, Command::DrawIndexed { .. })).unwrap();
    assert!(cmds[..first_draw].iter().any(|c| matches!(c, Command::SetUniformMat4(Uniform::ViewProj, _))));
}
```

Status: PROPOSED — confirm red at execution. Expected failure: the `any(...)` assertion is false, because no `ViewProj` is set. Match the exact `Command::SetUniformMat4` variant shape to `crates/simbiguity-render/src/commands.rs` at execution.

## Scenarios to Demonstrate

- RC demo chart visibly drawing the charging curve with labels beside it
- Chart with a vertical step showing constant line width
- Ticks over [0, 1] labelled exactly 0, 0.2, …, 1

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