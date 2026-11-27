use crate::constraints::{Ground, Rod};
use crate::energy::Energy;
use crate::error::PhysicsError;
use crate::forces::{Gravity, Spring};
use crate::integrate::Integrator;
use simbiguity_core::simulation::{Seconds, Simulation, Snapshot};
use simbiguity_math::vec::Vec3;

#[derive(Debug, Clone)]
pub struct Particle {
    pub mass: f64,
    pub position: Vec3,
    pub velocity: Vec3,
    pub force: Vec3,
    pub pinned: bool,
}

#[derive(Debug, Clone)]
pub enum PhysicsInput {
    Push { index: usize, force: Vec3 },
    SetPinned { index: usize, pinned: bool },
    SetGravity { acceleration: Vec3 },
}

#[derive(Debug, Clone)]
pub struct World {
    pub particles: alloc::vec::Vec<Particle>,
    pub springs: alloc::vec::Vec<Spring>,
    pub rods: alloc::vec::Vec<Rod>,
    pub gravity: Gravity,
    pub ground: Option<Ground>,
    pub integrator: Integrator,
    pub input_queue: alloc::vec::Vec<PhysicsInput>,
}

impl World {
    pub fn new() -> Self {
        World {
            particles: alloc::vec::Vec::new(),
            springs: alloc::vec::Vec::new(),
            rods: alloc::vec::Vec::new(),
            gravity: Gravity {
                acceleration: Vec3::new(0.0, -9.81, 0.0),
            },
            ground: None,
            integrator: Integrator::VelocityVerlet,
            input_queue: alloc::vec::Vec::new(),
        }
    }

    pub fn add_particle(
        &mut self,
        mass: f64,
        position: Vec3,
        pinned: bool,
    ) -> Result<usize, PhysicsError> {
        let mass = if pinned && (mass.is_nan() || mass <= 0.0) {
            1.0
        } else {
            mass
        };
        if mass.is_nan() || mass <= 0.0 {
            return Err(PhysicsError::NonPositiveMass);
        }
        let index = self.particles.len();
        self.particles.push(Particle {
            mass,
            position,
            velocity: Vec3::ZERO,
            force: Vec3::ZERO,
            pinned,
        });
        Ok(index)
    }

    pub fn add_spring(
        &mut self,
        p1: usize,
        p2: usize,
        rest_length: f64,
        k: f64,
        damping: f64,
    ) -> Result<(), PhysicsError> {
        if p1 >= self.particles.len() || p2 >= self.particles.len() {
            return Err(PhysicsError::IndexOutOfBounds);
        }
        if rest_length.is_nan()
            || rest_length < 0.0
            || k.is_nan()
            || k < 0.0
            || damping.is_nan()
            || damping < 0.0
        {
            return Err(PhysicsError::InvalidParameter);
        }
        if p1 == p2 {
            return Err(PhysicsError::SelfConnectedSpring);
        }
        self.springs.push(Spring {
            p1,
            p2,
            rest_length,
            k,
            damping,
        });
        Ok(())
    }

    pub fn add_rod(&mut self, p1: usize, p2: usize, length: f64) -> Result<(), PhysicsError> {
        if p1 >= self.particles.len() || p2 >= self.particles.len() {
            return Err(PhysicsError::IndexOutOfBounds);
        }
        if length.is_nan() || length <= 0.0 {
            return Err(PhysicsError::InvalidParameter);
        }
        if p1 == p2 {
            return Err(PhysicsError::SelfConnectedRod);
        }
        self.rods.push(Rod { p1, p2, length });
        Ok(())
    }

    pub fn compute_energy(&self) -> Energy {
        let mut e = Energy::default();
        for p in &self.particles {
            if !p.pinned {
                e.kinetic += 0.5 * p.mass * p.velocity.length_squared();
            }
            e.potential_gravity += p.mass * -self.gravity.acceleration.y * p.position.y;
        }
        for s in &self.springs {
            let p1 = &self.particles[s.p1];
            let p2 = &self.particles[s.p2];
            let diff = p2.position - p1.position;
            let dist = diff.length();
            let stretch = dist - s.rest_length;
            e.potential_spring += 0.5 * s.k * stretch * stretch;
        }
        e
    }

    fn apply_forces(&mut self, active_pushes: &[(usize, Vec3)]) {
        for p in &mut self.particles {
            if !p.pinned {
                p.force = p.mass * self.gravity.acceleration;
            } else {
                p.force = Vec3::ZERO;
            }
        }
        for &(index, force) in active_pushes {
            if index < self.particles.len() {
                self.particles[index].force += force;
            }
        }
        for s in &self.springs {
            let p1 = self.particles[s.p1].position;
            let p2 = self.particles[s.p2].position;
            let v1 = self.particles[s.p1].velocity;
            let v2 = self.particles[s.p2].velocity;

            let diff = p2 - p1;
            let dist = diff.length();
            if dist > 1e-9 {
                let dir = diff / dist;
                let stretch = dist - s.rest_length;
                let force_mag = s.k * stretch;

                let rel_v = v2 - v1;
                let damp_force_mag = s.damping * rel_v.dot(dir);

                let total_force = dir * (force_mag + damp_force_mag);
                self.particles[s.p1].force += total_force;
                self.particles[s.p2].force -= total_force;
            }
        }
    }

