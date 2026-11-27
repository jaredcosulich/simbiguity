#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
//! # simbiguity-plot
//!
//! A minimal rendering-side crate for 2D time-series plots.
//! It maps samples from data space to pixel space, computing "nice" axis ticks,
//! downsampling large datasets dynamically to match the screen's column count,
//! and building line-strip meshes for the existing WebGL executor to render.
#![no_std]
extern crate alloc;

pub mod axes;
pub mod chart;
pub mod line;
pub mod series;
pub mod ticks;

pub use axes::Axes;
pub use chart::Chart;
pub use series::Series;
pub use ticks::NiceTicks;
