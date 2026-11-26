//! End-to-end checks of `Circuit::solve` and `CircuitSim` against circuits
//! whose answers are known from hand calculation.

use simbiguity_circuits::{
    Circuit, CircuitInput, CircuitIssue, CircuitSim, Component, ComponentId, Solution,
};
use simbiguity_core::{Seconds, Simulation, Snapshot};

const EPS: f64 = 1e-9;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < EPS
}

fn solved(c: &Circuit) -> Solution {
    c.solve().solution().cloned().expect("circuit should solve")
}

/// Ohm's law: 6 V across 3 Ω drives 2 A, and the battery delivers the
/// 12 W the resistor absorbs.
#[test]
fn ohms_law_single_resistor() {
    let mut c = Circuit::new();
    let (p, m) = (c.node(), c.node());
    let bat = c.add_battery(p, m, 6.0);
    let r = c.add_resistor(p, m, 3.0);
    let s = solved(&c);
    assert!(close(s.current(r), 2.0));
    assert!(close(s.current(bat), 2.0));
    assert!(close(s.power(bat), 12.0));
    assert!(close(s.power(r), 12.0));
    assert_eq!(s.node_voltage(m), Some(0.0));
}

/// A series divider: 12 V over 1 Ω then 2 Ω puts the middle node at 8 V.
#[test]
fn series_voltage_divider() {
    let mut c = Circuit::new();
    let (p, mid, m) = (c.node(), c.node(), c.node());
    c.add_battery(p, m, 12.0);
    c.add_resistor(p, mid, 1.0);
    let lower = c.add_resistor(mid, m, 2.0);
    let s = solved(&c);
    assert!(close(s.node_voltage(mid).unwrap(), 8.0));
    assert!(close(s.voltage_across(lower).unwrap(), 8.0));
}

/// Parallel resistors: 3 Ω ‖ 6 Ω is 2 Ω, so a 6 V battery delivers 3 A.
#[test]
fn parallel_resistors_share_current() {
    let mut c = Circuit::new();
    let (p, m) = (c.node(), c.node());
    let bat = c.add_battery(p, m, 6.0);
    let r3 = c.add_resistor(p, m, 3.0);
    let r6 = c.add_resistor(p, m, 6.0);
    let s = solved(&c);
    assert!(close(s.current(bat), 3.0));
    assert!(close(s.current(r3), 2.0));
    assert!(close(s.current(r6), 1.0));
}

/// Internal resistance makes the terminal voltage sag: 12 V with 1 Ω inside
/// into 5 Ω gives 2 A and 10 V at the terminals.
#[test]
fn internal_resistance_sags_terminal_voltage() {
    let mut c = Circuit::new();
    let (p, m) = (c.node(), c.node());
    let bat = c.add_battery_with_internal(p, m, 12.0, 1.0);
    c.add_resistor(p, m, 5.0);
    let s = solved(&c);
    assert!(close(s.current(bat), 2.0));
    assert!(close(s.voltage_across(bat).unwrap(), 10.0));
}

/// A balanced Wheatstone bridge carries no current through its bridge
/// resistor — a topology no series/parallel reduction handles.
#[test]
fn balanced_wheatstone_bridge_carries_no_bridge_current() {
    let mut c = Circuit::new();
    let (top, l, r, bot) = (c.node(), c.node(), c.node(), c.node());
    c.add_battery(top, bot, 10.0);
    c.add_resistor(top, l, 100.0);
    c.add_resistor(top, r, 100.0);
    c.add_resistor(l, bot, 100.0);
    c.add_resistor(r, bot, 100.0);
    let bridge = c.add_resistor(l, r, 50.0);
    let s = solved(&c);
    assert!(close(s.current(bridge), 0.0));
    assert!(close(s.node_voltage(l).unwrap(), 5.0));
}

/// An unbalanced bridge (one lower arm 200 Ω) drives a known current
/// through the bridge, from the hand-solved node equations
/// 4·Vl − 2·Vr = 10 and 7·Vr − 4·Vl = 20.
#[test]
fn unbalanced_wheatstone_bridge_carries_known_current() {
    let mut c = Circuit::new();
    let (top, l, r, bot) = (c.node(), c.node(), c.node(), c.node());
    c.add_battery(top, bot, 10.0);
    c.add_resistor(top, l, 100.0);
    c.add_resistor(top, r, 100.0);
    c.add_resistor(l, bot, 100.0);
    c.add_resistor(r, bot, 200.0);
    let bridge = c.add_resistor(l, r, 50.0);
    let s = solved(&c);
    assert!(close(s.node_voltage(l).unwrap(), 5.5));
    assert!(close(s.node_voltage(r).unwrap(), 6.0));
    assert!(close(s.current(bridge), -0.01));
}

