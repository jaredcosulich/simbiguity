//! Unit tests for the MNA helpers in `mna.rs`.

use alloc::vec;
use alloc::vec::Vec;
use simbiguity_core::Simulation;

use super::*;

/// Two nodes joined by a battery and a resistor.
fn battery_and_resistor() -> Circuit {
    let mut c = Circuit::new();
    let (p, m) = (c.node(), c.node());
    c.add_battery(p, m, 6.0);
    c.add_resistor(p, m, 3.0);
    c
}

/// Without an explicit ground, an island measures from the − terminal of
/// its first battery.
#[test]
fn references_default_to_first_battery_minus() {
    let c = battery_and_resistor();
    let stamps = c.parts.iter().map(|p| p.component.stamp()).collect();
    let t = Topology::build(&c, stamps);
    assert_eq!(t.reference, vec![Some(1), None]);
}

/// An explicit ground wins for its own island only; another island still
/// falls back to its battery.
#[test]
fn explicit_ground_applies_to_its_island_only() {
    let mut c = battery_and_resistor();
    let (p2, m2) = (c.node(), c.node());
    c.add_battery(p2, m2, 1.5);
    c.set_ground(NodeId(0));
    let stamps = c.parts.iter().map(|p| p.component.stamp()).collect();
    let t = Topology::build(&c, stamps);
    assert_eq!(t.reference_of(0), Some(0));
    assert_eq!(t.reference_of(2), Some(3));
}

/// An island with no battery and no ground has no reference.
#[test]
fn island_without_battery_has_no_reference() {
    let mut c = Circuit::new();
    let (a, b) = (c.node(), c.node());
    c.add_resistor(a, b, 1.0);
    let stamps = c.parts.iter().map(|p| p.component.stamp()).collect();
    assert_eq!(Topology::build(&c, stamps).reference_of(0), None);
}

/// Only nodes with a component attached count as touched.
#[test]
fn touched_nodes_marks_only_connected_nodes() {
    let mut c = battery_and_resistor();
    c.node();
    assert_eq!(touched_nodes(&c), vec![true, true, false]);
}

/// A current source adds into `a` and out of `b`, skipping the reference.
#[test]
fn current_source_stamps_plus_into_a_minus_out_of_b() {
    let mut rhs = vec![0.0; 2];
    stamp_current_source(&mut rhs, Some(0), Some(1), 2.0);
    assert_eq!(rhs, vec![2.0, -2.0]);
    stamp_current_source(&mut rhs, None, Some(1), 1.0);
    assert_eq!(rhs, vec![2.0, -3.0]);
}

/// A conductance between two unknowns stamps the classic ±g pattern;
/// against the reference it only touches the diagonal.
#[test]
fn conductance_stamps_symmetric_block() {
    let mut m = DenseMatrix::zeros(2, 2);
    stamp_conductance(&mut m, Some(0), Some(1), 0.5);
    assert_eq!(
        (m[(0, 0)], m[(0, 1)], m[(1, 0)], m[(1, 1)]),
        (0.5, -0.5, -0.5, 0.5)
    );
    stamp_conductance(&mut m, Some(0), None, 1.0);
    assert_eq!((m[(0, 0)], m[(1, 1)]), (1.5, 0.5));
}

/// A voltage source puts ±1 in its row and column and the EMF on the
/// right-hand side.
#[test]
fn voltage_source_stamps_incidence_and_emf() {
    let mut m = DenseMatrix::zeros(3, 3);
    let mut rhs = vec![0.0; 3];
    stamp_voltage_source(&mut m, &mut rhs, Some(0), Some(1), 2, 9.0);
    assert_eq!(
        (m[(0, 2)], m[(2, 0)], m[(1, 2)], m[(2, 1)]),
        (1.0, 1.0, -1.0, -1.0)
    );
    assert_eq!(rhs[2], 9.0);
}

/// Each stamp turns the voltage across it into the right current, and an
/// ideal battery's unknown is negated to read "out of +".
#[test]
fn component_current_per_stamp() {
    assert_eq!(component_current(Stamp::Conductance(0.5), 4.0, None), 2.0);
    let norton = Stamp::Norton {
        g: 1.0,
        current: 6.0,
    };
    assert_eq!(component_current(norton, 4.0, None), 2.0);
    assert_eq!(
        component_current(Stamp::VoltageSource(6.0), 6.0, Some(-2.0)),
        2.0
    );
    assert_eq!(component_current(Stamp::Open, 6.0, None), 0.0);
    assert_eq!(component_current(Stamp::Merge, 0.0, None), 0.0);
}

