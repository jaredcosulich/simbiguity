//! Repository tooling for simbiguity.
//!
//! ```text
//! cargo run -p xtask -- site-report [--out <path>] [--reuse-logs]
//! ```
//!
//! `site-report` runs `cargo test --no-fail-fast` for every crate under
//! `crates/`, reads each test's description comment from source, joins them
//! with the crates' `capabilities.toml` manifests and writes the engine
//! status page's data to `site/status/report.json`. Failing tests do not
//! fail the command: showing them is the point of the page.

mod bench;
mod bench_fixtures;
mod capabilities;
mod check_budgets_tests;
mod libtest_output;
mod pkg_budget_tests;
mod repo_hygiene_tests;
mod report;
mod screenshot_tests;
mod test_descriptions;

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use report::{Build, CrateInput};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("site-report") => match site_report(&args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("site-report: {e}");
                ExitCode::FAILURE
            }
        },
        Some("bench") => match bench::run(&args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("bench: {e}");
                ExitCode::FAILURE
            }
        },
        Some("check-bench") => match bench::check_bench(&args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("check-bench: {e}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!(
                "usage: cargo run -p xtask -- [site-report|bench] [--out <path>] [--reuse-logs]"
            );
            ExitCode::FAILURE
        }
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives in the repo root")
        .to_path_buf()
}

fn site_report(args: &[String]) -> Result<(), String> {
    let root = repo_root();
    let mut out = root.join("site/status/report.json");
    let mut reuse_logs = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                out = PathBuf::from(args.get(i).ok_or("--out needs a path")?);
            }
            "--reuse-logs" => reuse_logs = true,
            other => return Err(format!("unknown argument `{other}`")),
        }
        i += 1;
    }

    let log_dir = root.join("target/xtask");
    fs::create_dir_all(&log_dir).map_err(|e| format!("creating {}: {e}", log_dir.display()))?;

    let mut inputs = Vec::new();
    for crate_dir in crate_dirs(&root)? {
        let manifest =
            fs::read_to_string(crate_dir.join("Cargo.toml")).map_err(|e| e.to_string())?;
        let name = manifest_value(&manifest, "name")
            .ok_or_else(|| format!("{} has no name", crate_dir.display()))?;
        let description = manifest_value(&manifest, "description").unwrap_or_default();
        let (workspace_dependencies, external_dependencies) = dependencies(&manifest);

        let log_path = log_dir.join(format!("{name}.log"));
        if !reuse_logs || !log_path.exists() {
            eprintln!("site-report: testing {name}");
            run_tests(&root, &name, &log_path)?;
        }
        let log = fs::read_to_string(&log_path).map_err(|e| e.to_string())?;
        let results = libtest_output::parse(&log);
        let build_failed = results.is_empty() && log.contains("error: could not compile");

        let mut targets: Vec<String> = results
            .iter()
            .map(|r| r.target.clone())
            .filter(|t| t != "doc")
            .collect();
        targets.sort();
        targets.dedup();
        let sources = targets
            .into_iter()
            .map(|t| (t.clone(), test_descriptions::scan(&crate_dir, &t)))
            .collect();

        let cap_path = crate_dir.join("capabilities.toml");
        let capabilities = match fs::read_to_string(&cap_path) {
            Ok(text) => capabilities::parse(&text)
                .map_err(|e| format!("{}:{}: {}", cap_path.display(), e.line, e.message))?,
            Err(_) => Vec::new(),
        };

        inputs.push(CrateInput {
            name,
            description,
            workspace_dependencies,
            external_dependencies,
            results,
            sources,
            capabilities,
            build_failed,
        });
    }

    let report = report::join(&inputs);
    let build = Build {
        commit: git(&root, &["rev-parse", "HEAD"]).unwrap_or_default(),
        committed_at: git(&root, &["log", "-1", "--format=%cI"]).unwrap_or_default(),
        generated_at: now_iso8601(),
        repository: workspace_repository(&root).unwrap_or_default(),
    };
    if let Some(dir) = out.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(&out, report::to_json(&report, &build))
        .map_err(|e| format!("writing {}: {e}", out.display()))?;

    let passed: usize = report.crates.iter().map(|c| c.passed).sum();
    let failed: usize = report.crates.iter().map(|c| c.failed).sum();
    let ignored: usize = report.crates.iter().map(|c| c.ignored).sum();
    let untested = report
        .capabilities
        .iter()
        .filter(|c| c.status == "untested")
        .count();
    eprintln!(
        "site-report: {passed} passed, {failed} failed, {ignored} ignored across {} crates; {} capabilities ({untested} untested) → {}",
        report.crates.len(),
        report.capabilities.len(),
        out.display()
    );
    for cap in &report.capabilities {
        for pattern in &cap.unmatched {
            eprintln!("site-report: warning: {} capability `{}` proves `{pattern}`, which matched no test", cap.krate, cap.name);
        }
    }
    Ok(())
}

/// Every `crates/<name>/Cargo.toml`, sorted by name.
fn crate_dirs(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(root.join("crates"))
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("Cargo.toml").exists())
        .collect();
    dirs.sort();
    Ok(dirs)
}

/// Run one crate's tests with stdout and stderr interleaved in one log, so
/// each `Running` header lands above the results it introduces.
fn run_tests(root: &Path, name: &str, log_path: &Path) -> Result<(), String> {
    let log = File::create(log_path).map_err(|e| e.to_string())?;
    let log_err = log.try_clone().map_err(|e| e.to_string())?;
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    Command::new(cargo)
        .current_dir(root)
        .args(["test", "-p", name, "--no-fail-fast", "--color", "never"])
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .status()
        .map_err(|e| format!("running cargo test -p {name}: {e}"))?;
    Ok(())
}

/// The first `key = "value"` in a manifest's `[package]` table.
fn manifest_value(manifest: &str, key: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == key {
                let v = v.trim();
                if let Some(s) = v.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
                    return Some(s.replace("\\\"", "\""));
                }
            }
        }
    }
    None
}

/// (workspace path dependencies, everything else) across the dependency,
/// dev-dependency and build-dependency tables.
fn dependencies(manifest: &str) -> (Vec<String>, Vec<String>) {
    let mut workspace = Vec::new();
    let mut external = Vec::new();
    let mut in_deps = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_deps = line.ends_with("dependencies]");
            continue;
        }
        if !in_deps || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((name, spec)) = line.split_once('=') {
            let name = name.trim().to_string();
            let list = if spec.contains("path") {
                &mut workspace
            } else {
                &mut external
            };
            if !list.contains(&name) {
                list.push(name);
            }
        }
    }
    (workspace, external)
}

fn workspace_repository(root: &Path) -> Option<String> {
    let manifest = fs::read_to_string(root.join("Cargo.toml")).ok()?;
    manifest.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == "repository").then(|| v.trim().trim_matches('"').to_string())
    })
}

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The current UTC time as `YYYY-MM-DDTHH:MM:SSZ`, without a date crate.
fn now_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (y, m, d) = civil_from_days(days as i64);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Howard Hinnant's days-since-epoch → (year, month, day).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
