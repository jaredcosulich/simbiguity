//! The contract every simbiguity engine implements.
//!
//! A circuit, a rigid-body world and a market all look the same from the
//! outside: something with a **state** that moves forward in time by
//! **steps**, and that reacts to **inputs** (flip a switch, push a box, place
//! an order). Agreeing on that shape is what lets engines compose — a
//! circuit can drive a motor that spins a rigid body, all stepped together
//! (see [`crate::Lockstep`]).
//!
//! **Determinism is part of the contract.** Given the same starting state,
//! the same inputs applied before the same steps, and the same `dt`s, an
//! implementation must produce a bit-identical state. That is what makes
//! puzzle checking exact, undo reliable and replays reproducible. In practice
//! it means: no wall-clock reads, no unseeded randomness (use [`crate::Rng`]),
//! no iteration over hash maps with random ordering, and float math from
//! `simbiguity_math::float` rather than the platform's.

use core::ops::{Add, AddAssign, Mul, Sub};

/// A duration in seconds. A newtype so a time step cannot be confused with
/// any other `f64` (a voltage, a mass, a price).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Seconds(pub f64);

impl Seconds {
    /// Zero duration.
    pub const ZERO: Seconds = Seconds(0.0);

    /// The duration as a plain `f64`.
    pub const fn get(self) -> f64 {
        self.0
    }

    /// The duration of one tick at `hz` ticks per second (e.g. `60.0` → 1/60 s).
    pub fn from_hz(hz: f64) -> Seconds {
        Seconds(1.0 / hz)
    }
}

impl Add for Seconds {
    type Output = Seconds;
    fn add(self, o: Seconds) -> Seconds {
        Seconds(self.0 + o.0)
    }
}

impl AddAssign for Seconds {
    fn add_assign(&mut self, o: Seconds) {
        self.0 += o.0;
    }
}

impl Sub for Seconds {
    type Output = Seconds;
    fn sub(self, o: Seconds) -> Seconds {
        Seconds(self.0 - o.0)
    }
}

impl Mul<f64> for Seconds {
    type Output = Seconds;
    fn mul(self, s: f64) -> Seconds {
        Seconds(self.0 * s)
    }
}

/// Something that evolves in discrete time steps and reacts to inputs.
pub trait Simulation {
    /// Everything an observer (a renderer, a test, a puzzle checker) can read.
    type State;
    /// A change requested from outside: a user action or another engine's output.
    type Input;

    /// Advances the simulation by `dt`. Engines normally receive the same
    /// fixed `dt` every call (see [`crate::FixedClock`]).
    fn step(&mut self, dt: Seconds);

    /// Applies an input. It takes effect from the next [`Simulation::step`].
    fn apply(&mut self, input: Self::Input);

    /// The current state.
    fn state(&self) -> &Self::State;
}

/// A simulation whose complete internal state can be saved and restored.
///
/// `restore(snapshot())` must put the simulation back *exactly* — including
/// any RNG state — so that stepping on from a restored snapshot gives
/// bit-identical results to never having left. Undo, rewind and "try it and
/// see" puzzle checks are all built on this.
pub trait Snapshot: Simulation {
    /// The saved form. Often just a clone of the internal state.
    type Snapshot: Clone;

    /// Captures the complete internal state.
    fn snapshot(&self) -> Self::Snapshot;

    /// Returns to a previously captured state.
    fn restore(&mut self, snapshot: &Self::Snapshot);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rng;

    /// A noisy counter: exercises step, apply, state and snapshots,
    /// including RNG state inside the snapshot.
    #[derive(Clone)]
    struct Noisy {
        value: f64,
        push: f64,
        rng: Rng,
    }

    impl Simulation for Noisy {
        type State = f64;
        type Input = f64;
        fn step(&mut self, dt: Seconds) {
            self.value += (self.push + self.rng.normal()) * dt.get();
        }
        fn apply(&mut self, input: f64) {
            self.push = input;
        }
        fn state(&self) -> &f64 {
            &self.value
        }
    }

    impl Snapshot for Noisy {
        type Snapshot = Noisy;
        fn snapshot(&self) -> Noisy {
            self.clone()
        }
        fn restore(&mut self, s: &Noisy) {
            *self = s.clone();
        }
    }

    /// Seconds arithmetic and from_hz behave like plain f64 durations.
    #[test]
    fn seconds_arithmetic() {
        assert_eq!(Seconds(1.5) + Seconds(0.5), Seconds(2.0));
        assert_eq!(Seconds(1.5) - Seconds(0.5), Seconds(1.0));
        assert_eq!(Seconds(2.0) * 3.0, Seconds(6.0));
        let mut s = Seconds::ZERO;
        s += Seconds(0.25);
        assert_eq!(s.get(), 0.25);
        assert_eq!(Seconds::from_hz(4.0), Seconds(0.25));
        assert!(Seconds(1.0) < Seconds(2.0));
    }

    /// Restoring a snapshot and replaying the same inputs reproduces the
    /// exact same state bit for bit, RNG included.
    #[test]
    fn restore_then_replay_is_bit_identical() {
        let mut sim = Noisy {
            value: 0.0,
            push: 0.0,
            rng: Rng::new(9),
        };
        sim.apply(2.0);
        let saved = sim.snapshot();
        for _ in 0..50 {
            sim.step(Seconds(0.01));
        }
        let first = *sim.state();
        sim.restore(&saved);
        for _ in 0..50 {
            sim.step(Seconds(0.01));
        }
        assert_eq!(sim.state().to_bits(), first.to_bits());
        assert_ne!(first, 0.0);
    }
}
