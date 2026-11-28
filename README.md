# simbiguity

**An open source simulation engine by The Ambiguity Project.**

simbiguity is a family of small, independently importable Rust crates for
simulation, 3D rendering and animation — built from scratch, with zero external
dependencies in the engine crates, and designed first for education.

Take only what you need: just the circuit solver, just the physics, just the
animation library. Every engine shares the same small set of contracts, so they
compose: a circuit can drive a motor that spins a rigid body, all stepped in
lockstep on one clock.

## Crates (planned)

| Crate | Purpose |
|---|---|
| `simbiguity-math` | Vectors, matrices, quaternions, linear solvers |
| `simbiguity-core` | `Simulation` trait, fixed-timestep clock, seeded RNG, snapshot/restore |
| `simbiguity-circuits` | Circuit solver: DC, then transient and AC |
| `simbiguity-physics` | Particles, springs, pendulums, orbits, collisions |
| `simbiguity-markets` | Order books, agent-based traders, price models |
| `simbiguity-anim` | Easing, tweens, timelines, keyframes, spring motion |
| `simbiguity-render` | Renderer-agnostic scene and draw-command data |
| `simbiguity-plot3d` | 3D surfaces, scatter and time-series charts |

## Design principles

- **Zero dependencies** in the engine crates — `std` only, `no_std + alloc` where possible.
- **Deterministic** — fixed timesteps and seeded randomness, so every run is reproducible
  and puzzle solutions can be checked exactly.
- **Readable** — the source is part of the lesson.
- **Portable** — engines run natively or in the browser. Web embedding uses a
  hand-written WASM interface (plain `extern "C"` exports and shared memory buffers)
  and a from-scratch WebGL2 renderer; no `wasm-bindgen`.

## Demos

Interactive demos live in [`site/`](site/) and are published to GitHub Pages. They
double as the reference for how your own project can embed the engines.

- [Hello, circuit](site/demos/hello-circuit/) — a battery, a switch and a bulb, solved
  live by `simbiguity-circuits` through the hand-written bridge in
  [`crates/simbiguity-circuits-wasm`](crates/simbiguity-circuits-wasm) and the
  ~150-line loader [`site/js/simbiguity.js`](site/js/simbiguity.js).
- [Circuit in 3D](site/demos/circuit-3d/) — the same circuit on a board you can orbit,
  with charge flowing as fast as the solved current and a bulb that warms up on a spring.
  `simbiguity-anim`, `simbiguity-render` and `simbiguity-circuits-view` build every frame
  as draw commands in Rust (bridge: [`crates/simbiguity-render-wasm`](crates/simbiguity-render-wasm));
  [`site/js/webgl2-executor.js`](site/js/webgl2-executor.js) only replays them with WebGL2.

To run them locally you need the `wasm32-unknown-unknown` target
(`rustup target add wasm32-unknown-unknown`) and Python 3 for a static server:

```sh
sh scripts/build-site.sh            # builds the .wasm into site/pkg/
python3 -m http.server -d site 8000 # then open http://localhost:8000
```

The demo pages' logic has its own tests, run with Node 22 or newer (no npm install needed):
`node --test "site/**/*.test.js"`.

### Engine status page

[`site/status/`](site/status/) lists everything the engines can do, each with its live
demo and the tests that prove it, plus every test in the workspace with the comment written
above it. Its data, `site/status/report.json`, is generated on deploy and not committed. To
build it locally:

```sh
cargo run -p xtask -- site-report   # runs every crate's tests, writes site/status/report.json
```

Capabilities are declared next to the code in each crate's `capabilities.toml`.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

<!-- codeyam:run-and-edit:start d=e0ed0650698f -->
## Develop this project with codeyam-editor

This project is built with [codeyam-editor](https://codeyam.com) — code and runnable data scenarios are authored side by side against a live preview.

```bash
# Clone the repo
git clone https://github.com/jaredcosulich/simbiguity && cd simbiguity

# Install codeyam-editor
npm install -g @codeyam-editor/codeyam-editor@latest

# Launch the editor (split-screen terminal + live preview)
codeyam-editor start
```
<!-- codeyam:run-and-edit:end -->

<!-- codeyam:scenario-gallery:start d=1fd3f4296d8a -->
## Scenario gallery

States captured as runnable scenarios with codeyam-editor:

### Circuit 3D - Bulb Blown

<img src=".codeyam/scenarios/screenshots/circuit-3d-bulb-blown--desktop.png" alt="Circuit 3D - Bulb Blown" width="280">

Far too much voltage burns the filament through: the glass goes smoky, the broken filament shows, the charges stop, and the panel offers to replace the bulb.

### Circuit 3D Parallel - Working

<img src=".codeyam/scenarios/screenshots/circuit-3d-parallel-working.png" alt="Circuit 3D Parallel - Working" width="280">

Parallel branches with current visibly splitting.

### Circuit Lab Demo

<img src=".codeyam/scenarios/screenshots/circuit-lab-demo--desktop.png" alt="Circuit Lab Demo" width="280">

Circuit Lab Demo.

### Physics Engine Demo

<img src=".codeyam/scenarios/screenshots/physics-engine-demo.png" alt="Physics Engine Demo" width="280">

Demonstrates physics features

### Engine Status - All Green

<img src=".codeyam/scenarios/screenshots/engine-status-all-green--desktop.png" alt="Engine Status - All Green" width="280">

Every test passes and all 25 capabilities are proven: the build line names the commit, the headline numbers are green, and the circuit capability cards lead with their live demos.

### Hello Circuit - Bulb Blown

<img src=".codeyam/scenarios/screenshots/hello-circuit-bulb-blown--desktop.png" alt="Hello Circuit - Bulb Blown" width="280">

Overvoltage: the filament burns out, the panel is marked as an issue, and the Replace the bulb button appears.

### Landing - Demos

<img src=".codeyam/scenarios/screenshots/landing-demos--desktop.png" alt="Landing - Demos" width="280">

The project landing page with its Demos list, linking the Hello, circuit demo and the new Circuit in 3D demo.

### Plot - Sine Wave

<img src=".codeyam/scenarios/screenshots/plot-sine-wave--desktop.png" alt="Plot - Sine Wave" width="280">

A sine wave plotted over a scrolling 5-second window
<!-- codeyam:scenario-gallery:end -->
