#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn test_pkg_budget_reasons_and_limits() {
        let root = repo_root();
        let budget_file = root.join("site/pkg-budget.txt");
        if !budget_file.exists() {
            return;
        }

        let content = fs::read_to_string(&budget_file).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        for i in 0..lines.len() {
            let line = lines[i].trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut has_reason = false;
            if i > 0 {
                let prev = lines[i - 1].trim();
                if prev.starts_with("# reason:") {
                    has_reason = true;
                }
            }
            if !has_reason {
                panic!("Budget line '{}' must have a '# reason: ...' comment on the line immediately above it", line);
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() != 2 {
                continue;
            }

            let name = parts[0];
            let budget: f64 = parts[1].parse().unwrap();

            let wasm_path = root.join("site/pkg").join(name);
            if !wasm_path.exists() {
                continue; // Not built in this context, script enforces runtime
            }

            let actual = fs::metadata(&wasm_path).unwrap().len() as f64;

            // ensure budget doesn't exceed measured size by more than 5%
            let max_budget = actual * 1.05;
            if budget > max_budget {
                panic!("Budget {} for {} is too loose. Actual size is {}, so max allowed budget is {}. Tighten it.", budget, name, actual, max_budget);
            }
        }
    }
}
