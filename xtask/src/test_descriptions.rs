//! Find every `#[test]` function in a test binary's source tree, with its
//! module path, line and the comment block written directly above it.
//!
//! The walk follows the crate's own module tree from its root file
//! (`src/lib.rs`, `tests/<name>.rs`): `mod name;` declarations (including
//! `#[path = "..."]` ones) and inline `mod name { ... }` blocks, so the
//! module path recorded for a test is the same path libtest prints.

use std::fs;
use std::path::{Path, PathBuf};

/// One `#[test]` function found in source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestSource {
    /// Module path inside the binary plus the function name, e.g.
    /// `mna::tests::references_default_to_first_battery_minus`.
    pub path: String,
    /// The file, relative to the directory the walk was rooted in.
    pub file: String,
    /// 1-based line of the `fn`.
    pub line: usize,
    /// The `///` or `//` comment block above the test, joined into one
    /// sentence-friendly line; `None` when the test has no comment.
    pub description: Option<String>,
}

/// Walk the module tree rooted at `crate_dir/root` (e.g. `src/lib.rs`).
pub fn scan(crate_dir: &Path, root: &str) -> Vec<TestSource> {
    let mut out = Vec::new();
    let root_path = crate_dir.join(root);
    let children_dir = root_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    walk(crate_dir, &root_path, &[], &children_dir, &mut out);
    out
}

/// Scan one file's text, as if it were the module at `module`. Declared
/// child modules (`mod x;`) are returned so the caller can follow them.
pub fn scan_source(
    text: &str,
    module: &[String],
    file: &str,
) -> (Vec<TestSource>, Vec<ChildModule>) {
    let lines: Vec<&str> = text.lines().collect();
    let mut tests = Vec::new();
    let mut children = Vec::new();
    // Inline `mod name {` blocks currently open, with the brace depth at
    // which each one closes.
    let mut inline: Vec<(String, i32)> = Vec::new();
    let mut depth: i32 = 0;
    let mut pending_test = false;
    let mut pending_path: Option<String> = None;
    let mut test_attr_line = 0;

    for (i, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        if let Some(p) = path_attribute(line) {
            pending_path = Some(p);
        } else if line == "#[test]" {
            pending_test = true;
            test_attr_line = i;
        } else if let Some((name, inline_block)) = mod_declaration(line) {
            if inline_block {
                inline.push((name, depth));
            } else {
                let mut path: Vec<String> = module.to_vec();
                path.extend(inline.iter().map(|(n, _)| n.clone()));
                path.push(name.clone());
                children.push(ChildModule {
                    name,
                    module: path,
                    path_attr: pending_path.take(),
                });
            }
        } else if pending_test {
            if let Some(name) = fn_name(line) {
                let mut path: Vec<String> = module.to_vec();
                path.extend(inline.iter().map(|(n, _)| n.clone()));
                path.push(name);
                tests.push(TestSource {
                    path: path.join("::"),
                    file: file.to_string(),
                    line: i + 1,
                    description: comment_above(&lines, test_attr_line),
                });
                pending_test = false;
            }
        }
        if !line.starts_with("#[path") && !line.is_empty() && !line.starts_with("#[") {
            // `#[path]` only applies to the very next item.
            if mod_declaration(line).is_none() {
                pending_path = None;
            }
        }
        depth += brace_delta(line);
        while let Some((_, open_depth)) = inline.last() {
            if depth <= *open_depth {
                inline.pop();
            } else {
                break;
            }
        }
    }
    (tests, children)
}

/// A `mod name;` found while scanning, still to be followed into its file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChildModule {
    pub name: String,
    pub module: Vec<String>,
    pub path_attr: Option<String>,
}

