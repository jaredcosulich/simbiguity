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

/// The simbiguity version this build was compiled from (the crate's
/// `Cargo.toml` version), for embedding projects and the status page to report.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The published version must match the crate manifest, so the engine
    /// status page and embedding projects can report exactly which simbiguity
    /// build they are running.
    #[test]
    fn version_matches_the_crate_manifest() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
        assert!(!version().is_empty());
    }
}
