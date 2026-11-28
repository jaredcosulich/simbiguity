#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn test_screenshot_uniqueness() {
        let root = repo_root();
        let screenshots_dir = root.join(".codeyam/scenarios/screenshots");
        if !screenshots_dir.exists() {
            return;
        }

        let mut hashes: HashMap<String, Vec<String>> = HashMap::new();

        for entry in fs::read_dir(screenshots_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().unwrap_or_default() == "hash" {
                let hash = fs::read_to_string(&path).unwrap().trim().to_string();
                let png_name = path.file_stem().unwrap().to_str().unwrap().to_string();
                hashes.entry(hash).or_default().push(png_name);
            }
        }

        for (hash, files) in hashes {
            if files.len() > 1 {
                panic!("Duplicate screenshot hash {}: {:?}", hash, files);
            }
        }
    }
}
