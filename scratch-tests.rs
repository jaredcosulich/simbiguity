/// SolverFailure when cache topology is missing
#[test]
fn solve_once_returns_failure_on_missing_topology() {
    let c = Circuit::new();
    let mut cache = SolverCache::default();
    let (solution, issues) = solve_once(&c, vec![], &mut cache, false);
    assert!(solution.is_none());
    assert_eq!(issues.len(), 1);
    assert!(matches!(issues[0], CircuitIssue::SolverFailure { .. }));
}

/// SolverFailure when unknowns are missing
#[test]
fn solve_once_returns_failure_on_missing_unknowns() {
    let c = Circuit::new();
    let mut cache = SolverCache::default();
    let stamps = vec![];
    cache.t = Some(Topology::build(&c, stamps.clone()));
    let (solution, issues) = solve_once(&c, stamps, &mut cache, false);
    assert!(solution.is_none());
    assert_eq!(issues.len(), 1);
    assert!(matches!(issues[0], CircuitIssue::SolverFailure { .. }));
}

/// assemble_matrix bubbles up an error if an ideal battery lacks a current unknown.
#[test]
fn assemble_matrix_bubbles_up_solver_failure() {
    let mut c = Circuit::new();
    let (p, m) = (c.node(), c.node());
    c.add_battery(p, m, 6.0);
    let stamps = c.parts.iter().map(|p| p.component.stamp()).collect::<Vec<_>>();
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
    let stamps = c.parts.iter().map(|p| p.component.stamp()).collect::<Vec<_>>();
    let t = Topology::build(&c, stamps);
    let u = Unknowns {
        voltage: vec![None, Some(0)],
        source: vec![None],
        count: 1,
    };
    let mut rhs = vec![0.0; 1];
    assert!(assemble_rhs(&c, &t, &u, &mut rhs).is_err());
}
