//! Modified Nodal Analysis: from a netlist to node voltages and currents.
//!
//! 1. **Merge** nodes joined by wires and closed switches (union-find).
//! 2. Find **islands** — pieces connected by anything that conducts.
//! 3. Catch what would make the system singular *before* solving, so it can
//!    be reported by name: shorted ideal batteries, loops of ideal batteries,
//!    islands with nothing to set their voltage.
//! 4. Pick a **reference** (0 V) node per island, number the unknowns, stamp
//!    the matrix, and solve it with LU decomposition.
//! 5. Recover each wire's current from Kirchhoff's current law.

use alloc::vec;
use alloc::vec::Vec;

use simbiguity_math::{float, DenseMatrix, SolveError};

use crate::component::{Component, Integrator, Stamp, BLOW_RATIO};
use crate::diagnostics::CircuitIssue;
use crate::dsu::Dsu;
use crate::netlist::{Circuit, ComponentId, NodeId};
use crate::solution::{Analysis, Solution};
use crate::wires::wire_currents;

/// A battery current at or below this many amps counts as "no current".
const NO_CURRENT: f64 = 1e-12;

/// The circuit's shape after merging, before any numbers are solved.
#[derive(Clone, PartialEq, Debug)]
struct Topology {
    stamps: Vec<Stamp>,
    /// Merged group (its root node) of each node.
    group: Vec<usize>,
    /// Island (its root node) of each node.
    island: Vec<usize>,
    /// The 0 V group of each island root, if the island has one.
    reference: Vec<Option<usize>>,
}

impl Topology {
    fn build(c: &Circuit, stamps: Vec<Stamp>) -> Topology {
        let n = c.node_count;
        let mut merge = Dsu::new(n);
        let mut islands = Dsu::new(n);
        for (p, s) in c.parts.iter().zip(&stamps) {
            if *s == Stamp::Merge {
                merge.union(p.a.0, p.b.0);
            }
            if *s != Stamp::Open {
                islands.union(p.a.0, p.b.0);
            }
        }
        let group: Vec<usize> = (0..n).map(|i| merge.find(i)).collect();
        let island: Vec<usize> = (0..n).map(|i| islands.find(i)).collect();
        let reference = Topology::references(c, &group, &island);
        Topology {
            stamps,
            group,
            island,
            reference,
        }
    }

    /// The 0 V group of each island root: the explicit ground for its own
    /// island, otherwise the `−` terminal of the island's first battery.
    fn references(c: &Circuit, group: &[usize], island: &[usize]) -> Vec<Option<usize>> {
        let mut reference = vec![None; c.node_count];
        if let Some(g) = c.ground {
            reference[island[g.0]] = Some(group[g.0]);
        }
        for p in &c.parts {
            if matches!(p.component, Component::Battery { .. })
                && reference[island[p.a.0]].is_none()
            {
                reference[island[p.a.0]] = Some(group[p.b.0]);
            }
        }
        reference
    }

    /// The reference group of the island `node` is in.
    fn reference_of(&self, node: usize) -> Option<usize> {
        self.reference[self.island[node]]
    }
}

/// Shorts and loops of ideal batteries. Returns the issues and whether any
/// of them is fatal.
fn source_problems(c: &Circuit, t: &Topology) -> Vec<CircuitIssue> {
    let mut issues = Vec::new();
    let mut sources = Dsu::new(c.node_count);
    for (i, (p, s)) in c.parts.iter().zip(&t.stamps).enumerate() {
        let battery = ComponentId(i);
        let (ga, gb) = (t.group[p.a.0], t.group[p.b.0]);
        match *s {
            Stamp::VoltageSource(_) if ga == gb => issues.push(CircuitIssue::ShortCircuit {
                battery,
                current: None,
            }),
            Stamp::Norton { current, .. } if ga == gb => issues.push(CircuitIssue::ShortCircuit {
                battery,
                current: Some(current),
            }),
            Stamp::VoltageSource(_) if !sources.union(ga, gb) => {
                issues.push(CircuitIssue::VoltageSourceLoop { battery })
            }
            _ => {}
        }
    }
    issues
}

/// Which nodes have at least one component attached.
fn touched_nodes(c: &Circuit) -> Vec<bool> {
    let mut touched = vec![false; c.node_count];
    for p in &c.parts {
        touched[p.a.0] = true;
        touched[p.b.0] = true;
    }
    touched
}

/// One `FloatingNode` per island that has components but no reference,
/// naming its lowest-numbered node.
fn floating_islands(c: &Circuit, t: &Topology) -> Vec<CircuitIssue> {
    let n = c.node_count;
    let touched = touched_nodes(c);
    let mut reported = vec![false; n];
    let mut issues = Vec::new();
    for node in 0..n {
        let isl = t.island[node];
        if touched[node] && t.reference[isl].is_none() && !reported[isl] {
            reported[isl] = true;
            issues.push(CircuitIssue::FloatingNode { node: NodeId(node) });
        }
    }
    issues
}

