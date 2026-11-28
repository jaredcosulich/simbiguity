#[cfg(test)]
mod tests {
    use crate::*;
    use std::fs;

    #[allow(dead_code)]
    fn fnv1a(bytes: &[u8]) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for &b in bytes {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    fn hash_f64s(vals: &[f64]) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for v in vals {
            for &b in &v.to_bits().to_le_bytes() {
                hash ^= b as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
        hash
    }

    fn results(h: u32) -> std::vec::Vec<f64> {
        let len = circuit_results_len(h) as usize;
        unsafe { std::slice::from_raw_parts(circuit_results_ptr(h), len) }.to_vec()
    }

    /// Verifies that the native WASM engine produces deterministic outputs matching golden hashes.
    #[test]
    fn golden_determinism_check() {
        let json_str = fs::read_to_string("golden-hashes.json").unwrap();
        // A minimal determinism check.
        // Hello circuit scenario
        let h = circuit_new();
        let p = circuit_node(h) as u32;
        let m = circuit_node(h) as u32;
        let t = circuit_node(h) as u32;
        circuit_add_battery(h, p, m, 12.0);
        circuit_add_switch(h, p, t, 1);
        circuit_add_bulb(h, t, m, 12.0, 6.0);
        circuit_solve(h);

        let hash_hello = hash_f64s(&results(h));
        circuit_free(h);

        assert!(json_str.contains(&std::format!("\"hello\": \"{}\"", hash_hello)));
    }
}
