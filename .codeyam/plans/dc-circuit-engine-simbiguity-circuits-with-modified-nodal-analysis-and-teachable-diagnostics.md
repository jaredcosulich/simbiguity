---
title: "DC circuit engine: simbiguity-circuits with Modified Nodal Analysis and teachable diagnostics"
mode: backend
createdAt: "2026-11-26T12:42:15Z"
source: manual
dependsOn: ["foundations-simbiguity-math-and-simbiguity-core-zero-dependency-math-simulation-contract-fixed-clock-seeded-rng"]
---

## Summary

Build **`simbiguity-circuits`**, a from-scratch DC circuit engine using Modified Nodal Analysis (MNA), sized and designed for education: batteries, resistors, light bulbs, switches and wires. It answers the questions a learner asks ("why is this bulb dim?", "what if I close this switch?", "why did that short out?") with exact numbers and clear, typed diagnostics. It implements the `simbiguity-core` `Simulation` contract, so it composes with other engines and with the 3D puzzle layer later. Zero dependencies beyond `simbiguity-math` and `simbiguity-core`.

## Key Decisions

- **MNA over hand-rolled series/parallel reduction.** MNA handles any topology (bridges, multiple sources), and it is the method real simulators (SPICE) use. That is worth teaching. Node voltages and source currents come from one `lu_solve`.
- **Components are data, not trait objects, for now.** `enum Component { Battery { emf, internal_r }, Resistor { r }, Bulb { rated_power, rated_voltage }, Switch { closed }, Wire }`, connecting two `NodeId`s. A bulb is a resistor whose resistance comes from its rating (R = V²/P). Its brightness is `P_actual / P_rated`, clamped, and it "blows" above a threshold. Wires are near-zero resistances, handled by node merging (union-find) rather than tiny resistors, which keeps the matrix well-conditioned.
- **Diagnostics are first-class output, not panics.** These come back as typed results the UI can explain: a short circuit (an ideal source across ~0 Ω), a floating node (no path to ground → singular matrix, the node identified), a voltage-source loop, an open circuit (no current), and a blown bulb.
- **Ground is implicit.** Ground is the node on the negative terminal of the first battery unless the builder sets one, so learners never have to think about it.
- **Transient (capacitors, inductors) and AC are later plans.** The `Simulation::step(dt)` is a re-solve today, but the struct leaves room for companion models without breaking the API.
- **Deterministic and snapshot-able.** The circuit state is its component list plus switch states, so `Snapshot` is trivial, and puzzle undo comes for free.

## Implementation

### 1. Netlist and builder

**New file**: `crates/simbiguity-circuits/Cargo.toml`
**New file**: `crates/simbiguity-circuits/src/lib.rs`
**New file**: `crates/simbiguity-circuits/src/netlist.rs`: `Circuit`, `NodeId`, `ComponentId`, a builder (`add_battery(a, b, emf)`, `add_bulb`, `add_switch`, `connect`)
**New file**: `crates/simbiguity-circuits/src/component.rs`: the `Component` enum and per-component stamping into the MNA matrix

### 2. Solver

**New file**: `crates/simbiguity-circuits/src/mna.rs`: merges wires via union-find, assembles G/B/C/D blocks into one dense system, calls `simbiguity_math::lu_solve`, and maps the results back
**New file**: `crates/simbiguity-circuits/src/solution.rs`: `Solution { node_voltage(n), current(c), power(c), brightness(bulb) }`
**New file**: `crates/simbiguity-circuits/src/diagnostics.rs`: `CircuitIssue` enum + detection (short, floating node, source loop, blown bulb, open)

### 3. Simulation integration

**New file**: `crates/simbiguity-circuits/src/sim.rs`: `impl Simulation for CircuitSim` (Input = `ToggleSwitch(id)` / `SetValue(id, v)`; State = latest `Solution` + issues), `impl Snapshot`

### 4. Umbrella + codeyam

**File**: `crates/simbiguity/src/lib.rs`

Re-export behind a `circuits` feature. Re-run `configure-test-commands` for the new partition.

## Reused existing code

- `lu_solve` / `SolveError::Singular` from `simbiguity-math` (plan: foundations)
- `Simulation` / `Snapshot` / `Seconds` from `simbiguity-core` (plan: foundations)
- No circuit code exists yet.

## Tests (TDD — written first)

Each test is a textbook circuit with a hand-computed answer, so the tests double as worked examples on the public test page:
- A single battery + resistor: Ohm's law, I = V/R.
- Two resistors in series and in parallel: the voltage divider and current divider.
- A battery with internal resistance: the terminal voltage drops under load.
- A Wheatstone bridge (balanced → zero bridge current; unbalanced → known value): proves MNA beyond series/parallel.
- Two batteries in parallel with different EMFs: the circulating current.
- A bulb at its rated voltage → brightness 1.0; at half voltage → 0.25 (P ∝ V²).
- A switch open → no current and bulb brightness 0; toggled closed → lit (through `Simulation::apply`).
- A wire straight across the battery → `ShortCircuit` issue naming the battery.
- A component on its own island → `FloatingNode` naming the node.
- Overvoltage → bulb `Blown`.
- Snapshot/restore round-trip across a switch toggle.
- Determinism: the same circuit solved twice gives bit-identical results.

## Scenarios to Demonstrate

- Library-only in this plan. The hello-circuit page (next plan) is the first visual demo of this engine.