use crate::collide::{find_pairs_broadphase, find_pairs_brute};
use crate::constraints::Ground;
use crate::world::World;
use simbiguity_math::vec::Vec3;

/// Verifies that an elastic collision between different masses results in correct post-collision velocities.
#[test]
fn elastic_collision_one_kg_into_three_kg_bounces_back_at_minus_one() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::ZERO;
    let p1 = world
        .add_ball(1.0, Vec3::new(-1.0, 0.0, 0.0), 0.1, 1.0)
        .unwrap();
    let p2 = world
        .add_ball(3.0, Vec3::new(0.0, 0.0, 0.0), 0.1, 1.0)
        .unwrap();

    world.particles[p1].velocity.x = 2.0;

    for _ in 0..100 {
        world.step(0.01).unwrap();
    }

    let v1 = world.particles[p1].velocity.x;
    let v2 = world.particles[p2].velocity.x;

    assert!((v1 - (-1.0)).abs() < 1e-9);
    assert!((v2 - 1.0).abs() < 1e-9);

    let momentum = world.particles[p1].mass * v1 + world.particles[p2].mass * v2;
    assert!((momentum - 2.0).abs() < 1e-12);

    let energy =
        0.5 * world.particles[p1].mass * v1 * v1 + 0.5 * world.particles[p2].mass * v2 * v2;
    assert!((energy - 2.0).abs() < 1e-9);
}

/// Verifies that a perfectly inelastic collision results in both objects sticking together with zero restitution.
#[test]
fn perfectly_inelastic_collision() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::ZERO;
    let p1 = world
        .add_ball(1.0, Vec3::new(-1.0, 0.0, 0.0), 0.1, 0.0)
        .unwrap();
    let p2 = world
        .add_ball(3.0, Vec3::new(0.0, 0.0, 0.0), 0.1, 0.0)
        .unwrap();

    world.particles[p1].velocity.x = 2.0;

    for _ in 0..100 {
        world.step(0.01).unwrap();
    }

    let v1 = world.particles[p1].velocity.x;
    let v2 = world.particles[p2].velocity.x;

    assert!((v1 - 0.5).abs() < 1e-9);
    assert!((v2 - 0.5).abs() < 1e-9);
}

/// Verifies that two equal masses with an elastic head-on collision swap their velocities.
#[test]
fn equal_masses_elastic_head_on() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::ZERO;
    let p1 = world
        .add_ball(2.0, Vec3::new(-1.0, 0.0, 0.0), 0.1, 1.0)
        .unwrap();
    let p2 = world
        .add_ball(2.0, Vec3::new(1.0, 0.0, 0.0), 0.1, 1.0)
        .unwrap();

    world.particles[p1].velocity.x = 1.0;
    world.particles[p2].velocity.x = -1.0;

    for _ in 0..200 {
        world.step(0.01).unwrap();
    }

    let v1 = world.particles[p1].velocity.x;
    let v2 = world.particles[p2].velocity.x;

    assert!((v1 - (-1.0)).abs() < 1e-9);
    assert!((v2 - 1.0).abs() < 1e-9);
}

/// Verifies Newton's cradle momentum transfer behavior across multiple aligned balls.
#[test]
fn newtons_cradle() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::ZERO;
    for i in 0..5 {
        world
            .add_ball(1.0, Vec3::new(i as f64 * 0.2, 0.0, 0.0), 0.1, 1.0)
            .unwrap();
    }

    world.particles[0].position.x -= 1.0;
    world.particles[0].velocity.x = 1.0;

    for _ in 0..200 {
        world.step(0.01).unwrap();
    }

    let v_last = world.particles[4].velocity.x;
    assert!((v_last - 1.0).abs() < 1e-6);

    for i in 0..4 {
        assert!(world.particles[i].velocity.length() < 1e-6);
    }
}

/// Verifies that a ball dropped on a restitution 0.5 ground bounces to exactly the expected height.
#[test]
fn bounce_drop() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::new(0.0, -9.81, 0.0);
    world.ground = Some(Ground {
        y: 0.0,
        restitution: 0.5,
        friction: 0.0,
    });

    let p = world
        .add_ball(1.0, Vec3::new(0.0, 1.1, 0.0), 0.1, 1.0)
        .unwrap();

    let dt = 1.0 / 1000.0;
    let mut max_y = 0.0;
    let mut hit_ground = false;

    for _ in 0..2000 {
        world.step(dt).unwrap();
        let y = world.particles[p].position.y - world.particles[p].radius;
        if world.particles[p].velocity.y > 0.0 {
            hit_ground = true;
        }
        if hit_ground && world.particles[p].velocity.y <= 0.0 && max_y == 0.0 {
            max_y = y;
        }
    }

    assert!((max_y - 0.250).abs() < 0.0025); // ± 1%
}

