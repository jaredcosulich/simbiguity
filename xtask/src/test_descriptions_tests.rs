use super::*;

fn module(path: &[&str]) -> Vec<String> {
    path.iter().map(|s| s.to_string()).collect()
}

// A test inside an inline tests module gets its module path, line and comment.
#[test]
fn finds_a_commented_test_in_an_inline_module() {
    let src = "\
pub fn clamp() {}

#[cfg(test)]
mod tests {
    use super::*;

    // Clamp keeps a value
    // inside its range.
    #[test]
    fn clamps() {
        assert!(true);
    }
}
";
    let (tests, children) = scan_source(src, &module(&["float"]), "src/float.rs");
    assert!(children.is_empty());
    assert_eq!(
        tests,
        vec![TestSource {
            path: "float::tests::clamps".into(),
            file: "src/float.rs".into(),
            line: 10,
            description: Some("Clamp keeps a value inside its range.".into()),
        }]
    );
}

// Attributes between the comment and the test do not hide the comment.
#[test]
fn comment_above_skips_other_attributes() {
    let src = "\
/// Dividing by zero panics.
#[should_panic]
#[test]
#[ignore]
fn divides() {}
";
    let (tests, _) = scan_source(src, &[], "tests/a.rs");
    assert_eq!(
        tests[0].description.as_deref(),
        Some("Dividing by zero panics.")
    );
    assert_eq!(tests[0].path, "divides");
}

// A test with no comment has no description, and module docs are not one.
#[test]
fn uncommented_test_has_no_description() {
    let src = "\
//! Module docs.
#[test]
fn bare() {}
";
    let (tests, _) = scan_source(src, &[], "src/lib.rs");
    assert_eq!(tests[0].description, None);
}

// Leaving an inline module returns later tests to the outer module.
#[test]
fn inline_module_closes_at_its_brace() {
    let src = "\
mod inner {
    #[test]
    fn a() { let s = \"}\"; let c = '{'; }
}
#[test]
fn b() {}
";
    let (tests, _) = scan_source(src, &module(&["m"]), "src/m.rs");
    let paths: Vec<&str> = tests.iter().map(|t| t.path.as_str()).collect();
    assert_eq!(paths, vec!["m::inner::a", "m::b"]);
}

// Out-of-line modules are reported for the walk, with any path attribute.
#[test]
fn reports_child_modules_with_path_attributes() {
    let src = "\
pub mod netlist;
#[cfg(test)]
#[path = \"mna_tests.rs\"]
mod tests;
pub(crate) mod dsu;
";
    let (_, children) = scan_source(src, &module(&["mna"]), "src/mna.rs");
    assert_eq!(
        children,
        vec![
            ChildModule {
                name: "netlist".into(),
                module: module(&["mna", "netlist"]),
                path_attr: None
            },
            ChildModule {
                name: "tests".into(),
                module: module(&["mna", "tests"]),
                path_attr: Some("mna_tests.rs".into()),
            },
            ChildModule {
                name: "dsu".into(),
                module: module(&["mna", "dsu"]),
                path_attr: None
            },
        ]
    );
}

// A path attribute applies only to the item right after it.
#[test]
fn path_attribute_does_not_leak_past_its_item() {
    let src = "\
#[path = \"x.rs\"]
fn not_a_module() {}
mod later;
";
    let (_, children) = scan_source(src, &[], "src/lib.rs");
    assert_eq!(children[0].path_attr, None);
}

// Declaration parsing understands visibility and rejects non-modules.
#[test]
fn parses_module_and_fn_declarations() {
    assert_eq!(mod_declaration("mod a;"), Some(("a".into(), false)));
    assert_eq!(
        mod_declaration("pub(crate) mod b {"),
        Some(("b".into(), true))
    );
    assert_eq!(mod_declaration("pub mod c;"), Some(("c".into(), false)));
    assert_eq!(mod_declaration("model = 3;"), None);
    assert_eq!(fn_name("pub async fn go() {"), Some("go".into()));
    assert_eq!(fn_name("let x = 1;"), None);
    assert_eq!(path_attribute("#[path = \"t.rs\"]"), Some("t.rs".into()));
    assert_eq!(path_attribute("#[test]"), None);
}

// Braces in strings, char literals and comments do not count.
#[test]
fn brace_delta_ignores_literals_and_comments() {
    assert_eq!(brace_delta("fn a() {"), 1);
    assert_eq!(brace_delta("}"), -1);
    assert_eq!(brace_delta("let s = \"{{\\\"\"; // }"), 0);
    assert_eq!(brace_delta("let c = '{'; let q = '\\''; f::<'a>() {"), 1);
}

// The walk follows mod declarations into flat, nested and path-attribute files.
#[test]
fn scan_walks_the_module_tree_on_disk() {
    let dir = std::env::temp_dir().join(format!("xtask-scan-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src/deep")).unwrap();
    std::fs::write(
        dir.join("src/lib.rs"),
        "mod flat;\nmod deep;\n#[path = \"other_tests.rs\"]\nmod tests;\n",
    )
    .unwrap();
    std::fs::write(dir.join("src/flat.rs"), "// Flat.\n#[test]\nfn f() {}\n").unwrap();
    std::fs::write(dir.join("src/deep/mod.rs"), "#[test]\nfn d() {}\n").unwrap();
    std::fs::write(dir.join("src/other_tests.rs"), "#[test]\nfn o() {}\n").unwrap();

    let found = scan(&dir, "src/lib.rs");
    let _ = std::fs::remove_dir_all(&dir);

    let got: Vec<(String, String)> = found.into_iter().map(|t| (t.path, t.file)).collect();
    assert_eq!(
        got,
        vec![
            ("flat::f".into(), "src/flat.rs".into()),
            ("deep::d".into(), "src/deep/mod.rs".into()),
            ("tests::o".into(), "src/other_tests.rs".into()),
        ]
    );
}

// A root file that does not exist yields no tests rather than an error.
#[test]
fn scan_of_a_missing_root_is_empty() {
    assert!(scan(Path::new("/nonexistent-xtask-dir"), "src/lib.rs").is_empty());
}
