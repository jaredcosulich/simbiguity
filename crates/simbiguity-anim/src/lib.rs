//! # simbiguity-anim
//!
//! Animation as plain math: no renderer, no clock of its own, no globals.
//! Every type here is a pure function of time (or a value you step by a
//! `dt` you choose), so animations are deterministic and testable.
//!
//! - [`Ease`] — the standard easing family (quad, cubic, expo, back,
//!   elastic, bounce; in / out / in-out).
//! - [`Lerp`] — anything that can be blended: `f64`, [`Vec3`], [`Quat`] (by slerp).
//! - [`Tween`] — one value going from `a` to `b` over a duration.
//! - [`Track`] — keyframes with per-segment easing.
//! - [`Timeline`] — clips placed in sequence or in parallel, with seek and loop.
//! - [`Spring`] — a damped spring that follows a moving target (critically
//!   damped never overshoots; under-damped wobbles).
//!
//! ```
//! use simbiguity_anim::{Ease, Tween};
//!
//! let fade = Tween::new(0.0, 1.0, 2.0).with_ease(Ease::CubicInOut);
//! assert_eq!(fade.sample(0.0), 0.0);
//! assert_eq!(fade.sample(1.0), 0.5); // the in-out curves are symmetric
//! assert_eq!(fade.sample(5.0), 1.0); // clamped past the end
//! ```
//!
//! [`Vec3`]: simbiguity_math::Vec3
//! [`Quat`]: simbiguity_math::Quat
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;
#[cfg(any(test, feature = "std"))]
extern crate std;

pub mod easing;
pub mod lerp;
pub mod spring;
pub mod timeline;
pub mod track;
pub mod tween;

pub use easing::Ease;
pub use lerp::Lerp;
pub use spring::Spring;
pub use timeline::{ClipId, Timeline};
pub use track::{Keyframe, Track};
pub use tween::Tween;
