#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::process::Command;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn test_no_junk_files_in_tree() {
        let root = repo_root();
        let output = Command::new("git")
            .current_dir(&root)
            .args(["ls-files"])
            .output()
            .unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.ends_with(".orig")
                || line.ends_with(".rej")
                || line.ends_with(".patch")
                || line == "patch.diff"
                || line.starts_with("scratch")
            {
                panic!("Junk file found in tree: {}", line);
            }
            if line.ends_with(".rs") && !line.contains("/") {
                panic!("Root-level Rust file found in tree: {}", line);
            }
        }
    }
}
