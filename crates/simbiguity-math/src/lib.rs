//! # simbiguity-math
//!
//! The zero-dependency math layer every simbiguity engine builds on:
//!
//! - [`Vec2`] / [`Vec3`] — positions, velocities, forces.
//! - [`Quat`] — 3D rotations without gimbal lock, with smooth interpolation.
//! - [`Mat4`] — 4×4 transforms for cameras and renderers (column-major, WebGL layout).
//! - [`DenseMatrix`] + [`lu_solve`] — runtime-sized linear systems, solved by
//!   LU decomposition with partial pivoting.
//! - [`float`] — `sqrt`, `sin`, `exp`, … computed in pure Rust so results are
//!   bit-identical on every platform, including WASM.
//!
//! Everything is `f64`: simulation accuracy matters more than GPU size at
//! this layer, and renderers convert to `f32` at their own boundary.
//!
//! `#![no_std]` with `alloc`; enable the `std` feature for
//! `std::error::Error` impls.
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;
#[cfg(any(test, feature = "std"))]
extern crate std;

pub mod float;
pub mod linsolve;
pub mod mat;
pub mod quat;
pub mod vec;

pub use linsolve::{lu_solve, Lu, SolveError};
pub use mat::{DenseMatrix, Mat4};
pub use quat::Quat;
pub use vec::{Vec2, Vec3};
