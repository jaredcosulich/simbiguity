use super::*;

fn result(target: &str, path: &str, status: Status) -> TestResult {
    TestResult {
        target: target.to_string(),
        path: path.to_string(),
        status,
    }
}

// A unit-test binary's results are attributed to its source root.
#[test]
fn unit_tests_belong_to_the_running_target() {
    let log = "\
     Running unittests src/lib.rs (target/debug/deps/simbiguity_math-1a2b)

running 2 tests
test float::tests::clamp ... ok
test float::tests::sqrt ... FAILED
";
    assert_eq!(
        parse(log),
        vec![
            result("src/lib.rs", "float::tests::clamp", Status::Passed),
            result("src/lib.rs", "float::tests::sqrt", Status::Failed),
        ]
    );
}

// An integration test binary is named by its file under tests/.
#[test]
fn integration_tests_are_attributed_to_their_file() {
    let log = "\
     Running tests/analysis.rs (target/debug/deps/analysis-9f)
test ohms_law_single_resistor ... ok
";
    assert_eq!(
        parse(log),
        vec![result(
            "tests/analysis.rs",
            "ohms_law_single_resistor",
            Status::Passed
        )]
    );
}

// Doc-tests switch the target to `doc` until the next binary starts.
#[test]
fn doc_tests_switch_the_target_to_doc() {
    let log = "\
   Doc-tests simbiguity_math
test crates/simbiguity-math/src/lib.rs - (line 5) ... ok
     Running unittests src/lib.rs (target/debug/deps/x-1)
test tests::after ... ok
";
    let parsed = parse(log);
    assert_eq!(parsed[0].target, "doc");
    assert_eq!(
        parsed[0].path,
        "crates/simbiguity-math/src/lib.rs - (line 5)"
    );
    assert_eq!(parsed[1].target, "src/lib.rs");
}

// An ignored test, with or without a reason, is recorded as ignored.
#[test]
fn ignored_tests_are_recorded_with_or_without_a_reason() {
    let log = "\
test slow ... ignored
test slower ... ignored, needs a GPU
";
    let statuses: Vec<Status> = parse(log).into_iter().map(|r| r.status).collect();
    assert_eq!(statuses, vec![Status::Ignored, Status::Ignored]);
}

// libtest's should-panic suffix is not part of the test's name.
#[test]
fn should_panic_suffix_is_stripped() {
    let parsed = parse("test tests::rejects_zero - should panic ... ok\n");
    assert_eq!(parsed[0].path, "tests::rejects_zero");
}

// Summaries, compiler progress and captured output are not results.
#[test]
fn unrelated_lines_are_ignored() {
    let log = "\
   Compiling simbiguity-math v0.1.0
running 1 test
test result: ok. 1 passed; 0 failed; 0 ignored
---- tests::x stdout ----
test something weird ... bench
";
    assert!(parse(log).is_empty());
}

// Each status renders as the word the report and page use.
#[test]
fn status_strings_match_the_report() {
    assert_eq!(Status::Passed.as_str(), "passed");
    assert_eq!(Status::Failed.as_str(), "failed");
    assert_eq!(Status::Ignored.as_str(), "ignored");
}

// A Running line without a binary in brackets still names the target.
#[test]
fn running_target_without_binary() {
    assert_eq!(running_target("tests/render.rs"), "tests/render.rs");
}
