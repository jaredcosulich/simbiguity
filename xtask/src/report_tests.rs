use super::*;
use crate::libtest_output::{Status, TestResult};
use crate::test_descriptions::TestSource;

fn result(target: &str, path: &str, status: Status) -> TestResult {
    TestResult {
        target: target.into(),
        path: path.into(),
        status,
    }
}

fn capability(name: &str, proves: &[&str]) -> Capability {
    Capability {
        name: name.into(),
        description: format!("{name} works."),
        demo: None,
        proves: proves.iter().map(|s| s.to_string()).collect(),
    }
}

fn input(results: Vec<TestResult>, capabilities: Vec<Capability>) -> CrateInput {
    CrateInput {
        name: "simbiguity-circuits".into(),
        description: "Circuits.".into(),
        workspace_dependencies: vec!["simbiguity-math".into()],
        external_dependencies: vec![],
        results,
        sources: vec![
            (
                "src/lib.rs".into(),
                vec![TestSource {
                    path: "mna::tests::solves".into(),
                    file: "src/mna_tests.rs".into(),
                    line: 12,
                    description: Some("It solves.".into()),
                }],
            ),
            (
                "tests/analysis.rs".into(),
                vec![TestSource {
                    path: "bridge".into(),
                    file: "tests/analysis.rs".into(),
                    line: 3,
                    description: Some("Bridges balance.".into()),
                }],
            ),
        ],
        capabilities,
        build_failed: false,
    }
}

// Unit tests keep their module; integration tests move under tests/<file>.
#[test]
fn join_namespaces_tests_and_attaches_sources() {
    let report = join(&[input(
        vec![
            result("src/lib.rs", "mna::tests::solves", Status::Passed),
            result("tests/analysis.rs", "bridge", Status::Failed),
        ],
        vec![],
    )]);
    let t = &report.tests;
    assert_eq!(t[0].id, "simbiguity-circuits::mna::tests::solves");
    assert_eq!(t[0].module, "mna::tests");
    assert_eq!(t[0].description.as_deref(), Some("It solves."));
    assert_eq!(
        t[0].file.as_deref(),
        Some("crates/simbiguity-circuits/src/mna_tests.rs")
    );
    assert_eq!(t[0].line, Some(12));
    assert_eq!(t[1].id, "simbiguity-circuits::tests/analysis::bridge");
    assert_eq!(t[1].module, "tests/analysis");
    assert_eq!(t[1].description.as_deref(), Some("Bridges balance."));
}

// A result with no matching source has no description, file or line.
#[test]
fn join_leaves_unknown_sources_empty() {
    let report = join(&[input(
        vec![result("src/lib.rs", "lonely", Status::Passed)],
        vec![],
    )]);
    let t = &report.tests[0];
    assert_eq!(t.module, "");
    assert_eq!(t.id, "simbiguity-circuits::lonely");
    assert_eq!(
        (t.description.clone(), t.file.clone(), t.line),
        (None, None, None)
    );
}

// Any failing proof marks a capability failing; passing proofs mark it passing.
#[test]
fn join_derives_capability_status_from_its_tests() {
    let report = join(&[input(
        vec![
            result("src/lib.rs", "mna::tests::solves", Status::Passed),
            result("tests/analysis.rs", "bridge", Status::Failed),
            result("src/lib.rs", "slow::tests::x", Status::Ignored),
        ],
        vec![
            capability("Solving", &["mna"]),
            capability("Bridges", &["mna::tests", "tests/analysis::bridge"]),
            capability("Slow", &["slow"]),
            capability("Capacitors", &["capacitor::tests"]),
        ],
    )]);
    let status: Vec<&str> = report.capabilities.iter().map(|c| c.status).collect();
    assert_eq!(status, vec!["passing", "failing", "untested", "untested"]);
    assert_eq!(report.capabilities[3].unmatched, vec!["capacitor::tests"]);
    assert!(report.capabilities[3].tests.is_empty());
}

// Overlapping proves patterns list each test only once.
#[test]
fn join_does_not_duplicate_overlapping_matches() {
    let report = join(&[input(
        vec![result("src/lib.rs", "mna::tests::solves", Status::Passed)],
        vec![capability(
            "Solving",
            &["mna", "mna::tests", "mna::tests::solves"],
        )],
    )]);
    assert_eq!(report.capabilities[0].tests.len(), 1);
    assert_eq!(
        report.capabilities[0].unmatched,
        vec!["mna::tests", "mna::tests::solves"]
    );
}

