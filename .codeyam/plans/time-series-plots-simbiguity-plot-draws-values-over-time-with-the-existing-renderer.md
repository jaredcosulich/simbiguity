---
title: "Time-series plots: simbiguity-plot draws values over time with the existing renderer"
mode: ui
createdAt: "2026-11-26T19:29:13Z"
source: manual
---

## Summary

Every simulation that moves through time needs the same picture: a value over time drawn as a line. Examples are a capacitor's voltage charging, a pendulum's angle and energy, and later a market's price. Today nothing in simbiguity can draw a 2D line or an axis. The renderer only knows 3D meshes (`Mesh::cuboid`, `cylinder`, `sphere`, `capsule`). This plan adds `simbiguity-plot`, a small rendering-side crate. It turns recorded samples into draw commands that the existing WebGL2 executor already replays: an orthographic plot area, line strips, "nice" axis ticks and a moving time window. The transient-circuit and physics plans both build their charts on it.

## Key Decisions

- **A separate crate, rendering side.** `simbiguity-plot` depends on `simbiguity-render` and `simbiguity-math` and never on a simulation crate. An engine hands it plain `(t, value)` samples, which keeps simulation → rendering one-way.
- **Lines are meshes, not a new GPU primitive.** Each series becomes a strip of thin quads (two triangles per segment, mitred at joints) in the existing vertex format, drawn with `DrawIndexed` and the unlit program. The draw-command format and the JS executor need no new opcodes. If an orthographic projection is missing from `simbiguity-render`'s camera, add it there (with tests), not in the plot crate.
- **Text stays in HTML.** The renderer draws no text, and writing a font rasteriser is out of scope. Rust computes each tick label's string and pixel position. A tiny JS helper only places `<span>`s at those coordinates, so the logic stays in Rust.
- **Nice ticks use the classic 1-2-5 rule**, documented as a worked example: range [0, 10] with about 5 ticks gives a step of 2 (0, 2, 4, 6, 8, 10), and range [0, 0.73] gives a step of 0.2.
- **Fixed memory and fast to draw.** A `Series` is a ring buffer with a fixed capacity, so recording at 60 Hz forever never grows memory. Drawing downsamples to at most two points per pixel column (keeping the min and max in each column), so a spike is never hidden and a 10,000-sample series still draws a few hundred segments.
- **Deterministic and `no_std` + `alloc`**, like every other crate here.

## Implementation

### 1. The plot crate

**New file**: `crates/simbiguity-plot/src/lib.rs`: crate docs explaining a plot as "data space → pixel space"
**New file**: `crates/simbiguity-plot/src/series.rs`: `Series` ring buffer (push, iter in time order, min/max over a window)
**New file**: `crates/simbiguity-plot/src/ticks.rs`: 1-2-5 nice-tick algorithm and label formatting (fixed decimals, no `format!` float surprises across targets)
**New file**: `crates/simbiguity-plot/src/axes.rs`: `Axes` mapping a data rectangle to a pixel rectangle; a scrolling time window ("last 5 s")
**New file**: `crates/simbiguity-plot/src/line.rs`: polyline → quad-strip mesh with constant pixel width; min/max-per-column downsampling
**New file**: `crates/simbiguity-plot/src/chart.rs`: `Chart` = axes + several series + colors → `CommandBuffer` plus a list of `(label, x_px, y_px)`
**New file**: `crates/simbiguity-plot/capabilities.toml`: capabilities for the status page

### 2. Orthographic projection (only if missing)

**File**: `crates/simbiguity-render/src/camera.rs`

Add an orthographic camera/projection for 2D overlays, if one does not already exist. Test it: the corners of the pixel rectangle map to clip-space ±1.

### 3. Wire into the umbrella crate and the bridge pattern

**File**: `crates/simbiguity/src/lib.rs`: re-export behind a `plot` feature.

Bridge exports for a chart (`plot_*` functions) go in whichever `*-wasm` crate first uses a chart. That is the transient-circuits plan, so no new bridge crate is created here.

**New file**: `site/js/plot-labels.js` (+ `plot-labels.test.js`): places label spans from a packed `(x, y, string index)` buffer; no layout logic.

## Reused existing code

- `Mesh` and the vertex format from `crates/simbiguity-render/src/mesh.rs`
- `CommandBuffer` / `Command::DrawIndexed` from `crates/simbiguity-render/src/commands.rs`
- `encode_frame` from `crates/simbiguity-render/src/frame.rs`
- `site/js/webgl2-executor.js` replays the output unchanged
- `capabilities.toml` format read by `xtask/src/capabilities.rs`

## Tests (written first)

- Nice ticks: [0, 10] → 0, 2, 4, 6, 8, 10; [0, 0.73] → 0, 0.2, 0.4, 0.6; [-3, 3] includes 0; a zero-width range doesn't divide by zero.
- Axes: the data rectangle's corners map exactly to the pixel rectangle's corners.
- Ring buffer: pushing capacity + 3 samples keeps the newest `capacity`, in time order.
- Downsampling: a single spike in 10,000 flat samples survives; the output has at most 2 × width points.
- Line mesh: an n-point polyline produces 2(n-1) triangles; the quad's width in pixels equals the requested width after projection.
- Determinism: the same series produces a bit-identical command buffer.

## Scenarios to Demonstrate

- A sine wave plotted over a scrolling 5-second window
- Two series with different colors (e.g. voltage and current) on shared axes
- A tiny range (millivolts) showing sensible tick labels
- An empty chart (no samples yet) showing axes only