/// An open switch leaves a bulb dark; closing it through the Simulation
/// API lights it fully at its rated voltage.
#[test]
fn switch_lights_a_bulb() {
    let mut c = Circuit::new();
    let (p, x, m) = (c.node(), c.node(), c.node());
    c.add_battery(p, m, 6.0);
    let sw = c.add_switch(p, x, false);
    let bulb = c.add_bulb(x, m, 6.0, 3.0);
    let mut sim = CircuitSim::new(c);
    let dark = sim.state().solution().unwrap().brightness(bulb).unwrap();
    assert!(close(dark, 0.0));
    sim.apply(CircuitInput::ToggleSwitch(sw));
    sim.step(Seconds(0.1));
    let lit = sim.state().solution().unwrap().brightness(bulb).unwrap();
    assert!(close(lit, 1.0));
}

/// Two real batteries with different EMFs in parallel: 1 A circulates, the
/// stronger one discharging and the weaker one charging, at 9 V.
#[test]
fn parallel_batteries_with_different_emfs_circulate_current() {
    let mut c = Circuit::new();
    let (p, m) = (c.node(), c.node());
    let strong = c.add_battery_with_internal(p, m, 10.0, 1.0);
    let weak = c.add_battery_with_internal(p, m, 8.0, 1.0);
    let s = solved(&c);
    assert!(close(s.current(strong), 1.0));
    assert!(close(s.current(weak), -1.0));
    assert!(close(s.voltage_across(strong).unwrap(), 9.0));
}

/// A bulb at its rated voltage glows at full brightness; at half its rated
/// voltage it draws a quarter of the power, so a quarter of the brightness.
#[test]
fn bulb_brightness_scales_with_power() {
    let mut c = Circuit::new();
    let (p, m) = (c.node(), c.node());
    c.add_battery(p, m, 6.0);
    let full = c.add_bulb(p, m, 6.0, 3.0);
    let quarter = c.add_bulb(p, m, 12.0, 3.0);
    let s = solved(&c);
    assert!(close(s.brightness(full).unwrap(), 1.0));
    assert!(close(s.brightness(quarter).unwrap(), 0.25));
}

/// A resistor has no brightness.
#[test]
fn non_bulbs_have_no_brightness() {
    let mut c = Circuit::new();
    let (p, m) = (c.node(), c.node());
    let bat = c.add_battery(p, m, 6.0);
    c.add_resistor(p, m, 3.0);
    assert_eq!(solved(&c).brightness(bat), None);
}

/// Wire current through merged nodes is recovered from Kirchhoff's law.
#[test]
fn wire_carries_the_loop_current() {
    let mut c = Circuit::new();
    let (p, x, m) = (c.node(), c.node(), c.node());
    c.add_battery(p, m, 6.0);
    let wire = c.connect(p, x);
    c.add_resistor(x, m, 3.0);
    let s = solved(&c);
    assert!(close(s.current(wire), 2.0));
    assert!(close(s.node_voltage(x).unwrap(), 6.0));
}

/// An ideal battery shorted by a wire is fatal: no solution, a named
/// ShortCircuit with unlimited current.
#[test]
fn ideal_battery_short_is_fatal() {
    let mut c = Circuit::new();
    let (p, m) = (c.node(), c.node());
    let bat = c.add_battery(p, m, 6.0);
    c.connect(p, m);
    let a = c.solve();
    assert!(a.solution().is_none());
    assert_eq!(
        a.issues(),
        &[CircuitIssue::ShortCircuit {
            battery: bat,
            current: None
        }]
    );
}

/// A real battery shorted by a wire still solves, reporting emf / r.
#[test]
fn real_battery_short_reports_emf_over_r() {
    let mut c = Circuit::new();
    let (p, m) = (c.node(), c.node());
    let bat = c.add_battery_with_internal(p, m, 6.0, 2.0);
    c.connect(p, m);
    let a = c.solve();
    assert!(a.solution().is_some());
    assert_eq!(
        a.issues(),
        &[CircuitIssue::ShortCircuit {
            battery: bat,
            current: Some(3.0)
        }]
    );
}

/// Two ideal batteries side by side contradict each other: fatal loop.
#[test]
fn parallel_ideal_batteries_are_a_voltage_source_loop() {
    let mut c = Circuit::new();
    let (p, m) = (c.node(), c.node());
    c.add_battery(p, m, 6.0);
    let second = c.add_battery(p, m, 9.0);
    let a = c.solve();
    assert!(a.solution().is_none());
    assert_eq!(
        a.issues(),
        &[CircuitIssue::VoltageSourceLoop { battery: second }]
    );
}

