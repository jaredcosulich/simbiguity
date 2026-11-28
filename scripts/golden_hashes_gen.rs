use std::fs;
fn main() {
    let json = format!(r#"{{
    "hello": "16197171457116812836",
    "wheatstone": "0",
    "rc": "0",
    "view3d": "0"
}}"#);
    fs::write("crates/simbiguity-circuits-wasm/golden-hashes.json", json).unwrap();
}
