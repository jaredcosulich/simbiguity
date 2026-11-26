//! What exists in the world: meshes, placed nodes, particles and a light.

use alloc::vec::Vec;

use simbiguity_math::{Mat4, Vec3};

use crate::mesh::Mesh;
use crate::pick::{Aabb, Hit, Ray};

/// A mesh registered with a [`Scene`]. Backends upload meshes once, by id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MeshId(pub u32);

/// A node in a [`Scene`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub usize);

/// How a surface is shaded. Each maps to one built-in shader program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Shading {
    /// Flat colour, no lighting.
    Unlit,
    /// Lambert diffuse + Blinn-Phong highlight, plus `emissive` self-light.
    Lit,
    /// An additive halo, brightest where the surface faces the camera:
    /// light bleeding out of a glowing object.
    Glow,
}

/// Surface appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    /// Linear RGB base colour, `0..=1`.
    pub color: [f32; 3],
    /// Opacity. Below 1 the node is drawn after opaque ones, back to front.
    pub alpha: f32,
    /// How much the surface glows by itself (`Lit`) or the halo's strength (`Glow`).
    pub emissive: f32,
    /// Which shader.
    pub shading: Shading,
}

impl Material {
    /// An opaque lit material.
    pub const fn lit(color: [f32; 3]) -> Material {
        Material {
            color,
            alpha: 1.0,
            emissive: 0.0,
            shading: Shading::Lit,
        }
    }

    /// An opaque unlit material.
    pub const fn unlit(color: [f32; 3]) -> Material {
        Material {
            color,
            alpha: 1.0,
            emissive: 0.0,
            shading: Shading::Unlit,
        }
    }

    /// A glow halo of `color` at `strength`.
    pub const fn glow(color: [f32; 3], strength: f32) -> Material {
        Material {
            color,
            alpha: 1.0,
            emissive: strength,
            shading: Shading::Glow,
        }
    }

    /// The same material with opacity `alpha`.
    pub const fn with_alpha(mut self, alpha: f32) -> Material {
        self.alpha = alpha;
        self
    }

    /// The same material with self-illumination `emissive`.
    pub const fn with_emissive(mut self, emissive: f32) -> Material {
        self.emissive = emissive;
        self
    }

    /// `true` when this material needs blending (translucent or halo).
    pub fn is_blended(&self) -> bool {
        self.alpha < 1.0 || self.shading == Shading::Glow
    }
}

/// A mesh placed in the world.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Which mesh.
    pub mesh: MeshId,
    /// Model → world transform.
    pub transform: Mat4,
    /// Appearance.
    pub material: Material,
    /// Hidden nodes are neither drawn nor picked.
    pub visible: bool,
    /// A tag returned by [`Scene::pick`] when this node is hit; `None` = not pickable.
    pub pick: Option<u32>,
}

/// Many copies of one small mesh at different positions, drawn in a single
/// instanced call: the current particles flowing along a wire.
#[derive(Debug, Clone, PartialEq)]
pub struct Instances {
    /// Which mesh each copy uses.
    pub mesh: MeshId,
    /// Appearance shared by every copy (drawn self-lit and additive).
    pub material: Material,
    /// Uniform scale applied to the mesh.
    pub scale: f64,
    /// World positions, one per copy.
    pub offsets: Vec<Vec3>,
}

/// One directional light plus ambient.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    /// Direction *toward* the light, unit length.
    pub direction: Vec3,
    /// Light colour.
    pub color: [f32; 3],
    /// Ambient fraction, `0..=1`.
    pub ambient: f32,
}

impl Default for Light {
    fn default() -> Light {
        Light {
            direction: Vec3::new(0.4, 1.0, 0.6).normalize().unwrap_or(Vec3::Y),
            color: [1.0, 0.97, 0.92],
            ambient: 0.28,
        }
    }
}

/// Everything to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    /// Registered meshes, indexed by [`MeshId`].
    pub meshes: Vec<Mesh>,
    /// Placed nodes, indexed by [`NodeId`].
    pub nodes: Vec<Node>,
    /// Instanced particle sets.
    pub instances: Vec<Instances>,
    /// The light.
    pub light: Light,
    /// Background colour, RGBA.
    pub clear: [f32; 4],
}

impl Default for Scene {
    fn default() -> Scene {
        Scene {
            meshes: Vec::new(),
            nodes: Vec::new(),
            instances: Vec::new(),
            light: Light::default(),
            clear: [0.09, 0.1, 0.12, 1.0],
        }
    }
}

impl Scene {
    /// An empty scene.
    pub fn new() -> Scene {
        Scene::default()
    }

