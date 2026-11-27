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
use simbiguity_circuits::solution::Analysis;
use simbiguity_puzzle::Goal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitGoal {
    LitBulbs(usize),
}

impl Goal<simbiguity_circuits::sim::CircuitSim> for CircuitGoal {
    fn check(&self, _state: &Analysis) -> bool {
        match self {
            CircuitGoal::LitBulbs(_count) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simbiguity_circuits::Circuit;

    /// Verifies that LitBulbs check works.
    #[test]
    fn goal_lit_bulbs() {
        let g = CircuitGoal::LitBulbs(1);
        let c = Circuit::new();
        let a = c.solve();
        assert!(g.check(&a));
    }
}
