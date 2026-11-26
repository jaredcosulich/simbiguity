//! Compiling a scene and a camera into one frame of draw commands.
//!
//! Draw order is decided here, not by the backend:
//! 1. opaque nodes, grouped by program (depth test + write, no blending);
//! 2. translucent nodes, back to front (alpha blending, no depth write);
//! 3. glow halos (additive, no depth write);
//! 4. instanced particles (additive, no depth write).

use alloc::vec::Vec;

use simbiguity_math::{Mat4, Vec3};

use crate::camera::OrbitCamera;
use crate::commands::{Blend, Command, CommandBuffer, Program, Uniform};
use crate::scene::{Node, Scene, Shading};

/// Writes one frame of `scene` seen through `camera` at `width × height`
/// pixels into `out` (which is cleared first).
pub fn encode_frame(
    scene: &Scene,
    camera: &OrbitCamera,
    width: u32,
    height: u32,
    out: &mut CommandBuffer,
) {
    out.clear();
    let (w, h) = (width.max(1), height.max(1));
    let view_proj = mat(&camera.view_projection(f64::from(w) / f64::from(h)));
    let eye = camera.eye();
    let mut enc = Encoder {
        out,
        program: None,
        view_proj,
        scene,
        eye,
    };
    enc.out.push(&Command::Viewport(w, h));
    enc.out.push(&Command::Clear(scene.clear));
    enc.out.push(&Command::SetDepth {
        test: true,
        write: true,
    });
    enc.out.push(&Command::SetBlend(Blend::Off));

    let DrawOrder {
        opaque,
        glass,
        glows,
    } = draw_order(scene, eye);
    for n in opaque {
        enc.draw(n);
    }

    if !glass.is_empty() {
        enc.out.push(&Command::SetDepth {
            test: true,
            write: false,
        });
        enc.out.push(&Command::SetBlend(Blend::Alpha));
        for n in glass {
            enc.draw(n);
        }
    }

    let has_particles = scene.instances.iter().any(|i| !i.offsets.is_empty());
    if !glows.is_empty() || has_particles {
        enc.out.push(&Command::SetDepth {
            test: true,
            write: false,
        });
        enc.out.push(&Command::SetBlend(Blend::Additive));
    }
    for n in glows {
        enc.draw(n);
    }
    for inst in scene.instances.iter().filter(|i| !i.offsets.is_empty()) {
        enc.use_program(Program::Instanced);
        let m = &inst.material;
        enc.out.push(&Command::SetUniformVec4(
            Uniform::Color,
            [m.color[0], m.color[1], m.color[2], m.alpha],
        ));
        enc.out
            .push(&Command::SetUniformF32(Uniform::Emissive, m.emissive));
        enc.out
            .push(&Command::SetUniformF32(Uniform::Scale, inst.scale as f32));
        enc.out.push(&Command::BindMesh(inst.mesh.0));
        let count = scene.meshes[inst.mesh.0 as usize].indices.len() as u32;
        let offsets = inst
            .offsets
            .iter()
            .map(|p| [p.x as f32, p.y as f32, p.z as f32])
            .collect();
        enc.out.push(&Command::DrawInstanced { count, offsets });
    }
    enc.out.push(&Command::SetDepth {
        test: true,
        write: true,
    });
    enc.out.push(&Command::SetBlend(Blend::Off));
}

/// The visible nodes of a scene, in the order a frame draws them.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawOrder<'a> {
    /// Opaque nodes, grouped by program then mesh.
    pub opaque: Vec<&'a Node>,
    /// Translucent (non-glow) nodes, farthest from the eye first.
    pub glass: Vec<&'a Node>,
    /// Glow halos that are lit at all (emissive above zero).
    pub glows: Vec<&'a Node>,
}

/// Sorts the visible nodes of `scene` into the three node passes of a frame
/// seen from `eye`. Hidden nodes and unlit glows are left out.
pub fn draw_order(scene: &Scene, eye: Vec3) -> DrawOrder<'_> {
    let visible = || scene.nodes.iter().filter(|n| n.visible);
    let mut opaque: Vec<&Node> = visible().filter(|n| !n.material.is_blended()).collect();
    opaque.sort_by_key(|n| (program_of(n.material.shading) as u32, n.mesh.0));
    let mut glass: Vec<&Node> = visible()
        .filter(|n| n.material.is_blended() && n.material.shading != Shading::Glow)
        .collect();
    glass.sort_by(|a, b| depth(b, eye).total_cmp(&depth(a, eye)));
    let glows = visible()
        .filter(|n| n.material.shading == Shading::Glow && n.material.emissive > 0.0)
        .collect();
    DrawOrder {
        opaque,
        glass,
        glows,
    }
}

