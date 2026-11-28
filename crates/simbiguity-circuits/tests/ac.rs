//! End-to-end AC tests.

use simbiguity_circuits::{
    sim::{Probe, ProbeId},
    Circuit, CircuitSim, Integrator,
};
use simbiguity_core::{Seconds, Simulation, Snapshot};

// const EPS: f64 = 1e-9;

fn assert_close(a: f64, b: f64, tol: f64) {
    if (a - b).abs() > tol {
        panic!("Assertion failed: {} not within {} of {}", a, tol, b);
    }
}

fn measure_amplitude(sim: &CircuitSim, probe: ProbeId) -> f64 {
    let samples = sim.probe_samples(probe);
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &(_t, val) in &samples {
        if val < min {
            min = val;
        }
        if val > max {
            max = val;
        }
    }
    (max - min) / 2.0
}

fn find_peak_time(sim: &CircuitSim, probe: ProbeId) -> f64 {
    let samples = sim.probe_samples(probe);
    let mut max = f64::NEG_INFINITY;
    let mut t_max = 0.0;
    for &(t, val) in &samples {
        if val > max {
            max = val;
            t_max = t;
        }
    }
    t_max
}

/// Verifies series RLC at resonance draws R-limited current.
#[test]
fn series_rlc_at_resonance_draws_r_limited_current() {
    let mut c = Circuit::new();
    let f = 1591.5494309189535;
    let (n1, n2, n3, n0) = (c.node(), c.node(), c.node(), c.node());
    c.set_ground(n0);
    let _src = c.add_signal_source(n1, n0, 0, 1.0, f, 0.0, 0.0);
    let r = c.add_resistor(n1, n2, 10.0);
    let _l = c.add_inductor(n2, n3, 0.010);
    let _cap = c.add_capacitor(n3, n0, 1e-6);

    let mut sim = CircuitSim::new(c).with_integrator(Integrator::Trapezoidal);
    let period = 1.0 / f;
    let dt = period / 200.0;
    let probe = sim.add_probe(Probe::Current(r), 1000);

    for _ in 0..5000 {
        sim.step(Seconds(dt));
    }
    let amp = measure_amplitude(&sim, probe);
    assert_close(amp, 0.1, 0.001);
}

/// Verifies series RLC at 2*f0.
#[test]
fn series_rlc_at_2f0() {
    let mut c = Circuit::new();
    let f = 1591.5494309189535 * 2.0;
    let (n1, n2, n3, n0) = (c.node(), c.node(), c.node(), c.node());
    c.set_ground(n0);
    let _src = c.add_signal_source(n1, n0, 0, 1.0, f, 0.0, 0.0);
    let r = c.add_resistor(n1, n2, 10.0);
    let _l = c.add_inductor(n2, n3, 0.010);
    let _cap = c.add_capacitor(n3, n0, 1e-6);

    let mut sim = CircuitSim::new(c).with_integrator(Integrator::Trapezoidal);
    let period = 1.0 / f;
    let dt = period / 200.0;
    let probe = sim.add_probe(Probe::Current(r), 1000);

    for _ in 0..5000 {
        sim.step(Seconds(dt));
    }
    let amp = measure_amplitude(&sim, probe);
    assert_close(amp, 0.006652, 0.006652 * 0.02);
}

/// Verifies series RLC at half f0.
#[test]
fn series_rlc_at_half_f0() {
    let mut c = Circuit::new();
    let f = 1591.5494309189535 / 2.0;
    let (n1, n2, n3, n0) = (c.node(), c.node(), c.node(), c.node());
    c.set_ground(n0);
    let _src = c.add_signal_source(n1, n0, 0, 1.0, f, 0.0, 0.0);
    let r = c.add_resistor(n1, n2, 10.0);
    let _l = c.add_inductor(n2, n3, 0.010);
    let _cap = c.add_capacitor(n3, n0, 1e-6);

    let mut sim = CircuitSim::new(c).with_integrator(Integrator::Trapezoidal);
    let period = 1.0 / f;
    let dt = period / 200.0;
    let probe = sim.add_probe(Probe::Current(r), 1000);

    for _ in 0..5000 {
        sim.step(Seconds(dt));
    }
    let amp = measure_amplitude(&sim, probe);
    assert_close(amp, 0.006652, 0.006652 * 0.02);
}

/// Verifies RC low pass at fc.
#[test]
fn rc_low_pass_at_fc() {
    let mut c = Circuit::new();
    let f = 159.15494309189535;
    let (n1, n2, n0) = (c.node(), c.node(), c.node());
    c.set_ground(n0);
    let _src = c.add_signal_source(n1, n0, 0, 1.0, f, 0.0, 0.0);
    let _r = c.add_resistor(n1, n2, 1000.0);
    let _cap = c.add_capacitor(n2, n0, 1e-6);

    let mut sim = CircuitSim::new(c).with_integrator(Integrator::Trapezoidal);
    let period = 1.0 / f;
    let dt = period / 200.0;
    let probe_in = sim.add_probe(Probe::NodeVoltage(n1), 200);
    let probe_out = sim.add_probe(Probe::NodeVoltage(n2), 200);

    for _ in 0..5000 {
        sim.step(Seconds(dt));
    }
    let amp = measure_amplitude(&sim, probe_out);
    assert_close(amp, core::f64::consts::FRAC_1_SQRT_2, 0.007071);

    let t_in = find_peak_time(&sim, probe_in);
    let t_out = find_peak_time(&sim, probe_out);
    let mut dt_peak = t_out - t_in;
    if dt_peak < 0.0 {
        dt_peak += period;
    }
    let phase_lag = 360.0 * dt_peak / period;
    assert_close(phase_lag, 45.0, 1.0);
}

