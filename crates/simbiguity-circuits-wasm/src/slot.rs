//! The handle table: every live circuit, and the ids JS knows its parts by.

use std::cell::RefCell;
use std::vec::Vec;

use simbiguity_circuits::{Circuit, CircuitIssue, CircuitSim, Component, ComponentId, NodeId};
use simbiguity_core::{Seconds, Simulation};

use crate::status::{fail, ERR_COMPONENT, ERR_HANDLE, ERR_INDEX, ERR_NODE, OK};
use crate::RESULT_STRIDE;

/// One circuit and the ids JS knows it by.
pub(crate) struct Slot {
    pub(crate) sim: CircuitSim,
    nodes: Vec<NodeId>,
    parts: Vec<ComponentId>,

    pub(crate) results: Vec<f64>,
    pub(crate) chart_series: simbiguity_plot::Series,
    pub(crate) chart_meshes: Vec<simbiguity_render::Mesh>,
    pub(crate) chart_commands: Vec<u32>,
    pub(crate) chart_labels: String,
    pub(crate) time: f64,
}

thread_local! {
    /// Freed slots stay `None` forever, so a stale handle can never alias a
    /// newer circuit.
    static TABLE: RefCell<Vec<Option<Slot>>> = const { RefCell::new(Vec::new()) };
}

/// Adds an empty circuit to the table; returns its handle (never `0`).
pub(crate) fn insert_empty() -> u32 {
    TABLE.with(|t| {
        let mut t = t.borrow_mut();
        t.push(Some(Slot {
            sim: CircuitSim::new(Circuit::new()),
            nodes: Vec::new(),
            parts: Vec::new(),

            results: Vec::new(),
            chart_series: simbiguity_plot::Series::new(200),
            chart_meshes: Vec::new(),
            chart_commands: Vec::new(),
            chart_labels: String::new(),
            time: 0.0,
        }));
        t.len() as u32
    })
}

/// Frees the circuit behind `h`; a dead or unknown handle is [`ERR_HANDLE`].
pub(crate) fn remove(h: u32) -> i32 {
    TABLE.with(|t| {
        let mut t = t.borrow_mut();
        match (h as usize).checked_sub(1).and_then(|i| t.get_mut(i)) {
            Some(slot @ Some(_)) => {
                *slot = None;
                OK
            }
            _ => fail(ERR_HANDLE, std::format!("no circuit with handle {h}")),
        }
    })
}

/// Runs `f` on the circuit behind `h`, or fails with [`ERR_HANDLE`].
pub(crate) fn with_slot(h: u32, f: impl FnOnce(&mut Slot) -> i32) -> i32 {
    TABLE.with(|t| {
        let mut t = t.borrow_mut();
        match (h as usize)
            .checked_sub(1)
            .and_then(|i| t.get_mut(i))
            .and_then(Option::as_mut)
        {
            Some(slot) => f(slot),
            None => fail(ERR_HANDLE, std::format!("no circuit with handle {h}")),
        }
    })
}

impl Slot {
    pub(crate) fn node(&self, n: u32) -> Result<NodeId, i32> {
        self.nodes
            .get(n as usize)
            .copied()
            .ok_or_else(|| fail(ERR_NODE, std::format!("no node {n} in this circuit")))
    }

    pub(crate) fn part(&self, c: u32) -> Result<ComponentId, i32> {
        self.parts.get(c as usize).copied().ok_or_else(|| {
            fail(
                ERR_COMPONENT,
                std::format!("no component {c} in this circuit"),
            )
        })
    }

    /// Edits the netlist. `CircuitSim` hands out no `&mut Circuit`, so this
    /// clones the circuit, applies `edit`, and rebuilds the sim around it.
    fn rebuild<T>(&mut self, edit: impl FnOnce(&mut Circuit) -> T) -> T {
        let mut circuit = self.sim.circuit().clone();
        let out = edit(&mut circuit);
        self.sim = CircuitSim::new(circuit);
        out
    }

    /// Adds a junction; returns its id.
    pub(crate) fn add_node(&mut self) -> i32 {
        let n = self.rebuild(Circuit::node);
        self.nodes.push(n);
        n.index() as i32
    }

    /// Places `component` between nodes `a` and `b`; returns its id.
    pub(crate) fn place(&mut self, component: Component, a: u32, b: u32) -> i32 {
        let (a, b) = match (self.node(a), self.node(b)) {
            (Ok(a), Ok(b)) => (a, b),
            (Err(e), _) | (_, Err(e)) => return e,
        };
        let id = self.rebuild(|c| c.add(component, a, b));
        self.parts.push(id);
        id.index() as i32
    }

    /// Steps forward in time by `dt` seconds, re-solves, and refreshes the shared results buffer.
    pub(crate) fn step(&mut self, dt: f64) -> i32 {
        self.sim.step(Seconds(dt));
        let analysis = self.sim.state();
        self.results.clear();
        for &id in &self.parts {
            let row = match analysis.solution() {
                Some(s) => [
                    s.voltage_across(id).unwrap_or(f64::NAN),
                    s.current(id),
                    s.power(id),
                    s.brightness(id).unwrap_or(f64::NAN),
                ],
                None => [f64::NAN; RESULT_STRIDE],
            };
            self.results.extend_from_slice(&row);
        }
        analysis.issues().len() as i32
    }

    /// Re-solves and refreshes the shared results buffer.
    pub(crate) fn solve(&mut self) -> i32 {
        self.sim.solve();
        let analysis = self.sim.state();
        self.results.clear();
        for &id in &self.parts {
            let row = match analysis.solution() {
                Some(s) => [
                    s.voltage_across(id).unwrap_or(f64::NAN),
                    s.current(id),
                    s.power(id),
                    s.brightness(id).unwrap_or(f64::NAN),
                ],
                None => [f64::NAN; RESULT_STRIDE],
            };
            self.results.extend_from_slice(&row);
        }
        analysis.issues().len() as i32
    }

    pub(crate) fn issue(&self, i: u32) -> Result<CircuitIssue, i32> {
        self.sim
            .state()
            .issues()
            .get(i as usize)
            .cloned()
            .ok_or_else(|| fail(ERR_INDEX, std::format!("no issue {i}")))
    }
}
