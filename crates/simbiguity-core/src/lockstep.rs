//! Stepping several simulations together on one clock.
//!
//! Composition — "a circuit drives a motor that spins a rigid body" — only
//! works if the engines agree on time. [`Lockstep`] owns one [`FixedClock`]
//! and, for each fixed step it yields, steps every simulation once with the
//! same `dt`, in a fixed order. No engine ever runs ahead of another.

use crate::clock::{FixedClock, StepCount};
use crate::simulation::{Seconds, Simulation};

/// The object-safe slice of [`Simulation`]: just "advance by `dt`".
///
/// [`Simulation`] has associated types, so different engines cannot share
/// one `&mut dyn Simulation` list. Every `Simulation` is automatically
/// `Steppable`, which can.
pub trait Steppable {
    /// Advances by one fixed step of length `dt`.
    fn tick(&mut self, dt: Seconds);
}

impl<S: Simulation> Steppable for S {
    fn tick(&mut self, dt: Seconds) {
        self.step(dt);
    }
}

/// Runs simulations in lockstep on a shared [`FixedClock`].
#[derive(Debug, Clone, PartialEq)]
pub struct Lockstep {
    clock: FixedClock,
}

impl Lockstep {
    /// A runner driven by `clock`.
    pub fn new(clock: FixedClock) -> Lockstep {
        Lockstep { clock }
    }

    /// The shared clock.
    pub fn clock(&self) -> &FixedClock {
        &self.clock
    }

    /// Banks `real_dt` and steps every simulation in `sims`, in order, once
    /// per fixed step owed.
    pub fn advance(&mut self, real_dt: Seconds, sims: &mut [&mut dyn Steppable]) -> StepCount {
        self.advance_with(real_dt, |dt| {
            for sim in sims.iter_mut() {
                sim.tick(dt);
            }
        })
    }

    /// Banks `real_dt` and calls `on_step(dt)` once per fixed step owed.
    ///
    /// Use this when engines are coupled: inside the closure, read one
    /// engine's state, `apply` it as another's input, then step both — the
    /// coupling then happens at the same moment of simulated time every run.
    pub fn advance_with(
        &mut self,
        real_dt: Seconds,
        mut on_step: impl FnMut(Seconds),
    ) -> StepCount {
        let count = self.clock.advance(real_dt);
        let dt = self.clock.step();
        for _ in 0..count.steps {
            on_step(dt);
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    /// Counts steps and total time received.
    struct Counter {
        steps: u32,
        time: f64,
    }

    impl Simulation for Counter {
        type State = u32;
        type Input = ();
        fn step(&mut self, dt: Seconds) {
            self.steps += 1;
            self.time += dt.get();
        }
        fn apply(&mut self, _: ()) {}
        fn state(&self) -> &u32 {
            &self.steps
        }
    }

    /// Every simulation, whatever its types, is stepped the same number of
    /// times with the clock's fixed dt.
    #[test]
    fn advance_steps_every_sim_equally() {
        let mut ls = Lockstep::new(FixedClock::new(Seconds(0.010)));
        let mut a = Counter {
            steps: 0,
            time: 0.0,
        };
        let mut b = Counter {
            steps: 0,
            time: 0.0,
        };
        let c = ls.advance(Seconds(0.035), &mut [&mut a, &mut b]);
        assert_eq!(c.steps, 3);
        assert_eq!((a.steps, b.steps), (3, 3));
        assert!((a.time - 0.03).abs() < 1e-12);
        assert_eq!(ls.clock().total_steps(), 3);
    }

    /// Across uneven frames (16, 17, 50, 3 ms) two simulations stay in
    /// lockstep and receive exactly the steps the banked time allows.
    #[test]
    fn sims_stay_in_step_across_uneven_frames() {
        let mut ls = Lockstep::new(FixedClock::new(Seconds(0.010)));
        let mut a = Counter {
            steps: 0,
            time: 0.0,
        };
        let mut b = Counter {
            steps: 0,
            time: 0.0,
        };
        for ms in [16.0, 17.0, 50.0, 3.0] {
            ls.advance(Seconds(ms / 1000.0), &mut [&mut a, &mut b]);
            assert_eq!(a.steps, b.steps);
        }
        assert_eq!(a.steps, 8);
        assert_eq!(a.time.to_bits(), b.time.to_bits());
    }

    /// advance_with calls the closure once per owed step with the fixed dt.
    #[test]
    fn advance_with_calls_closure_per_step() {
        let mut ls = Lockstep::new(FixedClock::new(Seconds(0.5)));
        let mut seen = Vec::new();
        let c = ls.advance_with(Seconds(1.2), |dt| seen.push(dt));
        assert_eq!(c.steps, 2);
        assert_eq!(seen, [Seconds(0.5), Seconds(0.5)]);
        assert_eq!(
            ls.advance_with(Seconds(0.1), |_| panic!("no step owed"))
                .steps,
            0
        );
    }

    /// The blanket Steppable impl forwards tick to Simulation::step.
    #[test]
    fn steppable_tick_forwards_to_step() {
        let mut a = Counter {
            steps: 0,
            time: 0.0,
        };
        Steppable::tick(&mut a, Seconds(1.0));
        assert_eq!(*a.state(), 1);
    }
}
