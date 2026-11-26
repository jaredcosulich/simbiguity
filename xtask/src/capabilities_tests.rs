use super::*;

// A full manifest yields one capability per table, with every key read.
#[test]
fn parses_every_key_of_each_capability() {
    let text = r#"
# The circuit engine.
[[capability]]
name = "Ohm's law"
description = "Current through a resistor."
demo = "demos/hello-circuit/#closed"
proves = ["tests/analysis::ohms_law_single_resistor", "mna::tests"]

[[capability]]
name = "Short circuits"
description = "Detected, not solved."
proves = []
"#;
    let caps = parse(text).unwrap();
    assert_eq!(caps.len(), 2);
    assert_eq!(caps[0].name, "Ohm's law");
    assert_eq!(caps[0].demo.as_deref(), Some("demos/hello-circuit/#closed"));
    assert_eq!(
        caps[0].proves,
        vec!["tests/analysis::ohms_law_single_resistor", "mna::tests"]
    );
    assert_eq!(caps[1].demo, None);
    assert!(caps[1].proves.is_empty());
}

// A proves array may span lines, with comments and a trailing comma.
#[test]
fn parses_a_multi_line_array() {
    let text = r#"
[[capability]]
name = "Bridges"
proves = [
  "a::tests", # the unit tests
  "tests/analysis::bridge",
]
"#;
    let caps = parse(text).unwrap();
    assert_eq!(caps[0].proves, vec!["a::tests", "tests/analysis::bridge"]);
}

// A hash inside a string is text, not a comment.
#[test]
fn hash_inside_a_string_is_kept() {
    let caps = parse("[[capability]]\nname = \"x\"\ndemo = \"demos/a/#closed\" # note\n").unwrap();
    assert_eq!(caps[0].demo.as_deref(), Some("demos/a/#closed"));
}

// Escaped quotes, backslashes, newlines and tabs are decoded.
#[test]
fn decodes_string_escapes() {
    let caps = parse(
        r#"[[capability]]
name = "say \"hi\" \\ there\n\tok""#,
    )
    .unwrap();
    assert_eq!(caps[0].name, "say \"hi\" \\ there\n\tok");
}

// A key before any table is an error naming its line.
#[test]
fn key_before_first_table_is_an_error() {
    let e = parse("name = \"x\"\n").unwrap_err();
    assert_eq!(e.line, 1);
    assert!(e.message.contains("before the first"));
}

// A misspelled key is reported rather than silently dropped.
#[test]
fn unknown_key_is_an_error() {
    let e = parse("[[capability]]\nname = \"x\"\ndemos = \"y\"\n").unwrap_err();
    assert_eq!(e.line, 3);
    assert!(e.message.contains("unknown key `demos`"));
}

// A line that is not key = value is an error.
#[test]
fn line_without_equals_is_an_error() {
    let e = parse("[[capability]]\nname\n").unwrap_err();
    assert_eq!(e.line, 2);
}

// A capability must have a name.
#[test]
fn unnamed_capability_is_an_error() {
    let e = parse("[[capability]]\ndescription = \"x\"\n").unwrap_err();
    assert!(e.message.contains("#1 has no name"));
}

// Malformed strings are refused with a reason.
#[test]
fn malformed_strings_are_errors() {
    assert!(parse("[[capability]]\nname = x\n").is_err());
    assert!(parse("[[capability]]\nname = \"open\n").is_err());
    assert!(parse("[[capability]]\nname = \"a\" b\n").is_err());
    assert!(parse("[[capability]]\nname = \"\\q\"\n").is_err());
    assert!(parse("[[capability]]\nname = \"x\"\nproves = \"a\"\n").is_err());
}

// An empty manifest has no capabilities.
#[test]
fn empty_manifest_is_empty() {
    assert_eq!(parse("# nothing yet\n\n").unwrap(), vec![]);
}

// A pattern matches one test by its full path.
#[test]
fn proves_matches_a_full_test_path() {
    assert!(proves(
        "tests/analysis::ohms_law",
        "tests/analysis",
        "ohms_law"
    ));
    assert!(!proves("tests/analysis::ohms_law", "tests/analysis", "kvl"));
}

// A pattern naming a module matches every test in it and below it.
#[test]
fn proves_matches_a_module_and_its_submodules() {
    assert!(proves("mna::tests", "mna::tests", "x"));
    assert!(proves("mna", "mna::tests", "x"));
    assert!(!proves("mna", "mna_extra::tests", "x"));
}

// A test at the binary root matches by its bare name.
#[test]
fn proves_matches_a_root_test_by_name() {
    assert!(proves("smoke", "", "smoke"));
    assert!(!proves("smoke", "", "other"));
}
