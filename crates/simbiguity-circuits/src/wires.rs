//! Currents through wires and closed switches.
//!
//! Merged nodes have no voltage difference, so Ohm's law says nothing about
//! the current in a wire. Kirchhoff's current law does: walk a spanning
//! forest of the wires from the leaves inwards, each node passing its net
//! inflow on to its parent along the wire that joins them.

use alloc::vec;
use alloc::vec::Vec;

use crate::component::{Component, Stamp};
use crate::netlist::Circuit;

/// For each node, the wires touching it as `(component index, other node)`.
type Adjacency = Vec<Vec<(usize, usize)>>;

/// A BFS spanning forest: nodes in visiting order (roots first in each
/// tree), and each non-root node's `(wire, parent node)`.
pub(crate) struct Forest {
    pub(crate) order: Vec<usize>,
    pub(crate) parent: Vec<Option<(usize, usize)>>,
}

/// The net current flowing into each node from every non-wire component,
/// and the wire adjacency to spread it over.
pub(crate) fn node_inflows(
    c: &Circuit,
    stamps: &[Stamp],
    currents: &[f64],
    node_voltages: &[Option<f64>],
) -> (Vec<f64>, Adjacency) {
    let n = c.node_count;
    let mut inflow = vec![0.0; n];
    let mut adj: Adjacency = vec![Vec::new(); n];
    for (i, (p, s)) in c.parts.iter().zip(stamps).enumerate() {
        let (a, b) = (p.a.0, p.b.0);
        if *s == Stamp::Merge {
            adj[a].push((i, b));
            adj[b].push((i, a));
            continue;
        }

        if let Component::Bjt {
            polarity,
            saturation_amps,
            beta_f,
            beta_r,
        } = p.component
        {
            let vc = node_voltages[a].unwrap_or(0.0);
            let vb = node_voltages[b].unwrap_or(0.0);
            let e = p.c.unwrap_or(crate::netlist::NodeId(0)).0;
            let ve = node_voltages[e].unwrap_or(0.0);

            let (v_be, v_bc) = match polarity {
                crate::component::Polarity::Npn => (vb - ve, vb - vc),
                crate::component::Polarity::Pnp => (ve - vb, vc - vb),
            };

            let vt = 0.025852;
            let exp_be = simbiguity_math::float::exp(v_be / vt);
            let exp_bc = simbiguity_math::float::exp(v_bc / vt);

            let mut i_c =
                saturation_amps * (exp_be - exp_bc) - (saturation_amps / beta_r) * (exp_bc - 1.0);
            let mut i_b = (saturation_amps / beta_f) * (exp_be - 1.0)
                + (saturation_amps / beta_r) * (exp_bc - 1.0);

            if polarity == crate::component::Polarity::Pnp {
                i_c = -i_c;
                i_b = -i_b;
            }

            let i_e = -(i_c + i_b);

            inflow[a] += i_c;
            inflow[b] += i_b;
            inflow[e] += i_e;
            continue;
        }

        // Batteries push current out of `a`; everything else carries it a → b.
        let into_a = match p.component {
            Component::Battery { .. } => currents[i],
            _ => -currents[i],
        };
        inflow[a] += into_a;
        inflow[b] -= into_a;
    }
    (inflow, adj)
}

/// A breadth-first spanning forest of the wire graph, rooted at the lowest
/// node of each tree. Wires that close a loop are left out (chords).
pub(crate) fn spanning_forest(adj: &Adjacency) -> Forest {
    let n = adj.len();
    let mut seen = vec![false; n];
    let mut parent: Vec<Option<(usize, usize)>> = vec![None; n];
    let mut order = Vec::with_capacity(n);
    for root in 0..n {
        if seen[root] {
            continue;
        }
        seen[root] = true;
        order.push(root);
        let mut next = order.len() - 1;
        while next < order.len() {
            let here = order[next];
            next += 1;
            for &(edge, other) in &adj[here] {
                if !seen[other] {
                    seen[other] = true;
                    parent[other] = Some((edge, here));
                    order.push(other);
                }
            }
        }
    }
    Forest { order, parent }
}

