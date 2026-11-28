fn main() {
    let json = format!(r#"{{
    "hello": "{}",
    "wheatstone": "0",
    "rc": "0",
    "view3d": "0"
}}"#, 0);
    std::fs::write("crates/simbiguity-circuits-wasm/golden-hashes.json", json).unwrap();
}
