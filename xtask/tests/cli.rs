//! The xtask binary's command line, run as a subprocess.

use std::process::Command;

fn xtask(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .output()
        .expect("xtask runs")
}

// With no subcommand the binary fails and prints its usage.
#[test]
fn no_subcommand_prints_usage() {
    let out = xtask(&[]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("usage: cargo run -p xtask -- site-report")
    );
}

// An unknown subcommand fails the same way.
#[test]
fn unknown_subcommand_prints_usage() {
    let out = xtask(&["build-everything"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("usage:"));
}

// A site-report error is reported with its prefix and a failing exit code.
#[test]
fn site_report_error_fails_the_command() {
    let out = xtask(&["site-report", "--bogus"]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("site-report: unknown argument `--bogus`")
    );
}
