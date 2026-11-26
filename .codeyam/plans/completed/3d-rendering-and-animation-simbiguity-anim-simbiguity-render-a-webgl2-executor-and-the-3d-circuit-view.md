---
title: "3D rendering and animation: simbiguity-anim, simbiguity-render, a WebGL2 executor and the 3D circuit view"
mode: ui
createdAt: "2026-11-26T12:42:15Z"
source: manual
dependsOn: ["hello-circuit-hand-written-wasm-bridge-js-loader-first-demo-page-and-github-pages-deploy"]
---

## Summary

Build the visual layer for 3D demos, from scratch, in two independently importable crates. **`simbiguity-anim`** handles easing curves, tweens, keyframe tracks, timelines and critically-damped springs. It is pure math with no renderer, usable anywhere. **`simbiguity-render`** handles a renderer-agnostic scene (meshes, materials, lights, orbit camera, picking rays) that compiles into a flat **draw-command buffer**. On the web, a thin handwritten **WebGL2 executor** in JS replays that buffer. The first consumer is a **3D view of the hello circuit**: a battery, switch, wires and bulb on a board, with current shown as animated particles flowing along the wires at a speed proportional to the solved current, and the bulb's glow driven by its brightness.

## Key Decisions

- **Rust decides what to draw; JS only issues GL calls.** `simbiguity-render` outputs a compact command buffer (`SetProgram`, `SetUniformMat4`, `BindMesh`, `DrawIndexed` …), plus mesh vertex/index buffers, through the same C-ABI + shared-memory pattern as the circuits bridge. The JS executor is a few hundred lines with no logic of its own. The same buffer can feed a native backend later (wgpu, Metal, or a software rasterizer for tests) without touching scene code, which keeps the door open for native apps.
- **Mesh generation in Rust** (box, cylinder, sphere, capsule, tube-along-polyline for wires), so geometry is testable and shared by every backend.
- **Tiny built-in shader set** (unlit, Lambert + Blinn-Phong, emissive glow, instanced points for current particles), written in GLSL ES 3.00 and stored next to the executor. No shader graph.
- **Animation drives from simulation state, never the reverse.** Particle speed and bulb glow are pure functions of the circuit's `Solution`. `simbiguity-anim` springs smooth the visual response (the bulb warms up and cools down) without changing the physics.
- **Picking in Rust.** Ray from camera through cursor → intersect component bounding volumes → component id. This is how a click on the 3D switch toggles it, and it is the basis of the puzzle interactions later.
- **Determinism for captures.** The demo accepts a fixed time (`#t=2.5`) so codeyam scenario captures are pixel-stable. Headless Chromium's WebGL2 (SwiftShader) is the capture backend, so confirm early at execution that it renders the shaders.

## Implementation

### 1. `simbiguity-anim`

**New file**: `crates/simbiguity-anim/src/lib.rs`
**New file**: `crates/simbiguity-anim/src/easing.rs`: the standard easing family (quad/cubic/expo/back/elastic/bounce, in/out/in-out)
**New file**: `crates/simbiguity-anim/src/tween.rs`: `Tween<T: Lerp>`
**New file**: `crates/simbiguity-anim/src/track.rs`: keyframe tracks with per-segment easing
**New file**: `crates/simbiguity-anim/src/timeline.rs`: sequence/parallel composition, seek, loop
**New file**: `crates/simbiguity-anim/src/spring.rs`: critically-damped / under-damped spring follower

### 2. `simbiguity-render`

**New file**: `crates/simbiguity-render/src/lib.rs`
**New file**: `crates/simbiguity-render/src/scene.rs`: nodes, transforms, materials, lights
**New file**: `crates/simbiguity-render/src/camera.rs`: perspective + orbit controls
**New file**: `crates/simbiguity-render/src/mesh.rs`: primitive and tube generation
**New file**: `crates/simbiguity-render/src/commands.rs`: the draw-command buffer encoding (versioned, documented byte layout)
**New file**: `crates/simbiguity-render/src/pick.rs`: ray casting

### 3. Web executor + bridge

**New file**: `crates/simbiguity-render-wasm/src/lib.rs`: C-ABI exports for scene building, frame encode, pick
**New file**: `site/js/webgl2-executor.js`: decode the command buffer → WebGL2 calls
**New file**: `site/js/shaders/`: GLSL ES 3.00 sources

### 4. 3D circuit demo

**New file**: `crates/simbiguity-circuits-view/src/lib.rs`: maps a `Circuit` + `Solution` to a scene (component models, wire tubes, current particles, glow), depending on circuits + render + anim
**New file**: `site/demos/circuit-3d/index.html`: orbit, click to toggle the switch, a voltage slider, readouts
**File**: `site/status/index.html`: add the 3D demo as the embed for its capabilities

## Reused existing code

- `Vec3` / `Quat` / `Mat4` from `simbiguity-math`; `FixedClock` from `simbiguity-core` (plan: foundations)
- `CircuitSim` / `Solution` from `simbiguity-circuits` (plan: circuits DC)
- The C-ABI + shared-memory bridge pattern and `site/js/simbiguity.js` (plan: WASM bridge + hello circuit)
- The status page capability cards (plan: engine status page)

## Tests (TDD — written first)

- Easing: every curve hits f(0)=0 and f(1)=1, plus monotonicity where it is promised; a spring settles within tolerance by a computed time and a critically-damped spring never overshoots; timeline seek is exact.
- Mesh: vertex/index counts per primitive, normals unit-length, tube follows the polyline; winding consistent.
- Camera: a known point projects to a known pixel; orbit clamps pitch.
- Command buffer: encode → decode round-trip; version header; unknown op rejected.
- Picking: a ray through the switch hits the switch; misses return `None`; the nearest hit wins.
- Circuit view: particle speed ∝ current (zero current → stationary); glow follows brightness through the spring.
- codeyam scenarios of the 3D demo at a fixed `t` in the dark and lit states.

## Scenarios to Demonstrate

- 3D circuit, switch open: dark bulb, no particles moving
- 3D circuit, switch closed: particles flowing, bulb glowing
- High voltage: fast particles, blown bulb effect
- Camera orbited to a side angle (deterministic via `#cam=`)