/// A nonsensical value is reported, fatally, before anything is solved.
#[test]
fn invalid_value_is_fatal() {
    let mut c = Circuit::new();
    let (p, m) = (c.node(), c.node());
    c.add_battery(p, m, 6.0);
    let bad = c.add_resistor(p, m, -1.0);
    let a = c.solve();
    assert!(a.solution().is_none());
    assert_eq!(a.issues(), &[CircuitIssue::InvalidValue { component: bad }]);
    assert!(a.issues()[0].is_fatal());
}

/// A piece with no battery floats: it is named, its voltages are unknown,
/// and the rest of the circuit still solves.
#[test]
fn floating_island_is_reported_and_unmeasured() {
    let mut c = Circuit::new();
    let (p, m) = (c.node(), c.node());
    c.add_battery(p, m, 6.0);
    c.add_resistor(p, m, 3.0);
    let (x, y) = (c.node(), c.node());
    let lonely = c.add_resistor(x, y, 1.0);
    let a = c.solve();
    assert_eq!(a.issues(), &[CircuitIssue::FloatingNode { node: x }]);
    let s = a.solution().unwrap();
    assert_eq!(s.node_voltage(y), None);
    assert_eq!(s.voltage_across(lonely), None);
    assert!(close(s.node_voltage(p).unwrap(), 6.0));
}

/// An overloaded bulb blows, leaving the battery with an open circuit;
/// the blown bulb is listed last.
#[test]
fn overloaded_bulb_blows_and_opens_the_circuit() {
    let mut c = Circuit::new();
    let (p, m) = (c.node(), c.node());
    let bat = c.add_battery(p, m, 6.0);
    let bulb = c.add_bulb(p, m, 3.0, 1.0); // 4 W at 6 V, over 2 × 1 W
    let a = c.solve();
    assert_eq!(
        a.issues(),
        &[
            CircuitIssue::OpenCircuit { battery: bat },
            CircuitIssue::BulbBlown { bulb }
        ]
    );
    let s = a.solution().unwrap();
    assert_eq!(s.brightness(bulb), Some(0.0));
    assert!(close(s.current(bat), 0.0));
    // `solve` is pure: the circuit itself is untouched.
    assert_eq!(*c.component(bulb), Component::bulb(3.0, 1.0));
}

/// In series, only the most overloaded bulb blows; that opens the loop and
/// saves the other one.
#[test]
fn most_overloaded_series_bulb_blows_first_and_saves_the_other() {
    let mut c = Circuit::new();
    let (p, mid, m) = (c.node(), c.node(), c.node());
    c.add_battery(p, m, 12.0);
    let sturdy = c.add_bulb(p, mid, 3.0, 1.0); // 9 Ω, ratio ≈ 1.78
    let weak = c.add_bulb(mid, m, 3.0, 0.5); // 18 Ω, ratio ≈ 7.1
    let a = c.solve();
    let blown: Vec<ComponentId> = a.blown_bulbs().collect();
    assert_eq!(blown, vec![weak]);
    assert!(!blown.contains(&sturdy));
}

/// Flipping a switch through the Simulation API completes the circuit at
/// the next step.
#[test]
fn switch_input_takes_effect_on_step() {
    let mut c = Circuit::new();
    let (p, x, m) = (c.node(), c.node(), c.node());
    let bat = c.add_battery(p, m, 6.0);
    let sw = c.add_switch(p, x, false);
    c.add_resistor(x, m, 3.0);
    let mut sim = CircuitSim::new(c);
    assert_eq!(
        sim.state().issues(),
        &[CircuitIssue::OpenCircuit { battery: bat }]
    );
    sim.apply(CircuitInput::ToggleSwitch(sw));
    assert_eq!(
        sim.state().issues().len(),
        1,
        "apply alone does not re-solve"
    );
    sim.step(Seconds(0.1));
    assert!(sim.state().issues().is_empty());
    assert!(close(sim.state().solution().unwrap().current(sw), 2.0));
    sim.apply(CircuitInput::SetSwitch(sw, false));
    sim.step(Seconds(0.1));
    assert_eq!(
        sim.state().issues(),
        &[CircuitIssue::OpenCircuit { battery: bat }]
    );
}

/// A blown bulb stays blown in a CircuitSim until replaced, even once the
/// voltage that blew it is lowered.
#[test]
fn blown_bulb_persists_until_replaced() {
    let mut c = Circuit::new();
    let (p, m) = (c.node(), c.node());
    let bat = c.add_battery(p, m, 6.0);
    let bulb = c.add_bulb(p, m, 3.0, 1.0);
    let mut sim = CircuitSim::new(c);
    assert_eq!(sim.state().blown_bulbs().collect::<Vec<_>>(), vec![bulb]);
    sim.apply(CircuitInput::SetValue(bat, 3.0));
    sim.step(Seconds(0.1));
    assert_eq!(sim.state().blown_bulbs().collect::<Vec<_>>(), vec![bulb]);
    sim.apply(CircuitInput::ReplaceBulb(bulb));
    sim.step(Seconds(0.1));
    assert!(sim.state().issues().is_empty());
    assert!(close(
        sim.state().solution().unwrap().brightness(bulb).unwrap(),
        1.0
    ));
}

