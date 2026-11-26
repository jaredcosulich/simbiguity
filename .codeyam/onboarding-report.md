# CodeYam Onboarding Report

**Project:** simbiguity — An open source simulation engine by The Ambiguity Project
**Branch taken:** greenfield
**Started:** 2026-11-26T12:05:00Z
**Completed:** 2026-11-26T12:30:00Z

## Translation decisions
None — greenfield (`migration-status` → `schema: none`, `legacy_artifacts: []`).

## Archived to .codeyam/_legacy/
Nothing.

## Skeleton created before init
- `Cargo.toml` — virtual workspace (`members = []`), shared `workspace.package`
  (edition 2021, `MIT OR Apache-2.0`, repo URL), release profile tuned for small WASM
  (`opt-level = "s"`, LTO, 1 codegen unit).
- `LICENSE-MIT`, `LICENSE-APACHE` (dual license, Rust convention).
- `README.md` — tagline, planned crate table, design principles.
- `site/index.html` — placeholder landing page for the GitHub Pages demo site.
- `.gitignore` — `/target`, `site/pkg/` (built WASM output), plus the codeyam-managed block.

## editor.json — what you populated
- `provider: claude` (via `install-hooks --provider claude`).
- `projectName` / `projectDescription` — simbiguity + one-paragraph description.
- `apps[]` — one entry: `site` (`dir: site`, `type: web`, `framework: plain-html`).
- Ports: the static site's dev server listens on **4781**
  (`python3 -m http.server 4781 --bind 127.0.0.1`, run inside `site/`); the editor's
  preview is served on **4780**. 8080 was avoided — another local editor was already on it.
- `stack.json` — seeded by `stack-from-apps` from the `plain-html` body
  (`stack-support --framework html` → **fallback**: visual preview + scenario capture work);
  `serverRoot: site`, dev command and default port aligned to the above.
- `testRunners[]` — `cargo` (`cargo test --workspace --no-fail-fast`, with changed /
  targeted / `cargo llvm-cov` coverage variants, `changedPartitionRoot: crates/*`).
- `staticChecks[]` — `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`.
- `formatters[]` — `cargo fmt --all`.
- Partitions (from `configure-test-commands`):
  ```
  partitions: 1 partition(s) for runner 'cargo', one per workspace unit
    simbiguity  -p simbiguity  dependsOn: -
  ```
  Re-run it as each new crate lands so every crate gets its own partition.

An empty virtual workspace fails both `cargo fmt --check` and `cargo clippy`
("no members"), which blocked the first commit, so the umbrella crate
`crates/simbiguity` (docs only, `#![no_std]`, no code) was added. It will
re-export the engine crates behind feature flags.

## Code index
- `dependency-graph/` — 1 site file analyzed (`site/index.html`).
- `glossary.json` — 0 entries auto-added, 0 skipped (no source entities yet).
- `test-registry.json` — not created: `bootstrap-registry` needs at least one crate with
  tests (see Pending).

## Journal migration
- `migrate-journal` result: ok
- Pre-migration shape: modern (empty)
- Entries migrated: 0 · Archive shards: 0

## Test partition cost
```
all              0          ?          ?       0           ?
no partition dominates
```
Empty, as expected before any crate exists.

## Scenarios registered
| slug | type | URL | captured |
|---|---|---|---|
| `landing-coming-soon` | application | `/` | yes (Desktop) |

## Health checks
`codeyam-editor editor health-status --format json` → **`passed: true`** (all static,
smoke and functional checks green).

## Pending / known gaps
- **Test registry.** Run `codeyam-editor editor bootstrap-registry` once the first crate
  (`simbiguity-math` / `simbiguity-core`) has tests; it currently fails with
  "no crates to be measured for coverage".
- **Partitions.** Re-run `codeyam-editor editor configure-test-commands` after crates are
  added so each crate gets its own test partition.
- **WASM toolchain.** `rustup target add wasm32-unknown-unknown` is needed before the
  first demo build (no `wasm-bindgen` / `wasm-pack` — the interface is hand-written).
- **Dev server dependency.** The static site is served with `python3 -m http.server`
  (dev-time only, never shipped). Swap for a tiny Rust static server later if desired.
- **Codespaces (optional).** `codeyam-editor editor devcontainer-setup` makes the repo open
  in GitHub Codespaces with the editor running. Reversible; not applied.
- **Notifications (optional).** `codeyam-editor editor notify-setup --topic <your-topic>`.
