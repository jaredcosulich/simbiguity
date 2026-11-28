use simbiguity_circuits::{Circuit, CircuitSim, Integrator};
use simbiguity_core::{Seconds, Simulation};

/// Verifies that an ideal inverting operational amplifier circuit with R_f/R_in = 10 provides a voltage gain of -10.
#[test]
fn inverting_amp_gain_of_minus_ten() {
    let mut c = Circuit::new();
    let gnd = c.node();
    let in_node = c.node();
    let inv = c.node();
    let out = c.node();

    c.add_battery(in_node, gnd, 0.5);
    c.add_resistor(in_node, inv, 1000.0);
    c.add_resistor(inv, out, 10000.0);
    c.add_opamp(gnd, inv, out, 1e5, 12.0);

    c.set_ground(gnd);
    let s = c.solve();
    if s.solution().is_none() {
        panic!("Solver failed: {:?}", s.issues());
    }

    let v_out = s.solution().unwrap().node_voltage(out).unwrap();
    assert!((v_out - (-4.999414)).abs() < 1e-5, "V_out was {}", v_out);
}

/// Verifies that an operational amplifier correctly saturates to its power rail limits when the theoretical linear output exceeds it.
#[test]
fn inverting_amp_saturated() {
    let mut c = Circuit::new();
    let gnd = c.node();
    let in_node = c.node();
    let inv = c.node();
    let out = c.node();

    c.add_battery(in_node, gnd, 2.0);
    c.add_resistor(in_node, inv, 1000.0);
    c.add_resistor(inv, out, 10000.0);
    c.add_opamp(gnd, inv, out, 1e5, 12.0);

    c.set_ground(gnd);
    let s = c.solve();
    if s.solution().is_none() {
        panic!("Solver failed: {:?}", s.issues());
    }

    let v_out = s.solution().unwrap().node_voltage(out).unwrap();
    assert!((v_out - (-12.0)).abs() < 1e-3, "V_out was {}", v_out);
    assert!(s.solution().unwrap().newton_iterations() <= 100);
}

/// Verifies that an operational amplifier configured with 100% negative feedback (voltage follower) maintains unity gain.
#[test]
fn voltage_follower() {
    let mut c = Circuit::new();
    let gnd = c.node();
    let in_node = c.node();
    let out = c.node();

    c.add_battery(in_node, gnd, 3.0);
    // Non-inverting input is in_node
    // Inverting input is out (feedback)
    c.add_opamp(in_node, out, out, 1e5, 12.0);

    c.set_ground(gnd);
    let s = c.solve();
    if s.solution().is_none() {
        panic!("Solver failed: {:?}", s.issues());
    }

    let v_out = s.solution().unwrap().node_voltage(out).unwrap();
    assert!((v_out - 2.999969).abs() < 1e-6, "V_out was {}", v_out);
}

/// Verifies that an open loop operational amplifier correctly rails to its saturation voltage when inputs differ.
#[test]
fn comparator_open_loop() {
    let mut c = Circuit::new();
    let gnd = c.node();
    let plus = c.node();
    let minus = c.node();
    let out = c.node();

    c.add_battery(plus, gnd, 1.0);
    c.add_battery(minus, gnd, 0.5);
    c.add_opamp(plus, minus, out, 1e5, 12.0);

    c.set_ground(gnd);
    let s = c.solve();
    if s.solution().is_none() {
        panic!("Solver failed: {:?}", s.issues());
    }

    let v_out = s.solution().unwrap().node_voltage(out).unwrap();
    assert!((v_out - 12.0).abs() < 1e-3, "V_out was {}", v_out);
}

/// Verifies that a non-inverting operational amplifier circuit with R_f/R_in = 9 provides a voltage gain of +10.
#[test]
fn non_inverting_amp() {
    let mut c = Circuit::new();
    let gnd = c.node();
    let in_node = c.node();
    let inv = c.node();
    let out = c.node();

    c.add_battery(in_node, gnd, 0.2);
    c.add_resistor(inv, gnd, 1000.0);
    c.add_resistor(inv, out, 9000.0);
    c.add_opamp(in_node, inv, out, 1e5, 12.0);

    c.set_ground(gnd);
    let s = c.solve();
    if s.solution().is_none() {
        panic!("Solver failed: {:?}", s.issues());
    }

    let v_out = s.solution().unwrap().node_voltage(out).unwrap();
    assert!((v_out - 2.0).abs() < 1e-3, "V_out was {}", v_out);
}

