---
title: "Site gallery, smoke test and graceful failure before GitHub Pages"
mode: ui
createdAt: "2026-11-28T02:43:18Z"
source: manual
dependsOn: ["round-3-puzzle-game-page-that-plays-the-real-levels-through-the-bridge", "physics-lab-demo-every-engine-capability-on-one-page", "circuit-lab-demo-leds-rectifiers-resonance-and-a-live-oscilloscope"]
---

## Summary

The site's landing page links only three demos:

- `site/index.html` links `status/`, `hello-circuit`, `circuit-3d` and `physics`.
- It omits `rc-charging`, `circuit-3d-parallel` and `circuit-puzzles`, and the new Physics Lab and Circuit Lab.

Nothing checks that a demo page loads at all. A broken import path or a missing WASM file would only show up when someone opened the page. Before turning on GitHub Pages, the site needs:
- a real gallery;
- a smoke test that every page and every module it imports resolve;
- a visible failure message when WebGL2 or WASM is unavailable.

## Key Decisions

**One source of truth for the demo list.** `site/demos/manifest.json` (new) holds, for each demo:
- slug and title;
- a one-line description;
- the engine capabilities it shows (by their `capabilities.toml` names);
- a thumbnail path.

The gallery and the smoke test both read it.

**Thumbnails come from captured scenarios.** Each demo's best scenario screenshot is downscaled to 480×300 PNG and committed under `site/demos/<slug>/thumb.png`. It must be under 60 KB.

**Smoke test without a browser.** A Node test parses each demo's `index.html` and every `import` reachable from its entry module, and asserts that each referenced file exists (including `../../pkg/*.wasm` after `build-site.sh`). This catches the failure mode cheaply and in CI.

**Graceful failure.** A shared `site/js/boot.js` (new) does three things:
1. Checks for WebGL2 and WebAssembly.
2. Shows a styled message instead of a blank canvas when either is missing.
3. Catches any rejected `main()` and shows its message.

Every demo uses it. Remove the bare `main().catch(console.error)` from each.

## Implementation

### 1. Manifest

**File**: `site/demos/manifest.json` (new)

It must list every directory under `site/demos/`. A test asserts that the directory listing and the manifest match exactly.

### 2. Gallery

**File**: `site/index.html`

Show a card grid built from the manifest: thumbnail, title, description and capability tags. Each tag links to that capability on `status/`. Include a short intro and a link to the status page.

### 3. `boot.js` in every demo

**Files**:
- `site/js/boot.js` (new)
- every `site/demos/*/demo.js` / `lab.js` / `game.js`

Test: `boot.js` with a stubbed `document` and no `WebGL2RenderingContext` renders the message element. A rejected promise shows its message text.

### 4. Smoke test

**File**: `site/smoke.test.js` (new)

Walk the import graph from every demo's `index.html`, and assert every file exists. Seed it with one deliberately broken path in a temp copy, and see it fail.

### 5. Pages workflow sanity

**File**: `.github/workflows/pages.yml`

1. Confirm it runs `scripts/build-site.sh` and uploads `site/`, including `site/pkg/`.
2. Add the smoke test to the `site` job in `.github/workflows/ci.yml`.
3. Don't change the deploy trigger (push to `main`).

Enabling Pages in the repo settings is a manual step for the repo owner and is not part of this plan.

## Reused existing code

- `site/status/demo-embed.js`: its loading placeholder pattern
- the existing `site` job in `.github/workflows/ci.yml`
- the scenario screenshots under `.codeyam/scenarios/screenshots/`

## Reproduction Test

`every_demo_directory_is_in_the_manifest` fails today, because there is no manifest and `site/index.html` omits at least three demos.

## Acceptance rules (the whole build must meet these)

Three earlier builds of this work passed their own checks while shipping stubs. Each rule below comes from a real failure:

- **Red first.** Write the tests named in this plan before the code. Run them on the starting tree and see them fail. Put the failure message in the commit body.
- **No hollow assertions.** All of these are forbidden:
  - asserting a constant (`assert_eq!(1, 1)`, `assert.strictEqual(typeof x, "string")` standing in for the real check)
  - asserting something that is always true (`x > -1.0`)
  - empty `if let` or loop bodies that check nothing
  - a goal or check that returns a hard-coded `true` or `false`
  - loosening a tolerance until the test passes
- **No stubs that only change shape.** Replacing `return true` with `return false`, or one constant with another, is not an implementation. So is a parser that ignores its input, or an encoder that copies its input. Placeholder comments are forbidden too: "stub", "would go here", "a real X would…", "once the TDD phase starts", "for prototype", "wait…", "omitted for brevity", "deferred".
- **Numbers come from this plan, not from the code.** The reference values here were worked out by hand or with an independent script. If the code disagrees with one, the code is wrong. Do not take the expected value from the code's own output.
- **No empty files, no fake screenshots.**
  - Every file this plan creates must have real content.
  - Each scenario must show a different state. Its screenshot must differ from every other scenario's (no identical hashes).
- **Budgets are not raised to fit.** Don't edit `site/pkg-budget.txt` or any bench budget upward to make a check pass. Make the code smaller or faster instead. If a budget truly must move, say why in the commit body and cap the increase at 5%.
- **Touch only what's listed.** Before committing, `git diff --stat <start>..HEAD` must show no changes to:
  - `rust-toolchain.toml`
  - `.gitignore`
  - crates or demos this plan doesn't name

  If anything like that shows up, restore it from the starting commit.
- **Everything green, and finalize runs.** No `Codeyam-Finalize: deferred`. All five must pass:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `sh scripts/build-site.sh` (it must exit 0)
  - `node --test "site/**/*.test.js"`
- **No junk.** Don't commit `patch.diff`, scratch files, or `.patch`, `.orig` or `.rej` files.
- **Every `proves` entry exists.** In each touched `capabilities.toml`, every entry must name a test that exists and passes. Every new capability must be listed there, so it appears on the status page.
- **Done means demonstrable.** The last commit's body must list each Implementation section and the test(s) that prove it, one line per section. If a section is not done, say so plainly. Do not mark the plan complete.