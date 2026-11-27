//! What can go wrong in a circuit, in terms a learner can be told.

use core::fmt;

use crate::netlist::{ComponentId, NodeId};

/// A problem found while solving. Each one names the part to point at, so a
/// UI can highlight it and explain it.
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitIssue {
    /// The MNA solver encountered an internal problem (non-square matrix or bug). Fatal.
    SolverFailure {
        /// A developer-readable message for the internal error.
        message: alloc::string::String,
    },
    /// A component has a value that makes no physical sense (a resistance
    /// of zero or less, a bulb rated at 0 W, a NaN). Fatal: nothing is solved.
    InvalidValue {
        /// The offending component.
        component: ComponentId,
    },
    /// A battery's two terminals are joined by wire alone. An ideal battery
    /// would push infinite current (`current: None`, fatal); a real one
    /// pushes `emf / internal_r` and drains fast (`current: Some(amps)`).
    ShortCircuit {
        /// The shorted battery.
        battery: ComponentId,
        /// The short-circuit current, or `None` when it would be infinite.
        current: Option<f64>,
    },
    /// Ideal batteries (and wires) form a closed loop, so the loop's
    /// voltages contradict each other or its current is undetermined.
    /// Fatal. `battery` is the battery that closed the loop.
    VoltageSourceLoop {
        /// The battery that closed the loop.
        battery: ComponentId,
    },
    /// A piece of the circuit has no battery and no ground, so its voltages
    /// are not defined. `node` is the lowest-numbered node in that piece.
    FloatingNode {
        /// A node in the floating piece.
        node: NodeId,
    },
    /// A battery has no complete loop to drive current around, so nothing
    /// flows from it (an open switch, a missing wire, a blown bulb).
    OpenCircuit {
        /// The battery with no current.
        battery: ComponentId,
    },
    /// A bulb was pushed past its limit and its filament broke. It now
    /// conducts nothing until it is replaced.
    BulbBlown {
        /// The blown bulb.
        bulb: ComponentId,
    },
    /// A capacitor is connected directly across an ideal battery. The battery
    /// tries to charge it instantly, causing infinite inrush current.
    Inrush {
        /// The capacitor.
        capacitor: ComponentId,
    },
    /// The simulation was asked to take a time step that is zero, negative,
    /// or not a real number. Time must flow forward.
    InvalidTimeStep,
}

impl CircuitIssue {
    /// `true` when the issue prevents any solution being reported.
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            CircuitIssue::SolverFailure { .. }
                | CircuitIssue::InvalidValue { .. }
                | CircuitIssue::ShortCircuit { current: None, .. }
                | CircuitIssue::VoltageSourceLoop { .. }
                | CircuitIssue::Inrush { .. }
                | CircuitIssue::InvalidTimeStep
        )
    }
}

impl fmt::Display for CircuitIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitIssue::SolverFailure { message } => {
                write!(f, "internal solver error: {message}")
            }
            CircuitIssue::InvalidValue { component } => write!(
                f,
                "component {} has a value that cannot exist (it must be positive and finite)",
                component.index()
            ),
            CircuitIssue::ShortCircuit {
                battery,
                current: None,
            } => write!(
                f,
                "battery {} is short-circuited: a wire joins its terminals with no resistance, \
                 so the current would be unlimited",
                battery.index()
            ),
            CircuitIssue::ShortCircuit {
                battery,
                current: Some(amps),
            } => write!(
                f,
                "battery {} is short-circuited: only its internal resistance limits the current \
                 ({amps} A)",
                battery.index()
            ),
            CircuitIssue::VoltageSourceLoop { battery } => write!(
                f,
                "battery {} closes a loop made only of batteries and wires, so the loop's \
                 voltages cannot all be true at once",
                battery.index()
            ),
            CircuitIssue::FloatingNode { node } => write!(
                f,
                "node {} is in a part of the circuit with no battery, so nothing sets its voltage",
                node.index()
            ),
            CircuitIssue::OpenCircuit { battery } => write!(
                f,
                "battery {} has no complete loop, so no current flows",
                battery.index()
            ),
            CircuitIssue::BulbBlown { bulb } => write!(
                f,
                "bulb {} blew: it drew more than {}× its rated power",
                bulb.index(),
                crate::component::BLOW_RATIO
            ),
            CircuitIssue::Inrush { capacitor } => write!(
                f,
                "capacitor {} is connected directly across an ideal battery, which would cause an infinite inrush current",
                capacitor.index()
            ),
            CircuitIssue::InvalidTimeStep => write!(
                f,
                "the time step must be a positive number"
            ),
        }
    }
}
