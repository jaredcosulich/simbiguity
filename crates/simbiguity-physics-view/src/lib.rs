#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
#![no_std]
extern crate alloc;

use simbiguity_math::float::acos;
use simbiguity_math::mat::Mat4;
use simbiguity_math::quat::Quat;
use simbiguity_math::vec::Vec3;
use simbiguity_physics::World;
use simbiguity_render::{Material, Mesh, MeshId, OrbitCamera, Scene};

pub struct PhysicsView {
    pub scene: Scene,
    pub camera: OrbitCamera,
    particle_mesh: MeshId,
    spring_mesh: MeshId,
    rod_mesh: MeshId,
    ground_mesh: MeshId,
    pub cloth_grid: Option<u32>,
    pub cloth_mesh: Option<MeshId>,
    pub orbit_trail: bool,
    pub trails: alloc::vec::Vec<alloc::vec::Vec<Vec3>>,
    pub norms_buf: alloc::vec::Vec<Vec3>,
}

impl PhysicsView {
    pub fn new() -> Self {
        let mut scene = Scene::new();
        let particle_mesh = scene.add_mesh(Mesh::sphere(1.0, 16, 24));
        let spring_mesh = scene.add_mesh(Mesh::cylinder(0.04, 1.0, 12));
        let rod_mesh = scene.add_mesh(Mesh::cylinder(0.06, 1.0, 12));
        let ground_mesh = scene.add_mesh(Mesh::cylinder(10.0, 0.1, 32)); // large flat cylinder for ground
        let camera = OrbitCamera::default();

        Self {
            scene,
            camera,
            particle_mesh,
            spring_mesh,
            rod_mesh,
            ground_mesh,
            cloth_grid: None,
            cloth_mesh: None,
            orbit_trail: false,
            trails: alloc::vec::Vec::new(),
            norms_buf: alloc::vec::Vec::new(),
        }
    }

    pub fn update(&mut self, world: &World) {
        self.scene.nodes.clear();

        let p_mats = [
            Material::lit([0.8, 0.2, 0.2]),
            Material::lit([0.2, 0.8, 0.2]),
            Material::lit([0.2, 0.2, 0.8]),
            Material::lit([0.8, 0.8, 0.2]),
            Material::lit([0.8, 0.2, 0.8]),
            Material::lit([0.2, 0.8, 0.8]),
        ];
        let s_mat = Material::lit([0.4, 0.4, 0.8]);
        let r_mat = Material::lit([0.6, 0.6, 0.6]);
        let g_mat = Material::lit([0.2, 0.8, 0.2]);

        // Add ground
        if let Some(ref g) = world.ground {
            self.scene.add_node(
                self.ground_mesh,
                Mat4::translation(Vec3::new(0.0, g.y - 0.05, 0.0)),
                g_mat,
            );
        }

        if let Some(n) = self.cloth_grid {
            let m_id = if let Some(id) = self.cloth_mesh {
                id
            } else {
                let id = self.scene.add_mesh(Mesh::default());
                self.cloth_mesh = Some(id);
                id
            };
            let m = &mut self.scene.meshes[m_id.0 as usize];
            m.vertices.clear();
            m.indices.clear();
            self.norms_buf.clear();
            self.norms_buf.resize(world.particles.len(), Vec3::ZERO);
            for z in 0..n - 1 {
                for x in 0..n - 1 {
                    let a = z * n + x;
                    let b = a + 1;
                    let c = a + n;
                    let d = c + 1;
                    let p0 = world.particles[a as usize].position;
                    let p1 = world.particles[b as usize].position;
                    let p2 = world.particles[c as usize].position;
                    let p3 = world.particles[d as usize].position;
                    let n1 = (p1 - p0).cross(p2 - p0);
                    let n2 = (p3 - p1).cross(p2 - p1);
                    self.norms_buf[a as usize] += n1;
                    self.norms_buf[b as usize] += n1 + n2;
                    self.norms_buf[c as usize] += n1 + n2;
                    self.norms_buf[d as usize] += n2;
                }
            }
            for (i, p) in world.particles.iter().enumerate() {
                let norm = self.norms_buf[i];
                let norm = norm.normalize().unwrap_or(Vec3::Y);
                m.vertices.extend_from_slice(&[
                    p.position.x as f32,
                    p.position.y as f32,
                    p.position.z as f32,
                    norm.x as f32,
                    norm.y as f32,
                    norm.z as f32,
                ]);
            }
            for z in 0..n - 1 {
                for x in 0..n - 1 {
                    let a = z * n + x;
                    let b = a + 1;
                    let c = a + n;
                    let d = c + 1;
                    m.indices.extend_from_slice(&[a, b, c, b, d, c]);
                }
            }
            self.scene
                .add_node(m_id, Mat4::IDENTITY, Material::lit([0.3, 0.8, 0.8]));
        } else {
            // Add particles
            for (i, p) in world.particles.iter().enumerate() {
                let mat = if p.pinned {
                    Material::lit([0.2, 0.2, 0.2])
                } else {
                    p_mats[i % p_mats.len()]
                };

                let r = if p.radius > 0.0 { p.radius } else { 0.1 };
                let t = Mat4::translation(p.position) * Mat4::scale(Vec3::new(r, r, r));
                let n = self.scene.add_node(self.particle_mesh, t, mat);
                self.scene.nodes[n.0].pick = Some(i as u32);
            }

            // Add springs
            for s in &world.springs {
                let p1 = world.particles[s.p1].position;
                let p2 = world.particles[s.p2].position;
                self.add_line(p1, p2, self.spring_mesh, s_mat);
            }

            // Add rods
            for r in &world.rods {
                let p1 = world.particles[r.p1].position;
                let p2 = world.particles[r.p2].position;
                self.add_line(p1, p2, self.rod_mesh, r_mat);
            }
        }

        if self.orbit_trail {
            if self.trails.len() < world.particles.len() {
                self.trails
                    .resize(world.particles.len(), alloc::vec::Vec::new());
            }
            for (i, p) in world.particles.iter().enumerate() {
                self.trails[i].push(p.position);
                if self.trails[i].len() > 500 {
                    self.trails[i].remove(0);
                }
                for j in 0..self.trails[i].len().saturating_sub(1) {
                    let w0 = self.trails[i][j];
                    let w1 = self.trails[i][j + 1];
                    self.add_line(w0, w1, self.rod_mesh, p_mats[i % p_mats.len()]);
                }
            }
        }
    }

    fn add_line(&mut self, p1: Vec3, p2: Vec3, mesh: MeshId, mat: Material) {
        let diff = p2 - p1;
        let length = diff.length();
        if length < 1e-6 {
            return;
        }
        let center = (p1 + p2) * 0.5;
        let dir = diff / length;
        let up = Vec3::new(0.0, 1.0, 0.0);
        let dot = up.dot(dir);
        let q = if (dot - 1.0).abs() < 1e-6 {
            Quat::IDENTITY
        } else if (dot + 1.0).abs() < 1e-6 {
            Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), core::f64::consts::PI)
        } else {
            let axis = up.cross(dir).normalize();
            let angle = acos(dot);
            if let Some(a) = axis {
                Quat::from_axis_angle(a, angle)
            } else {
                Quat::IDENTITY
            }
        };

        let t = Mat4::translation(center)
            * Mat4::rotation(q)
            * Mat4::scale(Vec3::new(1.0, length, 1.0));
        self.scene.add_node(mesh, t, mat);
    }
}

impl Default for PhysicsView {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Verifies view creation.
    #[test]
    fn test_view() {
        let view = PhysicsView::new();
        assert_eq!(view.scene.nodes.len(), 0);
    }
}
