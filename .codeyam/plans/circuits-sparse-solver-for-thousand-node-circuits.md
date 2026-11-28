---
title: "Circuits: sparse solver for thousand-node circuits"
mode: backend
createdAt: "2026-11-28T02:43:18Z"
source: manual
dependsOn: ["circuits-ac-signal-sources-oscilloscope-probes-and-rlc-resonance"]
---

## Summary

Every solve uses the dense LU in `crates/simbiguity-math/src/linsolve.rs` (`Lu::decompose` at line 87). That is O(n³) time and O(n²) memory. It's fine for a ten-part puzzle, but it rules out the big circuits that make a simulation impressive: a 1000-rung resistor ladder, a 32×32 resistor mesh, or a long transmission-line LC chain.

MNA matrices are very sparse: each row has about the node's degree plus one non-zero entries. This plan adds a sparse LU with fill-reducing ordering, and has MNA choose it automatically above a size threshold.

## Key Decisions

- **Sparse format and ordering.** Use compressed sparse columns with an approximate-minimum-degree ordering (a simple minimum-degree ordering is fine), then left-looking LU with partial pivoting and a threshold of 0.1.
  - Everything is written in-house. The workspace has zero external dependencies, and that must stay true.
- **Factor the pattern once, refactor numbers fast.** Splitting symbolic and numeric factorisation means each transient step, and each Newton iteration, only redoes the numbers. This plugs into the existing LU cache and dirty flag in `crates/simbiguity-circuits/src/mna.rs`.
- **Automatic choice, with an override.**
  - Dense below 64 unknowns, sparse at or above.
  - `Circuit::set_solver(SolverKind::Dense | Sparse | Auto)` overrides it, so tests can compare both.
  - The 64 threshold is a starting point. Measure the crossover with `cargo xtask bench` and put the measured value in the commit body.
- **No accuracy regression.** On the same circuit, sparse and dense agree to a relative difference of 1e-10 per node. The golden hashes may change only if `Auto` picks sparse for one of the four golden fixtures, and none of them has 64 unknowns, so they must stay identical.

## Implementation

### 1. Sparse matrix and LU

**New file**: `crates/simbiguity-math/src/sparse.rs`

Contents:
- `SparseMatrix` (built from triplets; duplicates summed)
- `SymbolicLu`
- `SparseLu::factor` / `refactor` / `solve`
- a singularity error matching `SolveError`

Unit tests:
- 200 seeded random diagonally dominant matrices of size 5–300, solved by both the dense and sparse LU, agree to 1e-10 relative;
- a singular matrix returns an error;
- `refactor` with new values gives the same answer as a fresh `factor`.

### 2. MNA builds sparse when it should

**File**: `crates/simbiguity-circuits/src/mna.rs`

Assemble triplets instead of a dense matrix, then choose the solver as described above.

### 3. Reference tests at scale

**File**: `crates/simbiguity-circuits/tests/large.rs` (new)

**Ladder.** A 1000-rung ladder of 1 Ω series and 1 Ω shunt resistors, driven by 1 V.
- The input resistance tends to the golden ratio φ = 1.6180340 Ω.
- So the source current is 0.61803399 A ± 1e-8.

**Mesh.** A 32×32 mesh of 1 Ω resistors with 1 V across opposite corners.
- KCL holds at every node to 1e-9.
- The equivalent resistance is symmetric: swapping the corners gives the same current to 1e-12.

**LC chain.** A 200-section LC chain (L = 1 µH, C = 1 nF) with a step input, trapezoidal.
- The wavefront's time of arrival at section k is k·√(LC) ± 10%.
- In other words, the delay is 31.6 ns per section.

**Agreement.** Run the 2000-seed `generate_circuit` set with `SolverKind::Dense` and with `Sparse`. Results agree to 1e-10 relative.

### 4. Bench and budget

**Files**:
- `xtask/src/bench_fixtures.rs`
- bench budget file

Add `ladder_1000_dc` and `mesh_32_transient_100_steps`, and record both in the bench output.

Requirements:
- On `ladder_1000_dc`, sparse must be at least 20× faster than dense.
- Add a budget entry for each new fixture at the measured time × 1.5.
- `scripts/check-budgets.sh` enforces it.

### 5. Capabilities

Add "Large circuits (sparse solver)" to the circuits `capabilities.toml`, proven by the ladder and mesh tests.

## Reused existing code

- `DenseMatrix`, `Lu` and `SolveError` from `crates/simbiguity-math/src/linsolve.rs`
- the LU cache and dirty flag in `crates/simbiguity-circuits/src/mna.rs`
- `generate_circuit` in `crates/simbiguity-circuits/src/invariants_tests.rs`
- `xtask/src/bench.rs` and `scripts/check-budgets.sh`

## Reproduction Test

`thousand_rung_ladder_draws_one_over_phi_amps` should be written against `SolverKind::Sparse`. It fails to compile today.

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