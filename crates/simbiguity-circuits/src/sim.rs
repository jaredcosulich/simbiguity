//! The circuit as a `simbiguity-core` [`Simulation`].

use simbiguity_core::{Seconds, Simulation, Snapshot};

use crate::component::{Component, Integrator};
use crate::netlist::{Circuit, ComponentId, NodeId};
use crate::solution::Analysis;

/// A probe measuring a node voltage or component current.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Probe {
    /// Voltage at a node relative to ground.
    NodeVoltage(NodeId),
    /// Current through a component.
    Current(ComponentId),
}

/// A handle to a registered probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProbeId(pub usize);

/// The internal state of a probe's ring buffer.
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeState {
    /// The target of this probe.
    pub probe: Probe,
    /// The maximum number of samples.
    pub capacity: usize,
    /// The samples collected.
    pub samples: alloc::vec::Vec<(f64, f64)>,
    /// The index of the oldest sample (where the next sample goes).
    pub head: usize,
}

impl ProbeState {
    fn push(&mut self, t: f64, val: f64) {
        if self.capacity == 0 {
            return;
        }
        if self.samples.len() < self.capacity {
            self.samples.push((t, val));
        } else {
            self.samples[self.head] = (t, val);
            self.head = (self.head + 1) % self.capacity;
        }
    }

