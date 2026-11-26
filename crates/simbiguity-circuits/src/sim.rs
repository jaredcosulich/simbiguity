//! The circuit as a `simbiguity-core` [`Simulation`].

use simbiguity_core::{Seconds, Simulation, Snapshot};

use crate::component::{Component, Integrator};
use crate::netlist::{Circuit, ComponentId};
use crate::solution::Analysis;

/// Something a learner (or another engine) does to the circuit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitInput {
    /// Flip a switch.
    ToggleSwitch(ComponentId),
    /// Set a switch closed (`true`) or open.
    SetSwitch(ComponentId, bool),
    /// Set a component's main value (see [`crate::Component::set_value`]).
    SetValue(ComponentId, f64),
    /// Screw in a fresh bulb in place of a blown one.
    ReplaceBulb(ComponentId),
}

/// A circuit that remembers what happened to it: blown bulbs stay blown.
///
/// DC circuits settle instantly, so each [`Simulation::step`] is a re-solve;
/// inputs take effect at the next step. (Capacitors and inductors will make
/// `dt` matter; the API already has room for them.)
#[derive(Debug, Clone, PartialEq)]
pub struct CircuitSim {
    circuit: Circuit,
    analysis: Analysis,
    integrator: Integrator,
}

impl CircuitSim {
    /// Wraps a circuit and solves it once, so [`Simulation::state`] is
    /// meaningful before the first step. Defaults to backward Euler.
    pub fn new(circuit: Circuit) -> CircuitSim {
        let analysis = circuit.solve();
        let mut sim = CircuitSim {
            circuit,
            analysis,
            integrator: Integrator::BackwardEuler,
        };
        sim.persist_blown();
        sim
    }

    /// Sets the integration method for capacitors and inductors.
    pub fn with_integrator(mut self, integrator: Integrator) -> Self {
        self.integrator = integrator;
        self
    }

    /// The circuit as it currently stands (including blown bulbs).
    pub fn circuit(&self) -> &Circuit {
        &self.circuit
    }

    fn persist_blown(&mut self) {
        let blown: alloc::vec::Vec<ComponentId> = self.analysis.blown_bulbs().collect();
        for id in blown {
            self.circuit.component_mut(id).blow();
        }
    }

    fn persist_transient_state(&mut self) {
        let Some(sol) = self.analysis.solution() else {
            return;
        };
        for (i, p) in self.circuit.parts.iter_mut().enumerate() {
            match &mut p.component {
                Component::Capacitor { volts, amps, .. } => {
                    if let Some(v) = sol.across[i] {
                        *volts = v;
                    }
                    *amps = sol.currents[i];
                }
                Component::Inductor { volts, amps, .. } => {
                    if let Some(v) = sol.across[i] {
                        *volts = v;
                    }
                    *amps = sol.currents[i];
                }
                _ => {}
            }
        }
    }
}

impl Simulation for CircuitSim {
    type State = Analysis;
    type Input = CircuitInput;

    fn step(&mut self, dt: Seconds) {
        self.analysis = crate::mna::analyze_step(&self.circuit, dt.0, self.integrator);
        self.persist_blown();
        self.persist_transient_state();
    }

    fn apply(&mut self, input: CircuitInput) {
        match input {
            CircuitInput::ToggleSwitch(id) => {
                self.circuit.toggle_switch(id);
            }
            CircuitInput::SetSwitch(id, closed) => self.circuit.set_switch(id, closed),
            CircuitInput::SetValue(id, v) => self.circuit.component_mut(id).set_value(v),
            CircuitInput::ReplaceBulb(id) => self.circuit.replace_bulb(id),
        }
    }

    fn state(&self) -> &Analysis {
        &self.analysis
    }
}

impl Snapshot for CircuitSim {
    type Snapshot = CircuitSim;

    fn snapshot(&self) -> CircuitSim {
        self.clone()
    }

    fn restore(&mut self, snapshot: &CircuitSim) {
        *self = snapshot.clone();
    }
}