/// Verifies that a pure VCVS (Voltage Controlled Voltage Source) scales the control voltage by exactly its configured gain.
#[test]
fn vcvs_gain_10() {
    let mut c = Circuit::new();
    let gnd = c.node();
    let ctrl_plus = c.node();
    let ctrl_minus = gnd;
    let out_plus = c.node();
    let out_minus = gnd;

    c.add_battery(ctrl_plus, gnd, 0.3);
    c.add_vcvs(out_plus, out_minus, ctrl_plus, ctrl_minus, 10.0);

    c.set_ground(gnd);
    let s = c.solve();
    let v_out = s.solution().unwrap().node_voltage(out_plus).unwrap();
    assert!((v_out - 3.0).abs() < 1e-12, "V_out was {}", v_out);
}

/// Verifies that an op-amp integrator circuit generates a linear voltage ramp given a constant input, until reaching rail saturation.
#[test]
fn integrator_ramp() {
    let mut c = Circuit::new();
    let gnd = c.node();
    let in_node = c.node();
    let inv = c.node();
    let out = c.node();

    // R = 10k, C = 1uF
    c.add_battery(in_node, gnd, 1.0);
    c.add_resistor(in_node, inv, 10_000.0);
    c.add_capacitor(inv, out, 1e-6);
    c.add_opamp(gnd, inv, out, 1e5, 12.0);

    c.set_ground(gnd);

    let mut sim = CircuitSim::new(c).with_integrator(Integrator::Trapezoidal);
    // V_out ramps at -100 V/s. dt = 1e-5.
    for _ in 0..5000 {
        sim.step(Seconds(1e-5));
    }
    // t = 50ms, V_out should be -5.0V
    let v_out = sim.state().solution().unwrap().node_voltage(out).unwrap();
    assert!((v_out - (-5.0)).abs() < 0.01, "V_out was {}", v_out);

    // t = 120ms
    for _ in 0..7000 {
        sim.step(Seconds(1e-5));
    }
    let v_out2 = sim.state().solution().unwrap().node_voltage(out).unwrap();
    assert!((v_out2 - (-12.0)).abs() < 5e-3, "V_out was {}", v_out2);
}

/// Verifies that an active low pass filter correctly attenuates high frequency AC signals by 3dB at its cutoff frequency.
#[test]
fn active_low_pass() {
    let mut c = Circuit::new();
    let gnd = c.node();
    let in_node = c.node();
    let inv = c.node();
    let out = c.node();

    // inverting, Rf || Cf with Rf = 10k, Cf = 15.915 nF, Rin = 10k
    // 1V sine at 1kHz
    c.add_signal_source(in_node, gnd, 0, 1.0, 1000.0, 0.0, 0.0);
    c.add_resistor(in_node, inv, 10_000.0);
    c.add_resistor(inv, out, 10_000.0);
    c.add_capacitor(inv, out, 15.915e-9);
    c.add_opamp(gnd, inv, out, 1e5, 12.0);

    c.set_ground(gnd);

    let mut sim = CircuitSim::new(c).with_integrator(Integrator::Trapezoidal);
    let mut max_v = 0.0f64;
    // let's run for 10ms (10 cycles)
    for _ in 0..10000 {
        sim.step(Seconds(1e-6));
        if sim.time().0 > 0.005 {
            let v = sim
                .state()
                .solution()
                .unwrap()
                .node_voltage(out)
                .unwrap()
                .abs();
            if v > max_v {
                max_v = v;
            }
        }
    }
    // amplitude 0.7071 +/- 1%
    let expected = core::f64::consts::FRAC_1_SQRT_2;
    let err = (max_v - expected).abs() / expected;
    assert!(err < 0.01, "Amplitude was {}, err = {}", max_v, err);
}
