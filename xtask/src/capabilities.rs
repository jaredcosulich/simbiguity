//! Read a crate's `capabilities.toml`: what the engine can do, which demo
//! shows it, and which tests prove it.
//!
//! Only the TOML subset the manifests need is understood, so the xtask
//! stays dependency-free:
//!
//! ```toml
//! # comment
//! [[capability]]
//! name = "Ohm's law"
//! description = "Current, voltage and power through a single resistor."
//! demo = "demos/hello-circuit/#closed"
//! proves = [
//!   "tests/analysis::ohms_law_single_resistor",
//!   "mna::tests",
//! ]
//! ```
//!
//! A `proves` entry matches a test whose module is that path, any module
//! under it, or the single test with that full name. Integration tests live
//! under `tests/<file>` (e.g. `tests/analysis::ohms_law_single_resistor`).

/// One `[[capability]]` table.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Capability {
    pub name: String,
    pub description: String,
    /// A path relative to the site root, e.g. `demos/hello-circuit/#closed`.
    pub demo: Option<String>,
    pub proves: Vec<String>,
}

/// A manifest line the reader could not understand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestError {
    pub line: usize,
    pub message: String,
}

/// Parse a whole manifest.
pub fn parse(text: &str) -> Result<Vec<Capability>, ManifestError> {
    let mut caps: Vec<Capability> = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line_no = i + 1;
        let line = strip_comment(lines[i]).trim().to_string();
        i += 1;
        if line.is_empty() {
            continue;
        }
        if line == "[[capability]]" {
            caps.push(Capability::default());
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(err(
                line_no,
                format!("expected `key = value`, found `{line}`"),
            ));
        };
        let key = key.trim();
        let mut value = value.trim().to_string();
        // A multi-line array continues until its closing bracket.
        if value.starts_with('[') {
            while !value.trim_end().ends_with(']') && i < lines.len() {
                value.push(' ');
                value.push_str(strip_comment(lines[i]).trim());
                i += 1;
            }
        }
        let Some(cap) = caps.last_mut() else {
            return Err(err(
                line_no,
                "a key appeared before the first [[capability]]".into(),
            ));
        };
        match key {
            "name" => cap.name = string(&value).map_err(|m| err(line_no, m))?,
            "description" => cap.description = string(&value).map_err(|m| err(line_no, m))?,
            "demo" => cap.demo = Some(string(&value).map_err(|m| err(line_no, m))?),
            "proves" => cap.proves = array(&value).map_err(|m| err(line_no, m))?,
            other => return Err(err(line_no, format!("unknown key `{other}`"))),
        }
    }
    if let Some(unnamed) = caps.iter().position(|c| c.name.is_empty()) {
        return Err(err(0, format!("capability #{} has no name", unnamed + 1)));
    }
    Ok(caps)
}

fn err(line: usize, message: String) -> ManifestError {
    ManifestError { line, message }
}

/// Drop a `#` comment that is not inside a string.
fn strip_comment(line: &str) -> &str {
    let mut in_str = false;
    let mut escaped = false;
    for (i, c) in line.char_indices() {
        match c {
            '\\' if in_str => escaped = !escaped,
            '"' if !escaped => in_str = !in_str,
            '#' if !in_str => return &line[..i],
            _ => escaped = false,
        }
        if c != '\\' {
            escaped = false;
        }
    }
    line
}

/// A basic `"..."` string with `\"`, `\\`, `\n` and `\t` escapes.
fn string(value: &str) -> Result<String, String> {
    let (s, rest) = take_string(value.trim())?;
    if !rest.trim().is_empty() {
        return Err(format!("unexpected text after string: `{}`", rest.trim()));
    }
    Ok(s)
}

fn take_string(value: &str) -> Result<(String, &str), String> {
    let body = value
        .strip_prefix('"')
        .ok_or_else(|| format!("expected a \"string\", found `{value}`"))?;
    let mut out = String::new();
    let mut chars = body.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Ok((out, &body[i + 1..])),
            '\\' => match chars.next().map(|(_, e)| e) {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                other => return Err(format!("unsupported escape `\\{}`", other.unwrap_or(' '))),
            },
            c => out.push(c),
        }
    }
    Err("unterminated string".into())
}

/// `["a", "b",]` → `vec!["a", "b"]`.
fn array(value: &str) -> Result<Vec<String>, String> {
    let inner = value
        .trim()
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .ok_or_else(|| format!("expected an [array], found `{value}`"))?;
    let mut items = Vec::new();
    let mut rest = inner.trim();
    while !rest.is_empty() {
        let (item, after) = take_string(rest)?;
        items.push(item);
        rest = after.trim_start();
        rest = rest.strip_prefix(',').unwrap_or(rest).trim_start();
    }
    Ok(items)
}

/// Does `pattern` (a `proves` entry) cover the test at `module::name`?
pub fn proves(pattern: &str, module: &str, name: &str) -> bool {
    let full = if module.is_empty() {
        name.to_string()
    } else {
        format!("{module}::{name}")
    };
    full == pattern || module == pattern || module.starts_with(&format!("{pattern}::"))
}

#[cfg(test)]
#[path = "capabilities_tests.rs"]
mod tests;
