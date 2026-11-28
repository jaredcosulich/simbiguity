//! The circuit as data: nodes, and components placed between them.

use alloc::vec::Vec;

use crate::component::Component;
use crate::solution::Analysis;

/// A junction in the circuit. Created by [`Circuit::node`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub(crate) usize);

impl NodeId {
    /// Position of this node in creation order (0, 1, 2, …).
    pub const fn index(self) -> usize {
        self.0
    }

    /// Reconstruct from an index.
    pub const fn from_index(index: usize) -> Self {
        Self(index)
    }
}

/// A placed component. Returned by the `add_*` builder methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentId(pub(crate) usize);

impl ComponentId {
    /// Position of this component in placement order (0, 1, 2, …).
    pub const fn index(self) -> usize {
        self.0
    }

    /// Reconstruct from an index.
    pub const fn from_index(index: usize) -> Self {
        Self(index)
    }
}

/// A component and the two nodes it connects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    /// What it is and its values.
    pub component: Component,
    /// Terminal `a` (a battery's `+`).
    pub a: NodeId,
    /// Terminal `b` (a battery's `−`).
    pub b: NodeId,
}

/// A DC circuit: the complete, deterministic description of what is on the
/// board. Its state is just this data, so a clone is a perfect snapshot.
///
/// **Ground is implicit**: unless [`Circuit::set_ground`] picks a node, each
/// connected piece of the circuit measures its voltages from the `−` terminal
/// of its first battery.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Circuit {
    pub(crate) node_count: usize,
    pub(crate) parts: Vec<Placed>,
    pub(crate) ground: Option<NodeId>,
    pub(crate) solver: SolverKind,
}

/// The linear solver to use for Modified Nodal Analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SolverKind {
    /// O(N^3) dense LU solver. Best for small circuits.
    Dense,
    /// Fill-reducing left-looking sparse LU solver. Best for large circuits.
    Sparse,
    /// Chooses Sparse if N >= 64, else Dense.
    #[default]
    Auto,
}

impl Circuit {
    /// An empty circuit.
    pub fn new() -> Circuit {
        Circuit::default()
    }

    /// Creates a new node (junction) and returns its id.
    pub fn node(&mut self) -> NodeId {
        self.node_count += 1;
        NodeId(self.node_count - 1)
    }

    /// Number of nodes created so far.
    pub fn node_count(&self) -> usize {
        self.node_count
    }

    /// Places any component between nodes `a` and `b`.
    ///
    /// # Panics
    /// If `a` or `b` was not created by this circuit's [`Circuit::node`].
    pub fn add(&mut self, component: Component, a: NodeId, b: NodeId) -> ComponentId {
        assert!(
            a.0 < self.node_count && b.0 < self.node_count,
            "node does not belong to this circuit"
        );
        self.parts.push(Placed { component, a, b });
        ComponentId(self.parts.len() - 1)
    }

    /// An ideal battery: `plus` sits `emf` volts above `minus`.
    pub fn add_battery(&mut self, plus: NodeId, minus: NodeId, emf: f64) -> ComponentId {
        self.add(Component::battery(emf), plus, minus)
    }

    /// A real battery with internal resistance: its terminal voltage sags as
    /// it delivers current.
    pub fn add_battery_with_internal(
        &mut self,
        plus: NodeId,
        minus: NodeId,
        emf: f64,
        internal_r: f64,
    ) -> ComponentId {
        self.add(Component::Battery { emf, internal_r }, plus, minus)
    }

    /// A resistor of `r` ohms.
    pub fn add_resistor(&mut self, a: NodeId, b: NodeId, r: f64) -> ComponentId {
        self.add(Component::Resistor { r }, a, b)
    }

    /// A diode (Shockley model).
    pub fn add_diode(
        &mut self,
        a: NodeId,
        b: NodeId,
        saturation_amps: f64,
        emission: f64,
    ) -> ComponentId {
        self.add(
            Component::Diode {
                saturation_amps,
                emission,
            },
            a,
            b,
        )
    }

    /// An LED.
    pub fn add_led(
        &mut self,
        a: NodeId,
        b: NodeId,
        color: &'static str,
        rated_amps: f64,
        saturation_amps: f64,
        emission: f64,
    ) -> ComponentId {
        self.add(
            Component::Led {
                color,
                rated_amps,
                saturation_amps,
                emission,
                blown: false,
            },
            a,
            b,
        )
    }

    /// A bulb rated `rated_power` watts at `rated_voltage` volts.
    pub fn add_bulb(
        &mut self,
        a: NodeId,
        b: NodeId,
        rated_voltage: f64,
        rated_power: f64,
    ) -> ComponentId {
        self.add(Component::bulb(rated_voltage, rated_power), a, b)
    }

    /// A switch, initially `closed` or open.
    pub fn add_switch(&mut self, a: NodeId, b: NodeId, closed: bool) -> ComponentId {
        self.add(Component::Switch { closed }, a, b)
    }

    /// A capacitor.
    pub fn add_capacitor(&mut self, a: NodeId, b: NodeId, farads: f64) -> ComponentId {
        self.add(Component::capacitor(farads), a, b)
    }

    /// An inductor.
    pub fn add_inductor(&mut self, a: NodeId, b: NodeId, henries: f64) -> ComponentId {
        self.add(Component::inductor(henries), a, b)
    }

