#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod collide;
pub mod constraints;
pub mod energy;
pub mod error;
pub mod forces;
pub mod integrate;
pub mod world;

pub use constraints::{Ground, Rod};
pub use energy::Energy;
pub use error::PhysicsError;
pub use forces::{Gravity, Spring};
pub use integrate::Integrator;
pub use world::{Particle, PhysicsInput, World};

#[cfg(test)]
mod collide_tests;
#[cfg(test)]
mod gravity_tests;
#[cfg(test)]
mod oscillator_tests;
#[cfg(test)]
mod world_tests;