/// Numbering of the unknowns: one voltage per non-reference group in a
/// referenced island, then one current per ideal battery.
#[derive(Clone, PartialEq, Debug)]
struct Unknowns {
    voltage: Vec<Option<usize>>,
    source: Vec<Option<usize>>,
    count: usize,
}

impl Unknowns {
    fn number(c: &Circuit, t: &Topology) -> Unknowns {
        let n = c.node_count;
        let mut voltage = vec![None; n];
        let mut count = 0;
        for root in 0..n {
            if t.group[root] == root && t.reference_of(root).is_some_and(|r| r != root) {
                voltage[root] = Some(count);
                count += 1;
            }
        }
        let mut source = vec![None; c.parts.len()];
        for (i, s) in t.stamps.iter().enumerate() {
            if matches!(s, Stamp::VoltageSource(_)) {
                source[i] = Some(count);
                count += 1;
            }
        }
        Unknowns {
            voltage,
            source,
            count,
        }
    }
}

/// Adds conductance `g` between unknowns `a` and `b` (`None` = reference).
fn stamp_conductance(m: &mut DenseMatrix, a: Option<usize>, b: Option<usize>, g: f64) {
    if let Some(a) = a {
        m[(a, a)] += g;
    }
    if let Some(b) = b {
        m[(b, b)] += g;
    }
    if let (Some(a), Some(b)) = (a, b) {
        m[(a, b)] -= g;
        m[(b, a)] -= g;
    }
}