struct Encoder<'a> {
    out: &'a mut CommandBuffer,
    program: Option<Program>,
    view_proj: [f32; 16],
    scene: &'a Scene,
    eye: Vec3,
}

impl Encoder<'_> {
    /// Binds `p` and its per-frame uniforms, if it is not already bound.
    fn use_program(&mut self, p: Program) {
        if self.program == Some(p) {
            return;
        }
        self.program = Some(p);
        let l = &self.scene.light;
        let o = &mut *self.out;
        o.push(&Command::SetProgram(p));
        o.push(&Command::SetUniformMat4(Uniform::ViewProj, self.view_proj));
        o.push(&Command::SetUniformVec4(
            Uniform::LightDir,
            [
                l.direction.x as f32,
                l.direction.y as f32,
                l.direction.z as f32,
                0.0,
            ],
        ));
        o.push(&Command::SetUniformVec4(
            Uniform::LightColor,
            [l.color[0], l.color[1], l.color[2], l.ambient],
        ));
        o.push(&Command::SetUniformVec4(
            Uniform::Eye,
            [self.eye.x as f32, self.eye.y as f32, self.eye.z as f32, 1.0],
        ));
    }

    fn draw(&mut self, n: &Node) {
        self.use_program(program_of(n.material.shading));
        let m = &n.material;
        let count = self.scene.meshes[n.mesh.0 as usize].indices.len() as u32;
        let o = &mut *self.out;
        o.push(&Command::SetUniformMat4(Uniform::Model, mat(&n.transform)));
        o.push(&Command::SetUniformVec4(
            Uniform::Color,
            [m.color[0], m.color[1], m.color[2], m.alpha],
        ));
        o.push(&Command::SetUniformF32(Uniform::Emissive, m.emissive));
        o.push(&Command::BindMesh(n.mesh.0));
        o.push(&Command::DrawIndexed { count, first: 0 });
    }
}

fn program_of(s: Shading) -> Program {
    match s {
        Shading::Unlit => Program::Unlit,
        Shading::Lit => Program::Lit,
        Shading::Glow => Program::Glow,
    }
}

/// Squared distance from the eye to a node's origin, for back-to-front sorting.
fn depth(n: &Node, eye: Vec3) -> f64 {
    let p = n.transform.transform_point(Vec3::ZERO);
    (p - eye).length_squared()
}

