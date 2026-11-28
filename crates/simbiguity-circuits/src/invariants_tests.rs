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
            if let Some(sol) = analysis.solution() {
                assert_eq!(
                    sol.newton_iterations(),
                    1,
                    "Linear circuit must solve in exactly 1 iteration"
                );
                let mut max_i = 0.0_f64;
                for i in 0..c.parts.len() {
                    let i_mag = sol.current(crate::netlist::ComponentId(i)).abs();
                    if i_mag > max_i {
                        max_i = i_mag;
                    }
                }
                for n in 0..c.node_count() {
                    let mut net = 0.0;
                    for (i, p) in c.parts.iter().enumerate() {
                        let mut current = sol.current(crate::netlist::ComponentId(i));
                        if matches!(p.component, crate::component::Component::Battery { .. }) {
                            current = -current;
                        }
                        if p.a.0 == n {
                            net -= current;
                        }
                        if p.b.0 == n {
                            net += current;
                        }
                    }
                    assert!(
                        net.abs() <= 1e-9 * max_i + 1e-12,
                        "KCL failed at node {} (net = {}, max_i = {})",
                        n,
                        net,
                        max_i
                    );
                }
            }
        }
    }

    /// Verifies Tellegen's theorem (|Σ V·I| over all elements is below 1e-9 × Σ |V·I|) across 2000 topologies.
    #[test]
    fn tellegen_invariant() {
        for seed in 0..2000 {
            let mut rng = Rng::new(seed as u64);
            let c = generate_circuit(&mut rng);
            let analysis = c.solve();
            if let Some(sol) = analysis.solution() {
                let mut sum_vi = 0.0;
                let mut sum_abs_vi = 0.0;
                for (i, p) in c.parts.iter().enumerate() {
                    let va = sol.node_voltage(p.a).unwrap_or(0.0);
                    let vb = sol.node_voltage(p.b).unwrap_or(0.0);
                    let v = va - vb;
                    let i_c = sol.current(crate::netlist::ComponentId(i));
                    let power =
                        if matches!(p.component, crate::component::Component::Battery { .. }) {
                            -v * i_c
                        } else {
                            v * i_c
                        };
                    sum_vi += power;
                    sum_abs_vi += power.abs();
                }
                assert!(
                    sum_vi.abs() <= 1e-9 * sum_abs_vi + 1e-12,
                    "Tellegen failed (sum = {}, abs_sum = {})",
                    sum_vi,
                    sum_abs_vi
                );
            }
        }
    }

    /// Verifies repeatability: two solves of the same circuit are bit-identical.
    #[test]
    fn repeatability_invariant() {
        for seed in 0..2000 {
            let mut rng = Rng::new(seed as u64);
            let c = generate_circuit(&mut rng);
            let a1 = c.solve();
            let a2 = c.solve();
            assert_eq!(a1, a2);
        }
    }

    /// Verifies snapshot replay: a cloned circuit yields bit-identical results.
    #[test]
    fn snapshot_replay_invariant() {
        for seed in 0..2000 {
            let mut rng = Rng::new(seed as u64);
            let c1 = generate_circuit(&mut rng);
            let c2 = c1.clone();
            assert_eq!(c1.solve(), c2.solve());
        }
    }

    /// Verifies that every failure has a reason: a missing solution comes with at least one fatal issue.
    #[test]
    fn every_failure_has_a_reason() {
        for seed in 0..2000 {
            let mut rng = Rng::new(seed as u64);
            let c = generate_circuit(&mut rng);
            let analysis = c.solve();
            if analysis.solution().is_none() {
                assert!(analysis.issues().iter().any(|i| i.is_fatal()));
            }
        }
    }

    /// Verifies KCL invariants on topologies that include diodes.
    #[test]
    fn kcl_with_diodes() {
        for seed in 0..500 {
            let mut rng = Rng::new(seed as u64);
            let mut c = generate_circuit(&mut rng);
            let num_diodes = (rng.next_u32() % 3) + 1;
            for _ in 0..num_diodes {
                let a = crate::netlist::NodeId(rng.next_u32() as usize % c.node_count());
                let b = crate::netlist::NodeId(rng.next_u32() as usize % c.node_count());
                c.add_diode(a, b, 1e-14, 1.0);
            }
            let analysis = c.solve();
            if let Some(sol) = analysis.solution() {
                let mut max_i = 0.0_f64;
                for i in 0..c.parts.len() {
                    let i_mag = sol.current(crate::netlist::ComponentId(i)).abs();
                    if i_mag > max_i {
                        max_i = i_mag;
                    }
                }
                for n in 0..c.node_count() {
                    let mut net = 0.0;
                    for (i, p) in c.parts.iter().enumerate() {
                        let mut current = sol.current(crate::netlist::ComponentId(i));
                        if matches!(p.component, crate::component::Component::Battery { .. }) {
                            current = -current;
                        }
                        if p.a.0 == n {
                            net -= current;
                        }
                        if p.b.0 == n {
                            net += current;
                        }
                    }
                    assert!(
                        net.abs() <= 1e-9 * max_i + 1e-12,
                        "KCL failed at node {} (net = {}, max_i = {})",
                        n,
                        net,
                        max_i
                    );
                }
            }
        }
    }

    /// Verifies that the dense and sparse solver backends produce identical solutions
    /// for random circuits, ensuring the numerical accuracy of SparseLu matches DenseLu.
    #[test]
    fn dense_and_sparse_agree() {
        let mut rng = Rng::new(0x1337);
        for _ in 0..2000 {
            let mut c = generate_circuit(&mut rng);
            let n = c.node_count();
            if n == 0 {
                continue;
            }
            c.set_ground(NodeId(0));

            c.set_solver(crate::netlist::SolverKind::Dense);
            let s_dense = c.solve();
            c.set_solver(crate::netlist::SolverKind::Sparse);
            let s_sparse = c.solve();

            if let (Some(dense), Some(sparse)) = (s_dense.solution, s_sparse.solution) {
                for i in 0..c.node_count() {
                    let vd = dense.node_voltage(NodeId(i)).unwrap_or(0.0);
                    let vs = sparse.node_voltage(NodeId(i)).unwrap_or(0.0);
                    if vd.abs() > 1e-6 {
                        assert!((vd - vs).abs() / vd.abs() < 1e-10, "mismatch at node {}", i);
                    } else {
                        assert!((vd - vs).abs() < 1e-10, "mismatch at node {}", i);
                    }
                }
            }
        }
    }
}