/// Fills in wire and closed-switch currents (a → b) from Kirchhoff's
/// current law. Wires closing a wire-only loop keep a current of zero.
pub(crate) fn wire_currents(
    c: &Circuit,
    stamps: &[Stamp],
    currents: &mut [f64],
    node_voltages: &[Option<f64>],
) {
    let (mut inflow, adj) = node_inflows(c, stamps, currents, node_voltages);
    let forest = spanning_forest(&adj);
    for &node in forest.order.iter().rev() {
        if let Some((edge, up)) = forest.parent[node] {
            let flow = inflow[node];
            inflow[up] += flow;
            currents[edge] = if c.parts[edge].a.0 == node {
                flow
            } else {
                -flow
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamps(c: &Circuit) -> Vec<Stamp> {
        c.parts.iter().map(|p| p.component.stamp(0.0)).collect()
    }

    /// A battery's current counts as flowing out of its + terminal and into
    /// its − terminal; a resistor's flows out of a and into b.
    #[test]
    fn inflow_follows_each_components_sign_convention() {
        let mut c = Circuit::new();
        let (p, m) = (c.node(), c.node());
        c.add_battery(p, m, 6.0);
        c.add_resistor(p, m, 3.0);
        let (inflow, _) = node_inflows(
            &c,
            &stamps(&c),
            &[2.0, 2.0],
            &vec![Some(0.0); c.node_count()],
        );
        assert_eq!(inflow, vec![0.0, 0.0]);
        let (inflow, _) = node_inflows(
            &c,
            &stamps(&c),
            &[2.0, 0.0],
            &vec![Some(0.0); c.node_count()],
        );
        assert_eq!(inflow, vec![2.0, -2.0]);
    }

    /// Wires contribute no inflow; they only appear in the adjacency, from
    /// both ends.
    #[test]
    fn wires_go_into_the_adjacency_not_the_inflow() {
        let mut c = Circuit::new();
        let (a, b) = (c.node(), c.node());
        c.connect(a, b);
        let (inflow, adj) =
            node_inflows(&c, &stamps(&c), &[99.0], &vec![Some(0.0); c.node_count()]);
        assert_eq!(inflow, vec![0.0, 0.0]);
        assert_eq!(adj[0], vec![(0, 1)]);
        assert_eq!(adj[1], vec![(0, 0)]);
    }

    /// Every node is visited once, each tree is rooted at its lowest node,
    /// and children point at their parent through the joining wire.
    #[test]
    fn forest_visits_every_node_with_lowest_roots() {
        // 0 —w0— 1 —w1— 2,   3 alone
        let adj: Adjacency = vec![vec![(0, 1)], vec![(0, 0), (1, 2)], vec![(1, 1)], vec![]];
        let f = spanning_forest(&adj);
        assert_eq!(f.order, vec![0, 1, 2, 3]);
        assert_eq!(f.parent, vec![None, Some((0, 0)), Some((1, 1)), None]);
    }

    /// In a loop of wires, the wire that closes the loop is not a tree edge.
    #[test]
    fn forest_leaves_out_the_chord_of_a_wire_loop() {
        // triangle: w0 = 0-1, w1 = 1-2, w2 = 2-0
        let adj: Adjacency = vec![
            vec![(0, 1), (2, 2)],
            vec![(0, 0), (1, 2)],
            vec![(1, 1), (2, 0)],
        ];
        let f = spanning_forest(&adj);
        let edges: Vec<usize> = f.parent.iter().flatten().map(|(e, _)| *e).collect();
        assert_eq!(edges, vec![0, 2]);
    }

    /// The current a battery drives through a resistor also flows through
    /// the wires on its way, with the sign of each wire's a → b direction.
    #[test]
    fn wire_currents_carry_the_loop_current() {
        let mut c = Circuit::new();
        let (p, x, y, m) = (c.node(), c.node(), c.node(), c.node());
        c.add_battery(p, m, 6.0); // 0
        c.connect(p, x); // 1: forward
        c.add_resistor(x, y, 3.0); // 2
        c.connect(m, y); // 3: backward (b is upstream)
        let mut currents = vec![2.0, 0.0, 2.0, 0.0];
        wire_currents(
            &c,
            &stamps(&c),
            &mut currents,
            &vec![Some(0.0); c.node_count()],
        );
        assert_eq!(currents, vec![2.0, 2.0, 2.0, -2.0]);
    }

    /// A loop made only of wires gets zero on its chord, as documented.
    #[test]
    fn wire_only_loop_chord_reports_zero() {
        let mut c = Circuit::new();
        let (a, b) = (c.node(), c.node());
        c.connect(a, b);
        c.connect(a, b);
        let mut currents = vec![0.0, 0.0];
        wire_currents(
            &c,
            &stamps(&c),
            &mut currents,
            &vec![Some(0.0); c.node_count()],
        );
        assert_eq!(currents, vec![0.0, 0.0]);
    }

    /// A circuit with no wires leaves every current untouched.
    #[test]
    fn no_wires_changes_nothing() {
        let mut c = Circuit::new();
        let (p, m) = (c.node(), c.node());
        c.add_battery(p, m, 6.0);
        c.add_resistor(p, m, 3.0);
        let mut currents = vec![2.0, 2.0];
        wire_currents(
            &c,
            &stamps(&c),
            &mut currents,
            &vec![Some(0.0); c.node_count()],
        );
        assert_eq!(currents, vec![2.0, 2.0]);
    }
}
