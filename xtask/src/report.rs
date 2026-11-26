//! Join test results, test sources and capability manifests into the
//! engine status report, and serialize it as JSON for `site/status/`.

use crate::capabilities::{self, Capability};
use crate::libtest_output::{Status, TestResult};
use crate::test_descriptions::TestSource;

/// Everything known about one crate before joining.
pub struct CrateInput {
    pub name: String,
    pub description: String,
    /// Dependencies that are other crates in this workspace.
    pub workspace_dependencies: Vec<String>,
    /// Dependencies fetched from a registry or git: the zero-dependency claim.
    pub external_dependencies: Vec<String>,
    pub results: Vec<TestResult>,
    /// Test sources per binary target (`src/lib.rs`, `tests/analysis.rs`).
    pub sources: Vec<(String, Vec<TestSource>)>,
    pub capabilities: Vec<Capability>,
    /// `cargo test` produced no results because the crate failed to build.
    pub build_failed: bool,
}

/// Build identity stamped at the top of the page.
pub struct Build {
    pub commit: String,
    pub committed_at: String,
    pub generated_at: String,
    pub repository: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Test {
    pub id: String,
    pub krate: String,
    /// `float::tests`, or `tests/analysis` for an integration test.
    pub module: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Status,
    /// Path from the repository root, when the source was found.
    pub file: Option<String>,
    pub line: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityReport {
    pub krate: String,
    pub name: String,
    pub description: String,
    pub demo: Option<String>,
    /// `passing`, `failing` or `untested`.
    pub status: &'static str,
    pub tests: Vec<String>,
    /// `proves` entries that matched no test at all.
    pub unmatched: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CrateReport {
    pub name: String,
    pub description: String,
    pub workspace_dependencies: Vec<String>,
    pub external_dependencies: Vec<String>,
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
    pub build_failed: bool,
}

pub struct Report {
    pub crates: Vec<CrateReport>,
    pub capabilities: Vec<CapabilityReport>,
    pub tests: Vec<Test>,
}

/// Split a libtest path into (module, name), placing integration tests
/// under `tests/<file>` so they never collide with a unit `tests` module.
fn module_and_name(result: &TestResult) -> (String, String) {
    let (module, name) = match result.path.rsplit_once("::") {
        Some((m, n)) => (m.to_string(), n.to_string()),
        None => (String::new(), result.path.clone()),
    };
    if let Some(file) = result.target.strip_prefix("tests/") {
        let bin = format!("tests/{}", file.trim_end_matches(".rs"));
        let module = if module.is_empty() {
            bin
        } else {
            format!("{bin}::{module}")
        };
        return (module, name);
    }
    (module, name)
}

/// A doc-test line reads `crates/x/src/lib.rs - item::path (line 16)`; the
/// item is empty for crate-level docs.
fn doc_test(krate: &str, result: &TestResult) -> Test {
    let (file, rest) = result
        .path
        .split_once(" - ")
        .unwrap_or((result.path.as_str(), ""));
    let (item, line) = match rest.rsplit_once("(line ") {
        Some((item, n)) => (item.trim(), n.trim_end_matches(')').parse().ok()),
        None => (rest.trim(), None),
    };
    let description = if item.is_empty() {
        "The crate-level documentation example compiles and runs as written.".to_string()
    } else {
        format!("The documentation example for `{item}` compiles and runs as written.")
    };
    let item = if item.is_empty() { "crate docs" } else { item };
    let short = file.rsplit('/').next().unwrap_or(file);
    let name = match line {
        Some(n) => format!("{item} ({short}:{n})"),
        None => format!("{item} ({short})"),
    };
    Test {
        id: format!("{krate}::doc::{name}"),
        krate: krate.to_string(),
        module: "doc".to_string(),
        name,
        description: Some(description),
        status: result.status,
        file: Some(file.to_string()),
        line,
    }
}

pub fn join(inputs: &[CrateInput]) -> Report {
    let mut crates = Vec::new();
    let mut capabilities = Vec::new();
    let mut tests = Vec::new();
    for input in inputs {
        let mut crate_tests = Vec::new();
        for result in &input.results {
            if result.target == "doc" {
                crate_tests.push(doc_test(&input.name, result));
                continue;
            }
            let (module, name) = module_and_name(result);
            let source = input
                .sources
                .iter()
                .find(|(target, _)| *target == result.target)
                .and_then(|(_, sources)| sources.iter().find(|s| s.path == result.path));
            crate_tests.push(Test {
                id: format!(
                    "{}::{}{}",
                    input.name,
                    if module.is_empty() {
                        String::new()
                    } else {
                        format!("{module}::")
                    },
                    name
                ),
                krate: input.name.clone(),
                module,
                name,
                description: source.and_then(|s| s.description.clone()),
                status: result.status,
                file: source.map(|s| format!("crates/{}/{}", input.name, s.file)),
                line: source.map(|s| s.line),
            });
        }
        for cap in &input.capabilities {
            let mut ids = Vec::new();
            let mut unmatched = Vec::new();
            for pattern in &cap.proves {
                let before = ids.len();
                for t in &crate_tests {
                    if capabilities::proves(pattern, &t.module, &t.name) && !ids.contains(&t.id) {
                        ids.push(t.id.clone());
                    }
                }
                if ids.len() == before {
                    unmatched.push(pattern.clone());
                }
            }
            let proving: Vec<&Test> = crate_tests.iter().filter(|t| ids.contains(&t.id)).collect();
            let status = if proving.iter().any(|t| t.status == Status::Failed) {
                "failing"
            } else if proving.iter().any(|t| t.status == Status::Passed) {
                "passing"
            } else {
                "untested"
            };
            capabilities.push(CapabilityReport {
                krate: input.name.clone(),
                name: cap.name.clone(),
                description: cap.description.clone(),
                demo: cap.demo.clone(),
                status,
                tests: ids,
                unmatched,
            });
        }
        let count = |s: Status| crate_tests.iter().filter(|t| t.status == s).count();
        crates.push(CrateReport {
            name: input.name.clone(),
            description: input.description.clone(),
            workspace_dependencies: input.workspace_dependencies.clone(),
            external_dependencies: input.external_dependencies.clone(),
            passed: count(Status::Passed),
            failed: count(Status::Failed),
            ignored: count(Status::Ignored),
            build_failed: input.build_failed,
        });
        tests.extend(crate_tests);
    }
    Report {
        crates,
        capabilities,
        tests,
    }
}

/// Serialize the report as pretty-printed JSON.
pub fn to_json(report: &Report, build: &Build) -> String {
    let mut w = Json::default();
    let total = |f: fn(&CrateReport) -> usize| report.crates.iter().map(f).sum::<usize>();
    w.open('{');
    w.key("commit").str(&build.commit);
    w.key("committedAt").str(&build.committed_at);
    w.key("generatedAt").str(&build.generated_at);
    w.key("repository").str(&build.repository);
    w.key("totals").open('{');
    w.key("passed").num(total(|c| c.passed));
    w.key("failed").num(total(|c| c.failed));
    w.key("ignored").num(total(|c| c.ignored));
    w.key("tests").num(report.tests.len());
    w.key("externalDependencies")
        .num(total(|c| c.external_dependencies.len()));
    w.key("capabilities").num(report.capabilities.len());
    w.close('}');
    w.key("crates").open('[');
    for c in &report.crates {
        w.item().open('{');
        w.key("name").str(&c.name);
        w.key("description").str(&c.description);
        w.key("workspaceDependencies")
            .strs(&c.workspace_dependencies);
        w.key("externalDependencies").strs(&c.external_dependencies);
        w.key("passed").num(c.passed);
        w.key("failed").num(c.failed);
        w.key("ignored").num(c.ignored);
        w.key("buildFailed").boolean(c.build_failed);
        w.close('}');
    }
    w.close(']');
    w.key("capabilities").open('[');
    for c in &report.capabilities {
        w.item().open('{');
        w.key("crate").str(&c.krate);
        w.key("name").str(&c.name);
        w.key("description").str(&c.description);
        w.key("demo").opt_str(c.demo.as_deref());
        w.key("status").str(c.status);
        w.key("tests").strs(&c.tests);
        w.key("unmatched").strs(&c.unmatched);
        w.close('}');
    }
    w.close(']');
    w.key("tests").open('[');
    for t in &report.tests {
        w.item().open('{');
        w.key("id").str(&t.id);
        w.key("crate").str(&t.krate);
        w.key("module").str(&t.module);
        w.key("name").str(&t.name);
        w.key("description").opt_str(t.description.as_deref());
        w.key("status").str(t.status.as_str());
        w.key("file").opt_str(t.file.as_deref());
        match t.line {
            Some(l) => w.key("line").num(l),
            None => w.key("line").raw("null"),
        };
        w.close('}');
    }
    w.close(']');
    w.close('}');
    w.out.push('\n');
    w.out
}

/// A tiny pretty-printing JSON writer.
#[derive(Default)]
struct Json {
    out: String,
    indent: usize,
    /// Whether the current container already has an element.
    has_item: Vec<bool>,
}

impl Json {
    fn newline(&mut self) {
        self.out.push('\n');
        for _ in 0..self.indent {
            self.out.push_str("  ");
        }
    }
    fn separator(&mut self) {
        if let Some(has) = self.has_item.last_mut() {
            if *has {
                self.out.push(',');
            }
            *has = true;
            self.newline();
        }
    }
    fn open(&mut self, c: char) -> &mut Self {
        self.out.push(c);
        self.indent += 1;
        self.has_item.push(false);
        self
    }
    fn close(&mut self, c: char) -> &mut Self {
        self.indent -= 1;
        if self.has_item.pop() == Some(true) {
            self.newline();
        }
        self.out.push(c);
        self
    }
    fn item(&mut self) -> &mut Self {
        self.separator();
        self
    }
    fn key(&mut self, k: &str) -> &mut Self {
        self.separator();
        self.out.push_str(&escape(k));
        self.out.push_str(": ");
        self
    }
    fn str(&mut self, s: &str) -> &mut Self {
        self.out.push_str(&escape(s));
        self
    }
    fn opt_str(&mut self, s: Option<&str>) -> &mut Self {
        match s {
            Some(s) => self.str(s),
            None => self.raw("null"),
        }
    }
    fn strs(&mut self, items: &[String]) -> &mut Self {
        self.out.push('[');
        for (i, s) in items.iter().enumerate() {
            if i > 0 {
                self.out.push_str(", ");
            }
            self.out.push_str(&escape(s));
        }
        self.out.push(']');
        self
    }
    fn num(&mut self, n: usize) -> &mut Self {
        self.out.push_str(&n.to_string());
        self
    }
    fn boolean(&mut self, b: bool) -> &mut Self {
        self.raw(if b { "true" } else { "false" })
    }
    fn raw(&mut self, s: &str) -> &mut Self {
        self.out.push_str(s);
        self
    }
}

/// Quote and escape a JSON string.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