/// Adds a current source pushing `current` into `a` and out of `b`.
fn stamp_current_source(rhs: &mut [f64], a: Option<usize>, b: Option<usize>, current: f64) {
    if let Some(a) = a {
        rhs[a] += current;
    }
    if let Some(b) = b {
        rhs[b] -= current;
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct SolverCache {
    pub matrix: DenseMatrix,
    pub rhs: Vec<f64>,
    pub lu: Option<simbiguity_math::Lu>,
    t: Option<Topology>,
    u: Option<Unknowns>,
}

impl Default for SolverCache {
    fn default() -> Self {
        Self {
            matrix: DenseMatrix::zeros(0, 0),
            rhs: Vec::new(),
            lu: None,
            t: None,
            u: None,
        }
    }
}

fn assemble_matrix(c: &Circuit, t: &Topology, u: &Unknowns, m: &mut DenseMatrix) {
    for r in 0..u.count {
        for col in 0..u.count {
            m[(r, col)] = 0.0;
        }
    }
    for (i, (p, s)) in c.parts.iter().zip(&t.stamps).enumerate() {
        let a = u.voltage[t.group[p.a.0]];
        let b = u.voltage[t.group[p.b.0]];
        match *s {
            Stamp::Conductance(g) => stamp_conductance(m, a, b, g),
            Stamp::Norton { g, .. } => {
                stamp_conductance(m, a, b, g);
            }
            Stamp::VoltageSource(_) => {
                let k = u.source[i].expect("every ideal battery has a current unknown");
                for (node, sign) in [(a, 1.0), (b, -1.0)] {
                    if let Some(node) = node {
                        m[(node, k)] += sign;
                        m[(k, node)] += sign;
                    }
                }
            }
            Stamp::Merge | Stamp::Open => {}
        }
    }
}

fn assemble_rhs(c: &Circuit, t: &Topology, u: &Unknowns, rhs: &mut [f64]) {
    rhs.fill(0.0);
    for (i, (p, s)) in c.parts.iter().zip(&t.stamps).enumerate() {
        let a = u.voltage[t.group[p.a.0]];
        let b = u.voltage[t.group[p.b.0]];
        match *s {
            Stamp::Norton { current, .. } => {
                stamp_current_source(rhs, a, b, current);
            }
            Stamp::VoltageSource(emf) => {
                let k = u.source[i].expect("every ideal battery has a current unknown");
                rhs[k] = emf;
            }
            _ => {}
        }
    }
}

/// Names the part behind an unexpected singular pivot (a safety net: the
/// checks before solving should already have caught it).
fn issue_for_pivot(u: &Unknowns, pivot: usize) -> CircuitIssue {
    if let Some(node) = u.voltage.iter().position(|v| *v == Some(pivot)) {
        return CircuitIssue::FloatingNode { node: NodeId(node) };
    }
    let battery = u.source.iter().position(|v| *v == Some(pivot)).unwrap_or(0);
    CircuitIssue::VoltageSourceLoop {
        battery: ComponentId(battery),
    }
}

/// Current through one component from the voltage `d` across it. An ideal
/// battery's comes from its MNA unknown, which is the current into `+`, so
/// it is negated to give the current delivered out of `+`. Wires get theirs
/// later, from [`wire_currents`].
fn component_current(s: Stamp, d: f64, source_unknown: Option<f64>) -> f64 {
    match s {
        Stamp::Conductance(g) => d * g,
        Stamp::Norton { g, current } => current - g * d,
        Stamp::VoltageSource(_) => source_unknown.map_or(0.0, |x| -x),
        Stamp::Merge | Stamp::Open => 0.0,
    }
}

/// Power of each component, `V × I` (a floating component's is zero).
fn powers(across: &[Option<f64>], currents: &[f64]) -> Vec<f64> {
    across
        .iter()
        .zip(currents)
        .map(|(v, i)| v.unwrap_or(0.0) * i)
        .collect()
}

/// Turns the solved unknowns into per-node and per-component results.
fn build_solution(c: &Circuit, t: &Topology, u: &Unknowns, x: &[f64]) -> Solution {
    let voltage_of = |node: usize| -> Option<f64> {
        let g = t.group[node];
        t.reference_of(node)?;
        Some(u.voltage[g].map_or(0.0, |k| x[k]))
    };
    let node_voltages: Vec<Option<f64>> = (0..c.node_count).map(voltage_of).collect();
    let mut across = Vec::with_capacity(c.parts.len());
    let mut currents = Vec::with_capacity(c.parts.len());
    for (i, (p, s)) in c.parts.iter().zip(&t.stamps).enumerate() {
        let v = match (node_voltages[p.a.0], node_voltages[p.b.0]) {
            (Some(va), Some(vb)) => Some(va - vb),
            _ => None,
        };
        let source_current = u.source[i].map(|k| x[k]);
        currents.push(component_current(*s, v.unwrap_or(0.0), source_current));
        across.push(v);
    }
    wire_currents(c, &t.stamps, &mut currents);
    let powers = powers(&across, &currents);
    let brightness = c
        .parts
        .iter()
        .zip(&powers)
        .map(|(p, w)| p.component.brightness(*w))
        .collect();
    let energies = c
        .parts
        .iter()
        .zip(&across)
        .zip(&currents)
        .map(|((p, v), i)| match p.component {
            Component::Capacitor { farads, .. } => {
                0.5 * farads * {
                    let val = v.unwrap_or(0.0);
                    val * val
                }
            }
            Component::Inductor { henries, .. } => 0.5 * henries * { i * i },
            _ => 0.0,
        })
        .collect();
    Solution {
        node_voltages,
        across,
        currents,
        powers,
        energies,
        brightness,
    }
}

/// Batteries (with a non-zero EMF, not shorted) that deliver no current at
/// all, because nothing completes a loop for them.
fn open_batteries(c: &Circuit, issues: &[CircuitIssue], s: &Solution) -> Vec<CircuitIssue> {
    let shorted = |id: ComponentId| {
        issues
            .iter()
            .any(|i| matches!(i, CircuitIssue::ShortCircuit { battery, .. } if *battery == id))
    };
    c.parts
        .iter()
        .enumerate()
        .filter_map(|(i, p)| match p.component {
            Component::Battery { emf, .. }
                if emf != 0.0
                    && !shorted(ComponentId(i))
                    && float::abs(s.currents[i]) <= NO_CURRENT =>
            {
                Some(CircuitIssue::OpenCircuit {
                    battery: ComponentId(i),
                })
            }
            _ => None,
        })
        .collect()
}

/// Solves the circuit as it stands (no bulbs blow here).
fn solve_once(
    c: &Circuit,
    stamps: Vec<Stamp>,
    cache: &mut SolverCache,
    needs_refactor: bool,
) -> (Option<Solution>, Vec<CircuitIssue>) {
    if needs_refactor || cache.t.is_none() {
        let t = Topology::build(c, stamps);
        let u = Unknowns::number(c, &t);
        cache.t = Some(t.clone());
        cache.u = Some(u.clone());
    } else {
        if let Some(t) = cache.t.as_mut() {
            t.stamps = stamps;
        }
    }
    let (t, u) = (cache.t.clone().unwrap(), cache.u.clone().unwrap());

    let mut issues = source_problems(c, &t);
    if issues.iter().any(CircuitIssue::is_fatal) {
        return (None, issues);
    }
    issues.extend(floating_islands(c, &t));

    if needs_refactor || cache.matrix.rows() != u.count {
        if cache.matrix.rows() != u.count {
            cache.matrix = DenseMatrix::zeros(u.count, u.count);
        }
        assemble_matrix(c, &t, &u, &mut cache.matrix);
        cache.lu = match simbiguity_math::Lu::decompose(&cache.matrix) {
            Ok(lu) => Some(lu),
            Err(SolveError::Singular { pivot_row }) => {
                issues.push(issue_for_pivot(&u, pivot_row));
                return (None, issues);
            }
            Err(e) => unreachable!("MNA always builds a square system: {e}"),
        };
    }

    if cache.rhs.len() != u.count {
        cache.rhs = vec![0.0; u.count];
    }
    assemble_rhs(c, &t, &u, &mut cache.rhs);

    let x = match cache.lu.as_ref().unwrap().solve(&cache.rhs) {
        Ok(x) => x,
        Err(_) => return (None, issues),
    };
    let solution = build_solution(c, &t, &u, &x);
    let open = open_batteries(c, &issues, &solution);
    issues.extend(open);
    (Some(solution), issues)
}

/// The intact bulb drawing the most power relative to its limit, if any is
/// over it (ties go to the lowest id).
fn most_overloaded(c: &Circuit, s: &Solution) -> Option<usize> {
    let mut worst: Option<(usize, f64)> = None;
    for (i, p) in c.parts.iter().enumerate() {
        if let Component::Bulb {
            rated_power,
            blown: false,
            ..
        } = p.component
        {
            let ratio = s.powers[i] / rated_power;
            if ratio > BLOW_RATIO && worst.map_or(true, |(_, w)| ratio > w) {
                worst = Some((i, ratio));
            }
        }
    }
    worst.map(|(i, _)| i)
}

/// One `InvalidValue` per component whose values make no physical sense.
fn invalid_components(c: &Circuit) -> Vec<CircuitIssue> {
    c.parts
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.component.is_valid())
        .map(|(i, _)| CircuitIssue::InvalidValue {
            component: ComponentId(i),
        })
        .collect()
}

/// One `BulbBlown` per blown bulb, in component order.
fn blown_bulb_issues(c: &Circuit) -> impl Iterator<Item = CircuitIssue> + '_ {
    c.parts.iter().enumerate().filter_map(|(i, p)| {
        matches!(p.component, Component::Bulb { blown: true, .. }).then_some(
            CircuitIssue::BulbBlown {
                bulb: ComponentId(i),
            },
        )
    })
}

