//! Nonlinear circuit tests

use simbiguity_circuits::Circuit;

fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// Verifies diode current for a given 1k series resistor.
#[test]
fn diode_with_1k_from_5v_drops_0_6925_volts() {
    let mut c = Circuit::new();
    let n0 = c.node();
    let n1 = c.node();
    let n2 = c.node();
    c.add_battery(n1, n0, 5.0);
    c.add_resistor(n1, n2, 1000.0);
    let d = c.add_diode(n2, n0, 1e-14, 1.0);

    let a = c.solve();
    let s = a.solution().unwrap();

    let vd = s.voltage_across(d).unwrap();
    let id = s.current(d);

    assert!(approx_eq(vd, 0.692544, 1e-5), "Vd was {vd}");
    assert!(approx_eq(id, 4.307456e-3, 1e-8), "Id was {id}");
}

/// Verifies solving a circuit with two diodes in series.
#[test]
fn two_diodes_in_series() {
    let mut c = Circuit::new();
    let n0 = c.node();
    let n1 = c.node();
    let n2 = c.node();
    let n3 = c.node();
    c.add_battery(n1, n0, 5.0);
    c.add_resistor(n1, n2, 1000.0);
    let d1 = c.add_diode(n2, n3, 1e-14, 1.0);
    let d2 = c.add_diode(n3, n0, 1e-14, 1.0);

    let a = c.solve();
    let s = a.solution().unwrap();

    let vd1 = s.voltage_across(d1).unwrap();
    let vd2 = s.voltage_across(d2).unwrap();
    let id = s.current(d1);

    assert!(approx_eq(vd1, 0.688076, 1e-5));
    assert!(approx_eq(vd2, 0.688076, 1e-5));
    assert!(approx_eq(id, 3.623848e-3, 1e-8));
}

/// Verifies reversed diode drops to negligible current.
#[test]
fn diode_reversed() {
    let mut c = Circuit::new();
    let n0 = c.node();
    let n1 = c.node();
    let n2 = c.node();
    c.add_battery(n1, n0, 5.0);
    c.add_resistor(n1, n2, 1000.0);
    // reversed: n0 to n2, or n2 to n0 with cathode at n2?
    // Diode flows from a to b. If n0 is a, n2 is b, it's reversed.
    let d = c.add_diode(n0, n2, 1e-14, 1.0);

    let a = c.solve();
    let s = a.solution().unwrap();

    let id = s.current(d); // flows n0 -> n2, should be roughly -1e-14.
                           // wait, if 5V is on n2, n0 is ground. So voltage is -5V across diode (a - b = 0 - 5 = -5).
                           // current is Is * (e^(-5/Vt) - 1) = -Is.
    assert!(approx_eq(id, -1e-14, 1e-15));
}

/// Verifies LED with a standard 160 ohm series resistor matches operating point.
#[test]
fn led_with_160_ohm() {
    let mut c = Circuit::new();
    let n0 = c.node();
    let n1 = c.node();
    let n2 = c.node();
    c.add_battery(n1, n0, 5.0);
    c.add_resistor(n1, n2, 160.0);
    let led = c.add_led(n2, n0, "red", 0.020, 1.5195e-17, 2.0);

    let a = c.solve();
    let s = a.solution().unwrap();

    let vf = s.voltage_across(led).unwrap();
    let id = s.current(led);
    let light = s.brightness(led).unwrap();

    assert!(approx_eq(vf, 1.80000, 1e-4), "Vf was {vf}");
    assert!(approx_eq(id, 0.020, 1e-6), "Id was {id}");
    assert!(approx_eq(light, 1.0, 1e-3), "Light was {light}");
}

/// Verifies LED with a standard 330 ohm series resistor matches operating point.
#[test]
fn led_with_330_ohm() {
    let mut c = Circuit::new();
    let n0 = c.node();
    let n1 = c.node();
    let n2 = c.node();
    c.add_battery(n1, n0, 9.0);
    c.add_resistor(n1, n2, 330.0);
    let led = c.add_led(n2, n0, "red", 0.020, 1.5195e-17, 2.0);

    let a = c.solve();
    let s = a.solution().unwrap();

    let vf = s.voltage_across(led).unwrap();
    let id = s.current(led);

    assert!(approx_eq(vf, 1.80447, 1e-4), "Vf was {vf}");
    assert!(approx_eq(id, 0.021805, 1e-6), "Id was {id}");
}

/// Verifies connecting LED with too small a resistor blows it.
#[test]
fn led_direct_blows() {
    let mut c = Circuit::new();
    let n0 = c.node();
    let n1 = c.node();
    let n2 = c.node();
    c.add_battery(n1, n0, 9.0);
    c.add_resistor(n1, n2, 10.0);
    let led = c.add_led(n2, n0, "red", 0.020, 1.5195e-17, 2.0);

    let a = c.solve();
    let _s = a.solution().unwrap();

    // Check that it's blown
    let issues = a.issues();
    let is_blown = issues.iter().any(|i| match i {
        simbiguity_circuits::CircuitIssue::BulbBlown { bulb } => *bulb == led,
        _ => false,
    });
    assert!(is_blown);
}

// "any circuit from invariants_tests.rs::generate_circuit (2000 seeds) newton_iterations() == 1; KCL still holds"
// Actually I don't have generate_circuit exported publicly, I will write the test in invariants_tests.rs.
// "Add KCL over 500 random circuits that each include 1–3 diodes: net current ≤ 1e-9·max|I| at every node."
// I will just do it in invariants_tests.rs!
