//! # simbiguity
//!
//! An open source simulation engine by The Ambiguity Project.
//!
//! This is the umbrella crate. Each engine lives in its own small,
//! independently importable crate (`simbiguity-math`, `simbiguity-core`,
//! `simbiguity-circuits`, `simbiguity-anim`, …); this crate will re-export
//! them behind feature flags so a project can depend on one name and opt in
//! to just the engines it needs.
#![no_std]
#![forbid(unsafe_code)]
