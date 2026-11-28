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
use crate::netlist::{Circuit, ComponentId, NodeId, SolverKind};
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
                if let Some(c_node) = p.c {
                    islands.union(p.a.0, c_node.0);
                }
                if let Some(d_node) = p.d {
                    islands.union(p.a.0, d_node.0);
                }
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
            if matches!(
                p.component,
                Component::Battery { .. } | Component::SignalSource { .. }
            ) && reference[island[p.a.0]].is_none()
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
            Stamp::VoltageSource(_) | Stamp::Vcvs { .. } if ga == gb => {
                issues.push(CircuitIssue::ShortCircuit {
                    battery,
                    current: None,
                })
            }
            Stamp::Norton { current, .. } if ga == gb => issues.push(CircuitIssue::ShortCircuit {
                battery,
                current: Some(current),
            }),
            Stamp::VoltageSource(_) | Stamp::Vcvs { .. } if !sources.union(ga, gb) => {
                issues.push(CircuitIssue::VoltageSourceLoop { battery })
            }
            _ => {}
        }
    }
    for (i, p) in c.parts.iter().enumerate() {
        if let Component::Capacitor { volts, .. } = p.component {
            if volts == 0.0 {
                let (ga, gb) = (t.group[p.a.0], t.group[p.b.0]);
                if ga != gb && sources.find(ga) == sources.find(gb) {
                    issues.push(CircuitIssue::Inrush {
                        capacitor: ComponentId(i),
                    });
                }
            }
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
        if let Some(c_node) = p.c {
            touched[c_node.0] = true;
        }
        if let Some(d_node) = p.d {
            touched[d_node.0] = true;
        }
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
    for (node, &isl) in t.island.iter().enumerate() {
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
        for (root, &group) in t.group.iter().enumerate() {
            if group == root && t.reference_of(root).is_some_and(|r| r != root) {
                voltage[root] = Some(count);
                count += 1;
            }
        }
        let mut source = vec![None; c.parts.len()];
        for (i, s) in t.stamps.iter().enumerate() {
            if matches!(
                s,
                Stamp::VoltageSource(_) | Stamp::Vcvs { .. } | Stamp::OpAmp { .. }
            ) {
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
    pub sparse_matrix: Option<simbiguity_math::SparseMatrix>,
    pub rhs: Vec<f64>,
    pub lu: Option<simbiguity_math::Lu>,
    pub sparse_sym: Option<simbiguity_math::SymbolicLu>,
    pub sparse_lu: Option<simbiguity_math::SparseLu>,
    pub solver_kind: Option<SolverKind>,
    t: Option<Topology>,
    u: Option<Unknowns>,
    dt: Option<f64>,
    integrator: Option<Integrator>,
}

impl Default for SolverCache {
    fn default() -> Self {
        Self {
            matrix: DenseMatrix::zeros(0, 0),
            sparse_matrix: None,
            rhs: Vec::new(),
            lu: None,
            sparse_sym: None,
            sparse_lu: None,
            solver_kind: None,
            t: None,
            u: None,
            dt: None,
            integrator: None,
        }
    }
}

fn assemble_matrix(
    c: &Circuit,
    t: &Topology,
    u: &Unknowns,
    m: &mut DenseMatrix,
) -> Result<(), alloc::string::String> {
    for r in 0..u.count {
        for col in 0..u.count {
            m[(r, col)] = 0.0;
        }
    }
    for (i, (p, s)) in c.parts.iter().zip(&t.stamps).enumerate() {
        let a = u.voltage[t.group[p.a.0]];
        let b = u.voltage[t.group[p.b.0]];
        let c_node = p.c.and_then(|n| u.voltage[t.group[n.0]]);
        match *s {
            Stamp::Conductance(g) => stamp_conductance(m, a, b, g),
            Stamp::Norton { g, .. } => {
                stamp_conductance(m, a, b, g);
            }
            Stamp::Bjt {
                g_b_be,
                g_b_bc,
                g_c_be,
                g_c_bc,
                ..
            } => {
                let e_term = c_node;
                let c_term = a;
                let b_term = b;

                let mut add_m = |row: Option<usize>, col: Option<usize>, val: f64| {
                    if let (Some(r), Some(c)) = (row, col) {
                        m[(r, c)] += val;
                    }
                };
                add_m(b_term, b_term, g_b_be + g_b_bc);
                add_m(b_term, e_term, -g_b_be);
                add_m(b_term, c_term, -g_b_bc);

                add_m(c_term, b_term, g_c_be + g_c_bc);
                add_m(c_term, e_term, -g_c_be);
                add_m(c_term, c_term, -g_c_bc);

                add_m(e_term, b_term, -(g_b_be + g_b_bc + g_c_be + g_c_bc));
                add_m(e_term, e_term, g_b_be + g_c_be);
                add_m(e_term, c_term, g_b_bc + g_c_bc);
            }
            Stamp::VoltageSource(_) => {
                let Some(k) = u.source[i] else {
                    return Err("every ideal battery has a current unknown".into());
                };
                for (node, sign) in [(a, 1.0), (b, -1.0)] {
                    if let Some(node) = node {
                        m[(node, k)] += sign;
                        m[(k, node)] += sign;
                    }
                }
            }
            Stamp::Vcvs { gain, .. } => {
                let Some(k) = u.source[i] else {
                    return Err("every VCVS has a current unknown".into());
                };
                let ctrl_plus = p.c.and_then(|n| u.voltage[t.group[n.0]]);
                let ctrl_minus = p.d.and_then(|n| u.voltage[t.group[n.0]]);
                for (node, sign) in [(a, 1.0), (b, -1.0), (ctrl_plus, -gain), (ctrl_minus, gain)] {
                    if let Some(node) = node {
                        m[(k, node)] += sign;
                    }
                }
                for (node, sign) in [(a, 1.0), (b, -1.0)] {
                    if let Some(node) = node {
                        m[(node, k)] += sign;
                    }
                }
            }
            Stamp::OpAmp { eq_gain, .. } => {
                let Some(k) = u.source[i] else {
                    return Err("every OpAmp has a current unknown".into());
                };
                let out = p.c.and_then(|n| u.voltage[t.group[n.0]]);
                for (node, sign) in [(out, 1.0), (a, -eq_gain), (b, eq_gain)] {
                    if let Some(node) = node {
                        m[(k, node)] += sign;
                    }
                }
                if let Some(node) = out {
                    m[(node, k)] += 1.0;
                }
            }
            Stamp::Merge | Stamp::Open => {}
        }
    }
    Ok(())
}

fn assemble_sparse_matrix(
    c: &Circuit,
    t: &Topology,
    u: &Unknowns,
) -> Result<simbiguity_math::SparseMatrix, alloc::string::String> {
    let mut m = simbiguity_math::SparseMatrix::new(u.count, u.count);
    for (i, (p, s)) in c.parts.iter().zip(&t.stamps).enumerate() {
        let a = u.voltage[t.group[p.a.0]];
        let b = u.voltage[t.group[p.b.0]];
        let c_node = p.c.and_then(|n| u.voltage[t.group[n.0]]);
        match *s {
            Stamp::Conductance(g) | Stamp::Norton { g, .. } => {
                if let Some(a) = a {
                    m.add(a, a, g);
                }
                if let Some(b) = b {
                    m.add(b, b, g);
                }
                if let (Some(a), Some(b)) = (a, b) {
                    m.add(a, b, -g);
                    m.add(b, a, -g);
                }
            }
            Stamp::Bjt {
                g_b_be,
                g_b_bc,
                g_c_be,
                g_c_bc,
                ..
            } => {
                let e_term = c_node;
                let c_term = a;
                let b_term = b;

                let mut add_m = |row: Option<usize>, col: Option<usize>, val: f64| {
                    if let (Some(r), Some(c)) = (row, col) {
                        m.add(r, c, val);
                    }
                };
                add_m(b_term, b_term, g_b_be + g_b_bc);
                add_m(b_term, e_term, -g_b_be);
                add_m(b_term, c_term, -g_b_bc);

                add_m(c_term, b_term, g_c_be + g_c_bc);
                add_m(c_term, e_term, -g_c_be);
                add_m(c_term, c_term, -g_c_bc);

                add_m(e_term, b_term, -(g_b_be + g_b_bc + g_c_be + g_c_bc));
                add_m(e_term, e_term, g_b_be + g_c_be);
                add_m(e_term, c_term, g_b_bc + g_c_bc);
            }
            Stamp::VoltageSource(_) => {
                let Some(k) = u.source[i] else {
                    return Err("every ideal battery has a current unknown".into());
                };
                for (node, sign) in [(a, 1.0), (b, -1.0)] {
                    if let Some(node) = node {
                        m.add(node, k, sign);
                        m.add(k, node, sign);
                    }
                }
            }
            Stamp::Vcvs { gain, .. } => {
                let Some(k) = u.source[i] else {
                    return Err("every VCVS has a current unknown".into());
                };
                let ctrl_plus = p.c.and_then(|n| u.voltage[t.group[n.0]]);
                let ctrl_minus = p.d.and_then(|n| u.voltage[t.group[n.0]]);
                for (node, sign) in [(a, 1.0), (b, -1.0), (ctrl_plus, -gain), (ctrl_minus, gain)] {
                    if let Some(node) = node {
                        m.add(k, node, sign);
                    }
                }
                for (node, sign) in [(a, 1.0), (b, -1.0)] {
                    if let Some(node) = node {
                        m.add(node, k, sign);
                    }
                }
            }
            Stamp::OpAmp { eq_gain, .. } => {
                let Some(k) = u.source[i] else {
                    return Err("every OpAmp has a current unknown".into());
                };
                let out = p.c.and_then(|n| u.voltage[t.group[n.0]]);
                for (node, sign) in [(out, 1.0), (a, -eq_gain), (b, eq_gain)] {
                    if let Some(node) = node {
                        m.add(k, node, sign);
                    }
                }
                if let Some(node) = out {
                    m.add(node, k, 1.0);
                }
            }
            Stamp::Merge | Stamp::Open => {}
        }
    }
    Ok(m)
}

fn assemble_rhs(
    c: &Circuit,
    t: &Topology,
    u: &Unknowns,
    rhs: &mut [f64],
) -> Result<(), alloc::string::String> {
    rhs.fill(0.0);
    for (i, (p, s)) in c.parts.iter().zip(&t.stamps).enumerate() {
        let a = u.voltage[t.group[p.a.0]];
        let b = u.voltage[t.group[p.b.0]];
        let c_node = p.c.and_then(|n| u.voltage[t.group[n.0]]);
        match *s {
            Stamp::Norton { current, .. } => {
                stamp_current_source(rhs, a, b, current);
            }
            Stamp::Bjt { i_eq_b, i_eq_c, .. } => {
                let e = c_node;
                let c_term = a;
                let b_term = b;
                if let Some(n) = b_term {
                    rhs[n] -= i_eq_b;
                }
                if let Some(n) = c_term {
                    rhs[n] -= i_eq_c;
                }
                if let Some(n) = e {
                    rhs[n] += i_eq_b + i_eq_c;
                }
            }
            Stamp::VoltageSource(emf) => {
                let Some(k) = u.source[i] else {
                    return Err("every ideal battery has a current unknown".into());
                };
                rhs[k] = emf;
            }
            Stamp::Vcvs { offset, .. } | Stamp::OpAmp { offset, .. } => {
                let Some(k) = u.source[i] else {
                    return Err("every VCVS/OpAmp has a current unknown".into());
                };
                rhs[k] = offset;
            }
            _ => {}
        }
    }
    Ok(())
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

/// Current through one component from the voltage `d` across it, in the passive
/// convention (current into `+`). Wires get theirs later, from [`wire_currents`].
fn component_current(s: Stamp, d: f64, source_unknown: Option<f64>) -> f64 {
    match s {
        Stamp::Conductance(g) => d * g,
        Stamp::Norton { g, current } => d * g - current,
        Stamp::VoltageSource(_) | Stamp::Vcvs { .. } | Stamp::OpAmp { .. } => {
            source_unknown.unwrap_or(0.0)
        }
        Stamp::Merge | Stamp::Open | Stamp::Bjt { .. } => 0.0,
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
        let v = match p.component {
            Component::OpAmp { .. } => p.c.and_then(|out| node_voltages[out.0]),
            _ => match (node_voltages[p.a.0], node_voltages[p.b.0]) {
                (Some(va), Some(vb)) => Some(va - vb),
                _ => None,
            },
        };
        let source_current = u.source[i].map(|k| x[k]);
        let mut i_val = component_current(*s, v.unwrap_or(0.0), source_current);
        if matches!(p.component, Component::Battery { .. }) {
            i_val = -i_val; // Present batteries in the active convention (delivering current)
        }
        currents.push(i_val);
        across.push(v);
    }
    wire_currents(c, &t.stamps, &mut currents, &node_voltages);
    let powers = powers(&across, &currents);
    let brightness = c
        .parts
        .iter()
        .zip(&powers)
        .zip(&currents)
        .map(|((p, w), i)| p.component.brightness(*w, *i))
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
        newton_iterations: 0,
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

fn pnjlim(vnew: f64, vold: f64, vt: f64, vcrit: f64) -> f64 {
    if vnew > vcrit && vnew > 0.0 {
        if vold > 0.0 {
            let arg = 1.0 + (vnew - vold) / vt;
            if arg > 0.0 {
                vold + vt * float::ln(arg)
            } else {
                vcrit
            }
        } else {
            vt * float::ln(vnew / vt)
        }
    } else {
        vnew
    }
}

fn linearize_diode(is: f64, n: f64, v: f64, vt_base: f64) -> Stamp {
    let vt = n * vt_base;
    let exp_term = float::exp(v / vt);
    let g = (is / vt) * exp_term;
    let i_v = is * (exp_term - 1.0);
    let i_eq = i_v - g * v;
    Stamp::Norton { g, current: -i_eq }
}

fn check_diode_convergence(is: f64, n: f64, v: f64, actual_i: f64, vt_base: f64) -> bool {
    let vt = n * vt_base;
    let exp_term = float::exp(v / vt);
    let expected_i = is * (exp_term - 1.0);
    let diff = (expected_i - actual_i).abs();
    diff <= 1e-12 + 1e-6 * actual_i.abs()
}

fn limit_diode_voltage(is: f64, n: f64, vnew: f64, vold: f64, vt_base: f64) -> f64 {
    let vt = n * vt_base;
    let vcrit = vt * float::ln(vt / (core::f64::consts::SQRT_2 * is));
    pnjlim(vnew, vold, vt, vcrit)
}

fn linearize_opamp(gain: f64, v_sat: f64, v_in_old: f64) -> Stamp {
    let x = (gain * v_in_old) / v_sat;
    let t = float::tanh(x);
    let v_out_actual = v_sat * t;
    let eq_gain = gain * (1.0 - t * t);
    let offset = v_out_actual - eq_gain * v_in_old;
    Stamp::OpAmp { eq_gain, offset }
}

fn check_opamp_convergence(gain: f64, v_sat: f64, v_in: f64, v_out: f64) -> bool {
    let x = (gain * v_in) / v_sat;
    let expected_out = v_sat * float::tanh(x);
    float::abs(expected_out - v_out) <= 1e-3
}

fn linearize_bjt(
    polarity: crate::component::Polarity,
    is: f64,
    beta_f: f64,
    beta_r: f64,
    v_be_old: f64,
    v_bc_old: f64,
    vt: f64,
) -> Stamp {
    let exp_be = float::exp(v_be_old / vt);
    let exp_bc = float::exp(v_bc_old / vt);

    let d_ic_d_vbe = (is / vt) * exp_be;
    let d_ic_d_vbc = -(is / vt) * exp_bc * (1.0 + 1.0 / beta_r);

    let d_ib_d_vbe = (is / vt) * exp_be / beta_f;
    let d_ib_d_vbc = (is / vt) * exp_bc / beta_r;

    let i_c_actual = is * (exp_be - exp_bc) - (is / beta_r) * (exp_bc - 1.0);
    let i_b_actual = (is / beta_f) * (exp_be - 1.0) + (is / beta_r) * (exp_bc - 1.0);

    let mut i_b = i_b_actual;
    let mut i_c = i_c_actual;

    if polarity == crate::component::Polarity::Pnp {
        i_b = -i_b;
        i_c = -i_c;
    }

    // For both NPN and PNP, the Jacobian is the same because for PNP:
    // I_B_in = - I_B(v_e - v_b, v_c - v_b)
    // and we take derivatives w.r.t (v_b - v_e) and (v_b - v_c).
    // Let's verify for PNP: v_be = -(v_b - v_e)
    // d I_B_in / d (v_b - v_e) = d (-I_B) / d (-v_be) = d I_B / d v_be = d_ib_d_vbe. Yes, identical.

    let (v_be_for_eq, v_bc_for_eq) = if polarity == crate::component::Polarity::Pnp {
        (-v_be_old, -v_bc_old)
    } else {
        (v_be_old, v_bc_old)
    };
    let i_eq_b = i_b - d_ib_d_vbe * v_be_for_eq - d_ib_d_vbc * v_bc_for_eq;
    let i_eq_c = i_c - d_ic_d_vbe * v_be_for_eq - d_ic_d_vbc * v_bc_for_eq;

    Stamp::Bjt {
        g_b_be: d_ib_d_vbe,
        g_b_bc: d_ib_d_vbc,
        g_c_be: d_ic_d_vbe,
        g_c_bc: d_ic_d_vbc,
        i_eq_b,
        i_eq_c,
    }
}

fn limit_bjt_voltages(
    is: f64,
    v_be_new: f64,
    v_bc_new: f64,
    v_be_old: f64,
    v_bc_old: f64,
    vt: f64,
) -> (f64, f64) {
    let vcrit = vt * float::ln(vt / (core::f64::consts::SQRT_2 * is));
    (
        pnjlim(v_be_new, v_be_old, vt, vcrit),
        pnjlim(v_bc_new, v_bc_old, vt, vcrit),
    )
}

fn check_bjt_convergence(
    _is: f64,
    _beta_f: f64,
    _beta_r: f64,
    _v_be: f64,
    _v_bc: f64,
    _vt: f64,
) -> bool {
    // We just rely on voltage convergence for BJT, similar to SPICE.
    true
}

fn check_voltage_convergence(prev: &Solution, current: &Solution) -> bool {
    for (v_new_opt, v_old_opt) in current.node_voltages.iter().zip(&prev.node_voltages) {
        if let (Some(v_new), Some(v_old)) = (v_new_opt, v_old_opt) {
            let dv = (v_new - v_old).abs();
            if dv > 1e-9 + 1e-6 * v_new.abs() {
                return false;
            }
        }
    }
    true
}

/// Solves the circuit as it stands (no bulbs blow here).
fn solve_once(
    c: &Circuit,
    base_stamps: Vec<Stamp>,
    cache: &mut SolverCache,
    mut needs_refactor: bool,
) -> (Option<Solution>, Vec<CircuitIssue>) {
    let vt_base = 0.025852;
    let mut v_junctions = alloc::vec![(0.6, 0.6); c.parts.len()];

    let has_nonlinear = c.parts.iter().any(|p| p.component.is_nonlinear());

    let max_iter = if has_nonlinear { 100 } else { 1 };
    let mut last_solution: Option<Solution> = None;
    let mut last_issues = alloc::vec::Vec::new();

    for iteration in 1..=max_iter {
        let mut stamps = base_stamps.clone();

        for (i, p) in c.parts.iter().enumerate() {
            if let Some((is, n)) = p.component.diode_params() {
                stamps[i] = linearize_diode(is, n, v_junctions[i].0, vt_base);
            } else if let Component::Bjt {
                polarity,
                saturation_amps,
                beta_f,
                beta_r,
            } = p.component
            {
                stamps[i] = linearize_bjt(
                    polarity,
                    saturation_amps,
                    beta_f,
                    beta_r,
                    v_junctions[i].0,
                    v_junctions[i].1,
                    vt_base,
                );
            } else if let Component::OpAmp { gain, v_sat } = p.component {
                stamps[i] = linearize_opamp(gain, v_sat, v_junctions[i].0);
            }
        }

        let (t, u) = match (cache.t.clone(), cache.u.clone()) {
            (Some(mut t), Some(u)) if !needs_refactor => {
                t.stamps = stamps;
                (t, u)
            }
            _ => {
                let t = Topology::build(c, stamps);
                let u = Unknowns::number(c, &t);
                (t, u)
            }
        };

        let mut issues = source_problems(c, &t);
        if issues.iter().any(CircuitIssue::is_fatal) {
            cache.t = None;
            cache.u = None;
            cache.lu = None;
            return (None, issues);
        }
        issues.extend(floating_islands(c, &t));

        let active_solver = match c.solver {
            SolverKind::Auto => {
                if u.count >= 64 {
                    SolverKind::Sparse
                } else {
                    SolverKind::Dense
                }
            }
            other => other,
        };

        if needs_refactor
            || cache.matrix.rows() != u.count
            || cache.solver_kind != Some(active_solver)
        {
            cache.solver_kind = Some(active_solver);
            if active_solver == SolverKind::Dense {
                if cache.matrix.rows() != u.count {
                    cache.matrix = DenseMatrix::zeros(u.count, u.count);
                }
                if let Err(e) = assemble_matrix(c, &t, &u, &mut cache.matrix) {
                    issues.push(CircuitIssue::SolverFailure { message: e });
                    cache.t = None;
                    cache.u = None;
                    cache.lu = None;
                    cache.solver_kind = None;
                    return (None, issues);
                }
                match simbiguity_math::Lu::decompose(&cache.matrix) {
                    Ok(lu) => {
                        cache.lu = Some(lu);
                        cache.sparse_lu = None;
                    }
                    Err(SolveError::Singular { pivot_row }) => {
                        issues.push(issue_for_pivot(&u, pivot_row));
                        cache.t = None;
                        cache.u = None;
                        cache.lu = None;
                        cache.solver_kind = None;
                        return (None, issues);
                    }
                    Err(e) => {
                        issues.push(CircuitIssue::SolverFailure {
                            message: alloc::format!("MNA always builds a square system: {e}"),
                        });
                        cache.t = None;
                        cache.u = None;
                        cache.lu = None;
                        cache.solver_kind = None;
                        return (None, issues);
                    }
                };
            } else {
                match assemble_sparse_matrix(c, &t, &u) {
                    Ok(mat) => {
                        let sym = cache
                            .sparse_sym
                            .take()
                            .filter(|s| s.n == u.count)
                            .unwrap_or_else(|| simbiguity_math::SymbolicLu::new(&mat));

                        if let Some(mut slu) = cache.sparse_lu.take() {
                            if slu.n == u.count {
                                match slu.refactor(&mat) {
                                    Ok(_) => {
                                        cache.sparse_lu = Some(slu);
                                        cache.sparse_sym = Some(sym.clone());
                                        cache.sparse_matrix = Some(mat.clone());
                                    }
                                    Err(SolveError::Singular { pivot_row }) => {
                                        issues.push(issue_for_pivot(&u, pivot_row));
                                        cache.t = None;
                                        cache.u = None;
                                        cache.sparse_lu = None;
                                        cache.solver_kind = None;
                                        return (None, issues);
                                    }
                                    Err(e) => {
                                        issues.push(CircuitIssue::SolverFailure {
                                            message: alloc::format!("MNA error: {e}"),
                                        });
                                        cache.t = None;
                                        cache.u = None;
                                        cache.sparse_lu = None;
                                        cache.solver_kind = None;
                                        return (None, issues);
                                    }
                                }
                            }
                        }

                        if cache.sparse_lu.is_none() {
                            match simbiguity_math::SparseLu::factor(&sym, &mat) {
                                Ok(slu) => {
                                    cache.sparse_lu = Some(slu);
                                    cache.sparse_sym = Some(sym);
                                    cache.sparse_matrix = Some(mat);
                                    cache.lu = None;
                                }
                                Err(SolveError::Singular { pivot_row }) => {
                                    issues.push(issue_for_pivot(&u, pivot_row));
                                    cache.t = None;
                                    cache.u = None;
                                    cache.sparse_lu = None;
                                    cache.solver_kind = None;
                                    return (None, issues);
                                }
                                Err(e) => {
                                    issues.push(CircuitIssue::SolverFailure {
                                        message: alloc::format!("MNA error: {e}"),
                                    });
                                    cache.t = None;
                                    cache.u = None;
                                    cache.sparse_lu = None;
                                    cache.solver_kind = None;
                                    return (None, issues);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        issues.push(CircuitIssue::SolverFailure { message: e });
                        cache.t = None;
                        cache.u = None;
                        cache.sparse_lu = None;
                        cache.solver_kind = None;
                        return (None, issues);
                    }
                }
            }
        }

        if cache.rhs.len() != u.count {
            cache.rhs = vec![0.0; u.count];
        }
        if let Err(e) = assemble_rhs(c, &t, &u, &mut cache.rhs) {
            issues.push(CircuitIssue::SolverFailure { message: e });
            cache.t = None;
            cache.u = None;
            cache.lu = None;
            return (None, issues);
        }

        let x_res = if active_solver == SolverKind::Dense {
            if let Some(lu) = cache.lu.as_ref() {
                lu.solve(&cache.rhs)
            } else {
                Err(SolveError::DimensionMismatch {
                    expected: 0,
                    got: 0,
                })
            }
        } else {
            if let Some(slu) = cache.sparse_lu.as_ref() {
                slu.solve(&cache.rhs)
            } else {
                Err(SolveError::DimensionMismatch {
                    expected: 0,
                    got: 0,
                })
            }
        };

        let x = match x_res {
            Ok(x) => x,
            Err(e) => {
                issues.push(CircuitIssue::SolverFailure {
                    message: alloc::format!("LU solve failed: {e}"),
                });
                cache.t = None;
                cache.u = None;
                cache.lu = None;
                return (None, issues);
            }
        };
        cache.t = Some(t.clone());
        cache.u = Some(u.clone());
        let mut solution = build_solution(c, &t, &u, &x);

        let mut converged;
        if let Some(ref prev) = last_solution {
            converged = check_voltage_convergence(prev, &solution);
            if converged {
                for (i, p) in c.parts.iter().enumerate() {
                    if let Some((is, n)) = p.component.diode_params() {
                        let v = solution.across[i].unwrap_or(0.0);
                        if !check_diode_convergence(is, n, v, solution.currents[i], vt_base) {
                            converged = false;
                            break;
                        }
                    } else if let Component::Bjt {
                        polarity,
                        saturation_amps,
                        beta_f,
                        beta_r,
                    } = p.component
                    {
                        let vb = solution.node_voltages[p.b.0].unwrap_or(0.0);
                        let ve = p.c.and_then(|c| solution.node_voltages[c.0]).unwrap_or(0.0);
                        let vc = solution.node_voltages[p.a.0].unwrap_or(0.0);
                        let (v_be_new, v_bc_new) = match polarity {
                            crate::component::Polarity::Npn => (vb - ve, vb - vc),
                            crate::component::Polarity::Pnp => (ve - vb, vc - vb),
                        };
                        if !check_bjt_convergence(
                            saturation_amps,
                            beta_f,
                            beta_r,
                            v_be_new,
                            v_bc_new,
                            vt_base,
                        ) {
                            converged = false;
                            break;
                        }
                    } else if let Component::OpAmp { gain, v_sat } = p.component {
                        let ctrl_plus = p.a;
                        let ctrl_minus = p.b;
                        if let Some(out) = p.c {
                            let v_plus = solution.node_voltages[ctrl_plus.0].unwrap_or(0.0);
                            let v_minus = solution.node_voltages[ctrl_minus.0].unwrap_or(0.0);
                            let v_out = solution.node_voltages[out.0].unwrap_or(0.0);
                            if !check_opamp_convergence(gain, v_sat, v_plus - v_minus, v_out) {
                                converged = false;
                                break;
                            }
                        }
                    }
                }
            }
        } else {
            converged = !has_nonlinear;
        }

        if converged {
            solution.newton_iterations = iteration;
            let open = open_batteries(c, &issues, &solution);
            issues.extend(open);
            return (Some(solution), issues);
        }

        last_solution = Some(solution.clone());
        last_issues = issues;

        for (i, p) in c.parts.iter().enumerate() {
            if let Some((is, n)) = p.component.diode_params() {
                let vnew = solution.across[i].unwrap_or(0.0);
                v_junctions[i].0 = limit_diode_voltage(is, n, vnew, v_junctions[i].0, vt_base);
            } else if let Component::Bjt {
                polarity,
                saturation_amps,
                ..
            } = p.component
            {
                let vb = solution.node_voltages[p.b.0].unwrap_or(0.0);
                let ve = p.c.and_then(|c| solution.node_voltages[c.0]).unwrap_or(0.0);
                let vc = solution.node_voltages[p.a.0].unwrap_or(0.0);
                let (v_be_new, v_bc_new) = match polarity {
                    crate::component::Polarity::Npn => (vb - ve, vb - vc),
                    crate::component::Polarity::Pnp => (ve - vb, vc - vb),
                };
                let (limited_be, limited_bc) = limit_bjt_voltages(
                    saturation_amps,
                    v_be_new,
                    v_bc_new,
                    v_junctions[i].0,
                    v_junctions[i].1,
                    vt_base,
                );
                v_junctions[i] = (limited_be, limited_bc);
            } else if let Component::OpAmp { .. } = p.component {
                let v_plus = solution.node_voltages[p.a.0].unwrap_or(0.0);
                let v_minus = solution.node_voltages[p.b.0].unwrap_or(0.0);
                let mut v_in_new = v_plus - v_minus;
                if v_in_new * v_junctions[i].0 < 0.0 {
                    v_in_new = 0.0;
                }
                v_junctions[i].0 = v_in_new;
            }
        }

        needs_refactor = true;
    } // end for loop

    last_issues.push(CircuitIssue::SolverFailure {
        message: alloc::string::String::from("Newton-Raphson did not converge"),
    });
    (last_solution, last_issues)
}

/// The intact bulb drawing the most power relative to its limit, if any is
/// over it (ties go to the lowest id).
fn most_overloaded(c: &Circuit, s: &Solution) -> Option<usize> {
    let mut worst: Option<(usize, f64)> = None;
    for (i, p) in c.parts.iter().enumerate() {
        match p.component {
            Component::Bulb {
                rated_power,
                blown: false,
                ..
            } => {
                let ratio = s.powers[i] / rated_power;
                if ratio > BLOW_RATIO && worst.map_or(true, |(_, w)| ratio > w) {
                    worst = Some((i, ratio));
                }
            }
            Component::Led {
                rated_amps,
                blown: false,
                ..
            } => {
                let ratio = s.currents[i].abs() / rated_amps;
                if ratio > 1.5 && worst.map_or(true, |(_, w)| ratio > w) {
                    worst = Some((i, ratio));
                }
            }
            _ => {}
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
        if matches!(
            p.component,
            Component::Bulb { blown: true, .. } | Component::Led { blown: true, .. }
        ) {
            Some(CircuitIssue::BulbBlown {
                bulb: ComponentId(i),
            })
        } else {
            None
        }
    })
}

pub(crate) fn analyze_step(
    c: &Circuit,
    dt: f64,
    integrator: Integrator,
    t_eval: f64,
    cache: &mut SolverCache,
    mut needs_refactor: bool,
) -> Analysis {
    if !dt.is_finite() || dt < 0.0 {
        return Analysis {
            solution: None,
            issues: vec![CircuitIssue::InvalidTimeStep],
        };
    }
    let has_transients = c.parts.iter().any(|p| {
        matches!(
            p.component,
            Component::Capacitor { .. } | Component::Inductor { .. }
        )
    });
    if dt == 0.0 && has_transients {
        return Analysis {
            solution: None,
            issues: vec![CircuitIssue::InvalidTimeStep],
        };
    }
    let invalid = invalid_components(c);
    if !invalid.is_empty() {
        return Analysis {
            solution: None,
            issues: invalid,
        };
    }
    if cache.dt != Some(dt) || cache.integrator != Some(integrator) {
        needs_refactor = true;
    }
    let mut work = c.clone();
    loop {
        let stamps: Vec<Stamp> = work
            .parts
            .iter()
            .map(|p| p.component.transient_stamp(dt, integrator, t_eval))
            .collect();
        let (solution, mut issues) = solve_once(&work, stamps, cache, needs_refactor);
        let Some(solution) = solution else {
            cache.dt = None;
            cache.integrator = None;
            return Analysis {
                solution: None,
                issues,
            };
        };
        cache.dt = Some(dt);
        cache.integrator = Some(integrator);
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
        let stamps: Vec<Stamp> = work.parts.iter().map(|p| p.component.stamp(0.0)).collect();
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
