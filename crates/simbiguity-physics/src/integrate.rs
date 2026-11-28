#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Integrator {
    ExplicitEuler,
    SymplecticEuler,
    VelocityVerlet,
}

use crate::world::Particle;

pub fn step_explicit_euler(particles: &mut [Particle], dt: f64) {
    for p in particles {
        if !p.pinned {
            p.position += p.velocity * dt;
            p.velocity += (p.force / p.mass) * dt;
        }
    }
}

pub fn step_symplectic_euler(particles: &mut [Particle], dt: f64) {
    for p in particles {
        if !p.pinned {
            p.velocity += (p.force / p.mass) * dt;
            p.position += p.velocity * dt;
        }
    }
}

pub fn step_velocity_verlet_part1(particles: &mut [Particle], dt: f64) {
    for p in particles {
        if !p.pinned {
            p.velocity += (p.force / p.mass) * (dt * 0.5);
            p.position += p.velocity * dt;
        }
    }
}

pub fn step_velocity_verlet_part2(particles: &mut [Particle], dt: f64) {
    for p in particles {
        if !p.pinned {
            p.velocity += (p.force / p.mass) * (dt * 0.5);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simbiguity_math::vec::Vec3;

    /// Verifies Explicit Euler advances position using old velocity.
    #[test]
    fn test_explicit_euler() {
        let mut particles = [Particle {
            mass: 1.0,
            position: Vec3::ZERO,
            velocity: Vec3::new(1.0, 0.0, 0.0),
            force: Vec3::new(1.0, 0.0, 0.0),
            pinned: false,
            radius: 0.0,
            restitution: 0.0,
        }];
        step_explicit_euler(&mut particles, 1.0);
        assert_eq!(particles[0].position.x, 1.0);
        assert_eq!(particles[0].velocity.x, 2.0);
    }

    /// Verifies Symplectic Euler advances position using new velocity.
    #[test]
    fn test_symplectic_euler() {
        let mut particles = [Particle {
            mass: 1.0,
            position: Vec3::ZERO,
            velocity: Vec3::new(1.0, 0.0, 0.0),
            force: Vec3::new(1.0, 0.0, 0.0),
            pinned: false,
            radius: 0.0,
            restitution: 0.0,
        }];
        step_symplectic_euler(&mut particles, 1.0);
        assert_eq!(particles[0].velocity.x, 2.0);
        assert_eq!(particles[0].position.x, 2.0);
    }

    /// Verifies Velocity Verlet splits integration correctly.
    #[test]
    fn test_velocity_verlet() {
        let mut particles = [Particle {
            mass: 1.0,
            position: Vec3::ZERO,
            velocity: Vec3::new(1.0, 0.0, 0.0),
            force: Vec3::new(1.0, 0.0, 0.0),
            pinned: false,
            radius: 0.0,
            restitution: 0.0,
        }];
        step_velocity_verlet_part1(&mut particles, 1.0);
        assert_eq!(particles[0].velocity.x, 1.5);
        assert_eq!(particles[0].position.x, 1.5);
        particles[0].force = Vec3::new(2.0, 0.0, 0.0);
        step_velocity_verlet_part2(&mut particles, 1.0);
        assert_eq!(particles[0].velocity.x, 2.5);
    }
}