/// Verifies RC low pass at 10*fc.
#[test]
fn rc_low_pass_at_10fc() {
    let mut c = Circuit::new();
    let f = 1591.5494309189535;
    let (n1, n2, n0) = (c.node(), c.node(), c.node());
    c.set_ground(n0);
    let _src = c.add_signal_source(n1, n0, 0, 1.0, f, 0.0, 0.0);
    let _r = c.add_resistor(n1, n2, 1000.0);
    let _cap = c.add_capacitor(n2, n0, 1e-6);

    let mut sim = CircuitSim::new(c).with_integrator(Integrator::Trapezoidal);
    let period = 1.0 / f;
    let dt = period / 200.0;
    let probe_out = sim.add_probe(Probe::NodeVoltage(n2), 1000);

    for _ in 0..5000 {
        sim.step(Seconds(dt));
    }
    let amp = measure_amplitude(&sim, probe_out);
    assert_close(amp, 0.0995, 0.0995 * 0.02);
}

/// Verifies half wave rectifier behavior.
#[test]
fn half_wave_rectifier() {
    let mut c = Circuit::new();
    let (n1, n2, n0) = (c.node(), c.node(), c.node());
    c.set_ground(n0);
    let _src = c.add_signal_source(n1, n0, 0, 10.0, 50.0, 0.0, 0.0);
    let _d = c.add_diode(n1, n2, 1e-14, 1.0);
    let _r = c.add_resistor(n2, n0, 1000.0);

    let mut sim = CircuitSim::new(c).with_integrator(Integrator::Trapezoidal);
    let period = 1.0 / 50.0;
    let dt = period / 2000.0;
    let probe = sim.add_probe(Probe::NodeVoltage(n2), 2000);

    for _ in 0..50000 {
        sim.step(Seconds(dt));
    }

    let samples = sim.probe_samples(probe);
    let mut peak = f64::NEG_INFINITY;
    let mut min = f64::INFINITY;
    for &(_t, val) in &samples {
        if val > peak {
            peak = val;
        }
        if val < min {
            min = val;
        }
    }
    assert_close(peak, 9.2876, 0.01);
    assert!(min >= -1e-9, "min is {}", min);
}

/// Verifies square wave into RC circuit.
#[test]
fn square_wave_into_rc() {
    let mut c = Circuit::new();
    let f = 10.0;
    let (n1, n2, n0) = (c.node(), c.node(), c.node());
    c.set_ground(n0);
    let _src = c.add_signal_source(n1, n0, 1, 1.0, f, 0.0, 0.0);
    let _r = c.add_resistor(n1, n2, 1000.0);
    let _cap = c.add_capacitor(n2, n0, 1e-6);

    let mut sim = CircuitSim::new(c).with_integrator(Integrator::Trapezoidal);
    let period = 1.0 / f;
    let dt = period / 200.0;
    let probe = sim.add_probe(Probe::NodeVoltage(n2), 200);

    for _ in 0..5000 {
        sim.step(Seconds(dt));
    }

    let samples = sim.probe_samples(probe);
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &(_t, val) in &samples {
        if val < min {
            min = val;
        }
        if val > max {
            max = val;
        }
    }
    assert_close(max, 1.0, 1e-6);
    assert_close(min, -1.0, 1e-6);
}

/// Verifies backward Euler damps resonance.
#[test]
fn backward_euler_damps_resonance() {
    let mut c = Circuit::new();
    let f = 1591.5494309189535;
    let (n1, n2, n3, n0) = (c.node(), c.node(), c.node(), c.node());
    c.set_ground(n0);
    let _src = c.add_signal_source(n1, n0, 0, 1.0, f, 0.0, 0.0);
    let r = c.add_resistor(n1, n2, 10.0);
    let _l = c.add_inductor(n2, n3, 0.010);
    let _cap = c.add_capacitor(n3, n0, 1e-6);

    let mut sim = CircuitSim::new(c).with_integrator(Integrator::BackwardEuler);
    let period = 1.0 / f;
    let dt = period / 20.0;
    let probe = sim.add_probe(Probe::Current(r), 100);

    for _ in 0..500 {
        sim.step(Seconds(dt));
    }
    let amp = measure_amplitude(&sim, probe);
    assert!(amp < 0.095, "amplitude was {}", amp);
}

/// Verifies AC snapshot restore is bit identical.
#[test]
fn ac_snapshot_restore_is_bit_identical() {
    let mut c = Circuit::new();
    let f = 1591.5494309189535;
    let (n1, n2, n3, n0) = (c.node(), c.node(), c.node(), c.node());
    c.set_ground(n0);
    let _src = c.add_signal_source(n1, n0, 0, 1.0, f, 0.0, 0.0);
    let r = c.add_resistor(n1, n2, 10.0);
    let _l = c.add_inductor(n2, n3, 0.010);
    let _cap = c.add_capacitor(n3, n0, 1e-6);

    let mut sim = CircuitSim::new(c).with_integrator(Integrator::Trapezoidal);
    let period = 1.0 / f;
    let dt = period / 200.0;
    let probe_id = sim.add_probe(Probe::Current(r), 200);

    for _ in 0..2000 {
        sim.step(Seconds(dt));
    }

    let _snap = sim.snapshot();
    let mut sim2 = sim.snapshot();
    sim2 = sim2.with_integrator(Integrator::Trapezoidal);

    for _ in 0..1000 {
        sim.step(Seconds(dt));
        sim2.step(Seconds(dt));
    }

    let s1 = sim.probe_samples(probe_id);
    let s2 = sim2.probe_samples(probe_id);
    assert_eq!(s1.len(), s2.len());
    for (a, b) in s1.iter().zip(s2.iter()) {
        assert_eq!(a.0, b.0);
        assert_eq!(a.1, b.1);
    }
}
