use crate::constraints::Ground;
use crate::world::Particle;
use alloc::vec;
use alloc::vec::Vec;
use simbiguity_math::float::floor;

pub fn find_pairs_brute(particles: &[Particle]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for i in 0..particles.len() {
        if particles[i].radius <= 0.0 {
            continue;
        }
        for j in i + 1..particles.len() {
            if particles[j].radius <= 0.0 {
                continue;
            }
            let dist_sq = (particles[i].position - particles[j].position).length_squared();
            let r = particles[i].radius + particles[j].radius;
            if dist_sq < r * r {
                pairs.push((i, j));
            }
        }
    }
    pairs
}

pub fn build_spatial_grid(particles: &[Particle], cell_size: f64) -> Vec<((i32, i32, i32), usize)> {
    let mut entries = Vec::new();

    for (i, p) in particles.iter().enumerate() {
        if p.radius <= 0.0 {
            continue;
        }
        let cx = floor(p.position.x / cell_size) as i32;
        let cy = floor(p.position.y / cell_size) as i32;
        let cz = floor(p.position.z / cell_size) as i32;
        entries.push(((cx, cy, cz), i));
    }

    entries.sort_unstable_by_key(|e| e.0);
    entries
}

pub fn find_cell_overlaps(
    entries: &[((i32, i32, i32), usize)],
    particles: &[Particle],
) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    let mut checked = Vec::new();

    for i in 0..entries.len() {
        let (cell1, p1_idx) = entries[i];
        for &(cell2, p2_idx) in &entries[i + 1..] {
            let dx = cell1.0 - cell2.0;
            if dx < -1 {
                break;
            }
            if dx.abs() <= 1 {
                let dy = cell1.1 - cell2.1;
                let dz = cell1.2 - cell2.2;
                if dy.abs() <= 1 && dz.abs() <= 1 {
                    let mut a = p1_idx;
                    let mut b = p2_idx;
                    if a > b {
                        core::mem::swap(&mut a, &mut b);
                    }
                    checked.push((a, b));
                }
            }
        }
    }

    checked.sort_unstable();
    checked.dedup();

    for (i, j) in checked {
        let dist_sq = (particles[i].position - particles[j].position).length_squared();
        let r = particles[i].radius + particles[j].radius;
        if dist_sq < r * r {
            pairs.push((i, j));
        }
    }

    pairs.sort_unstable();
    pairs
}

pub fn find_pairs_broadphase(particles: &[Particle]) -> Vec<(usize, usize)> {
    let mut max_radius = 0.0_f64;
    for p in particles {
        if p.radius > max_radius {
            max_radius = p.radius;
        }
    }
    if max_radius <= 0.0 {
        return vec![];
    }

    let cell_size = 2.0 * max_radius;
    let entries = build_spatial_grid(particles, cell_size);
    find_cell_overlaps(&entries, particles)
}

