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

//! A generic puzzle engine over any `simbiguity_core` simulation.

use simbiguity_core::{Simulation, Snapshot};

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

pub struct Level<G> {
    pub id: String,
    pub hint: String,
    pub goals: Vec<G>,
}

pub trait Goal<S: Simulation> {
    fn check(&self, state: &S::State) -> bool;
}

pub struct UndoStack<S: Snapshot> {
    history: Vec<S::Snapshot>,
    redo: Vec<S::Snapshot>,
}

impl<S: Snapshot> UndoStack<S> {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn push(&mut self, snapshot: S::Snapshot) {
        self.history.push(snapshot);
        self.redo.clear();
    }

    pub fn undo(&mut self, current: S::Snapshot) -> Option<S::Snapshot> {
        let prev = self.history.pop()?;
        self.redo.push(current);
        Some(prev)
    }

    pub fn redo(&mut self, current: S::Snapshot) -> Option<S::Snapshot> {
        let next = self.redo.pop()?;
        self.history.push(current);
        Some(next)
    }
}

impl<S: Snapshot> Default for UndoStack<S> {
    fn default() -> Self {
        Self::new()
    }
}

pub fn check<S: Simulation, G: Goal<S>>(level: &Level<G>, state: &S::State) -> bool {
    level.goals.iter().all(|g| g.check(state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use simbiguity_core::Seconds;

    struct MockSim;
    impl Simulation for MockSim {
        type State = bool;
        type Input = ();
        fn step(&mut self, _dt: Seconds) {}
        fn apply(&mut self, _input: ()) {}
        fn state(&self) -> &bool {
            &true
        }
    }
    impl Snapshot for MockSim {
        type Snapshot = usize;
        fn snapshot(&self) -> usize {
            1
        }
        fn restore(&mut self, _snapshot: &usize) {}
    }

    struct MockGoal(bool);
    impl Goal<MockSim> for MockGoal {
        fn check(&self, _state: &bool) -> bool {
            self.0
        }
    }

    /// Verifies that check matches all goals
    #[test]
    fn check_all_goals() {
        let l = Level {
            id: "1".into(),
            hint: "".into(),
            goals: vec![MockGoal(true), MockGoal(true)],
        };
        assert!(check(&l, &true));

        let l2 = Level {
            id: "1".into(),
            hint: "".into(),
            goals: vec![MockGoal(true), MockGoal(false)],
        };
        assert!(!check(&l2, &true));
    }

    /// Verifies undo redo behavior
    #[test]
    fn test_undo_redo() {
        let mut stack: UndoStack<MockSim> = UndoStack::new();
        stack.push(1);
        stack.push(2);

        assert_eq!(stack.undo(3), Some(2));
        assert_eq!(stack.undo(2), Some(1));
        assert_eq!(stack.undo(1), None);

        assert_eq!(stack.redo(1), Some(2));
        assert_eq!(stack.redo(2), Some(3));
        assert_eq!(stack.redo(3), None);
    }
}
