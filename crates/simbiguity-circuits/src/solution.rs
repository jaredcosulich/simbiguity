//! The answer: every node's voltage and every component's current and power.

use alloc::vec::Vec;

use crate::diagnostics::CircuitIssue;
use crate::netlist::{ComponentId, NodeId};

/// The solved state of a circuit.
///
/// **Sign conventions.** For a battery, [`Solution::current`] is the current
/// it delivers out of its `+` terminal (positive while discharging) and
/// [`Solution::power`] the power it delivers to the rest of the circuit. For
/// every other component, current flows from terminal `a` to terminal `b`
/// and power is what the component absorbs.
#[derive(Debug, Clone, PartialEq)]
pub struct Solution {
    pub(crate) node_voltages: Vec<Option<f64>>,
    pub(crate) across: Vec<Option<f64>>,
    pub(crate) currents: Vec<f64>,
    pub(crate) powers: Vec<f64>,
    pub(crate) brightness: Vec<Option<f64>>,
}

impl Solution {
    /// Voltage of `node` relative to ground, or `None` if the node is in a
    /// floating piece of the circuit.
    pub fn node_voltage(&self, node: NodeId) -> Option<f64> {
        self.node_voltages[node.0]
    }

    /// Voltage of terminal `a` minus terminal `b` (a battery's terminal
    /// voltage), or `None` if the component is floating.
    pub fn voltage_across(&self, c: ComponentId) -> Option<f64> {
        self.across[c.0]
    }

    /// Current through the component, in amps (see the sign conventions).
    ///
    /// A loop made *only* of wires carries an undetermined circulating
    /// current; such a loop's share is reported as zero.
    pub fn current(&self, c: ComponentId) -> f64 {
        self.currents[c.0]
    }

    /// Power in watts: delivered by a battery, absorbed by anything else.
    pub fn power(&self, c: ComponentId) -> f64 {
        self.powers[c.0]
    }

    /// A bulb's brightness, `P_actual / P_rated` clamped to `0..=1`
    /// (`0` for a blown bulb). `None` if the component is not a bulb.
    pub fn brightness(&self, c: ComponentId) -> Option<f64> {
        self.brightness[c.0]
    }
}

/// Everything [`crate::Circuit::solve`] found: a solution when one exists,
/// and every issue worth telling the learner about.
#[derive(Debug, Clone, PartialEq)]
pub struct Analysis {
    pub(crate) solution: Option<Solution>,
    pub(crate) issues: Vec<CircuitIssue>,
}

impl Analysis {
    /// The solution, or `None` when a fatal issue (see
    /// [`CircuitIssue::is_fatal`]) prevents one.
    pub fn solution(&self) -> Option<&Solution> {
        self.solution.as_ref()
    }

    /// Issues found, fatal ones first, blown bulbs last.
    pub fn issues(&self) -> &[CircuitIssue] {
        &self.issues
    }

    /// Bulbs that are blown in this solution.
    pub fn blown_bulbs(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.issues.iter().filter_map(|i| match i {
            CircuitIssue::BulbBlown { bulb } => Some(*bulb),
            _ => None,
        })
    }
}