/// Power is V × I, and a floating component's is zero.
#[test]
fn powers_multiply_and_treat_floating_as_zero() {
    assert_eq!(powers(&[Some(6.0), None], &[2.0, 5.0]), vec![12.0, 0.0]);
    assert!(powers(&[], &[]).is_empty());
}

/// Every component with a nonsensical value is named, in order.
#[test]
fn invalid_components_names_each_bad_part() {
    let mut c = Circuit::new();
    let (a, b) = (c.node(), c.node());
    c.add_resistor(a, b, 0.0);
    c.add_resistor(a, b, 1.0);
    c.add_bulb(a, b, f64::NAN, 1.0);
    let ids: Vec<CircuitIssue> = invalid_components(&c);
    assert_eq!(
        ids,
        vec![
            CircuitIssue::InvalidValue {
                component: ComponentId(0)
            },
            CircuitIssue::InvalidValue {
                component: ComponentId(2)
            },
        ]
    );
    assert!(invalid_components(&battery_and_resistor()).is_empty());
}

/// Blown bulbs are reported in component order; intact ones are not.
#[test]
fn blown_bulb_issues_lists_blown_bulbs_in_order() {
    let mut c = Circuit::new();
    let (a, b) = (c.node(), c.node());
    let first = c.add_bulb(a, b, 6.0, 3.0);
    c.add_bulb(a, b, 6.0, 3.0);
    let third = c.add_bulb(a, b, 6.0, 3.0);
    c.component_mut(third).blow();
    c.component_mut(first).blow();
    let issues: Vec<CircuitIssue> = blown_bulb_issues(&c).collect();
    assert_eq!(
        issues,
        vec![
            CircuitIssue::BulbBlown { bulb: first },
            CircuitIssue::BulbBlown { bulb: third },
        ]
    );
}

/// The unexpected-singular safety net names a node for a voltage pivot and
/// a battery for a current pivot.
#[test]
fn issue_for_pivot_names_node_or_battery() {
    let u = Unknowns {
        voltage: vec![None, Some(0)],
        source: vec![None, Some(1)],
        count: 2,
    };
    assert_eq!(
        issue_for_pivot(&u, 0),
        CircuitIssue::FloatingNode { node: NodeId(1) }
    );
    assert_eq!(
        issue_for_pivot(&u, 1),
        CircuitIssue::VoltageSourceLoop {
            battery: ComponentId(1)
        }
    );
}

/// A circuit with nothing in it has nothing to solve: no voltages, no
/// currents, and nothing wrong with it. Reporting a "solver failure" here
/// would tell a learner who has just cleared the board that something broke.
#[test]
fn empty_circuit_solves_with_no_issues() {
    let analysis = Circuit::new().solve();
    assert!(analysis.issues().is_empty(), "{:?}", analysis.issues());
}

/// The internal cache-miss path must be driven through CircuitSim's history
/// to prove it initializes properly.
#[test]
fn circuit_sim_solves_empty_circuit_cleanly() {
    let mut sim = crate::CircuitSim::new(Circuit::new());
    sim.step(simbiguity_core::Seconds(0.01));
    let analysis = sim.circuit().solve();
    assert!(analysis.issues().is_empty(), "{:?}", analysis.issues());
}

/// assemble_matrix bubbles up an error if an ideal battery lacks a current unknown.
#[test]
fn assemble_matrix_bubbles_up_solver_failure() {
    let mut c = Circuit::new();
    let (p, m) = (c.node(), c.node());
    c.add_battery(p, m, 6.0);
    let stamps = c
        .parts
        .iter()
        .map(|p| p.component.stamp())
        .collect::<Vec<_>>();
    let t = Topology::build(&c, stamps);
    let u = Unknowns {
        voltage: vec![None, Some(0)],
        source: vec![None],
        count: 1,
    };
    let mut matrix = DenseMatrix::zeros(1, 1);
    assert!(assemble_matrix(&c, &t, &u, &mut matrix).is_err());
}

/// assemble_rhs bubbles up an error if an ideal battery lacks a current unknown.
#[test]
fn assemble_rhs_bubbles_up_solver_failure() {
    let mut c = Circuit::new();
    let (p, m) = (c.node(), c.node());
    c.add_battery(p, m, 6.0);
    let stamps = c
        .parts
        .iter()
        .map(|p| p.component.stamp())
        .collect::<Vec<_>>();
    let t = Topology::build(&c, stamps);
    let u = Unknowns {
        voltage: vec![None, Some(0)],
        source: vec![None],
        count: 1,
    };
    let mut rhs = vec![0.0; 1];
    assert!(assemble_rhs(&c, &t, &u, &mut rhs).is_err());
}
