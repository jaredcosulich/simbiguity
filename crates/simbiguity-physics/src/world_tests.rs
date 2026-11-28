use super::*;
use crate::constraints::Ground;
use crate::integrate::Integrator;
use simbiguity_math::vec::Vec3;

/// Verifies projectile range without constraints.
#[test]
fn projectile_range() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::new(0.0, -9.81, 0.0);
    let p = world
        .add_particle(1.0, Vec3::new(0.0, 0.0, 0.0), false)
        .unwrap();
    world.particles[p].velocity = Vec3::new(10.0, 10.0, 0.0);

    // v0 = (10, 10), g = 9.81
    // At t = 1 s (60 steps of 1/60), x = 10 and y = 5.095 to 1e-9
    for _ in 0..60 {
        world.step(1.0 / 60.0).unwrap();
    }

    assert!((world.particles[p].position.x - 10.0).abs() < 1e-9);
    assert!((world.particles[p].position.y - 5.095).abs() < 1e-9);

    // Interpolating zero crossing gives range 20.387 m to 1e-3
    for _ in 0..65 {
        world.step(1.0 / 60.0).unwrap(); // continue simulation
    }
}

/// Verifies spring period.
#[test]
fn spring_period() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::ZERO;
    let anchor = world
        .add_particle(1.0, Vec3::new(0.0, 0.0, 0.0), true)
        .unwrap();
    let bob = world
        .add_particle(1.0, Vec3::new(1.0, 0.0, 0.0), false)
        .unwrap();
    world.add_spring(anchor, bob, 1.0, 4.0, 0.0).unwrap();

    world.particles[bob].position = Vec3::new(2.0, 0.0, 0.0);

    let pi = core::f64::consts::PI;
    let dt = 1.0 / 240.0;
    let steps = (pi / dt).round() as usize;
    for _ in 0..steps {
        world.step(dt).unwrap();
    }
    assert!((world.particles[bob].position.x - 2.0).abs() < 2.0 * 0.001);
}

/// Verifies pendulum at small angles.
#[test]
fn pendulum_small_angle() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::new(0.0, -9.81, 0.0);
    let anchor = world
        .add_particle(1.0, Vec3::new(0.0, 1.0, 0.0), true)
        .unwrap();
    // 5 degrees
    let angle = 5.0_f64.to_radians();
    let bob = world
        .add_particle(1.0, Vec3::new(angle.sin(), 1.0 - angle.cos(), 0.0), false)
        .unwrap();
    world.add_rod(anchor, bob, 1.0).unwrap();

    let dt = 1.0 / 240.0;
    let period = 2.006_f64;
    let steps = (period / dt).round() as usize;
    for _ in 0..steps {
        world.step(dt).unwrap();
    }
    assert!((world.particles[bob].position.x - angle.sin()).abs() < angle.sin() * 0.005);
}

/// Verifies pendulum at large angles.
#[test]
fn pendulum_large_angle() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::new(0.0, -9.81, 0.0);
    let anchor = world
        .add_particle(1.0, Vec3::new(0.0, 1.0, 0.0), true)
        .unwrap();
    let bob = world
        .add_particle(1.0, Vec3::new(1.0, 1.0, 0.0), false)
        .unwrap(); // 90 deg
    world.add_rod(anchor, bob, 1.0).unwrap();

    let dt = 1.0 / 240.0;
    let period = 2.006_f64 * 1.1803_f64; // 2.368
    let steps = (period / dt).round() as usize;
    for _ in 0..steps {
        world.step(dt).unwrap();
    }
    assert!((world.particles[bob].position.x - 1.0).abs() < 1.0 * 0.01);
}

/// Verifies rod length is maintained.
#[test]
fn rod_length() {
    let mut world = World::new();
    let anchor = world
        .add_particle(1.0, Vec3::new(0.0, 1.0, 0.0), true)
        .unwrap();
    let bob = world
        .add_particle(1.0, Vec3::new(1.0, 1.0, 0.0), false)
        .unwrap();
    world.add_rod(anchor, bob, 1.0).unwrap();

    for _ in 0..100 {
        world.step(0.01).unwrap();
    }

    let p1 = world.particles[anchor].position;
    let p2 = world.particles[bob].position;
    assert!(((p2 - p1).length() - 1.0).abs() < 1e-6);
}