    fn satisfy_constraints(&mut self) {
        // Project positions 10 iterations
        for _ in 0..10 {
            // Ground
            if let Some(ref g) = self.ground {
                for p in &mut self.particles {
                    if !p.pinned && p.position.y < g.y {
                        p.position.y = g.y;
                        if p.velocity.y < 0.0 {
                            p.velocity.y *= -g.restitution;
                        }
                    }
                }
            }

            // Rods
            for r in &self.rods {
                let p1 = self.particles[r.p1].position;
                let p2 = self.particles[r.p2].position;
                let diff = p2 - p1;
                let dist = diff.length();
                if dist > 1e-9 {
                    let err = dist - r.length;
                    let correction = diff * (err / dist);
                    let m1 = if self.particles[r.p1].pinned {
                        0.0
                    } else {
                        self.particles[r.p1].mass
                    };
                    let m2 = if self.particles[r.p2].pinned {
                        0.0
                    } else {
                        self.particles[r.p2].mass
                    };
                    let w1 = if m1 > 0.0 { 1.0 / m1 } else { 0.0 };
                    let w2 = if m2 > 0.0 { 1.0 / m2 } else { 0.0 };
                    let w = w1 + w2;
                    if w > 0.0 {
                        if w1 > 0.0 {
                            self.particles[r.p1].position += correction * (w1 / w);
                        }
                        if w2 > 0.0 {
                            self.particles[r.p2].position -= correction * (w2 / w);
                        }

                        // Correct velocity
                        let dir = diff / dist;
                        let v1 = self.particles[r.p1].velocity;
                        let v2 = self.particles[r.p2].velocity;
                        let rel_v = v2 - v1;
                        let v_corr = dir * rel_v.dot(dir);
                        if w1 > 0.0 {
                            self.particles[r.p1].velocity += v_corr * (w1 / w);
                        }
                        if w2 > 0.0 {
                            self.particles[r.p2].velocity -= v_corr * (w2 / w);
                        }
                    }
                }
            }
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl Simulation for World {
    type State = Self;
    type Input = PhysicsInput;

    fn step(&mut self, dt: Seconds) {
        let dt = dt.get();
        let mut active_pushes = alloc::vec::Vec::new();
        for input in self.input_queue.drain(..) {
            match input {
                PhysicsInput::Push { index, force } => {
                    active_pushes.push((index, force));
                }
                PhysicsInput::SetPinned { index, pinned } => {
                    if index < self.particles.len() {
                        self.particles[index].pinned = pinned;
                    }
                }
                PhysicsInput::SetGravity { acceleration } => {
                    self.gravity.acceleration = acceleration;
                }
            }
        }

        self.apply_forces(&active_pushes);

        match self.integrator {
            Integrator::ExplicitEuler => {
                for p in &mut self.particles {
                    if !p.pinned {
                        p.position += p.velocity * dt;
                        p.velocity += (p.force / p.mass) * dt;
                    }
                }
            }
            Integrator::SymplecticEuler => {
                for p in &mut self.particles {
                    if !p.pinned {
                        p.velocity += (p.force / p.mass) * dt;
                        p.position += p.velocity * dt;
                    }
                }
            }
            Integrator::VelocityVerlet => {
                // v(t+dt/2) = v(t) + a(t)*dt/2
                // x(t+dt) = x(t) + v(t+dt/2)*dt
                for p in &mut self.particles {
                    if !p.pinned {
                        p.velocity += (p.force / p.mass) * (dt * 0.5);
                        p.position += p.velocity * dt;
                    }
                }
                self.apply_forces(&active_pushes);
                // v(t+dt) = v(t+dt/2) + a(t+dt)*dt/2
                for p in &mut self.particles {
                    if !p.pinned {
                        p.velocity += (p.force / p.mass) * (dt * 0.5);
                    }
                }
            }
        }

        self.satisfy_constraints();
    }

    fn apply(&mut self, input: Self::Input) {
        self.input_queue.push(input);
    }

    fn state(&self) -> &Self::State {
        self
    }
}

impl Snapshot for World {
    type Snapshot = Self;
    fn snapshot(&self) -> Self::Snapshot {
        self.clone()
    }
    fn restore(&mut self, snapshot: &Self::Snapshot) {
        *self = snapshot.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verifies that a new World is initialized with empty lists.
    #[test]
    fn test_world_new() {
        let world = World::new();
        assert_eq!(world.particles.len(), 0);
    }
}
