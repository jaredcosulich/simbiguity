//! How a [`CircuitIssue`] crosses the bridge: a kind code and a part id.

use simbiguity_circuits::CircuitIssue;

/// The number JS reads back for an issue's kind: 1 invalid value, 2 short
/// circuit, 3 battery loop, 4 floating node, 5 open circuit, 6 blown bulb.
/// `site/js/simbiguity.js` maps these to names in `IssueKind`.
pub fn issue_kind_code(issue: &CircuitIssue) -> i32 {
    match issue {
        CircuitIssue::SolverFailure { .. } => 7,
        CircuitIssue::InvalidValue { .. } => 1,
        CircuitIssue::ShortCircuit { .. } => 2,
        CircuitIssue::VoltageSourceLoop { .. } => 3,
        CircuitIssue::FloatingNode { .. } => 4,
        CircuitIssue::OpenCircuit { .. } => 5,
        CircuitIssue::BulbBlown { .. } => 6,
    }
}

/// The id of the part an issue points at: a component id, or a node id for
/// a floating node.
pub fn issue_part_index(issue: &CircuitIssue) -> u32 {
    (match issue {
        CircuitIssue::SolverFailure { .. } => 0,
        CircuitIssue::InvalidValue { component } => component.index(),
        CircuitIssue::ShortCircuit { battery, .. }
        | CircuitIssue::VoltageSourceLoop { battery }
        | CircuitIssue::OpenCircuit { battery } => battery.index(),
        CircuitIssue::FloatingNode { node } => node.index(),
        CircuitIssue::BulbBlown { bulb } => bulb.index(),
    }) as u32
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;
    use alloc::string::String;
    use simbiguity_circuits::CircuitIssue;

    /// Bridge monkey tests for solver failure mappings.
    #[test]
    fn issue_kind_code_solver_failure() {
        let issue = CircuitIssue::SolverFailure {
            message: String::from("boom"),
        };
        assert_eq!(issue_kind_code(&issue), 7);
    }

    /// Verifies that solver failures point to part index 0 since they lack a specific component.
    #[test]
    fn issue_part_index_solver_failure() {
        let issue = CircuitIssue::SolverFailure {
            message: String::from("boom"),
        };
        assert_eq!(issue_part_index(&issue), 0);
    }
}