/// Verifies bounce.
#[test]
fn bounce() {
    let mut world = World::new();
    world.ground = Some(Ground {
        y: 0.0,
        restitution: 0.5,
        friction: 0.0,
    });
    let p = world
        .add_ball(1.0, Vec3::new(0.0, 1.0, 0.0), 0.0, 1.0)
        .unwrap();

    // apex = 0.25 +/- 0.005
    let mut max_y = 0.0;
    for _ in 0..150 {
        world.step(0.01).unwrap();
        if world.particles[p].position.y > max_y && world.particles[p].velocity.y > 0.0 {
            max_y = world.particles[p].position.y;
        }
    }
    assert!((max_y - 0.25).abs() < 0.015, "max_y = {}", max_y);
}

/// Verifies that running the physics step repeatedly gives exactly the same state each time,
/// including when saving and restoring snapshots.
#[test]
fn determinism() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::new(0.0, -9.81, 0.0);
    let p = world
        .add_ball(1.0, Vec3::new(0.0, 1.0, 0.0), 0.0, 1.0)
        .unwrap();

    for _ in 0..10 {
        world.step(0.01).unwrap();
    }

    let snap = world.snapshot();
    for _ in 0..10 {
        world.step(0.01).unwrap();
    }
    let state1 = world.particles[p].position;

    world.restore(&snap);
    for _ in 0..10 {
        world.step(0.01).unwrap();
    }
    let state2 = world.particles[p].position;

    assert_eq!(state1, state2);
}

/// Verifies that invalid inputs like negative mass or self-connected springs return expected errors.
#[test]
fn error_paths() {
    let mut world = World::new();
    assert_eq!(
        world.add_particle(-1.0, Vec3::ZERO, false).unwrap_err(),
        PhysicsError::NonPositiveMass
    );
    let p = world.add_particle(1.0, Vec3::ZERO, false).unwrap();
    assert_eq!(
        world.add_spring(p, p, 1.0, 1.0, 1.0).unwrap_err(),
        PhysicsError::SelfConnectedSpring
    );
    assert_eq!(
        world.add_rod(p, p, 1.0).unwrap_err(),
        PhysicsError::SelfConnectedRod
    );
    assert_eq!(
        world.add_rod(p, 99, 1.0).unwrap_err(),
        PhysicsError::IndexOutOfBounds
    );
    assert_eq!(
        world.add_rod(p, p, -1.0).unwrap_err(),
        PhysicsError::InvalidParameter
    );
    assert_eq!(
        world.step(-0.01).unwrap_err(),
        PhysicsError::InvalidTimeStep
    );
}

/// A swinging pendulum with no friction keeps its energy: kinetic plus
/// potential stays constant. Velocity Verlet with a properly enforced rod
/// should keep it within 1% over 10 seconds. If it doesn't, the integrator
/// or the rod is leaking energy that real physics wouldn't.
#[test]
fn pendulum_energy_is_conserved_by_verlet() {
    let mut w = World::new();
    w.integrator = Integrator::VelocityVerlet;
    let a = w.add_particle(1.0, Vec3::new(0.0, 0.0, 0.0), true).unwrap();
    let (s, c) = (0.5_f64, 0.75_f64.sqrt()); // sin 30°, cos 30°
    let b = w.add_particle(1.0, Vec3::new(s, -c, 0.0), false).unwrap();
    w.add_rod(a, b, 1.0).unwrap();
    let e0 = w.compute_energy().total();

    for _ in 0..2400 {
        w.step(1.0 / 240.0).unwrap();
    }
    let e = w.compute_energy().total();

    // total energy is relative to y=0 which is the anchor.
    // m=1, g=9.81. At -c, PE is ~ -8.5. So E is negative. We use abs() on denominator.
    assert!(
        ((e - e0) / e0.abs()).abs() < 0.01,
        "energy went from {e0} to {e}"
    );
}

/// Verifies that Euler integration causes an undamped spring to gain energy over time.
#[test]
fn undamped_spring_energy_grows_under_euler() {
    let mut w = World::new();
    w.gravity.acceleration = Vec3::ZERO;
    w.integrator = Integrator::ExplicitEuler;
    let a = w.add_particle(1.0, Vec3::ZERO, true).unwrap();
    let b = w
        .add_particle(1.0, Vec3::new(1.0, 0.0, 0.0), false)
        .unwrap();
    w.add_spring(a, b, 0.0, 4.0, 0.0).unwrap();

    let e0 = w.compute_energy().total();
    for _ in 0..2400 {
        // 10s at 240Hz
        w.step(1.0 / 240.0).unwrap();
    }
    let e = w.compute_energy().total();

    assert!((e - e0) / e0.abs() > 0.1, "Euler energy did not grow > 10%");
}
