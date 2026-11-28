import sys

with open("crates/simbiguity-physics/src/collide.rs", "r") as f:
    code = f.read()

new_functions = """

pub fn resolve_particle_collision(
    p1: &mut Particle,
    p2: &mut Particle,
    max_impulse: &mut f64,
) {
    let diff = p2.position - p1.position;
    let dist = diff.length();
    let r = p1.radius + p2.radius;

    if dist < r && dist > 0.0 {
        let n = diff / dist;
        let pen = r - dist;

        let w1 = if p1.pinned { 0.0 } else { 1.0 / p1.mass };
        let w2 = if p2.pinned { 0.0 } else { 1.0 / p2.mass };
        let w_sum = w1 + w2;
        if w_sum > 0.0 {
            let move1 = -n * (pen * w1 / w_sum);
            let move2 = n * (pen * w2 / w_sum);

            p1.position += move1;
            p2.position += move2;

            let v_rel = p2.velocity - p1.velocity;
            let v_rel_n = v_rel.dot(n);

            if v_rel_n < 0.0 {
                let e = p1.restitution.min(p2.restitution);
                let j_mag = -(1.0 + e) * v_rel_n / w_sum;
                if j_mag > *max_impulse {
                    *max_impulse = j_mag;
                }

                let impulse = n * j_mag;
                p1.velocity -= impulse * w1;
                p2.velocity += impulse * w2;
            }
        }
    }
}

pub fn resolve_ground_collision(
    p: &mut Particle,
    g: &Ground,
    dt: f64,
    gravity_len: f64,
    max_impulse: &mut f64,
) {
    if p.pinned {
        return;
    }
    let bottom = p.position.y - p.radius;
    if bottom < g.y {
        let pen = g.y - bottom;
        p.position.y += pen;

        let v_rel_y = p.velocity.y;
        let mut j_normal = 0.0;
        if v_rel_y < 0.0 {
            let e = p.restitution.min(g.restitution);
            j_normal = -(1.0 + e) * v_rel_y * p.mass;
            p.velocity.y += j_normal / p.mass;
        }

        if g.friction > 0.0 {
            let mut v_tangent = p.velocity;
            v_tangent.y = 0.0;
            let v_tangent_len = v_tangent.length();
            if v_tangent_len > 0.0 {
                let max_friction_j = g.friction * j_normal;
                let max_kin_friction_j = g.friction * p.mass * gravity_len * dt;
                let mut applied_j = max_friction_j;
                if j_normal == 0.0 && p.velocity.y.abs() < 1e-6 {
                    applied_j = max_kin_friction_j;
                }

                let j_req = p.mass * v_tangent_len;
                let j_fric = applied_j.min(j_req);
                if j_fric > 0.0 {
                    let dir = -v_tangent / v_tangent_len;
                    p.velocity += dir * (j_fric / p.mass);
                    if j_fric > *max_impulse {
                        *max_impulse = j_fric;
                    }
                }
            }
        }
        if j_normal > *max_impulse {
            *max_impulse = j_normal;
        }
    }
}

pub fn resolve_collisions(
    particles: &mut [Particle],
    ground: &Option<Ground>,
    dt: f64,
    gravity_len: f64,
) {
    let mut passes = 0;
    loop {
        passes += 1;
        let mut max_impulse = 0.0_f64;

        let pairs = find_pairs_broadphase(particles);
        for (i, j) in pairs {
            let (p1, p2) = if i < j {
                let (left, right) = particles.split_at_mut(j);
                (&mut left[i], &mut right[0])
            } else {
                let (left, right) = particles.split_at_mut(i);
                (&mut right[0], &mut left[j])
            };
            resolve_particle_collision(p1, p2, &mut max_impulse);
        }

        if let Some(g) = ground {
            for p in particles.iter_mut() {
                resolve_ground_collision(p, g, dt, gravity_len, &mut max_impulse);
            }
        }

        if max_impulse <= 1e-12 || passes >= 8 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simbiguity_math::vec::Vec3;

    /// Verifies that broadphase returns correct pairs.
    #[test]
    fn test_find_pairs_broadphase() {
        let p1 = Particle { mass: 1.0, position: Vec3::ZERO, velocity: Vec3::ZERO, force: Vec3::ZERO, pinned: false, radius: 1.0, restitution: 1.0 };
        let p2 = Particle { mass: 1.0, position: Vec3::new(1.0, 0.0, 0.0), velocity: Vec3::ZERO, force: Vec3::ZERO, pinned: false, radius: 1.0, restitution: 1.0 };
        let pairs = find_pairs_broadphase(&[p1, p2]);
        assert_eq!(pairs, vec![(0, 1)]);
    }

    /// Verifies that two overlapping particles are pushed apart and velocities updated.
    #[test]
    fn test_resolve_particle_collision() {
        let mut p1 = Particle {
            mass: 1.0, position: Vec3::new(0.0, 0.0, 0.0), velocity: Vec3::new(1.0, 0.0, 0.0), force: Vec3::ZERO, pinned: false, radius: 1.0, restitution: 1.0
        };
        let mut p2 = Particle {
            mass: 1.0, position: Vec3::new(1.0, 0.0, 0.0), velocity: Vec3::new(-1.0, 0.0, 0.0), force: Vec3::ZERO, pinned: false, radius: 1.0, restitution: 1.0
        };
        let mut max_impulse = 0.0;
        resolve_particle_collision(&mut p1, &mut p2, &mut max_impulse);
        assert_eq!(p1.position.x, -0.5);
        assert_eq!(p2.position.x, 1.5);
        assert_eq!(p1.velocity.x, -1.0);
        assert_eq!(p2.velocity.x, 1.0);
        assert_eq!(max_impulse, 2.0);
    }

    /// Verifies that a particle colliding with the ground is corrected and bounces.
    #[test]
    fn test_resolve_ground_collision() {
        let mut p = Particle {
            mass: 1.0, position: Vec3::new(0.0, 0.5, 0.0), velocity: Vec3::new(1.0, -10.0, 0.0), force: Vec3::ZERO, pinned: false, radius: 1.0, restitution: 0.5
        };
        let g = Ground { y: 0.0, restitution: 1.0, friction: 0.0 };
        let mut max_impulse = 0.0;
        resolve_ground_collision(&mut p, &g, 0.016, 9.8, &mut max_impulse);
        assert_eq!(p.position.y, 1.0);
        assert_eq!(p.velocity.y, 5.0);
        assert_eq!(max_impulse, 15.0);
    }
}
"""

start_idx = code.find("pub fn resolve_collisions(")
code = code[:start_idx] + new_functions

with open("crates/simbiguity-physics/src/collide.rs", "w") as f:
    f.write(code)

print("Rewrote collide.rs")
