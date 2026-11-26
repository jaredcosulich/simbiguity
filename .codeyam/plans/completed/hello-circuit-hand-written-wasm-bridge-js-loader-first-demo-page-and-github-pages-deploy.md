---
title: "Hello circuit: hand-written WASM bridge, JS loader, first demo page and GitHub Pages deploy"
mode: ui
createdAt: "2026-11-26T12:42:15Z"
source: manual
dependsOn: ["dc-circuit-engine-simbiguity-circuits-with-modified-nodal-analysis-and-teachable-diagnostics"]
---

## Summary

Put the first engine in front of people. This plan has four parts: a **hand-written WASM interface** (no `wasm-bindgen`) that exposes `simbiguity-circuits` to JavaScript, a small **handwritten JS loader**, a **"Hello, circuit" demo page** where you close a switch and a bulb lights up (with live voltage/current readouts), and a **GitHub Pages deploy** of `site/` from CI. The demo is also the reference for how another project embeds simbiguity. Its source is meant to be read and copied.

## Key Decisions

- **A plain C ABI plus shared linear memory, no codegen.** The bridge crate exports `extern "C"` functions over opaque handles (`circuit_new() -> u32`, `circuit_add_bulb(h, a, b, watts, volts) -> u32`, `circuit_toggle(h, id)`, `circuit_solve(h) -> i32`, `circuit_bulb_brightness(h, id) -> f64`). Bulk results go out as a pointer + length into a `Vec<f64>` the JS side reads via `Float64Array` on `memory.buffer`. Errors come back as status codes plus `last_error_ptr/len` for a UTF-8 message. Every export is a normal Rust function, so it is **unit-tested natively**. The ABI is not WASM-only.
- **One bridge crate per engine** (`simbiguity-circuits-wasm`), not one mega-bridge, so a project that only wants circuits ships only circuits. It depends on its engine and nothing else.
- **The loader is a tiny ES module** (`site/js/simbiguity.js`): `WebAssembly.instantiateStreaming`, memory views refreshed after any call that may grow memory, string decode for errors, and a thin idiomatic wrapper class per engine (`new Circuit()` …). Zero npm dependencies, no bundler.
- **2D first.** The hello demo draws the circuit with inline SVG (bulb glow = brightness). The from-scratch WebGL2 renderer is its own plan. Keeping it out of here gets the embedding story shipped and testable sooner.
- **Build = cargo only.** `cargo build -p simbiguity-circuits-wasm --target wasm32-unknown-unknown --release`, then copy the `.wasm` into `site/pkg/` (gitignored). A small script does both, and the codeyam `startCommand` runs it before serving, so the preview always serves a fresh build.
- **Pages deploy from GitHub Actions** (`actions/upload-pages-artifact` + `deploy-pages`) on pushes to `main`: install the wasm target, build, upload `site/`. CI also runs `cargo fmt --check`, `clippy -D warnings`, and `cargo test --workspace`. Turn on Pages for the repo (source: GitHub Actions) as part of this plan, with the user's OK, since it changes repo settings.

## Implementation

### 1. WASM bridge crate

**New file**: `crates/simbiguity-circuits-wasm/Cargo.toml`: `crate-type = ["cdylib", "rlib"]` (the rlib so native tests link)
**New file**: `crates/simbiguity-circuits-wasm/src/lib.rs`: handle table, exports, status codes, `last_error`
**New file**: `crates/simbiguity-circuits-wasm/src/alloc.rs`: `sb_alloc(len)` / `sb_free(ptr, len)` so JS can pass byte buffers in

### 2. JS loader and wrapper

**New file**: `site/js/simbiguity.js`: `loadSimbiguity(url)` → typed wrappers; documented line by line as embedding docs

### 3. Hello-circuit demo

**New file**: `site/demos/hello-circuit/index.html`: an explainer paragraph, an SVG circuit (battery, switch, bulb), a clickable switch, live readouts (V, I, P), and a brightness slider for battery voltage that shows P ∝ V² and the bulb blowing above its rating
**New file**: `site/demos/hello-circuit/demo.js`
**File**: `site/index.html`: link to the demo

### 4. Build + dev server

**New file**: `scripts/build-site.sh`: cargo build for wasm32 + copy to `site/pkg/`
**File**: `.codeyam/editor.json`: `apps[0].startCommand` becomes a build-then-serve command (keep the literal app port `4780`, which the editor rewrites to its internal port; it runs inside `site/`, so call the script as `../scripts/build-site.sh`)

### 5. CI + Pages

**New file**: `.github/workflows/ci.yml`: fmt, clippy, test, wasm build
**New file**: `.github/workflows/pages.yml`: build `site/` and deploy to GitHub Pages on `main`

## Reused existing code

- `CircuitSim` / `Solution` / `CircuitIssue` from `simbiguity-circuits` (plan: circuits DC)
- `site/index.html` landing page and the `landing-coming-soon` scenario

## Tests (TDD — written first)

- Native unit tests of every export: create/build/solve/read brightness; an unknown handle → error status and a readable `last_error`; a short circuit → status + message; toggle round-trip; the alloc/free round-trip.
- Handle hygiene: `circuit_free` invalidates the handle, and double free is an error, not UB.
- A codeyam scenario for the demo page in two states, **switch open (bulb dark)** and **switch closed (bulb lit)**, seeded through the URL hash (`#closed`). That makes the capture deterministic and proves the WASM path end to end in a real browser.

## Scenarios to Demonstrate

- Hello circuit, switch open: dark bulb, 0 A readout
- Hello circuit, switch closed: lit bulb, real V/I/P
- Overvoltage: blown bulb with the explanation shown
- Short circuit: the diagnostic message rendered on the page