//! Parse libtest's stable plain-text output (`cargo test` with the default
//! formatter) into one result per test.
//!
//! Only two kinds of line matter:
//!
//! - `Running <target> (<binary>)` / `Doc-tests <crate>` start a new test
//!   binary, so the tests that follow belong to that target;
//! - `test <path> ... ok|FAILED|ignored` is one test's outcome.
//!
//! Everything else (compiler progress, captured stdout of failures, the
//! `test result:` summary) is ignored.

/// The outcome libtest printed for one test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Passed,
    Failed,
    Ignored,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Passed => "passed",
            Status::Failed => "failed",
            Status::Ignored => "ignored",
        }
    }
}

/// One `test <path> ... <status>` line, attributed to the binary it ran in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestResult {
    /// The source root of the test binary: `src/lib.rs` for unit tests,
    /// `tests/<name>.rs` for an integration test, `doc` for doc-tests.
    pub target: String,
    /// The test's path inside that binary, e.g. `float::tests::clamp_works`.
    pub path: String,
    pub status: Status,
}

/// Parse a whole `cargo test` log (stdout and stderr interleaved).
pub fn parse(output: &str) -> Vec<TestResult> {
    let mut target = String::new();
    let mut results = Vec::new();
    for raw in output.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("Running ") {
            target = running_target(rest);
        } else if line.starts_with("Doc-tests ") {
            target = "doc".to_string();
        } else if let Some(result) = test_line(line, &target) {
            results.push(result);
        }
    }
    results
}

/// `unittests src/lib.rs (target/debug/deps/x-123)` → `src/lib.rs`;
/// `tests/analysis.rs (target/debug/deps/analysis-123)` → `tests/analysis.rs`.
fn running_target(rest: &str) -> String {
    let rest = rest.strip_prefix("unittests ").unwrap_or(rest);
    let source = match rest.find(" (") {
        Some(i) => &rest[..i],
        None => rest,
    };
    source.trim().to_string()
}

fn test_line(line: &str, target: &str) -> Option<TestResult> {
    let rest = line.strip_prefix("test ")?;
    let (path, outcome) = rest.rsplit_once(" ... ")?;
    let status = match outcome.split([',', ' ']).next()? {
        "ok" => Status::Passed,
        "FAILED" => Status::Failed,
        "ignored" => Status::Ignored,
        _ => return None,
    };
    // `#[should_panic]` tests are printed as `name - should panic`.
    let path = path.trim();
    let path = path.strip_suffix(" - should panic").unwrap_or(path);
    Some(TestResult {
        target: target.to_string(),
        path: path.to_string(),
        status,
    })
}

#[cfg(test)]
#[path = "libtest_output_tests.rs"]
mod tests;
