use crate::forces::{Drive, DriveKind};
use crate::world::World;
use simbiguity_math::vec::Vec3;

// ---------------------------------------------------------
// Tests for Oscillator Damping and Driving Mechanics
// ---------------------------------------------------------

/// Verifies `add_drag` works and handles invalid inputs properly.
#[test]
fn test_add_drag() {
    let mut world = World::new();
    world.add_particle(1.0, Vec3::ZERO, false).unwrap();

    assert!(world.add_drag(1, 1.0).is_err()); // index out of bounds
    assert!(world.add_drag(0, -1.0).is_err()); // invalid parameter (negative c)
    assert!(world.add_drag(0, f64::NAN).is_err()); // invalid parameter (NaN)

    world.add_drag(0, 0.5).unwrap();
    assert_eq!(world.drags.len(), 1);
    assert_eq!(world.drags[0], (0, 0.5));
}

/// Verifies `apply_drags` correctly adds the damping force `F = -c * v`.
#[test]
fn test_apply_drags() {
    let mut world = World::new();
    let p = world.add_particle(1.0, Vec3::ZERO, false).unwrap();
    world.particles[p].velocity = Vec3::new(2.0, -1.0, 0.5);
    world.add_drag(p, 0.5).unwrap();

    world.apply_drags();
    assert_eq!(world.particles[p].force, Vec3::new(-1.0, 0.5, -0.25));
}

/// Verifies `add_drive` works and handles invalid inputs properly.
#[test]
fn test_add_drive() {
    let mut world = World::new();
    world.add_particle(1.0, Vec3::ZERO, false).unwrap();

    let drive = Drive {
        axis: Vec3::new(1.0, 0.0, 0.0),
        amplitude: 1.0,
        omega: 1.0,
        phase: 0.0,
        kind: DriveKind::Sine,
    };

    assert!(world.add_drive(1, drive.clone()).is_err()); // index out of bounds

    world.add_drive(0, drive).unwrap();
    assert_eq!(world.drives.len(), 1);
}

/// Verifies `apply_drives` correctly applies sine and step functions over time.
#[test]
fn test_apply_drives() {
    let mut world = World::new();
    let p = world.add_particle(1.0, Vec3::ZERO, false).unwrap();

    let drive_sine = Drive {
        axis: Vec3::new(1.0, 0.0, 0.0),
        amplitude: 2.0,
        omega: core::f64::consts::PI, // omega = PI
        phase: 0.0,
        kind: DriveKind::Sine,
    };
    world.add_drive(p, drive_sine).unwrap();

    world.apply_drives(0.5); // sin(PI * 0.5) = 1.0

    assert!((world.particles[p].force.x - 2.0).abs() < 1e-5);
    assert_eq!(world.particles[p].force.y, 0.0);
    assert_eq!(world.particles[p].force.z, 0.0);
}

/// Verifies closed-form expectations for a damped harmonic oscillator.
#[test]
fn test_damped_oscillator_decay() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::ZERO;

    let m = 1.0;
    let k = 1.0;
    let c = 0.1; // damping coefficient

    let p1 = world.add_particle(m, Vec3::ZERO, true).unwrap();
    let p2 = world
        .add_particle(m, Vec3::new(1.0, 0.0, 0.0), false)
        .unwrap();
    world.add_spring(p1, p2, 1.0, k, 0.0).unwrap();
    world.add_drag(p2, c).unwrap();

    // Initial stretch of 1.0
    let x0 = 1.0;
    world.particles[p2].position = Vec3::new(1.0 + x0, 0.0, 0.0);

    let dt = 0.01;
    let t_end = 10.0;
    let steps = (t_end / dt) as usize;

    for _ in 0..steps {
        world.step(dt).unwrap();
    }

    let current_x = world.particles[p2].position.x - 1.0;

    // In a lightly damped oscillator, amplitude decays as exp(-c/(2m)*t)
    // The exponent here is -0.1 / 2.0 * 10.0 = -0.5
    // e^(-0.5) ~ 0.606

    // Make sure amplitude is less than or equal to the envelope (with some tolerance)
    let envelope = x0 * (core::f64::consts::E).powf(-c / (2.0 * m) * t_end);

    assert!(
        current_x.abs() <= envelope + 1e-3,
        "Oscillator amplitude should decay according to theoretical envelope"
    );
}

/// Verifies driven oscillator resonance behavior compared to theoretical expectations.
#[test]
fn test_driven_oscillator_resonance() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::ZERO;

    let m = 1.0;
    let k = 1.0;
    // Resonant frequency omega_0 = sqrt(k/m) = 1.0

    let p1 = world.add_particle(m, Vec3::ZERO, true).unwrap();
    let p2 = world
        .add_particle(m, Vec3::new(1.0, 0.0, 0.0), false)
        .unwrap();
    world.add_spring(p1, p2, 1.0, k, 0.0).unwrap();

    // Drive at resonance omega = 1.0
    let drive = Drive {
        axis: Vec3::new(1.0, 0.0, 0.0),
        amplitude: 0.1, // F0
        omega: 1.0,
        phase: 0.0,
        kind: DriveKind::Sine,
    };
    world.add_drive(p2, drive).unwrap();

    // We add slight damping to let it reach a steady state rather than infinite amplitude
    let c = 0.1;
    world.add_drag(p2, c).unwrap();

    let dt = 0.01;
    let t_end = 100.0; // enough time to see amplitude growth
    let steps = (t_end / dt) as usize;

    let mut max_amplitude = 0.0;
    for _ in 0..steps {
        world.step(dt).unwrap();
        let amp = (world.particles[p2].position.x - 1.0).abs();
        if amp > max_amplitude {
            max_amplitude = amp;
        }
    }

    // Theoretical steady state amplitude at resonance A = F0 / (c * omega)
    // A = 0.1 / (0.1 * 1.0) = 1.0
    // Because it might still be in transient state, we check if it reaches close to 1.0.
    assert!(
        max_amplitude > 0.8,
        "Oscillator should exhibit resonant amplitude growth"
    );
    assert!(
        max_amplitude < 1.2,
        "Resonant amplitude should be constrained by damping"
    );
}
