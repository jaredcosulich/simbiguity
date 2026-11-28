use crate::world::World;
use crate::Integrator;
use core::f64::consts::TAU;
use simbiguity_math::vec::Vec3;

fn run_orbit(world: &mut World, p_id: usize, dt: f64, steps: usize) -> (f64, usize, f64) {
    let mut last_y = world.particles[p_id].position.y;
    let mut crossings = 0;
    let mut last_crossing_time = 0.0;
    let mut max_r = 0.0;

    for i in 1..=steps {
        world.step(dt).unwrap();
        let y = world.particles[p_id].position.y;
        let x = world.particles[p_id].position.x;
        let r = world.particles[p_id].position.length();
        if r > max_r {
            max_r = r;
        }
        if last_y <= 0.0 && y > 0.0 && x > 0.0 {
            crossings += 1;
            if last_crossing_time == 0.0 {
                let fraction = (0.0 - last_y) / (y - last_y);
                last_crossing_time = (i as f64 - 1.0 + fraction) * dt;
            }
        }
        last_y = y;
    }
    (last_crossing_time, crossings, max_r)
}

fn measure_swept_area(world: &mut World, p_id: usize, dt: f64, steps: usize) -> f64 {
    let mut area = 0.0;
    for _ in 0..steps {
        let pos0 = world.particles[p_id].position;
        world.step(dt).unwrap();
        let pos1 = world.particles[p_id].position;
        area += 0.5 * (pos0.x * pos1.y - pos0.y * pos1.x).abs();
    }
    area
}

/// Verifies circular orbit period is two pi
#[test]
fn circular_orbit_period_is_two_pi() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::ZERO;
    world.add_attractor(Vec3::ZERO, 1.0, 0.0);
    let p = world
        .add_particle(1.0, Vec3::new(1.0, 0.0, 0.0), false)
        .unwrap();
    world.particles[p].velocity = Vec3::new(0.0, 1.0, 0.0);

    let dt = 1e-3;
    let (t, _, _) = run_orbit(&mut world, p, dt, 10000);
    assert!((t - TAU).abs() < 1e-3, "Expected {}, got {}", TAU, t);
    let r = world.particles[p].position.length();
    assert!((r - 1.0).abs() < 1e-4, "Radius drift {}", r - 1.0);
}

/// Verifies circular orbit energy drift is bounded
#[test]
fn circular_orbit_energy_drift_is_bounded() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::ZERO;
    world.add_attractor(Vec3::ZERO, 1.0, 0.0);
    let p = world
        .add_particle(1.0, Vec3::new(1.0, 0.0, 0.0), false)
        .unwrap();
    world.particles[p].velocity = Vec3::new(0.0, 1.0, 0.0);

    let e0 = world.compute_energy().total();
    let dt = 1e-3;
    let steps = (10.0 * TAU / dt) as usize;
    for _ in 0..steps {
        world.step(dt).unwrap();
    }
    let e1 = world.compute_energy().total();
    assert!(((e1 - e0) / e0).abs() < 1e-6);
}

/// Verifies circular orbit explodes with euler integration
#[test]
fn circular_orbit_euler_explodes() {
    let mut world = World::new();
    world.integrator = Integrator::ExplicitEuler;
    world.gravity.acceleration = Vec3::ZERO;
    world.add_attractor(Vec3::ZERO, 1.0, 0.0);
    let p = world
        .add_particle(1.0, Vec3::new(1.0, 0.0, 0.0), false)
        .unwrap();
    world.particles[p].velocity = Vec3::new(0.0, 1.0, 0.0);

    let e0 = world.compute_energy().total();
    let dt = 1e-3;
    let steps = (10.0 * TAU / dt) as usize;
    for _ in 0..steps {
        world.step(dt).unwrap();
    }
    let e1 = world.compute_energy().total();
    let r = world.particles[p].position.length();

    // e increases (gets less negative or positive) -> e1 > e0 * 1.01 ? wait, e is negative (-0.5). 1% means it becomes -0.495 or greater.
    assert!((e1 - e0) > e0.abs() * 0.01);
    assert!(r > 1.01);
}

