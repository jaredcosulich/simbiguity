#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
pub trait Simulation {
    type State;
    type Input;
}

pub trait Snapshot {
    type Data;
}

pub trait Goal<S: Simulation> {
    fn check(&self, state: &S::State) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSim;
    impl Simulation for MockSim {
        type State = bool;
        type Input = ();
    }

    struct MockGoal;
    impl Goal<MockSim> for MockGoal {
        fn check(&self, state: &bool) -> bool {
            *state
        }
    }

    /// Verifies that Goal correctly returns true/false based on state.
    #[test]
    fn test_goal() {
        let goal = MockGoal;
        assert!(goal.check(&true));
        assert!(!goal.check(&false));
    }
}
