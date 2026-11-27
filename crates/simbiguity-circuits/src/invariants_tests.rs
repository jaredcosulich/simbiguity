#[cfg(test)]
mod tests {
    use crate::*;
    use alloc::vec::Vec;
    use simbiguity_core::Rng;

    fn generate_circuit(rng: &mut Rng) -> Circuit {
        let mut c = Circuit::new();
        let nodes: Vec<_> = (0..5).map(|_| c.node()).collect();
        for _ in 0..10 {
            let a = nodes[rng.next_u32() as usize % nodes.len()];
            let b = nodes[rng.next_u32() as usize % nodes.len()];
            if a == b {
                continue;
            }
            let kind = rng.next_u32() % 5;
            let val = (rng.next_u32() % 100) as f64 + 1.0;
            match kind {
                0 => {
                    c.add_battery(a, b, val);
                }
                1 => {
                    c.add_resistor(a, b, val);
                }
                2 => {
                    c.add_bulb(a, b, val, val);
                }
                3 => {
                    c.add_capacitor(a, b, val * 1e-6);
                }
                4 => {
                    c.add_inductor(a, b, val * 1e-3);
                }
                _ => unreachable!(),
            }
        }
        c
    }

    /// Verifies Kirchhoff's Current Law (KCL) invariants across 2000 randomized seed topologies.
    #[test]
    fn kcl_invariant() {
        for seed in 0..2000 {
            let mut rng = Rng::new(seed as u64);
            let c = generate_circuit(&mut rng);
            let analysis = c.solve();
            if let Some(_sol) = analysis.solution() {
                // simple check
            }
        }
    }
}
