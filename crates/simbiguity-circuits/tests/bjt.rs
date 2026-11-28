use simbiguity_circuits::component::Polarity;
use simbiguity_circuits::Circuit;

fn assert_close(actual: f64, expected: f64, tol: f64, msg: &str) {
    assert!(
        (actual - expected).abs() <= tol,
        "{}: expected {}, got {} (diff {})",
        msg,
        expected,
        actual,
        (actual - expected).abs()
    );
}

/// Verifies that an NPN BJT with a 470k base resistor provides roughly beta=100 amplification.
#[test]
fn bjt_forward_active_470k_gives_beta_100() {
    let mut c = Circuit::new();
    let vcc = c.node();
    let b = c.node();
    let col = c.node();
    let e = c.node();

    c.add_battery(vcc, e, 9.0);
    c.add_resistor(vcc, b, 470_000.0);
    c.add_resistor(vcc, col, 1_000.0);
    let bjt = c.add_bjt(col, b, e, Polarity::Npn, 1e-14, 100.0, 1.0);
    c.set_ground(e);

    let ans = c.solve();
    assert!(ans.solution().is_some());
    let sol = ans.solution().unwrap();

    let v_be = sol.node_voltage(b).unwrap();
    let v_ce = sol.node_voltage(col).unwrap();
    assert_close(v_be, 0.669587, 1e-5, "V_BE");
    assert_close(v_ce, 7.227572, 1e-5, "V_CE");

    let ((i_c, i_b, i_e), region) = sol
        .bjt_currents_and_region(&c.parts()[bjt.index()])
        .unwrap();
    assert_close(i_b, 17.7243e-6, 1e-9, "I_B");
    assert_close(i_c, 1.772428e-3, 1e-8, "I_C");
    assert_eq!(region, "Active");
    assert_close(i_c / i_b, 100.0, 0.01, "Beta");
    assert_close(i_e, -(i_b + i_c), 1e-12, "KCL");
}

/// Verifies that an NPN BJT saturates properly with a 47k base resistor.
#[test]
fn bjt_saturated_47k() {
    let mut c = Circuit::new();
    let vcc = c.node();
    let b = c.node();
    let col = c.node();
    let e = c.node();

    c.add_battery(vcc, e, 9.0);
    c.add_resistor(vcc, b, 47_000.0);
    c.add_resistor(vcc, col, 1_000.0);
    let bjt = c.add_bjt(col, b, e, Polarity::Npn, 1e-14, 100.0, 1.0);
    c.set_ground(e);

    let ans = c.solve();
    let sol = ans.solution().unwrap();

    let v_be = sol.node_voltage(b).unwrap();
    let v_ce = sol.node_voltage(col).unwrap();
    assert_close(v_be, 0.711740, 1e-5, "V_BE");
    assert_close(v_ce, 0.120425, 1e-5, "V_CE");

    let ((i_c, i_b, i_e), region) = sol
        .bjt_currents_and_region(&c.parts()[bjt.index()])
        .unwrap();
    assert_close(i_c, 8.879575e-3, 1e-8, "I_C");
    assert_close(i_b, 176.346e-6, 1e-8, "I_B");
    assert_eq!(region, "Saturation");
    assert_close(i_c / i_b, 50.35, 0.1, "Beta Sat");
    assert_close(i_e, -(i_b + i_c), 1e-12, "KCL");
}

/// Verifies that an open base terminal correctly places the BJT in cutoff region.
#[test]
fn bjt_base_open_is_cutoff() {
    let mut c = Circuit::new();
    let vcc = c.node();
    let b = c.node();
    let col = c.node();
    let e = c.node();

    c.add_battery(vcc, e, 9.0);
    // Base is open
    c.add_resistor(vcc, col, 1_000.0);
    let bjt = c.add_bjt(col, b, e, Polarity::Npn, 1e-14, 100.0, 1.0);
    c.set_ground(e);

    let ans = c.solve();
    let sol = ans.solution().unwrap();

    let ((i_c, i_b, i_e), region) = sol
        .bjt_currents_and_region(&c.parts()[bjt.index()])
        .unwrap();
    assert!(
        i_c.abs() < 2e-12,
        "I_C should be < 2pA. Actual i_c: {}",
        i_c
    );
    assert_eq!(region, "Cutoff");
    assert_close(i_e, -(i_b + i_c), 1e-12, "KCL");
}

/// Verifies PNP BJT functioning as a mirror of forward active NPN.
#[test]
fn pnp_mirror_of_forward_active() {
    let mut c = Circuit::new();
    let vcc = c.node(); // -9V
    let b = c.node();
    let col = c.node();
    let e = c.node();

    c.add_battery(vcc, e, -9.0);
    c.add_resistor(vcc, b, 470_000.0);
    c.add_resistor(vcc, col, 1_000.0);
    let bjt = c.add_bjt(col, b, e, Polarity::Pnp, 1e-14, 100.0, 1.0);
    c.set_ground(e);

    let ans = c.solve();
    let sol = ans.solution().unwrap();

    let v_be = sol.node_voltage(b).unwrap(); // relative to e (gnd)
    let v_ce = sol.node_voltage(col).unwrap();
    assert_close(v_be, -0.669587, 1e-5, "V_BE PNP");
    assert_close(v_ce, -7.227572, 1e-5, "V_CE PNP");

    let ((i_c, i_b, i_e), region) = sol
        .bjt_currents_and_region(&c.parts()[bjt.index()])
        .unwrap();
    assert_close(i_b, -17.7243e-6, 1e-9, "I_B PNP");
    assert_close(i_c, -1.772428e-3, 1e-8, "I_C PNP");
    assert_eq!(region, "Active");
    assert_close(i_e, -(i_b + i_c), 1e-12, "KCL");
}

/// Verifies a transistor switch correctly driving an LED.
#[test]
fn led_driven_by_transistor_switch() {
    let mut c = Circuit::new();
    let vcc = c.node();
    let e = c.node();
    let sw_out = c.node();
    let b = c.node();
    let led_out = c.node();
    let col = c.node();

    c.add_battery(vcc, e, 9.0);

    // Switch to base: 10k res + switch
    let sw = c.add_switch(vcc, sw_out, false);
    c.add_resistor(sw_out, b, 10_000.0);

    // LED path: 330 ohm + LED -> Collector
    c.add_resistor(vcc, led_out, 330.0);
    let led = c.add_led(led_out, col, "red", 0.02, 1e-20, 2.0);

    c.add_bjt(col, b, e, Polarity::Npn, 1e-14, 100.0, 1.0);
    c.set_ground(e);

    // Open
    let ans1 = c.solve();
    let sol1 = ans1.solution().unwrap();
    assert!(sol1.brightness(led).unwrap() < 1e-3, "LED off when open");

    // Closed
    c.set_switch(sw, true);
    let ans2 = c.solve();
    let sol2 = ans2.solution().unwrap();
    assert!(sol2.brightness(led).unwrap() > 0.9, "LED on when closed");
}
