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
}

impl PhysicsView {
    pub fn new() -> Self {
        let mut scene = Scene::new();
        let particle_mesh = scene.add_mesh(Mesh::sphere(0.1, 16, 24));
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
        }
    }

    pub fn update(&mut self, world: &World) {
        self.scene.nodes.clear();

        let p_mat = Material::lit([0.8, 0.2, 0.2]);
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

        // Add particles
        for (i, p) in world.particles.iter().enumerate() {
            let n = self.scene.add_node(
                self.particle_mesh,
                Mat4::translation(p.position),
                if p.pinned {
                    Material::lit([0.2, 0.2, 0.2])
                } else {
                    p_mat
                },
            );
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
