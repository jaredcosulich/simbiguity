#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
//! # simbiguity-circuits
//!
//! A DC circuit engine sized for teaching: batteries, resistors, light
//! bulbs, switches and wires. It answers the questions a learner asks —
//! *why is this bulb dim? what happens if I close this switch? why did that
//! short out?* — with exact numbers and typed, explainable diagnostics.
//!
//! ```
//! use simbiguity_circuits::Circuit;
//!
//! // A 6 V battery driving a 3 Ω resistor.
//! let mut c = Circuit::new();
//! let (plus, minus) = (c.node(), c.node());
//! let battery = c.add_battery(plus, minus, 6.0);
//! let load = c.add_resistor(plus, minus, 3.0);
//!
//! let analysis = c.solve();
//! let solution = analysis.solution().unwrap();
//! assert!((solution.current(load) - 2.0).abs() < 1e-12); // Ohm's law: I = V / R
//! assert!((solution.current(battery) - 2.0).abs() < 1e-12);
//! assert!(analysis.issues().is_empty());
//! ```
//!
//! ## How it solves: Modified Nodal Analysis
//!
//! Every junction is a **node** with an unknown voltage. Kirchhoff's current
//! law says the currents into each node sum to zero; Ohm's law turns each
//! resistor's current into `(V_a − V_b) / R`. Ideal batteries add one more
//! unknown each (their current) and one more equation (`V_+ − V_− = EMF`).
//! That is one linear system, solved by `simbiguity_math::lu_solve`. It is
//! the method SPICE uses, and it handles any topology — bridges, several
//! sources — not just series and parallel.
//!
//! Wires and closed switches are not modelled as tiny resistors: the nodes
//! they join are **merged** first, which keeps the system well-conditioned.
//!
//! ## Diagnostics, not panics
//!
//! [`Circuit::solve`] always returns an [`Analysis`]: a [`Solution`] when the
//! circuit has one, plus a list of [`CircuitIssue`]s (short circuits, floating
//! parts, loops of batteries, incomplete circuits, blown bulbs).
//!
//! `#![no_std]` with `alloc`; deterministic (no hashing, no platform floats).
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;
#[cfg(any(test, feature = "std"))]
extern crate std;

pub mod component;
pub mod diagnostics;
mod dsu;
mod mna;
pub mod netlist;
pub mod sim;
pub mod solution;
mod wires;

#[cfg(test)]
mod invariants_tests;

pub use component::{Component, Integrator};
pub use diagnostics::CircuitIssue;
pub use netlist::{Circuit, ComponentId, NodeId, Placed, SolverKind};
pub use sim::{CircuitInput, CircuitSim};
pub use solution::{Analysis, Solution};