/// Verifies that kinematic friction applied to a sliding ball reduces its velocity correctly over time.
#[test]
fn ground_friction_slide() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::new(0.0, -9.81, 0.0);
    world.ground = Some(Ground {
        y: 0.0,
        restitution: 0.0,
        friction: 0.5,
    });

    let p = world
        .add_ball(1.0, Vec3::new(0.0, 0.1, 0.0), 0.1, 0.0)
        .unwrap();
    world.particles[p].velocity.x = 2.0;

    let dt = 0.01;
    for _ in 0..100 {
        world.step(dt).unwrap();
    }

    let x = world.particles[p].position.x;
    assert!((x - 0.4177).abs() < 0.005);
    assert_eq!(world.particles[p].velocity.x, 0.0);
}

/// Verifies momentum conservation for a large number of randomly overlapping balls inside a box.
#[test]
fn balls_in_box_overlap_and_momentum() {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::new(0.0, -9.81, 0.0);
    world.ground = Some(Ground {
        y: 0.0,
        restitution: 0.2,
        friction: 0.5,
    });

    // Add 4 pinned walls of balls
    for x in -10..=10 {
        let p = world
            .add_ball(1.0, Vec3::new(x as f64 * 0.2, 0.1, 10.2), 0.1, 0.2)
            .unwrap();
        world.particles[p].pinned = true;
        let p = world
            .add_ball(1.0, Vec3::new(x as f64 * 0.2, 0.1, -10.2), 0.1, 0.2)
            .unwrap();
        world.particles[p].pinned = true;
    }
    for z in -10..=10 {
        let p = world
            .add_ball(1.0, Vec3::new(10.2, 0.1, z as f64 * 0.2), 0.1, 0.2)
            .unwrap();
        world.particles[p].pinned = true;
        let p = world
            .add_ball(1.0, Vec3::new(-10.2, 0.1, z as f64 * 0.2), 0.1, 0.2)
            .unwrap();
        world.particles[p].pinned = true;
    }

    // Drop 400 balls
    use simbiguity_core::Rng;
    let mut rng = Rng::new(42);
    for _ in 0..400 {
        let x = (rng.next_f64() - 0.5) * 18.0;
        let y = rng.next_f64() * 10.0 + 1.0;
        let z = (rng.next_f64() - 0.5) * 18.0;
        world.add_ball(1.0, Vec3::new(x, y, z), 0.1, 0.2).unwrap();
    }

    let dt = 0.01;
    for _ in 0..1000 {
        world.step(dt).unwrap();
    }

    let mut mom_x = 0.0;
    let mut mom_z = 0.0;
    for p in &world.particles {
        if !p.pinned {
            mom_x += p.mass * p.velocity.x;
            mom_z += p.mass * p.velocity.z;
        }
    }

    assert!(mom_x.abs() < 1e-9);
    assert!(mom_z.abs() < 1e-9);

    for i in 0..world.particles.len() {
        for j in i + 1..world.particles.len() {
            let p1 = &world.particles[i];
            let p2 = &world.particles[j];
            if p1.pinned && p2.pinned {
                continue;
            }
            let dist = (p1.position - p2.position).length();
            let r = p1.radius + p2.radius;
            if dist < r {
                let overlap = r - dist;
                assert!(overlap <= 0.01 * p1.radius.min(p2.radius));
            }
        }
    }
}

/// Verifies that the broadphase collision pair generation matches the brute-force O(N^2) pair generation exactly.
#[test]
fn broadphase_equivalence() {
    use simbiguity_core::Rng;
    let mut rng = Rng::new(12345);

    for _ in 0..100 {
        let mut world = World::new();
        for _ in 0..300 {
            let x = (rng.next_f64() - 0.5) * 20.0;
            let y = (rng.next_f64() - 0.5) * 20.0;
            let z = (rng.next_f64() - 0.5) * 20.0;
            let r = rng.next_f64() * 0.5 + 0.1;
            world.add_ball(1.0, Vec3::new(x, y, z), r, 0.5).unwrap();
        }

        let mut brute = find_pairs_brute(&world.particles);
        let mut broad = find_pairs_broadphase(&world.particles);

        brute.sort_unstable();
        broad.sort_unstable();

        assert_eq!(brute, broad);
    }
}