/// Verifies ellipse period and aphelion are correct
#[test]
fn ellipse_period_and_aphelion() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::ZERO;
    world.add_attractor(Vec3::ZERO, 1.0, 0.0);
    let p = world
        .add_particle(1.0, Vec3::new(1.0, 0.0, 0.0), false)
        .unwrap();
    world.particles[p].velocity = Vec3::new(0.0, 1.22474487139, 0.0);

    let dt = 1e-3;
    let expected_period = 17.7715;
    let steps = (expected_period / dt) as usize + 1000;

    let (period, _, max_r) = run_orbit(&mut world, p, dt, steps);

    assert!((max_r - 3.0).abs() < 1e-3);
    assert!((period - 17.7715).abs() < 1e-2, "period = {}", period);
}

/// Verifies Kepler's second law for an ellipse
#[test]
fn ellipse_keplers_second_law() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::ZERO;
    world.add_attractor(Vec3::ZERO, 1.0, 0.0);
    let p = world
        .add_particle(1.0, Vec3::new(1.0, 0.0, 0.0), false)
        .unwrap();
    world.particles[p].velocity = Vec3::new(0.0, 1.22474487139, 0.0);

    let dt = 1e-3;
    let window_steps = 500; // 0.5s

    // area at perihelion
    let area_peri = measure_swept_area(&mut world, p, dt, window_steps);

    let steps_to_aphelion = (17.7715 / 2.0 / dt) as usize - window_steps;
    for _ in 0..steps_to_aphelion {
        world.step(dt).unwrap();
    }

    let area_ap = measure_swept_area(&mut world, p, dt, window_steps);

    let expected = 0.306186;
    assert!((area_peri - expected).abs() / expected < 1e-4);
    assert!((area_ap - expected).abs() / expected < 1e-4);
}

/// Verifies binary momentum is conserved and separation is constant
#[test]
fn two_body_binary_momentum_and_separation() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::ZERO;
    world.mutual_gravity = Some(1.0);
    let p1 = world
        .add_particle(1.0, Vec3::new(1.0, 0.0, 0.0), false)
        .unwrap();
    world.particles[p1].velocity = Vec3::new(0.0, 0.5, 0.0);
    let p2 = world
        .add_particle(1.0, Vec3::new(-1.0, 0.0, 0.0), false)
        .unwrap();
    world.particles[p2].velocity = Vec3::new(0.0, -0.5, 0.0);

    let dt = 1e-3;
    let steps = (10.0 * 4.0 * core::f64::consts::PI / dt) as usize;
    for _ in 0..steps {
        world.step(dt).unwrap();
    }

    let pos1 = world.particles[p1].position;
    let pos2 = world.particles[p2].position;
    let com = (pos1 + pos2) * 0.5;
    assert!(com.length() < 1e-12);

    let mom = world.particles[p1].velocity + world.particles[p2].velocity;
    assert!(mom.length() < 1e-12);

    let sep = (pos1 - pos2).length();
    assert!((sep - 2.0).abs() < 1e-4);
}

/// Verifies figure eight system returns to start
#[test]
fn three_body_figure_eight_returns_to_start() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::ZERO;
    world.mutual_gravity = Some(1.0);
    let p1_start = Vec3::new(0.97000436, -0.24308753, 0.0);
    let p2_start = Vec3::new(-0.97000436, 0.24308753, 0.0);
    let p3_start = Vec3::new(0.0, 0.0, 0.0);
    let p1 = world.add_particle(1.0, p1_start, false).unwrap();
    world.particles[p1].velocity = Vec3::new(0.466203685, 0.43236573, 0.0);
    let p2 = world.add_particle(1.0, p2_start, false).unwrap();
    world.particles[p2].velocity = Vec3::new(0.466203685, 0.43236573, 0.0);
    let p3 = world.add_particle(1.0, p3_start, false).unwrap();
    world.particles[p3].velocity = Vec3::new(-0.93240737, -0.86473146, 0.0);

    let dt = 1e-3;
    let t = 6.32591;
    let steps = (t / dt) as usize;
    for _ in 0..steps {
        world.step(dt).unwrap();
    }

    assert!((world.particles[p1].position - p1_start).length() < 2e-3);
    assert!((world.particles[p2].position - p2_start).length() < 2e-3);
    assert!(
        (world.particles[p3].position - p3_start).length() < 2e-3,
        "err was {}",
        (world.particles[p3].position - p3_start).length()
    );
}