pub fn resolve_particle_collision(p1: &mut Particle, p2: &mut Particle, max_impulse: &mut f64) {
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

pub fn apply_normal_impulse(p: &mut Particle, g: &Ground) -> f64 {
    let v_rel_y = p.velocity.y;
    if v_rel_y < 0.0 {
        let e = p.restitution.min(g.restitution);
        let j_normal = -(1.0 + e) * v_rel_y * p.mass;
        p.velocity.y += j_normal / p.mass;
        j_normal
    } else {
        0.0
    }
}

pub fn apply_friction_impulse(
    p: &mut Particle,
    g: &Ground,
    dt: f64,
    gravity_len: f64,
    j_normal: f64,
) -> f64 {
    if g.friction <= 0.0 {
        return 0.0;
    }
    let mut v_tangent = p.velocity;
    v_tangent.y = 0.0;
    let v_tangent_len = v_tangent.length();
    if v_tangent_len == 0.0 {
        return 0.0;
    }

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
        let delta_v = dir * (j_fric / p.mass);
        p.velocity += delta_v;
        p.position.x += delta_v.x * (dt * 0.5);
        p.position.z += delta_v.z * (dt * 0.5);
    }
    j_fric
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

        let j_normal = apply_normal_impulse(p, g);
        let j_fric = apply_friction_impulse(p, g, dt, gravity_len, j_normal);

        if j_normal > *max_impulse {
            *max_impulse = j_normal;
        }
        if j_fric > *max_impulse {
            *max_impulse = j_fric;
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
        let p1 = Particle {
            mass: 1.0,
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            force: Vec3::ZERO,
            pinned: false,
            radius: 1.0,
            restitution: 1.0,
        };
        let p2 = Particle {
            mass: 1.0,
            position: Vec3::new(1.0, 0.0, 0.0),
            velocity: Vec3::ZERO,
            force: Vec3::ZERO,
            pinned: false,
            radius: 1.0,
            restitution: 1.0,
        };
        let pairs = find_pairs_broadphase(&[p1, p2]);
        assert_eq!(pairs, vec![(0, 1)]);
    }

    /// Verifies that two overlapping particles are pushed apart and velocities updated.
    #[test]
    fn test_resolve_particle_collision() {
        let mut p1 = Particle {
            mass: 1.0,
            position: Vec3::new(0.0, 0.0, 0.0),
            velocity: Vec3::new(1.0, 0.0, 0.0),
            force: Vec3::ZERO,
            pinned: false,
            radius: 1.0,
            restitution: 1.0,
        };
        let mut p2 = Particle {
            mass: 1.0,
            position: Vec3::new(1.0, 0.0, 0.0),
            velocity: Vec3::new(-1.0, 0.0, 0.0),
            force: Vec3::ZERO,
            pinned: false,
            radius: 1.0,
            restitution: 1.0,
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
            mass: 1.0,
            position: Vec3::new(0.0, 0.5, 0.0),
            velocity: Vec3::new(1.0, -10.0, 0.0),
            force: Vec3::ZERO,
            pinned: false,
            radius: 1.0,
            restitution: 0.5,
        };
        let g = Ground {
            y: 0.0,
            restitution: 1.0,
            friction: 0.0,
        };
        let mut max_impulse = 0.0;
        resolve_ground_collision(&mut p, &g, 0.016, 9.8, &mut max_impulse);
        assert_eq!(p.position.y, 1.0);
        assert_eq!(p.velocity.y, 5.0);
        assert_eq!(max_impulse, 15.0);
    }

    /// Verifies that apply_normal_impulse applies bounce properly
    #[test]
    fn test_apply_normal_impulse() {
        let mut p = Particle {
            mass: 2.0,
            position: Vec3::ZERO,
            velocity: Vec3::new(0.0, -10.0, 0.0),
            force: Vec3::ZERO,
            pinned: false,
            radius: 1.0,
            restitution: 0.5,
        };
        let g = Ground {
            y: 0.0,
            restitution: 1.0,
            friction: 0.0,
        };
        let j = apply_normal_impulse(&mut p, &g);
        assert_eq!(p.velocity.y, 5.0);
        assert_eq!(j, 30.0);
    }

    /// Verifies that apply_friction_impulse applies sliding friction correctly
    #[test]
    fn test_apply_friction_impulse() {
        let mut p = Particle {
            mass: 2.0,
            position: Vec3::ZERO,
            velocity: Vec3::new(10.0, 0.0, 0.0),
            force: Vec3::ZERO,
            pinned: false,
            radius: 1.0,
            restitution: 0.5,
        };
        let g = Ground {
            y: 0.0,
            restitution: 1.0,
            friction: 0.5,
        };
        let j = apply_friction_impulse(&mut p, &g, 0.1, 9.8, 0.0);
        // max kin = 0.5 * 2 * 9.8 * 0.1 = 0.98
        // j_req = 2 * 10 = 20
        assert!((j - 0.98).abs() < 1e-9);
        assert!(p.velocity.x < 10.0);
    }

    /// Verifies build_spatial_grid correctly outputs entries
    #[test]
    fn test_build_spatial_grid() {
        let p = Particle {
            mass: 1.0,
            position: Vec3::new(1.0, 2.0, 3.0),
            velocity: Vec3::ZERO,
            force: Vec3::ZERO,
            pinned: false,
            radius: 1.0,
            restitution: 1.0,
        };
        let entries = build_spatial_grid(&[p], 2.0);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, (0, 1, 1));
    }
}
