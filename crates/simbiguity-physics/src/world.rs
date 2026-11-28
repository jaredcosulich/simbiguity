use crate::constraints::{Ground, Rod};
use crate::energy::Energy;
use crate::error::PhysicsError;
use crate::forces::{Attractor, Gravity, Spring};
use crate::integrate::Integrator;
use simbiguity_math::vec::Vec3;

#[derive(Debug, Clone)]
pub struct Particle {
    pub mass: f64,
    pub position: Vec3,
    pub velocity: Vec3,
    pub force: Vec3,
    pub pinned: bool,
    pub radius: f64,
    pub restitution: f64,
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
    pub attractors: alloc::vec::Vec<Attractor>,
    pub mutual_gravity: Option<f64>,
    pub time: f64,
    pub drags: alloc::vec::Vec<(usize, f64)>,
    pub drives: alloc::vec::Vec<(usize, crate::forces::Drive)>,
}

impl World {
    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn add_drive(
        &mut self,
        particle: usize,
        drive: crate::forces::Drive,
    ) -> Result<(), PhysicsError> {
        if particle >= self.particles.len() {
            return Err(PhysicsError::IndexOutOfBounds);
        }
        self.drives.push((particle, drive));
        Ok(())
    }

    pub fn add_drag(&mut self, particle: usize, c: f64) -> Result<(), PhysicsError> {
        if particle >= self.particles.len() {
            return Err(PhysicsError::IndexOutOfBounds);
        }
        if c.is_nan() || c < 0.0 {
            return Err(PhysicsError::InvalidParameter);
        }
        self.drags.push((particle, c));
        Ok(())
    }

    pub fn apply_drags(&mut self) {
        for &(p, c) in &self.drags {
            if !self.particles[p].pinned {
                let v = self.particles[p].velocity;
                self.particles[p].force -= v * c;
            }
        }
    }

