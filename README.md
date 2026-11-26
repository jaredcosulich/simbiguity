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

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
