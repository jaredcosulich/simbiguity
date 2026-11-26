#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
//! # simbiguity-core
//!
//! The shared contracts that let simbiguity engines compose:
//!
//! - [`Simulation`] — state, `step(dt)`, `apply(input)`; deterministic by contract.
//! - [`Snapshot`] — exact save/restore, the basis of undo, rewind and replay.
//! - [`FixedClock`] — turns uneven frame times into whole fixed steps.
//! - [`Rng`] — a seeded PCG32; there is no global randomness anywhere.
//! - [`Lockstep`] — steps several simulations together on one clock.
//!
//! `#![no_std]`, zero external dependencies (only `simbiguity-math`).
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[cfg(test)]
extern crate std;

pub mod clock;
pub mod lockstep;
pub mod rng;
pub mod simulation;

pub use clock::{FixedClock, StepCount};
pub use lockstep::{Lockstep, Steppable};
pub use rng::Rng;
pub use simulation::{Seconds, Simulation, Snapshot};