    fn to_vec(&self) -> alloc::vec::Vec<(f64, f64)> {
        let mut out = alloc::vec::Vec::with_capacity(self.samples.len());
        out.extend_from_slice(&self.samples[self.head..]);
        out.extend_from_slice(&self.samples[..self.head]);
        out
    }
}

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
    dirty: bool,
    needs_refactor: bool,
    cache: crate::mna::SolverCache,
    time: Seconds,
    probes: alloc::vec::Vec<ProbeState>,
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
            dirty: false,
            needs_refactor: true,
            cache: crate::mna::SolverCache::default(),
            time: Seconds(0.0),
            probes: alloc::vec::Vec::new(),
        };
        sim.persist_blown();
        sim
    }

    /// Runs a DC solve, treating capacitors as open and inductors as wires.
    pub fn solve(&mut self) {
        self.analysis = self.circuit.solve();
        self.dirty = false;
        self.persist_blown();
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

    /// The current simulation time.
    pub fn time(&self) -> Seconds {
        self.time
    }

    /// Adds a probe and returns its ID.
    pub fn add_probe(&mut self, probe: Probe, capacity: usize) -> ProbeId {
        let id = ProbeId(self.probes.len());
        self.probes.push(ProbeState {
            probe,
            capacity,
            samples: alloc::vec::Vec::with_capacity(capacity),
            head: 0,
        });
        id
    }

    /// Returns the samples collected by a probe, oldest first.
    pub fn probe_samples(&self, id: ProbeId) -> alloc::vec::Vec<(f64, f64)> {
        if let Some(p) = self.probes.get(id.0) {
            p.to_vec()
        } else {
            alloc::vec::Vec::new()
        }
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

    /// Force a refactor (for testing)
    #[cfg(test)]
    pub fn force_refactor(&mut self) {
        self.needs_refactor = true;
    }
}

impl Simulation for CircuitSim {
    type State = Analysis;
    type Input = CircuitInput;

    fn step(&mut self, dt: Seconds) {
        let has_transients = self.circuit.parts.iter().any(|p| {
            matches!(
                p.component,
                Component::Capacitor { .. }
                    | Component::Inductor { .. }
                    | Component::SignalSource { .. }
            )
        });
        if !self.dirty && !has_transients {
            return;
        }
        let t_eval = self.time.0 + dt.0;
        self.dirty = false;
        self.analysis = crate::mna::analyze_step(
            &self.circuit,
            dt.0,
            self.integrator,
            t_eval,
            &mut self.cache,
            self.needs_refactor,
        );
        if let Some(sol) = self.analysis.solution() {
            self.needs_refactor = false;
            self.time = Seconds(t_eval);
            for p in &mut self.probes {
                let val = match p.probe {
                    Probe::NodeVoltage(n) => sol.node_voltages[n.0].unwrap_or(0.0),
                    Probe::Current(c) => sol.currents[c.0],
                };
                p.push(t_eval, val);
            }
        }
        self.persist_blown();
        self.persist_transient_state();
    }

    fn apply(&mut self, input: CircuitInput) {
        self.dirty = true;
        match input {
            CircuitInput::ToggleSwitch(id) => {
                self.needs_refactor = true;
                self.circuit.toggle_switch(id);
            }
            CircuitInput::SetSwitch(id, closed) => {
                self.needs_refactor = true;
                self.circuit.set_switch(id, closed);
            }
            CircuitInput::SetValue(id, v) => {
                if !matches!(self.circuit.component(id), Component::Battery { .. }) {
                    self.needs_refactor = true;
                }
                self.circuit.component_mut(id).set_value(v);
            }
            CircuitInput::ReplaceBulb(id) => {
                self.needs_refactor = true;
                self.circuit.replace_bulb(id);
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use simbiguity_core::Seconds;

    /// Verifies bit-identical outputs for the `dirty`-flag cache and EMF-only matrix factorization cache, ensuring optimization does not alter results.
    #[test]
    fn cache_produces_bit_identical_results() {
        let mut c = Circuit::new();
        let (p, m) = (c.node(), c.node());
        let bat = c.add_battery(p, m, 9.0);
        c.add_resistor(p, m, 100.0);

        let mut sim1 = CircuitSim::new(c.clone());
        let mut sim2 = CircuitSim::new(c.clone());

        // Step without changes (dirty flag cache kicks in)
        sim1.step(Seconds(0.1));

        // Force sim2 to re-evaluate by setting dirty and needs_refactor
        sim2.dirty = true;
        sim2.needs_refactor = true;
        sim2.step(Seconds(0.1));

        assert_eq!(sim1.state(), sim2.state());

        // EMF-only cache: change battery voltage
        sim1.apply(CircuitInput::SetValue(bat, 12.0));
        sim1.step(Seconds(0.1));

        // sim2 does the same, but we force needs_refactor to bypass the EMF-only cache
        sim2.apply(CircuitInput::SetValue(bat, 12.0));
        sim2.needs_refactor = true; // force full matrix rebuild
        sim2.step(Seconds(0.1));

        assert_eq!(sim1.state(), sim2.state());
    }

    /// A capacitor's step model is a conductance C/dt, so the matrix depends on
    /// dt. If dt changes, a cached simulation must give exactly what a fresh
    /// simulation (no cache) gives for the same history.
    #[test]
    fn changing_dt_refactors_the_matrix() {
        let build = || {
            let mut c = Circuit::new();
            let (p, mid, m) = (c.node(), c.node(), c.node());
            c.add_battery(p, m, 10.0);
            c.add_resistor(p, mid, 1000.0);
            let cap = c.add_capacitor(mid, m, 0.001);
            (c, cap)
        };
        let (c, cap) = build();
        let mut cached = CircuitSim::new(c);
        cached.step(Seconds(0.1));
        cached.step(Seconds(0.01));
        let v_cached = cached
            .state()
            .solution()
            .unwrap()
            .voltage_across(cap)
            .unwrap();

        let (c2, _) = build();
        let mut fresh = CircuitSim::new(c2);
        fresh.step(Seconds(0.1));
        fresh.force_refactor(); // test-only hook, or rebuild from snapshot; decide at execution
        fresh.step(Seconds(0.01));
        let v_fresh = fresh
            .state()
            .solution()
            .unwrap()
            .voltage_across(cap)
            .unwrap();
        assert_eq!(v_cached.to_bits(), v_fresh.to_bits());
    }

    /// Seeded random-history equivalence test: 500 histories mixing toggles, value changes, EMF changes, capacitors and dt changes.
    #[test]
    fn random_history_equivalence() {
        use simbiguity_core::rng::Rng;
        let mut rng = Rng::new(0x12345678);
        let mut c = Circuit::new();
        let (n0, n1, n2, n3) = (c.node(), c.node(), c.node(), c.node());

        let b1 = c.add_battery(n0, n1, 5.0);
        let r1 = c.add_resistor(n1, n2, 100.0);
        let _c1 = c.add_capacitor(n2, n0, 0.01);
        let _b2 = c.add_battery(n2, n3, 3.0);
        let sw1 = c.add_switch(n3, n0, false);

        let mut cached = CircuitSim::new(c.clone());
        let mut fresh = CircuitSim::new(c.clone());

        for _ in 0..500 {
            if rng.chance(0.2) {
                let v = rng.range(1.0, 10.0);
                cached.apply(CircuitInput::SetValue(b1, v));
                fresh.apply(CircuitInput::SetValue(b1, v));
            }
            if rng.chance(0.2) {
                let v = rng.range(10.0, 500.0);
                cached.apply(CircuitInput::SetValue(r1, v));
                fresh.apply(CircuitInput::SetValue(r1, v));
            }
            if rng.chance(0.1) {
                cached.apply(CircuitInput::ToggleSwitch(sw1));
                fresh.apply(CircuitInput::ToggleSwitch(sw1));
            }
            let dt = if rng.chance(0.5) {
                Seconds(0.1)
            } else {
                Seconds(0.01)
            };
            cached.step(dt);
            fresh.force_refactor();
            fresh.step(dt);

            assert_eq!(
                cached.state(),
                fresh.state(),
                "random history mismatch seed 0x12345678"
            );
        }
    }

    /// Wheatstone EMF factorisation reuse: changing only a battery voltage should reuse the factorisation.
    #[test]
    fn wheatstone_emf_factorisation_reuse() {
        let mut c = Circuit::new();
        let (top, bot, left, right) = (c.node(), c.node(), c.node(), c.node());
        let bat = c.add_battery(bot, top, 10.0);
        c.add_resistor(top, left, 100.0);
        c.add_resistor(top, right, 100.0);
        c.add_resistor(left, bot, 100.0);
        c.add_resistor(right, bot, 100.0);
        c.add_resistor(left, right, 100.0);

        let mut cached = CircuitSim::new(c.clone());
        let mut fresh = CircuitSim::new(c.clone());

        cached.step(Seconds(0.1));
        fresh.step(Seconds(0.1));

        cached.apply(CircuitInput::SetValue(bat, 12.0));
        fresh.apply(CircuitInput::SetValue(bat, 12.0));

        cached.step(Seconds(0.1));
        fresh.force_refactor();
        fresh.step(Seconds(0.1));

        assert_eq!(cached.state(), fresh.state());
    }

    /// Resistance change invalidation: changing a resistor value must force a refactor.
    #[test]
    fn resistance_change_invalidation() {
        let mut c = Circuit::new();
        let (p, m) = (c.node(), c.node());
        c.add_battery(p, m, 10.0);
        let r = c.add_resistor(p, m, 100.0);

        let mut cached = CircuitSim::new(c.clone());
        let mut fresh = CircuitSim::new(c.clone());

        cached.step(Seconds(0.1));
        fresh.step(Seconds(0.1));

        cached.apply(CircuitInput::SetValue(r, 50.0));
        fresh.apply(CircuitInput::SetValue(r, 50.0));

        cached.step(Seconds(0.1));
        fresh.force_refactor();
        fresh.step(Seconds(0.1));

        assert_eq!(cached.state(), fresh.state());
    }
}