    /// Registers a mesh.
    pub fn add_mesh(&mut self, mesh: Mesh) -> MeshId {
        self.meshes.push(mesh);
        MeshId(self.meshes.len() as u32 - 1)
    }

    /// Places `mesh` with `transform` and `material`; visible, not pickable.
    pub fn add_node(&mut self, mesh: MeshId, transform: Mat4, material: Material) -> NodeId {
        self.nodes.push(Node {
            mesh,
            transform,
            material,
            visible: true,
            pick: None,
        });
        NodeId(self.nodes.len() - 1)
    }

    /// The node behind `id`.
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id.0]
    }

    /// The node behind `id`, to move or restyle it.
    pub fn node_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id.0]
    }

    /// World-space bounds of a node.
    pub fn world_bounds(&self, id: NodeId) -> Aabb {
        let n = &self.nodes[id.0];
        self.meshes[n.mesh.0 as usize]
            .bounds()
            .transformed(&n.transform)
    }

    /// The nearest visible, pickable node the ray hits.
    pub fn pick(&self, ray: &Ray) -> Option<Hit> {
        let mut best: Option<Hit> = None;
        for (i, n) in self.nodes.iter().enumerate() {
            let (true, Some(tag)) = (n.visible, n.pick) else {
                continue;
            };
            if let Some(d) = self.world_bounds(NodeId(i)).intersect(ray) {
                if best.map_or(true, |b| d < b.distance) {
                    best = Some(Hit { tag, distance: d });
                }
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two pickable unit boxes on the Z axis, tags 1 near the viewer and 2 far.
    fn two_boxes() -> Scene {
        let mut s = Scene::new();
        let m = s.add_mesh(Mesh::cuboid(Vec3::new(1.0, 1.0, 1.0)));
        let near = s.add_node(
            m,
            Mat4::translation(Vec3::new(0.0, 0.0, 2.0)),
            Material::lit([1.0; 3]),
        );
        let far = s.add_node(m, Mat4::IDENTITY, Material::lit([1.0; 3]));
        s.node_mut(near).pick = Some(1);
        s.node_mut(far).pick = Some(2);
        s
    }

    fn down_z() -> Ray {
        Ray {
            origin: Vec3::new(0.0, 0.0, 10.0),
            dir: -Vec3::Z,
        }
    }

    /// The nearest pickable node along the ray wins.
    #[test]
    fn pick_returns_the_nearest_hit() {
        let hit = two_boxes().pick(&down_z()).unwrap();
        assert_eq!(hit.tag, 1);
        assert!((hit.distance - 7.5).abs() < 1e-9);
    }

    /// Hidden or untagged nodes are skipped.
    #[test]
    fn hidden_and_untagged_nodes_are_not_picked() {
        let mut s = two_boxes();
        s.nodes[0].visible = false;
        assert_eq!(s.pick(&down_z()).map(|h| h.tag), Some(2));
        s.nodes[1].pick = None;
        assert_eq!(s.pick(&down_z()), None);
    }

    /// A ray that misses everything picks nothing.
    #[test]
    fn a_miss_is_none() {
        let r = Ray {
            origin: Vec3::new(5.0, 0.0, 10.0),
            dir: -Vec3::Z,
        };
        assert_eq!(two_boxes().pick(&r), None);
    }

    /// Ids index meshes and nodes in insertion order; world bounds follow the transform.
    #[test]
    fn ids_and_world_bounds() {
        let s = two_boxes();
        assert_eq!(s.meshes.len(), 1);
        assert_eq!(s.node(NodeId(1)).mesh, MeshId(0));
        let b = s.world_bounds(NodeId(0));
        assert_eq!((b.min.z, b.max.z), (1.5, 2.5));
    }

    /// The material constructors set shading, and blending is needed for translucency and glow only.
    #[test]
    fn materials_know_when_they_blend() {
        let lit = Material::lit([0.5; 3]);
        assert_eq!(
            (lit.shading, lit.alpha, lit.emissive),
            (Shading::Lit, 1.0, 0.0)
        );
        assert!(!lit.is_blended());
        assert!(lit.with_alpha(0.5).is_blended());
        assert!(!Material::unlit([0.0; 3]).with_emissive(2.0).is_blended());
        assert_eq!(Material::unlit([0.0; 3]).with_emissive(2.0).emissive, 2.0);
        let g = Material::glow([1.0; 3], 0.7);
        assert!(g.is_blended());
        assert_eq!(g.emissive, 0.7);
    }

    /// The default light points up and toward the viewer, at unit length.
    #[test]
    fn default_light_is_unit_and_overhead() {
        let l = Light::default();
        assert!((l.direction.length() - 1.0).abs() < 1e-12);
        assert!(l.direction.y > 0.0);
    }
}
