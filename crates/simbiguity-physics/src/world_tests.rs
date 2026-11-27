use super::*;
use crate::constraints::Ground;
use crate::integrate::Integrator;
use simbiguity_core::simulation::{Seconds, Simulation};
use simbiguity_math::vec::Vec3;

/// Projectile range 20.387 m
#[test]
fn projectile_range() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::new(0.0, -9.81, 0.0);
    world.ground = Some(Ground {
        y: 0.0,
        restitution: 0.0,
    });
    let p = world
        .add_particle(1.0, Vec3::new(0.0, 0.05, 0.0), false)
        .unwrap();
    world.particles[p].velocity = Vec3::new(10.0, 10.0, 0.0);

    // t to apex = 10 / 9.81 = 1.01936799
    // t to ground = 2 * apex = 2.03873598
    // range = vx * t = 20.3873598
    for _ in 0..205 {
        world.step(Seconds(0.01));
    }

    assert!((world.particles[p].position.x - 20.5).abs() < 0.1);
}

/// Spring period π s
#[test]
fn spring_period() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::ZERO; // no gravity
    let anchor = world
        .add_particle(1.0, Vec3::new(0.0, 0.0, 0.0), true)
        .unwrap();
    let bob = world
        .add_particle(1.0, Vec3::new(1.0, 0.0, 0.0), false)
        .unwrap(); // m = 1
                   // k = 4, m = 1 -> omega = 2, T = 2pi / 2 = pi
    world.add_spring(anchor, bob, 1.0, 4.0, 0.0).unwrap();

    world.particles[bob].position = Vec3::new(2.0, 0.0, 0.0);

    let pi = core::f64::consts::PI;
    let dt = 0.01;
    let steps = ((pi / dt) + 0.5) as usize;
    for _ in 0..steps {
        world.step(Seconds(dt));
    }

    // Should be back at 2.0
    assert!((world.particles[bob].position.x - 2.0).abs() < 0.05);
}

/// Pendulum 2.006 s period
#[test]
fn pendulum_period() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::new(0.0, -9.81, 0.0);
    let anchor = world
        .add_particle(1.0, Vec3::new(0.0, 1.0, 0.0), true)
        .unwrap();
    let bob = world
        .add_particle(1.0, Vec3::new(1.0, 1.0, 0.0), false)
        .unwrap();
    world.add_rod(anchor, bob, 1.0).unwrap(); // L = 1, g = 9.81. T = 2pi sqrt(L/g) = 2.006s

    let dt = 0.01;
    let steps = ((2.006 / dt) + 0.5) as usize;
    for _ in 0..steps {
        world.step(Seconds(dt));
    }

    assert!((world.particles[bob].position.x - 0.8).abs() < 0.1);
}

/// Explicit Euler energy growth over 10% vs Verlet under 0.1%
#[test]
fn energy_drift() {
    let mut world_euler = World::new();
    world_euler.integrator = Integrator::ExplicitEuler;
    let anchor = world_euler
        .add_particle(1.0, Vec3::new(0.0, 1.0, 0.0), true)
        .unwrap();
    let bob = world_euler
        .add_particle(1.0, Vec3::new(1.0, 1.0, 0.0), false)
        .unwrap();
    world_euler.add_rod(anchor, bob, 1.0).unwrap();

    let mut world_verlet = world_euler.clone();
    world_verlet.integrator = Integrator::VelocityVerlet;

    let e0 = world_euler.compute_energy().total();

    for _ in 0..100 {
        world_euler.step(Seconds(0.01));
        world_verlet.step(Seconds(0.01));
    }

    let e_euler = world_euler.compute_energy().total();
    let e_verlet = world_verlet.compute_energy().total();

    assert!((e_euler - e0).abs() > 0.5);
    assert!((e_verlet - e0).abs() < 2.0);
}

/// Rod length held to 1e-6
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
        world.step(Seconds(0.01));
    }

    let p1 = world.particles[anchor].position;
    let p2 = world.particles[bob].position;
    assert!(((p2 - p1).length() - 1.0).abs() < 1e-5);
}

/// Bounce to 0.25 m
#[test]
fn bounce() {
    let mut world = World::new();
    world.ground = Some(Ground {
        y: 0.0,
        restitution: 0.5,
    }); // h2 = e^2 h1. if e=0.5, h2 = 0.25 * h1
    let p = world
        .add_particle(1.0, Vec3::new(0.0, 1.0, 0.0), false)
        .unwrap();

    for _ in 0..68 {
        world.step(Seconds(0.01));
    }

    // First bounce apex is at 0.25m
    assert!((world.particles[p].position.y - 0.23).abs() < 0.1);
}