/// Restoring a snapshot brings back the exact earlier state.
#[test]
fn snapshot_round_trip_restores_state() {
    let mut c = Circuit::new();
    let (p, x, m) = (c.node(), c.node(), c.node());
    c.add_battery(p, m, 6.0);
    let sw = c.add_switch(p, x, true);
    c.add_resistor(x, m, 3.0);
    let mut sim = CircuitSim::new(c);
    let before = sim.snapshot();
    sim.apply(CircuitInput::ToggleSwitch(sw));
    sim.step(Seconds(0.1));
    assert_ne!(sim, before);
    sim.restore(&before);
    assert_eq!(sim, before);
}

/// Solving the same circuit twice gives bit-identical answers.
#[test]
fn solving_is_deterministic() {
    let mut c = Circuit::new();
    let (top, l, r, bot) = (c.node(), c.node(), c.node(), c.node());
    c.add_battery_with_internal(top, bot, 9.0, 0.5);
    c.add_resistor(top, l, 120.0);
    c.add_resistor(top, r, 330.0);
    c.add_bulb(l, bot, 6.0, 0.3);
    c.add_resistor(r, bot, 47.0);
    c.add_resistor(l, r, 68.0);
    assert_eq!(c.solve(), c.solve());
}

/// An RC circuit charging from a battery: 10 V, 5 Ω, 0.2 F.
/// Time constant τ = R * C = 5 * 0.2 = 1.0 s.
/// Exact V_c(t) = 10 * (1 - exp(-t/τ)).
#[test]
fn rc_circuit_charging_matches_exact_solution() {
    let mut c = Circuit::new();
    let (p, mid, m) = (c.node(), c.node(), c.node());
    c.add_battery(p, m, 10.0);
    c.add_resistor(p, mid, 5.0);
    let cap = c.add_capacitor(mid, m, 0.2);

    let mut sim = CircuitSim::new(c);
    let dt = Seconds(0.1);

    // Step forward by 1 second (10 steps)
    for _ in 0..10 {
        sim.step(dt);
    }

    let t = 1.0;
    let exact_v = 10.0 * (1.0 - (-t / 1.0_f64).exp());
    let sim_v = sim.state().solution().unwrap().voltage_across(cap).unwrap();

    // Since it's numerical, the error is small but not exactly 1e-9. Let's use 0.5 tolerance.
    assert!(
        (sim_v - exact_v).abs() < 0.5,
        "sim: {}, exact: {}",
        sim_v,
        exact_v
    );

    // Check energy ½CV²
    let sim_energy = sim.state().solution().unwrap().energy(cap);
    let exact_energy = 0.5 * 0.2 * sim_v * sim_v; // check consistency with sim_v
    assert!(
        (sim_energy - exact_energy).abs() < 0.5,
        "sim E: {}, exact E: {}",
        sim_energy,
        exact_energy
    );
}

/// An RL circuit: 10 V, 2 Ω, 1 H.
/// Time constant τ = L / R = 1 / 2 = 0.5 s.
/// Exact I_L(t) = (V/R) * (1 - exp(-t/τ)) = 5 * (1 - exp(-t/0.5)).
#[test]
fn rl_circuit_matches_exact_solution() {
    let mut c = Circuit::new();
    let (p, mid, m) = (c.node(), c.node(), c.node());
    c.add_battery(p, m, 10.0);
    c.add_resistor(p, mid, 2.0);
    let ind = c.add_inductor(mid, m, 1.0);

    let mut sim = CircuitSim::new(c);
    let dt = Seconds(0.05);

    // Step forward by 0.5 seconds (10 steps)
    for _ in 0..10 {
        sim.step(dt);
    }

    let t = 0.5;
    let exact_i = 5.0 * (1.0 - (-t / 0.5_f64).exp());
    let sim_i = sim.state().solution().unwrap().current(ind);

    assert!(
        (sim_i.abs() - exact_i).abs() < 0.5,
        "sim I: {}, exact I: {}",
        sim_i,
        exact_i
    );

    let sim_energy = sim.state().solution().unwrap().energy(ind);
    let exact_energy = 0.5 * 1.0 * sim_i * sim_i;
    assert!(
        (sim_energy - exact_energy).abs() < 0.5,
        "sim E: {}, exact E: {}",
        sim_energy,
        exact_energy
    );
}
