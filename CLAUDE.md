# simbiguity

An open source simulation engine by The Ambiguity Project: small, independently
importable Rust crates for simulation, animation and 3D rendering, built for
education. Work is driven by the plans in `.codeyam/plans/` (run them in their
`dependsOn` order).

## Non-negotiables

- **Zero external dependencies in engine crates** (`crates/simbiguity-*`). Only
  path dependencies on other simbiguity crates. No `libm`, no `rand`, no `serde`.
  If something is needed, write it, document it, and test it.
- **`#![no_std]` + `alloc` where possible**, with an opt-in `std` feature. Engines
  must run natively and in WASM.
- **One concern per crate.** Simulation crates never depend on rendering or
  animation; rendering/animation consume simulation output, never the reverse.
- **Deterministic.** Fixed timesteps, explicitly seeded RNG, no global state. The
  same inputs at the same steps must give bit-identical states (puzzle checking,
  undo, replay and tests depend on it).
- **No `wasm-bindgen`, no bundler, no npm dependencies.** The web interface is a
  hand-written `extern "C"` ABI over shared memory (`*-wasm` bridge crates) plus
  small handwritten ES modules in `site/js/`. Bridge exports are plain Rust
  functions and are unit-tested natively.
- **Rendering is from scratch:** Rust builds a draw-command buffer; a thin JS
  WebGL2 executor replays it. Keep logic out of the JS.

## Code and tests

- Test-driven: write the failing test first. Tests live next to the code
  (`#[cfg(test)] mod tests`).
- **Every `#[test]` has a `///` description comment directly above it** that
  explains what it proves and why. These descriptions are published on the
  public engine status page, so write them for a learner.
- Prefer textbook examples with hand-computed answers (e.g. Ohm's law, a
  Wheatstone bridge) so tests double as worked examples.
- Public items carry doc comments that teach the idea, not just the signature.
- `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` must pass before every commit (codeyam runs these as
  static checks and in the pre-commit hook).
- No `unwrap()` in library code; each crate defines its own error enum.

## Layout

- `crates/simbiguity`: umbrella crate re-exporting engines behind features.
- `crates/simbiguity-<engine>`: one engine per crate; `*-wasm` crates are its
  browser bridge.
- `site/`: the static GitHub Pages site (demos + status page), served locally by
  codeyam's preview. Built WASM goes to `site/pkg/` (gitignored).
