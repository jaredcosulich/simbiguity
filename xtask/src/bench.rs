use std::fs;
use std::path::PathBuf;

use crate::bench_fixtures;

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
