use super::*;

const MANIFEST: &str = r#"
[package]
name = "simbiguity-circuits"
description = "DC circuits, \"solved\"."
version.workspace = true

[dependencies]
simbiguity-math = { path = "../simbiguity-math" }
# a comment
serde = "1"

[dev-dependencies]
simbiguity-core = { path = "../simbiguity-core" }
simbiguity-math = { path = "../simbiguity-math" }

[build-dependencies]
cc = { version = "1" }

[features]
default = []
"#;

// Package keys are read from the package table only, with escaped quotes.
#[test]
fn manifest_value_reads_the_package_table() {
    assert_eq!(
        manifest_value(MANIFEST, "name").as_deref(),
        Some("simbiguity-circuits")
    );
    assert_eq!(
        manifest_value(MANIFEST, "description").as_deref(),
        Some("DC circuits, \"solved\".")
    );
    assert_eq!(manifest_value(MANIFEST, "version"), None);
    assert_eq!(manifest_value(MANIFEST, "default"), None);
    assert_eq!(manifest_value("[lib]\nname = \"x\"\n", "name"), None);
}

// Path dependencies are workspace crates; everything else is external.
#[test]
fn dependencies_split_workspace_from_external() {
    let (workspace, external) = dependencies(MANIFEST);
    assert_eq!(workspace, vec!["simbiguity-math", "simbiguity-core"]);
    assert_eq!(external, vec!["serde", "cc"]);
}

// A manifest with no dependency tables has none of either kind.
#[test]
fn dependencies_of_a_bare_manifest() {
    let (workspace, external) = dependencies("[package]\nname = \"x\"\n");
    assert!(workspace.is_empty() && external.is_empty());
}

// Day zero is the Unix epoch and leap days land correctly.
#[test]
fn civil_from_days_converts_known_dates() {
    assert_eq!(civil_from_days(0), (1970, 1, 1));
    assert_eq!(civil_from_days(-1), (1969, 12, 31));
    assert_eq!(civil_from_days(11_016), (2000, 2, 29));
    assert_eq!(civil_from_days(20_783), (2026, 11, 26));
}

// The timestamp is in the ISO 8601 UTC shape the page parses.
#[test]
fn now_iso8601_has_the_utc_shape() {
    let now = now_iso8601();
    assert_eq!(now.len(), 20);
    assert!(now.ends_with('Z'));
    assert_eq!(&now[4..5], "-");
    assert_eq!(&now[10..11], "T");
}

// Only directories under crates/ with a Cargo.toml count, sorted by name.
#[test]
fn crate_dirs_lists_crates_sorted() {
    let root = std::env::temp_dir().join(format!("xtask-crates-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    for dir in ["crates/b", "crates/a", "crates/not-a-crate"] {
        fs::create_dir_all(root.join(dir)).unwrap();
    }
    fs::write(root.join("crates/b/Cargo.toml"), "").unwrap();
    fs::write(root.join("crates/a/Cargo.toml"), "").unwrap();
    fs::write(root.join("crates/README.md"), "").unwrap();

    let dirs = crate_dirs(&root);
    let _ = fs::remove_dir_all(&root);

    let names: Vec<String> = dirs
        .unwrap()
        .iter()
        .map(|d| d.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["a", "b"]);
}

// A repository without a crates directory is an error, not an empty list.
#[test]
fn crate_dirs_without_crates_is_an_error() {
    assert!(crate_dirs(Path::new("/nonexistent-xtask-root")).is_err());
}

// An unknown flag is refused before any test runs.
#[test]
fn site_report_rejects_an_unknown_argument() {
    let err = site_report(&["--bogus".to_string()]).unwrap_err();
    assert_eq!(err, "unknown argument `--bogus`");
}

// The out flag needs a path after it.
#[test]
fn site_report_requires_a_path_after_out() {
    let err = site_report(&["--out".to_string()]).unwrap_err();
    assert_eq!(err, "--out needs a path");
}
