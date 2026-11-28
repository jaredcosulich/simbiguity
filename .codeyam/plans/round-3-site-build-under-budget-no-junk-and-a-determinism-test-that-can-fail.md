---
title: "Round 3: site build under budget, no junk, and a determinism test that can fail"
mode: backend
createdAt: "2026-11-28T02:43:18Z"
source: manual
---

## Summary

Round 2 left three loose ends that keep the branch from being merge-ready:

- `sh scripts/build-site.sh` exits 1. The render WASM is 230,501 bytes against a 230,000 budget (`site/pkg-budget.txt`).
- A stray `patch.diff` was committed at the repo root by `624512d`.
- `site/js/determinism.test.js` checks nothing real. It asserts `typeof json.hello === "string"`, under a comment that says "We will test the hash logic once the TDD phase starts". That test was the browser half of the determinism guarantee; this plan makes it real.

## Key Decisions

- **Shrink the binary, don't raise the budget.** 501 bytes is small. Find the growth `624512d` added to `crates/simbiguity-render-wasm/src/lib.rs` (the `puzzle_*` exports and their `CString` handling, +48 lines) and the new `simbiguity-circuits-puzzle` dependency. Check `[profile.release]` in the root `Cargo.toml`: `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`. If those are already set, remove `core::fmt` usage from the new exports.
- **Hash in Node the same way Rust does.** The JS test loads the real `.wasm` with `fs.readFile` + `WebAssembly.instantiate`, because Node's `fetch` can't read `file:` URLs. It then runs the same four fixtures (`hello`, `wheatstone`, `rc`, `view3d`) and hashes the f64 result buffer with the same FNV-1a over little-endian bytes as `crates/simbiguity-circuits-wasm/src/golden.rs`. It compares against `crates/simbiguity-circuits-wasm/golden-hashes.json`. A match proves the browser produces bit-identical numbers to native Rust.

## Implementation

### 1. Get the site build under budget

**Files**:
- `Cargo.toml`
- `crates/simbiguity-render-wasm/src/lib.rs`

Bring `simbiguity_render_wasm.wasm` to at most 230,000 bytes, without editing `site/pkg-budget.txt`. Record the before and after sizes in the commit body.

### 2. Delete `patch.diff`

Run `git rm patch.diff`. Add nothing to `.gitignore`.

### 3. A determinism test that can fail

**Files**:
- `site/js/determinism.test.js`
- `site/js/simbiguity.js`

1. Give `loadCircuits` a way to accept bytes or a `WebAssembly.Module`, so Node can use it. Keep the URL path for browsers.
2. Rebuild each of the four golden fixtures through the JS API, exactly as `golden.rs` builds them.
3. Hash the results and `assert.strictEqual` each one against the JSON.
4. Add a second test that changes one resistor in the `hello` fixture by 1e-12 Ω and asserts its hash *differs*. This proves the hash is sensitive.

Red first: point the test at a deliberately wrong expected hash and see it fail.

## Reused existing code

- `fnv1a` / `hash_f64s` in `crates/simbiguity-circuits-wasm/src/golden.rs`: port these exactly.
- `xtask/src/golden_hashes_gen.rs`: the fixture definitions live here. Mirror them.
- `loadCircuits` in `site/js/simbiguity.js:43`.

## Reproduction Test

`node --test site/js/determinism.test.js` currently passes without checking any hash. After this plan, it must fail if `golden-hashes.json`'s `rc` value is edited by one digit.

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