// Each crate's row counts its own passed, failed and ignored tests.
#[test]
fn join_counts_results_per_crate() {
    let mut i = input(
        vec![
            result("src/lib.rs", "a", Status::Passed),
            result("src/lib.rs", "b", Status::Failed),
            result("src/lib.rs", "c", Status::Ignored),
            result("src/lib.rs", "d", Status::Passed),
        ],
        vec![],
    );
    i.build_failed = true;
    let c = &join(&[i]).crates[0];
    assert_eq!(
        (c.passed, c.failed, c.ignored, c.build_failed),
        (2, 1, 1, true)
    );
    assert_eq!(c.workspace_dependencies, vec!["simbiguity-math"]);
}

// A doc-test is named by its item and file:line with a generated description.
#[test]
fn join_names_doc_tests() {
    let report = join(&[input(
        vec![
            result(
                "doc",
                "crates/x/src/float.rs - float::clamp (line 16)",
                Status::Passed,
            ),
            result("doc", "crates/x/src/lib.rs - (line 3)", Status::Passed),
        ],
        vec![],
    )]);
    let item = &report.tests[0];
    assert_eq!(item.module, "doc");
    assert_eq!(item.name, "float::clamp (float.rs:16)");
    assert_eq!(
        item.id,
        "simbiguity-circuits::doc::float::clamp (float.rs:16)"
    );
    assert_eq!(item.file.as_deref(), Some("crates/x/src/float.rs"));
    assert_eq!(item.line, Some(16));
    assert!(item
        .description
        .as_deref()
        .unwrap()
        .contains("`float::clamp`"));
    let krate = &report.tests[1];
    assert_eq!(krate.name, "crate docs (lib.rs:3)");
    assert!(krate
        .description
        .as_deref()
        .unwrap()
        .starts_with("The crate-level"));
}

// A doc-test line without a line number still gets a name.
#[test]
fn doc_test_without_a_line_number() {
    let t = doc_test("k", &result("doc", "src/lib.rs - thing", Status::Ignored));
    assert_eq!(t.name, "thing (lib.rs)");
    assert_eq!(t.line, None);
    assert_eq!(t.status, Status::Ignored);
}

fn build() -> Build {
    Build {
        commit: "abc123".into(),
        committed_at: "2026-11-26T18:49:08+00:00".into(),
        generated_at: "2026-11-26T19:00:00Z".into(),
        repository: "https://github.com/example/simbiguity".into(),
    }
}

// The JSON carries the build identity, totals and every list the page reads.
#[test]
fn to_json_writes_the_shape_the_page_reads() {
    let report = join(&[input(
        vec![
            result("src/lib.rs", "mna::tests::solves", Status::Passed),
            result("src/lib.rs", "lonely", Status::Failed),
        ],
        vec![capability("Solving", &["mna"])],
    )]);
    let json = to_json(&report, &build());
    for needle in [
        "\"commit\": \"abc123\"",
        "\"repository\": \"https://github.com/example/simbiguity\"",
        "\"passed\": 1",
        "\"failed\": 1",
        "\"tests\": 2",
        "\"externalDependencies\": 0",
        "\"workspaceDependencies\": [\"simbiguity-math\"]",
        "\"buildFailed\": false",
        "\"demo\": null",
        "\"status\": \"passing\"",
        "\"tests\": [\"simbiguity-circuits::mna::tests::solves\"]",
        "\"unmatched\": []",
        "\"line\": 12",
        "\"line\": null",
        "\"description\": null",
    ] {
        assert!(json.contains(needle), "missing {needle} in\n{json}");
    }
    assert!(json.starts_with("{\n  \"commit\""));
    assert!(json.ends_with("}\n"));
}

// Empty containers close on the same line.
#[test]
fn to_json_of_an_empty_report() {
    let json = to_json(&join(&[]), &build());
    assert!(json.contains("\"crates\": []"));
    assert!(json.contains("\"capabilities\": []"));
}

// Strings are quoted with JSON escapes, including control characters.
#[test]
fn escape_quotes_and_escapes() {
    assert_eq!(escape("plain"), "\"plain\"");
    assert_eq!(escape("a\"b\\c"), "\"a\\\"b\\\\c\"");
    assert_eq!(escape("\n\r\t"), "\"\\n\\r\\t\"");
    assert_eq!(escape("\u{1}"), "\"\\u0001\"");
    assert_eq!(escape("Ohm’s"), "\"Ohm’s\"");
}
