#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;

    /// Verifies the budget check script correctly fails builds that exceed limits and parses correctly.
    #[test]
    fn test_check_budgets_script() {
        let dir = std::env::temp_dir().join(format!("check-budgets-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let budget_path = dir.join("pkg-budget.txt");
        fs::write(&budget_path, "simbiguity_circuits_wasm.wasm 10\nsimbiguity_render_wasm.wasm 20\nsimbiguity_physics_wasm.wasm 30\n").unwrap();

        fs::write(dir.join("simbiguity_circuits_wasm.wasm"), "12345").unwrap();
        fs::write(dir.join("simbiguity_render_wasm.wasm"), "123456789012345").unwrap();
        fs::write(dir.join("simbiguity_physics_wasm.wasm"), "1").unwrap();

        let script = std::env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .join("scripts/check-budgets.sh");

        let output = Command::new("sh")
            .arg(&script)
            .arg(&budget_path)
            .arg(&dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "should succeed when under budget: stdout: {}, stderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        fs::write(dir.join("simbiguity_circuits_wasm.wasm"), "123456789012345").unwrap();
        let status = Command::new("sh")
            .arg(&script)
            .arg(&budget_path)
            .arg(&dir)
            .status()
            .unwrap();
        assert!(!status.success(), "should fail when budget exceeded");

        let status = Command::new("sh")
            .arg(&script)
            .arg(dir.join("missing.txt"))
            .arg(&dir)
            .status()
            .unwrap();
        assert!(!status.success(), "should fail when budget file missing");

        let _ = fs::remove_dir_all(&dir);
    }
}