fn walk(
    crate_dir: &Path,
    file: &Path,
    module: &[String],
    children_dir: &Path,
    out: &mut Vec<TestSource>,
) {
    let Ok(text) = fs::read_to_string(file) else {
        return;
    };
    let rel = file
        .strip_prefix(crate_dir)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/");
    let (tests, children) = scan_source(&text, module, &rel);
    out.extend(tests);
    let file_dir = file.parent().map(Path::to_path_buf).unwrap_or_default();
    for child in children {
        let (child_file, grandchildren_dir) = match &child.path_attr {
            Some(p) => {
                let f = file_dir.join(p);
                let stem = f
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                (
                    f.clone(),
                    f.parent().map(|d| d.join(stem)).unwrap_or_default(),
                )
            }
            None => {
                let flat = children_dir.join(format!("{}.rs", child.name));
                if flat.exists() {
                    (flat, children_dir.join(&child.name))
                } else {
                    let nested: PathBuf = children_dir.join(&child.name).join("mod.rs");
                    (nested, children_dir.join(&child.name))
                }
            }
        };
        walk(
            crate_dir,
            &child_file,
            &child.module,
            &grandchildren_dir,
            out,
        );
    }
}

/// `#[path = "mna_tests.rs"]` → `mna_tests.rs`.
fn path_attribute(line: &str) -> Option<String> {
    let rest = line
        .strip_prefix("#[path")?
        .trim_start()
        .strip_prefix('=')?
        .trim();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// `mod name;` → (name, false); `pub(crate) mod name {` → (name, true).
fn mod_declaration(line: &str) -> Option<(String, bool)> {
    let line = strip_visibility(line);
    let rest = line.strip_prefix("mod ")?;
    let name_end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
    let name = &rest[..name_end];
    let after = rest[name_end..].trim_start();
    if after.starts_with(';') {
        Some((name.to_string(), false))
    } else if after.starts_with('{') {
        Some((name.to_string(), true))
    } else {
        None
    }
}

/// `fn name(` (optionally `pub`, `async`) → `name`.
fn fn_name(line: &str) -> Option<String> {
    let line = strip_visibility(line);
    let line = line.strip_prefix("async ").unwrap_or(line);
    let rest = line.strip_prefix("fn ")?;
    let end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
    Some(rest[..end].to_string())
}

fn strip_visibility(line: &str) -> &str {
    if let Some(rest) = line.strip_prefix("pub(") {
        if let Some(close) = rest.find(')') {
            return rest[close + 1..].trim_start();
        }
    }
    line.strip_prefix("pub ").unwrap_or(line)
}

/// The contiguous `///` / `//` block above the attribute group that holds
/// `#[test]` (other attributes like `#[should_panic]` may sit in between).
fn comment_above(lines: &[&str], test_attr_line: usize) -> Option<String> {
    let mut i = test_attr_line;
    let mut parts: Vec<String> = Vec::new();
    while i > 0 {
        i -= 1;
        let line = lines[i].trim();
        if line.starts_with("#[") {
            continue;
        }
        if line.starts_with("//!") {
            break;
        }
        if let Some(text) = line.strip_prefix("///").or_else(|| line.strip_prefix("//")) {
            parts.push(text.trim().to_string());
            continue;
        }
        break;
    }
    parts.reverse();
    let joined = parts
        .into_iter()
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}

/// Net `{` minus `}` on a line, ignoring string and char literals and
/// line comments.
fn brace_delta(line: &str) -> i32 {
    let mut delta = 0;
    let mut chars = line.chars().peekable();
    let mut in_str = false;
    while let Some(c) = chars.next() {
        if in_str {
            match c {
                '\\' => {
                    chars.next();
                }
                '"' => in_str = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '/' if chars.peek() == Some(&'/') => break,
            '\'' => {
                // A char literal like '{' or '\''; lifetimes ('a) fall through harmlessly.
                let mut look = chars.clone();
                let first = look.next();
                let second = look.next();
                if first == Some('\\') {
                    chars.next();
                    chars.next();
                    if chars.peek() == Some(&'\'') {
                        chars.next();
                    }
                } else if second == Some('\'') {
                    chars.next();
                    chars.next();
                }
            }
            '{' => delta += 1,
            '}' => delta -= 1,
            _ => {}
        }
    }
    delta
}

#[cfg(test)]
#[path = "test_descriptions_tests.rs"]
mod tests;