pub(crate) fn analyze_step(
    c: &Circuit,
    dt: f64,
    integrator: Integrator,
    cache: &mut SolverCache,
    mut needs_refactor: bool,
) -> Analysis {
    let invalid = invalid_components(c);
    if !invalid.is_empty() {
        return Analysis {
            solution: None,
            issues: invalid,
        };
    }
    let mut work = c.clone();
    loop {
        let stamps: Vec<Stamp> = work
            .parts
            .iter()
            .map(|p| p.component.transient_stamp(dt, integrator))
            .collect();
        let (solution, mut issues) = solve_once(&work, stamps, cache, needs_refactor);
        let Some(solution) = solution else {
            return Analysis {
                solution: None,
                issues,
            };
        };
        if let Some(i) = most_overloaded(&work, &solution) {
            work.parts[i].component.blow();
            needs_refactor = true;
            continue;
        }
        issues.extend(blown_bulb_issues(&work));
        return Analysis {
            solution: Some(solution),
            issues,
        };
    }
}

/// Full analysis: validate, solve, blow overloaded bulbs one at a time
/// (re-solving after each), and collect every issue.
pub(crate) fn analyze(c: &Circuit) -> Analysis {
    let invalid = invalid_components(c);
    if !invalid.is_empty() {
        return Analysis {
            solution: None,
            issues: invalid,
        };
    }
    let mut work = c.clone();
    let mut cache = SolverCache::default();
    let mut needs_refactor = true;
    loop {
        let stamps: Vec<Stamp> = work.parts.iter().map(|p| p.component.stamp()).collect();
        let (solution, mut issues) = solve_once(&work, stamps, &mut cache, needs_refactor);
        let Some(solution) = solution else {
            return Analysis {
                solution: None,
                issues,
            };
        };
        if let Some(i) = most_overloaded(&work, &solution) {
            work.parts[i].component.blow();
            needs_refactor = true;
            continue;
        }
        issues.extend(blown_bulb_issues(&work));
        return Analysis {
            solution: Some(solution),
            issues,
        };
    }
}

#[cfg(test)]
pub(crate) fn stamp_voltage_source(
    m: &mut DenseMatrix,
    rhs: &mut [f64],
    a: Option<usize>,
    b: Option<usize>,
    k: usize,
    emf: f64,
) {
    for (node, sign) in [(a, 1.0), (b, -1.0)] {
        if let Some(node) = node {
            m[(node, k)] += sign;
            m[(k, node)] += sign;
        }
    }
    rhs[k] = emf;
}

#[cfg(test)]
#[path = "mna_tests.rs"]
mod tests;
