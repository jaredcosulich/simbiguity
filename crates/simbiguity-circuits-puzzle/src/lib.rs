#![no_std]
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]

pub mod level_format;
pub mod share;

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use simbiguity_circuits::sim::CircuitSim;
use simbiguity_circuits::{Circuit, CircuitIssue, Component, ComponentId};
use simbiguity_core::Simulation;
use simbiguity_puzzle::Goal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueKind {
    SolverFailure,
    InvalidValue,
    ShortCircuit,
    VoltageSourceLoop,
    FloatingNode,
    OpenCircuit,
    BulbBlown,
    Inrush,
    InvalidTimeStep,
}

impl IssueKind {
    pub fn matches(&self, issue: &CircuitIssue) -> bool {
        matches!(
            (self, issue),
            (IssueKind::SolverFailure, CircuitIssue::SolverFailure { .. })
                | (IssueKind::InvalidValue, CircuitIssue::InvalidValue { .. })
                | (IssueKind::ShortCircuit, CircuitIssue::ShortCircuit { .. })
                | (
                    IssueKind::VoltageSourceLoop,
                    CircuitIssue::VoltageSourceLoop { .. }
                )
                | (IssueKind::FloatingNode, CircuitIssue::FloatingNode { .. })
                | (IssueKind::OpenCircuit, CircuitIssue::OpenCircuit { .. })
                | (IssueKind::BulbBlown, CircuitIssue::BulbBlown { .. })
                | (IssueKind::Inrush, CircuitIssue::Inrush { .. })
                | (IssueKind::InvalidTimeStep, CircuitIssue::InvalidTimeStep)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartKind {
    Battery,
    Resistor,
    Bulb,
    Switch,
    Capacitor,
    Inductor,
    Wire,
}

impl PartKind {
    pub fn matches_component(&self, comp: &Component) -> bool {
        matches!(
            (self, comp),
            (PartKind::Battery, Component::Battery { .. })
                | (PartKind::Resistor, Component::Resistor { .. })
                | (PartKind::Bulb, Component::Bulb { .. })
                | (PartKind::Switch, Component::Switch { .. })
                | (PartKind::Capacitor, Component::Capacitor { .. })
                | (PartKind::Inductor, Component::Inductor { .. })
                | (PartKind::Wire, Component::Wire)
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitGoal {
    LitBulbs(usize),
    BulbBrightness {
        bulb: ComponentId,
        min: f64,
        max: f64,
    },
    NoIssue(IssueKind),
    PartBudget {
        kind: PartKind,
        max: usize,
    },
    CurrentThrough {
        component: ComponentId,
        amps: f64,
        tolerance: f64,
    },
    All(Vec<CircuitGoal>),
    Any(Vec<CircuitGoal>),
    Not(Box<CircuitGoal>),
}

pub fn calculate_brightness(watts: f64, rated_power: f64) -> f64 {
    watts.abs() / rated_power
}

pub fn count_parts_by_kind(circuit: &Circuit, kind: PartKind) -> usize {
    circuit
        .parts()
        .iter()
        .filter(|p| kind.matches_component(&p.component))
        .count()
}

pub fn check_current_tolerance(current: f64, amps: f64, tolerance: f64) -> bool {
    (current.abs() - amps.abs()).abs() <= tolerance
}

impl Goal<CircuitSim> for CircuitGoal {
    fn check(&self, sim: &CircuitSim) -> bool {
        let analysis = sim.state();
        let circuit = sim.circuit();
        match self {
            CircuitGoal::LitBulbs(_) => false,
            CircuitGoal::BulbBrightness { bulb, min, max } => {
                if let Some(sol) = analysis.solution() {
                    let watts = sol.power(*bulb);
                    if let Component::Bulb { rated_power, .. } = circuit.component(*bulb) {
                        let brightness = calculate_brightness(watts, *rated_power);
                        return brightness >= *min && brightness <= *max;
                    }
                }
                false
            }
            CircuitGoal::NoIssue(kind) => !analysis.issues().iter().any(|i| kind.matches(i)),
            CircuitGoal::PartBudget { kind, max } => count_parts_by_kind(circuit, *kind) <= *max,
            CircuitGoal::CurrentThrough {
                component,
                amps,
                tolerance,
            } => {
                if let Some(sol) = analysis.solution() {
                    check_current_tolerance(sol.current(*component), *amps, *tolerance)
                } else {
                    false
                }
            }
            CircuitGoal::All(goals) => goals.iter().all(|g| g.check(sim)),
            CircuitGoal::Any(goals) => goals.iter().any(|g| g.check(sim)),
            CircuitGoal::Not(goal) => !goal.check(sim),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simbiguity_circuits::Circuit;

    /// A goal must be able to say "no". A bulb across an open switch carries no
    /// current and stays dark, so "light this bulb" has to fail. A goal that
    /// always passes would let every learner "solve" every level.
    #[test]
    fn lit_bulb_goal_fails_when_the_bulb_is_dark() {
        let mut c = Circuit::new();
        let (p, mid, m) = (c.node(), c.node(), c.node());
        c.add_battery(p, m, 6.0);
        c.add_switch(p, mid, false);
        let bulb = c.add_bulb(mid, m, 6.0, 3.0);
        let sim = CircuitSim::new(c);
        let goal = CircuitGoal::BulbBrightness {
            bulb,
            min: 0.5,
            max: 1.0,
        };
        assert!(!goal.check(&sim));
    }

    /// Verifies calculation of brightness from power
    #[test]
    fn test_calculate_brightness() {
        assert_eq!(calculate_brightness(10.0, 5.0), 2.0);
        assert_eq!(calculate_brightness(-10.0, 5.0), 2.0);
    }

    /// Verifies part counting
    #[test]
    fn test_count_parts_by_kind() {
        let mut c = Circuit::new();
        let p = c.node();
        let m = c.node();
        c.add_battery(p, m, 6.0);
        c.add_switch(p, m, false);
        assert_eq!(count_parts_by_kind(&c, PartKind::Battery), 1);
        assert_eq!(count_parts_by_kind(&c, PartKind::Bulb), 0);
    }

    /// Verifies tolerance checking for currents
    #[test]
    fn test_check_current_tolerance() {
        assert!(check_current_tolerance(1.0, 1.0, 0.1));
        assert!(check_current_tolerance(-1.0, 1.0, 0.1));
        assert!(check_current_tolerance(1.05, 1.0, 0.1));
        assert!(!check_current_tolerance(1.2, 1.0, 0.1));
    }
}
