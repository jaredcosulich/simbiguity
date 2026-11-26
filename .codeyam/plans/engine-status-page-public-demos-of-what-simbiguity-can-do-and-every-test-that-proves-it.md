---
title: "Engine status page: public demos of what simbiguity can do and every test that proves it"
mode: ui
createdAt: "2026-11-26T12:42:15Z"
source: manual
dependsOn: ["hello-circuit-hand-written-wasm-bridge-js-loader-first-demo-page-and-github-pages-deploy"]
---

## Summary

Add a public **"Engine status" page** to the simbiguity site. In one place anyone can see **what the engine can do** (a gallery of live, embedded demos per capability) and **how it is tested** (every test, grouped by crate and capability, with its plain-English description, its pass/fail from the latest `main` build, and a link to its source). It serves two audiences. A teacher or learner sees capabilities and trusts them. A developer evaluating the library sees the rigor behind each engine before importing it. The page regenerates on every deploy, so it never drifts from the code.

## Key Decisions

- **Generate the data, never hand-write it.** A zero-dependency `xtask` (`cargo run -p xtask -- site-report`) runs `cargo test --workspace --no-fail-fast` and parses libtest's stable plain output (`test <path> ... ok|FAILED|ignored`). It then walks each crate's source to attach the `///` description comment above every `#[test]` (the same comment-above-test convention codeyam enforces), and writes `site/status/report.json`. CI runs it before the Pages upload. A stale or hand-edited report is impossible by construction.
- **Tests are presented as worked examples, not a CI dump.** Group by crate → module → capability ("Ohm's law", "Wheatstone bridge", "Snapshot round-trip"), show the description first and the Rust name second, and link to the line on GitHub at the exact commit SHA. The circuit tests were written as textbook circuits on purpose, so they read as lessons here.
- **Capabilities are declared next to the code.** Each engine crate gets a small `capabilities.toml` (name, one-line description, which demo shows it, which test modules prove it), parsed by the xtask with a tiny hand-written TOML-subset reader to stay dependency-free, or as plain JSON if that proves simpler. Decide at execution. The page joins capabilities ↔ demos ↔ tests from that file, and a capability with no test is shown as "untested", which is visible pressure to keep coverage honest.
- **Demos embed live, not as screenshots.** Each capability card embeds its demo (the same pages from the demos library) in a lazy-loaded iframe, with a static poster image as the fallback until it is scrolled into view.
- **Show the build identity.** Commit SHA, date, test counts (passed/failed/ignored), and per-crate dependency count (expected: zero external) appear at the top. "Zero dependencies" is a claim the page proves rather than states.
- **Plain HTML + the existing loader, no framework.** The page renders `report.json` with a small handwritten module, consistent with the rest of `site/`.

## Implementation

### 1. Report generator

**New file**: `xtask/Cargo.toml` (a workspace member, `publish = false`, zero dependencies)
**New file**: `xtask/src/main.rs`: the `site-report` subcommand
**New file**: `xtask/src/libtest_output.rs`: parse libtest plain output into results
**New file**: `xtask/src/test_descriptions.rs`: find `#[test]` fns and their `///` / `//` comment blocks
**New file**: `xtask/src/capabilities.rs`: read each crate's capability manifest
**New file**: `xtask/src/report.rs`: join the three and write `site/status/report.json`

### 2. Capability manifests

**New file**: `crates/simbiguity-circuits/capabilities.toml` (and one per engine crate as each lands)

### 3. The page

**New file**: `site/status/index.html`: header with build identity and totals; capability cards (description, live demo embed, proving tests with pass/fail); a full filterable test table (by crate, status, text)
**New file**: `site/status/status.js`
**File**: `site/index.html`: prominent "See what it can do, and how it's tested" link

### 4. CI

**File**: `.github/workflows/pages.yml`: run `cargo run -p xtask -- site-report` before uploading `site/` (created by the hello-circuit plan)

## Reused existing code

- The `///`-above-`#[test]` convention the project already follows for codeyam's registry
- `site/js/simbiguity.js` loader and the demo pages (plan: WASM bridge + hello circuit)
- `.github/workflows/pages.yml` deploy (plan: WASM bridge + hello circuit)

## Tests (TDD — written first)

- `libtest_output`: parses ok / FAILED / ignored lines, nested module paths, the summary line, and output from several crates in one run; ignores unrelated lines; empty output → empty report.
- `test_descriptions`: picks up `///` and `//` blocks directly above `#[test]`, with other attributes in between; a test with no comment → `None` (rendered as "no description", which is itself visible pressure); handles nested `mod tests`.
- `capabilities`: a valid manifest; a capability naming a missing test module → reported as unproven, not a crash.
- `report`: join correctness, and a failing test makes its capability show failing.
- A codeyam scenario of the status page rendered from a fixture `report.json` covering all passing, one failing, and an untested capability.

## Scenarios to Demonstrate

- All green: every capability proven, demos embedded
- One failing test: its capability card flagged, test row highlighted
- An untested capability: shown as "untested"
- The filterable test table filtered to one crate