    pub fn apply_drives(&mut self, time: f64) {
        for (p, drive) in &self.drives {
            if !self.particles[*p].pinned {
                let mag = match drive.kind {
                    crate::forces::DriveKind::Sine => {
                        drive.amplitude
                            * simbiguity_math::float::sin(drive.omega * time + drive.phase)
                    }
                    crate::forces::DriveKind::Step => {
                        if time >= 0.0 {
                            drive.amplitude
                        } else {
                            0.0
                        }
                    }
                };
                self.particles[*p].force += drive.axis * mag;
            }
        }
    }

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
            attractors: alloc::vec::Vec::new(),
            mutual_gravity: None,
            time: 0.0,
            drags: alloc::vec::Vec::new(),
            drives: alloc::vec::Vec::new(),
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
            radius: 0.0,
            restitution: 0.0,
        });
        Ok(index)
    }

    pub fn add_ball(
        &mut self,
        mass: f64,
        position: Vec3,
        radius: f64,
        restitution: f64,
    ) -> Result<usize, PhysicsError> {
        let mass = if mass.is_nan() || mass <= 0.0 {
            return Err(PhysicsError::NonPositiveMass);
        } else {
            mass
        };
        let index = self.particles.len();
        self.particles.push(Particle {
            mass,
            position,
            velocity: Vec3::ZERO,
            force: Vec3::ZERO,
            pinned: false,
            radius,
            restitution,
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

    pub fn add_attractor(&mut self, position: Vec3, gm: f64, softening: f64) {
        self.attractors.push(Attractor {
            position,
            gm,
            softening,
        });
    }

    fn compute_kinetic_energy(&self) -> f64 {
        let mut k = 0.0;
        for p in &self.particles {
            if !p.pinned {
                k += 0.5 * p.mass * p.velocity.length_squared();
            }
        }
        k
    }

    fn compute_uniform_gravity_potential(&self) -> f64 {
        let mut u = 0.0;
        for p in &self.particles {
            u += p.mass * -self.gravity.acceleration.y * p.position.y;
        }
        u
    }

    fn compute_attractors_potential(&self) -> f64 {
        let mut u = 0.0;
        for p in &self.particles {
            for a in &self.attractors {
                let diff = a.position - p.position;
                let r2 = diff.length_squared();
                let r = simbiguity_math::float::sqrt(r2);
                if r > 1e-12 {
                    u -= a.gm * p.mass / r;
                }
            }
        }
        u
    }

    fn compute_mutual_gravity_potential(&self) -> f64 {
        let mut u = 0.0;
        if let Some(g) = self.mutual_gravity {
            for i in 0..self.particles.len() {
                for j in (i + 1)..self.particles.len() {
                    let diff = self.particles[i].position - self.particles[j].position;
                    let r = diff.length();
                    if r > 1e-12 {
                        u -= g * self.particles[i].mass * self.particles[j].mass / r;
                    }
                }
            }
        }
        u
    }

    fn compute_spring_potential(&self) -> f64 {
        let mut u = 0.0;
        for s in &self.springs {
            let p1 = &self.particles[s.p1];
            let p2 = &self.particles[s.p2];
            let diff = p2.position - p1.position;
            let dist = diff.length();
            let stretch = dist - s.rest_length;
            u += 0.5 * s.k * stretch * stretch;
        }
        u
    }

    pub fn compute_energy(&self) -> Energy {
        Energy {
            kinetic: self.compute_kinetic_energy(),
            potential_gravity: self.compute_uniform_gravity_potential()
                + self.compute_attractors_potential()
                + self.compute_mutual_gravity_potential(),
            potential_spring: self.compute_spring_potential(),
        }
    }

    pub fn setup_orbit(&mut self) -> Result<(), PhysicsError> {
        self.gravity.acceleration = Vec3::ZERO;
        self.add_attractor(Vec3::ZERO, 1.0, 0.0);
        let p = self.add_particle(1.0, Vec3::new(1.0, 0.0, 0.0), false)?;
        self.particles[p].velocity = Vec3::new(0.0, 1.0, 0.0);
        Ok(())
    }

    pub fn setup_binary(&mut self) -> Result<(), PhysicsError> {
        self.gravity.acceleration = Vec3::ZERO;
        self.mutual_gravity = Some(1.0);
        let p1 = self.add_particle(1.0, Vec3::new(1.0, 0.0, 0.0), false)?;
        self.particles[p1].velocity = Vec3::new(0.0, 0.5, 0.0);
        let p2 = self.add_particle(1.0, Vec3::new(-1.0, 0.0, 0.0), false)?;
        self.particles[p2].velocity = Vec3::new(0.0, -0.5, 0.0);
        Ok(())
    }

    pub fn setup_figure_eight(&mut self) -> Result<(), PhysicsError> {
        self.gravity.acceleration = Vec3::ZERO;
        self.mutual_gravity = Some(1.0);
        let p1 = Vec3::new(0.97000436, -0.24308753, 0.0);
        let p2 = Vec3::new(-0.97000436, 0.24308753, 0.0);
        let p3 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(0.466203685, 0.43236573, 0.0);
        let v2 = Vec3::new(0.466203685, 0.43236573, 0.0);
        let v3 = Vec3::new(-0.93240737, -0.86473146, 0.0);

        let idx1 = self.add_particle(1.0, p1, false)?;
        self.particles[idx1].velocity = v1;
        let idx2 = self.add_particle(1.0, p2, false)?;
        self.particles[idx2].velocity = v2;
        let idx3 = self.add_particle(1.0, p3, false)?;
        self.particles[idx3].velocity = v3;
        Ok(())
    }

    pub fn setup_deterministic_ball_pit(&mut self, count: u32) {
        self.gravity.acceleration = Vec3::new(0.0, -9.81, 0.0);
        self.ground = Some(Ground {
            y: 0.0,
            restitution: 0.2,
            friction: 0.5,
        });

        for x in -10..=10 {
            if let Ok(p) = self.add_ball(1.0, Vec3::new(x as f64 * 0.2, 0.1, 10.2), 0.1, 0.2) {
                self.particles[p].pinned = true;
            }
            if let Ok(p) = self.add_ball(1.0, Vec3::new(x as f64 * 0.2, 0.1, -10.2), 0.1, 0.2) {
                self.particles[p].pinned = true;
            }
        }
        for z in -10..=10 {
            if let Ok(p) = self.add_ball(1.0, Vec3::new(10.2, 0.1, z as f64 * 0.2), 0.1, 0.2) {
                self.particles[p].pinned = true;
            }
            if let Ok(p) = self.add_ball(1.0, Vec3::new(-10.2, 0.1, z as f64 * 0.2), 0.1, 0.2) {
                self.particles[p].pinned = true;
            }
        }

        let mut seed = 42u32;
        for _ in 0..count {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let x = ((seed as f64 / u32::MAX as f64) - 0.5) * 18.0;
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let y = (seed as f64 / u32::MAX as f64) * 10.0 + 1.0;
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let z = ((seed as f64 / u32::MAX as f64) - 0.5) * 18.0;
            let _ = self.add_ball(1.0, Vec3::new(x, y, z), 0.1, 0.2);
        }
    }

    pub fn apply_gravity(&mut self) {
        for p in &mut self.particles {
            if !p.pinned {
                p.force = p.mass * self.gravity.acceleration;
            } else {
                p.force = Vec3::ZERO;
            }
        }
    }

    pub fn apply_external_pushes(&mut self, active_pushes: &[(usize, Vec3)]) {
        for &(index, force) in active_pushes {
            if index < self.particles.len() {
                self.particles[index].force += force;
            }
        }
    }

    pub fn apply_spring_forces(&mut self) {
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

    pub fn apply_attractors(&mut self) {
        for i in 0..self.particles.len() {
            if self.particles[i].pinned {
                continue;
            }
            let pos = self.particles[i].position;
            let m = self.particles[i].mass;
            let mut force = Vec3::ZERO;

            for a in &self.attractors {
                let diff = a.position - pos;
                let r2 = diff.length_squared();
                let soft2 = a.softening * a.softening;
                let base = r2 + soft2;
                let denom = base * simbiguity_math::float::sqrt(base);
                if denom > 1e-12 {
                    force += diff * (a.gm * m / denom);
                }
            }

            self.particles[i].force += force;
        }
    }

    pub fn apply_mutual_gravity(&mut self) {
        if let Some(g) = self.mutual_gravity {
            for i in 0..self.particles.len() {
                for j in (i + 1)..self.particles.len() {
                    let p1 = &self.particles[i];
                    let p2 = &self.particles[j];
                    let diff = p2.position - p1.position;
                    let r2 = diff.length_squared();
                    let denom = r2 * simbiguity_math::float::sqrt(r2);
                    if denom > 1e-12 {
                        let force = diff * (g * p1.mass * p2.mass / denom);
                        if !self.particles[i].pinned {
                            self.particles[i].force += force;
                        }
                        if !self.particles[j].pinned {
                            self.particles[j].force -= force;
                        }
                    }
                }
            }
        }
    }

    pub fn apply_forces(&mut self, active_pushes: &[(usize, Vec3)], time: f64) {
        self.apply_gravity();
        self.apply_attractors();
        self.apply_mutual_gravity();
        self.apply_external_pushes(active_pushes);
        self.apply_spring_forces();
        self.apply_drags();
        self.apply_drives(time);
    }

    fn satisfy_position_constraints(&mut self, dt: f64, old_diffs: &[Vec3]) {
        for _ in 0..10 {
            for (i, r) in self.rods.iter().enumerate() {
                let n0 = if old_diffs[i].length_squared() > 1e-9 {
                    old_diffs[i].normalize().unwrap_or(Vec3::new(1.0, 0.0, 0.0))
                } else {
                    Vec3::new(1.0, 0.0, 0.0)
                };
                let diff = self.particles[r.p2].position - self.particles[r.p1].position;
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
                    let dot = diff.dot(n0);
                    if dot.abs() > 1e-6 {
                        let lambda =
                            (r.length * r.length - diff.length_squared()) / (2.0 * w * dot);
                        let delta1 = n0 * (-lambda * w1);
                        let delta2 = n0 * (lambda * w2);
                        self.particles[r.p1].position += delta1;
                        self.particles[r.p1].velocity += delta1 / dt;
                        self.particles[r.p2].position += delta2;
                        self.particles[r.p2].velocity += delta2 / dt;
                    }
                }
            }
        }
    }

    fn satisfy_velocity_constraints(&mut self) {
        for _ in 0..10 {
            for r in &self.rods {
                let diff = self.particles[r.p2].position - self.particles[r.p1].position;
                let dist = diff.length();
                if dist > 1e-9 {
                    let dir = diff / dist;
                    let v1 = self.particles[r.p1].velocity;
                    let v2 = self.particles[r.p2].velocity;
                    let rel_v = v2 - v1;
                    let v_corr = dir * rel_v.dot(dir);
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

    /// Validates `dt` and executes one integration step.
    pub fn step(&mut self, dt: f64) -> Result<(), PhysicsError> {
        if dt.is_nan() || dt <= 0.0 {
            return Err(PhysicsError::InvalidTimeStep);
        }
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

        let t = self.time;
        self.apply_forces(&active_pushes, t);

        let mut old_diffs = alloc::vec::Vec::with_capacity(self.rods.len());
        for r in &self.rods {
            old_diffs.push(self.particles[r.p2].position - self.particles[r.p1].position);
        }

        match self.integrator {
            Integrator::ExplicitEuler => {
                crate::integrate::step_explicit_euler(&mut self.particles, dt);
                self.satisfy_position_constraints(dt, &old_diffs);
                self.satisfy_velocity_constraints();
            }
            Integrator::SymplecticEuler => {
                crate::integrate::step_symplectic_euler(&mut self.particles, dt);
                self.satisfy_position_constraints(dt, &old_diffs);
                self.satisfy_velocity_constraints();
            }
            Integrator::VelocityVerlet => {
                crate::integrate::step_velocity_verlet_part1(&mut self.particles, dt);
                self.satisfy_position_constraints(dt, &old_diffs);
                self.apply_forces(&active_pushes, t + dt);
                crate::integrate::step_velocity_verlet_part2(&mut self.particles, dt);
                self.satisfy_velocity_constraints();
            }
        }

        crate::collide::resolve_collisions(
            &mut self.particles,
            &self.ground,
            dt,
            self.gravity.acceleration.length(),
        );

        self.time += dt;

        Ok(())
    }

    pub fn apply(&mut self, input: PhysicsInput) {
        self.input_queue.push(input);
    }

    pub fn snapshot(&self) -> Self {
        self.clone()
    }

    pub fn restore(&mut self, snapshot: &Self) {
        *self = snapshot.clone();
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies compute_energy returns correct total energy and partitions.
    #[test]
    fn test_compute_energy_partitions() {
        let mut world = World::new();
        world.gravity.acceleration = Vec3::new(0.0, -10.0, 0.0);
        let p = world
            .add_particle(1.0, Vec3::new(0.0, 10.0, 0.0), false)
            .unwrap();
        world.particles[p].velocity = Vec3::new(2.0, 0.0, 0.0);

        let p2 = world
            .add_particle(1.0, Vec3::new(0.0, 5.0, 0.0), false)
            .unwrap();
        world.add_spring(p, p2, 5.0, 10.0, 0.0).unwrap(); // Rest length is 5, distance is 5 => no stretch

        world.add_attractor(Vec3::ZERO, 1.0, 0.0);

        let e = world.compute_energy();
        assert_eq!(e.kinetic, 2.0);
        assert_eq!(e.potential_spring, 0.0);
        assert!((e.potential_gravity - (150.0 - 0.3)).abs() < 1e-6);
    }

    /// Verifies mutual gravity is calculated and applied correctly.
    #[test]
    fn test_mutual_gravity() {
        let mut world = World::new();
        world.gravity.acceleration = Vec3::ZERO;
        world.mutual_gravity = Some(1.0);
        world
            .add_particle(1.0, Vec3::new(1.0, 0.0, 0.0), false)
            .unwrap();
        world
            .add_particle(1.0, Vec3::new(-1.0, 0.0, 0.0), false)
            .unwrap();

        world.apply_mutual_gravity();
        assert!(world.particles[0].force.x < 0.0); // Pulled towards origin
        assert!(world.particles[1].force.x > 0.0);

        let e = world.compute_energy();
        assert_eq!(e.potential_gravity, -0.5); // -G*m1*m2/r = -1.0*1.0*1.0/2.0
    }

    /// Verifies attractors force
    #[test]
    fn test_apply_attractors() {
        let mut world = World::new();
        world.add_attractor(Vec3::ZERO, 1.0, 0.0);
        world
            .add_particle(1.0, Vec3::new(2.0, 0.0, 0.0), false)
            .unwrap();
        world.apply_attractors();
        // Force = -GmM/r^2 = -1.0 * 1.0 * 1.0 / 4.0 = -0.25
        assert_eq!(world.particles[0].force, Vec3::new(-0.25, 0.0, 0.0));
    }

    /// Verifies orbit setup
    #[test]
    fn test_setup_orbit() {
        let mut world = World::new();
        world.setup_orbit().unwrap();
        assert_eq!(world.particles.len(), 1);
        assert_eq!(world.attractors.len(), 1);
    }

    #[test]
    fn test_world_new() {
        let world = World::new();
        assert_eq!(world.particles.len(), 0);
    }

    /// Verifies that gravity is correctly applied to unpinned particles.
    #[test]
    fn test_apply_gravity() {
        let mut world = World::new();
        world.gravity.acceleration = Vec3::new(0.0, -10.0, 0.0);
        world.add_particle(2.0, Vec3::ZERO, false).unwrap();
        world.add_particle(1.0, Vec3::ZERO, true).unwrap();
        world.apply_gravity();
        assert_eq!(world.particles[0].force, Vec3::new(0.0, -20.0, 0.0));
        assert_eq!(world.particles[1].force, Vec3::ZERO);
    }

    /// Verifies that external forces map correctly by particle index.
    #[test]
    fn test_apply_external_pushes() {
        let mut world = World::new();
        world.add_particle(1.0, Vec3::ZERO, false).unwrap();
        world.add_particle(1.0, Vec3::ZERO, false).unwrap();
        world
            .apply_external_pushes(&[(0, Vec3::new(1.0, 2.0, 3.0)), (0, Vec3::new(1.0, 0.0, 0.0))]);
        assert_eq!(world.particles[0].force, Vec3::new(2.0, 2.0, 3.0));
        assert_eq!(world.particles[1].force, Vec3::ZERO);
    }

    /// Verifies Hooke's law forces between connected particles.
    #[test]
    fn test_apply_spring_forces() {
        let mut world = World::new();
        world
            .add_particle(1.0, Vec3::new(0.0, 0.0, 0.0), false)
            .unwrap();
        world
            .add_particle(1.0, Vec3::new(2.0, 0.0, 0.0), false)
            .unwrap();
        world.add_spring(0, 1, 1.0, 10.0, 0.0).unwrap();
        world.apply_spring_forces();
        assert_eq!(world.particles[0].force, Vec3::new(10.0, 0.0, 0.0));
        assert_eq!(world.particles[1].force, Vec3::new(-10.0, 0.0, 0.0));
    }

    /// Verifies deterministic setup produces the exact correct particle count.
    #[test]
    fn test_setup_deterministic_ball_pit() {
        let mut world = World::new();
        world.setup_deterministic_ball_pit(100);
        assert_eq!(world.particles.len(), 184); // 84 boundary balls + 100 random balls
    }
}