    /// A signal source.
    #[allow(clippy::too_many_arguments)]
    pub fn add_signal_source(
        &mut self,
        a: NodeId,
        b: NodeId,
        waveform: u8,
        amplitude: f64,
        frequency: f64,
        phase: f64,
        offset: f64,
    ) -> ComponentId {
        self.add(
            Component::SignalSource {
                waveform,
                amplitude,
                frequency,
                phase,
                offset,
            },
            a,
            b,
        )
    }

    /// Joins two nodes with an ideal wire.
    pub fn connect(&mut self, a: NodeId, b: NodeId) -> ComponentId {
        self.add(Component::Wire, a, b)
    }

    /// Chooses the node that reads 0 V.
    pub fn set_ground(&mut self, node: NodeId) {
        self.ground = Some(node);
    }

    /// The explicitly chosen ground, if any.
    pub fn ground(&self) -> Option<NodeId> {
        self.ground
    }

    /// Chooses the linear solver explicitly.
    pub fn set_solver(&mut self, kind: SolverKind) {
        self.solver = kind;
    }

    /// Every placed component, in placement order.
    pub fn parts(&self) -> &[Placed] {
        &self.parts
    }

    /// The component with this id.
    pub fn component(&self, id: ComponentId) -> &Component {
        &self.parts[id.0].component
    }

    /// Mutable access to a component, to change its values.
    pub fn component_mut(&mut self, id: ComponentId) -> &mut Component {
        &mut self.parts[id.0].component
    }

    /// Flips a switch. Returns the new state (`true` = closed), or `None` if
    /// the component is not a switch.
    pub fn toggle_switch(&mut self, id: ComponentId) -> Option<bool> {
        match self.component_mut(id) {
            Component::Switch { closed } => {
                *closed = !*closed;
                Some(*closed)
            }
            _ => None,
        }
    }

    /// Sets a switch closed (`true`) or open. Anything that is not a switch
    /// is left unchanged.
    pub fn set_switch(&mut self, id: ComponentId, closed: bool) {
        if let Component::Switch { closed: c } = self.component_mut(id) {
            *c = closed;
        }
    }

    /// Replaces a blown bulb with a fresh one of the same rating.
    pub fn replace_bulb(&mut self, id: ComponentId) {
        if let Component::Bulb { blown, .. } = self.component_mut(id) {
            *blown = false;
        }
    }

    /// Solves the circuit. Never panics on a bad circuit: problems come back
    /// as [`crate::CircuitIssue`]s in the [`Analysis`].
    ///
    /// Bulbs pushed past their limit blow (most overloaded first, then the
    /// circuit is re-solved), so the returned solution is what the learner
    /// would see once the smoke clears. `solve` does not modify `self`; use
    /// [`crate::CircuitSim`] for bulbs that stay blown.
    pub fn solve(&self) -> Analysis {
        crate::mna::analyze(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A switch with one component in a circuit, plus that switch's id.
    fn one(component: Component) -> (Circuit, ComponentId) {
        let mut c = Circuit::new();
        let (a, b) = (c.node(), c.node());
        let id = c.add(component, a, b);
        (c, id)
    }

    /// set_switch sets a switch to exactly the requested state.
    #[test]
    fn set_switch_sets_the_state() {
        let (mut c, id) = one(Component::Switch { closed: false });
        c.set_switch(id, true);
        assert_eq!(*c.component(id), Component::Switch { closed: true });
        c.set_switch(id, true);
        assert_eq!(*c.component(id), Component::Switch { closed: true });
        c.set_switch(id, false);
        assert_eq!(*c.component(id), Component::Switch { closed: false });
    }

    /// set_switch on anything else is a no-op.
    #[test]
    fn set_switch_ignores_non_switches() {
        let (mut c, id) = one(Component::Resistor { r: 2.0 });
        c.set_switch(id, true);
        assert_eq!(*c.component(id), Component::Resistor { r: 2.0 });
    }

    /// toggle_switch flips and reports the new state, or None for a
    /// non-switch.
    #[test]
    fn toggle_switch_flips_and_reports() {
        let (mut c, id) = one(Component::Switch { closed: false });
        assert_eq!(c.toggle_switch(id), Some(true));
        assert_eq!(c.toggle_switch(id), Some(false));
        let (mut c, id) = one(Component::Wire);
        assert_eq!(c.toggle_switch(id), None);
    }

    /// replace_bulb restores a blown bulb to a fresh one of the same rating.
    #[test]
    fn replace_bulb_unblows() {
        let (mut c, id) = one(Component::bulb(6.0, 3.0));
        c.component_mut(id).blow();
        c.replace_bulb(id);
        assert_eq!(*c.component(id), Component::bulb(6.0, 3.0));
    }

    /// Ids are handed out in creation order.
    #[test]
    fn ids_follow_creation_order() {
        let mut c = Circuit::new();
        let (a, b) = (c.node(), c.node());
        assert_eq!((a.index(), b.index(), c.node_count()), (0, 1, 2));
        assert_eq!(c.connect(a, b).index(), 0);
        assert_eq!(c.add_resistor(a, b, 1.0).index(), 1);
        assert_eq!(c.parts().len(), 2);
        c.set_ground(b);
        assert_eq!(c.ground(), Some(b));
    }

    /// Placing a component on a node from elsewhere panics.
    #[test]
    #[should_panic(expected = "node does not belong")]
    fn foreign_node_panics() {
        let mut c = Circuit::new();
        let a = c.node();
        c.add_resistor(a, NodeId(5), 1.0);
    }
}
