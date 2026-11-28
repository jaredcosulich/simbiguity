use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::bench_fixtures;

pub fn parse_budget(content: &str) -> Result<HashMap<String, f64>, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 2 {
            continue;
        }
        let name = parts[0].to_string();
        let limit: f64 = parts[1]
            .parse()
            .map_err(|_| format!("invalid limit {}", parts[1]))?;
        map.insert(name, limit);
    }
    Ok(map)
}

pub fn parse_bench_results(json: &str) -> Result<HashMap<String, f64>, String> {
    let mut map = HashMap::new();
    let mut current_name = None;
    for jline in json.lines() {
        if let Some((_, name_val)) = jline.split_once("\"name\": \"") {
            if let Some((name, _)) = name_val.split_once("\"") {
                current_name = Some(name.to_string());
            }
        }
        if let Some(name) = &current_name {
            if jline.contains("\"medianUs\"") {
                if let Some((_, val)) = jline.split_once("\"medianUs\": ") {
                    let val = val.split(',').next().unwrap_or("").trim();
                    if let Ok(v) = val.parse::<f64>() {
                        map.insert(name.clone(), v);
                        current_name = None;
                        continue;
                    }
                }
            }
            if jline.contains("}") {
                current_name = None;
            }
        }
    }
    Ok(map)
}

pub fn check_bench(args: &[String]) -> Result<(), String> {
    if args.len() < 2 {
        return Err("usage: check-bench <budget-file> <bench-json>".into());
    }
    let budget_file = &args[0];
    let bench_json = &args[1];

    let budget_str =
        fs::read_to_string(budget_file).map_err(|e| format!("reading {}: {}", budget_file, e))?;
    let json_str =
        fs::read_to_string(bench_json).map_err(|e| format!("reading {}: {}", bench_json, e))?;

    let budget = parse_budget(&budget_str)?;
    let actuals = parse_bench_results(&json_str)?;

    for (name, limit) in budget {
        let actual = actuals
            .get(&name)
            .ok_or_else(|| format!("Missing bench result for {}", name))?;
        if *actual > limit {
            return Err(format!(
                "Benchmark {} took {} us, exceeds budget {} us",
                name, actual, limit
            ));
        }
    }
    Ok(())
}

pub fn run(args: &[String]) -> Result<(), String> {
    let mut out = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--out" {
            i += 1;
            out = args.get(i).cloned();
        }
        i += 1;
    }

    let results = bench_fixtures::run_all();
    let json = bench_fixtures::to_json(&results);

    if let Some(path) = out {
        let p = PathBuf::from(path);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(p, json).map_err(|e| e.to_string())?;
    } else {
        println!("{}", json);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies parsing a standard budget file with comments, empty lines, and valid numbers
    #[test]
    fn test_parse_budget_valid() {
        let content = "\
# reason: some reason
physics_step 50.0

# reason: other
render_frame 16.6
";
        let map = parse_budget(content).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("physics_step"), Some(&50.0));
        assert_eq!(map.get("render_frame"), Some(&16.6));
    }

    /// Verifies parsing fails on malformed numbers
    #[test]
    fn test_parse_budget_invalid_number() {
        let content = "physics_step abc";
        let err = parse_budget(content).unwrap_err();
        assert!(err.contains("invalid limit abc"));
    }

    /// Verifies parsing a typical bench JSON extracts the median values
    #[test]
    fn test_parse_bench_results_valid() {
        let json = r#"[
            {
                "name": "physics_step",
                "medianUs": 42.5,
                "other": 0
            },
            {
                "name": "render_frame",
                "medianUs": 15.0
            }
        ]"#;
        let map = parse_bench_results(json).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("physics_step"), Some(&42.5));
        assert_eq!(map.get("render_frame"), Some(&15.0));
    }

    /// Verifies parsing bench results handles missing medians gracefully
    #[test]
    fn test_parse_bench_results_missing_median() {
        let json = r#"[
            {
                "name": "physics_step"
            }
        ]"#;
        let map = parse_bench_results(json).unwrap();
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn check_bench_rejects_a_fixture_over_budget() {
        let budget = "ladder_1000_dc 35586.0";
        let json = r#"[
            {
                "name": "ladder_1000_dc",
                "medianUs": 50000.0
            }
        ]"#;

        let budget_map = parse_budget(budget).unwrap();
        let actuals = parse_bench_results(json).unwrap();

        let mut err = None;
        for (name, limit) in budget_map {
            let actual = actuals.get(&name).unwrap();
            if *actual > limit {
                err = Some(format!(
                    "Benchmark {} took {} us, exceeds budget {} us",
                    name, actual, limit
                ));
            }
        }

        assert_eq!(
            err.unwrap(),
            "Benchmark ladder_1000_dc took 50000 us, exceeds budget 35586 us"
        );
    }
}