fn mat(m: &Mat4) -> [f32; 16] {
    let mut out = [0.0; 16];
    for (o, v) in out.iter_mut().zip(m.m.iter()) {
        *o = *v as f32;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::Mesh;
    use crate::scene::{Instances, Material};
    use alloc::vec;

    /// Opaque lit and unlit boxes, two glass boxes at different depths, a lit and an unlit halo, one hidden box.
    fn scene() -> Scene {
        let mut s = Scene::new();
        let m = s.add_mesh(Mesh::cuboid(Vec3::new(1.0, 1.0, 1.0)));
        let at = |z: f64| Mat4::translation(Vec3::new(0.0, 0.0, z));
        s.add_node(m, at(0.0), Material::lit([1.0; 3]));
        s.add_node(m, at(0.0), Material::unlit([1.0; 3]));
        s.add_node(m, at(3.0), Material::lit([1.0; 3]).with_alpha(0.5));
        s.add_node(m, at(-3.0), Material::lit([1.0; 3]).with_alpha(0.5));
        s.add_node(m, at(0.0), Material::glow([1.0; 3], 0.8));
        s.add_node(m, at(0.0), Material::glow([1.0; 3], 0.0));
        let hidden = s.add_node(m, at(0.0), Material::lit([1.0; 3]));
        s.node_mut(hidden).visible = false;
        s
    }

    fn eye() -> Vec3 {
        Vec3::new(0.0, 0.0, 10.0)
    }

    /// Opaque nodes are grouped by program, unlit before lit, and hidden nodes are left out.
    #[test]
    fn opaque_nodes_group_by_program() {
        let s = scene();
        let o = draw_order(&s, eye());
        let shading: Vec<Shading> = o.opaque.iter().map(|n| n.material.shading).collect();
        assert_eq!(shading, [Shading::Unlit, Shading::Lit]);
    }

    /// Glass is drawn back to front: the far box before the near one.
    #[test]
    fn glass_is_sorted_back_to_front() {
        let s = scene();
        let o = draw_order(&s, eye());
        let z: Vec<f64> = o.glass.iter().map(|n| n.transform.get(2, 3)).collect();
        assert_eq!(z, [-3.0, 3.0]);
    }

    /// Only lit halos are drawn.
    #[test]
    fn unlit_glows_are_skipped() {
        let s = scene();
        let o = draw_order(&s, eye());
        assert_eq!(o.glows.len(), 1);
        assert_eq!(o.glows[0].material.emissive, 0.8);
    }

    fn encode(s: &Scene) -> Vec<crate::commands::Command> {
        let mut out = CommandBuffer::new();
        encode_frame(s, &OrbitCamera::default(), 640, 400, &mut out);
        CommandBuffer::decode(out.words()).unwrap()
    }

    /// A frame starts with viewport, clear and opaque state, ends restoring it, and draws every visible lit node.
    #[test]
    fn frame_brackets_its_passes() {
        let cs = encode(&scene());
        assert_eq!(cs[0], Command::Viewport(640, 400));
        assert!(matches!(cs[1], Command::Clear(_)));
        let n = cs.len();
        assert_eq!(cs[n - 1], Command::SetBlend(Blend::Off));
        assert_eq!(
            cs[n - 2],
            Command::SetDepth {
                test: true,
                write: true
            }
        );
        let draws = cs
            .iter()
            .filter(|c| matches!(c, Command::DrawIndexed { .. }))
            .count();
        assert_eq!(draws, 2 + 2 + 1);
        assert!(cs.contains(&Command::SetBlend(Blend::Alpha)));
        assert!(cs.contains(&Command::SetBlend(Blend::Additive)));
    }

    /// Each program is bound once per pass, not once per node.
    #[test]
    fn programs_are_bound_only_on_change() {
        let mut s = Scene::new();
        let m = s.add_mesh(Mesh::cuboid(Vec3::new(1.0, 1.0, 1.0)));
        for _ in 0..3 {
            s.add_node(m, Mat4::IDENTITY, Material::lit([1.0; 3]));
        }
        let binds = encode(&s)
            .iter()
            .filter(|c| matches!(c, Command::SetProgram(_)))
            .count();
        assert_eq!(binds, 1);
    }

    /// Instanced particles draw once with every offset; an empty set draws nothing and skips the additive pass.
    #[test]
    fn instances_draw_in_one_call() {
        let mut s = Scene::new();
        let m = s.add_mesh(Mesh::sphere(1.0, 4, 6));
        s.instances.push(Instances {
            mesh: m,
            material: Material::unlit([1.0; 3]),
            scale: 0.1,
            offsets: vec![],
        });
        assert!(!encode(&s).contains(&Command::SetBlend(Blend::Additive)));
        s.instances[0].offsets = vec![Vec3::X, Vec3::Y];
        let cs = encode(&s);
        let inst: Vec<&Command> = cs
            .iter()
            .filter(|c| matches!(c, Command::DrawInstanced { .. }))
            .collect();
        assert_eq!(
            inst,
            [&Command::DrawInstanced {
                count: s.meshes[0].indices.len() as u32,
                offsets: vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            }]
        );
        assert!(cs.contains(&Command::SetProgram(Program::Instanced)));
        assert!(cs.contains(&Command::SetUniformF32(Uniform::Scale, 0.1)));
    }

    /// A zero-size viewport is drawn as one pixel rather than dividing by zero.
    #[test]
    fn zero_viewport_is_clamped() {
        let mut out = CommandBuffer::new();
        encode_frame(&Scene::new(), &OrbitCamera::default(), 0, 0, &mut out);
        assert_eq!(
            CommandBuffer::decode(out.words()).unwrap()[0],
            Command::Viewport(1, 1)
        );
    }
}
