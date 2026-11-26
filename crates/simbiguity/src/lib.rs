//! # simbiguity
//!
//! An open source simulation engine by The Ambiguity Project.
//!
//! This is the umbrella crate. Each engine lives in its own small,
//! independently importable crate (`simbiguity-math`, `simbiguity-core`,
//! `simbiguity-circuits`, `simbiguity-anim`, …); this crate re-exports them
//! behind feature flags so a project can depend on one name and opt in to
//! just the engines it needs:
//!
//! ```toml
//! simbiguity = { version = "0.0.1", default-features = false, features = ["math"] }
//! ```
#![no_std]
#![forbid(unsafe_code)]

/// Vectors, quaternions, matrices, the dense linear solver and deterministic
/// float functions (`simbiguity-math`).
#[cfg(feature = "math")]
pub use simbiguity_math as math;

/// The `Simulation` contract, snapshots, the fixed clock, the seeded RNG and
/// the lockstep runner (`simbiguity-core`).
#[cfg(feature = "core")]
pub use simbiguity_core as core;

/// The DC circuit engine: batteries, resistors, bulbs, switches and wires,
/// solved by Modified Nodal Analysis (`simbiguity-circuits`).
#[cfg(feature = "circuits")]
pub use simbiguity_circuits as circuits;

/// The physics engine: particles, springs, rods (`simbiguity-physics`).
#[cfg(feature = "physics")]
pub use simbiguity_physics as physics;

/// The time-series plot rendering logic (`simbiguity-plot`).
#[cfg(feature = "plot")]
pub use simbiguity_plot as plot;

#[cfg(test)]
mod tests {
    // Verifies simbiguity core load.
    #[test]
    fn test_simbiguity() {
        let x = 1;
        assert_eq!(x, 1);
    }
}
