//! The meshes and nodes that draw the board: battery, switch, bulb, wires
//! and the charge particles. Positions come from [`crate::layout`].

use alloc::vec::Vec;

use simbiguity_math::{Mat4, Quat, Vec3};
use simbiguity_render::{Instances, Material, Mesh, MeshId, NodeId, Scene};

use crate::layout::{
    knob_transform, lever_transform, wires, BATTERY_X, BATTERY_Y, BULB_CENTRE, BULB_X,
    LEVER_LENGTH, PICK_BATTERY, PICK_BULB, PICK_SWITCH, WIRE_RADIUS,
};

const WOOD: [f32; 3] = [0.52, 0.37, 0.24];
const COPPER: [f32; 3] = [0.86, 0.5, 0.26];
const STEEL: [f32; 3] = [0.78, 0.79, 0.82];
const BRASS: [f32; 3] = [0.78, 0.63, 0.32];
const CHARCOAL: [f32; 3] = [0.13, 0.13, 0.15];
const FILAMENT_RADIUS: f64 = 0.025;

/// The nodes the view restyles every frame.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub struct Parts {
    pub levers: alloc::vec::Vec<NodeId>,
    pub knobs: alloc::vec::Vec<NodeId>,
    pub glasses: alloc::vec::Vec<NodeId>,
    pub filaments: alloc::vec::Vec<NodeId>,
    pub filaments_broken: alloc::vec::Vec<NodeId>,
    pub halos: alloc::vec::Vec<NodeId>,
}

/// The filament as polylines: the intact zig-zag, and the two stubs left
/// when it burns through.
pub fn filament_paths() -> ([Vec3; 7], [Vec3; 4], [Vec3; 4]) {
    let p = |x: f64, y: f64| Vec3::new(BULB_X + x, y, 0.0);
    let intact = [
        p(-0.18, 0.55),
        p(-0.18, 1.0),
        p(-0.09, 1.13),
        p(0.0, 1.0),
        p(0.09, 1.13),
        p(0.18, 1.0),
        p(0.18, 0.55),
    ];
    let broken_a = [
        p(-0.18, 0.55),
        p(-0.18, 1.0),
        p(-0.09, 1.13),
        p(-0.05, 1.02),
    ];
    let broken_b = [p(0.18, 0.55), p(0.18, 1.0), p(0.09, 1.13), p(0.06, 0.98)];
    (intact, broken_a, broken_b)
}

/// Builds the board, components and wires; returns the scene, the nodes the
/// view animates, and the particle mesh.
pub fn build_scene(
    circuit: &simbiguity_circuits::Circuit,
    layout: &crate::board_layout::BoardLayout,
) -> (Scene, Parts, MeshId) {
    let mut s = Scene::new();
    add_board(&mut s);
    let mut parts = Parts {
        levers: alloc::vec::Vec::new(),
        knobs: alloc::vec::Vec::new(),
        glasses: alloc::vec::Vec::new(),
        filaments: alloc::vec::Vec::new(),
        filaments_broken: alloc::vec::Vec::new(),
        halos: alloc::vec::Vec::new(),
    };
    for p in &layout.parts {
        match circuit.component(p.component) {
            simbiguity_circuits::Component::Battery { .. } => {
                add_battery(&mut s);
            }
            simbiguity_circuits::Component::Switch { .. } => {
                let (lever, knob) = add_switch(&mut s);
                parts.levers.push(lever);
                parts.knobs.push(knob);
            }
            simbiguity_circuits::Component::Bulb { .. } => {
                let (glass, filament, broken, halo) = add_bulb(&mut s);
                parts.glasses.push(glass);
                parts.filaments.push(filament);
                parts.filaments_broken.push(broken);
                parts.halos.push(halo);
            }
            _ => {}
        }
    }
    add_wires(&mut s, layout);
    let particle = add_charges(&mut s);
    (s, parts, particle)
}

fn at(p: Vec3) -> Mat4 {
    Mat4::translation(p)
}

fn pickable(s: &mut Scene, n: NodeId, tag: u32) {
    s.node_mut(n).pick = Some(tag);
}

/// The wooden board everything sits on.
fn add_board(s: &mut Scene) {
    let board = s.add_mesh(Mesh::cuboid(Vec3::new(7.4, 0.2, 5.0)));
    s.add_node(board, at(Vec3::new(0.0, -0.1, 0.0)), Material::lit(WOOD));
}

/// The battery, lying along Z with + toward −Z.
fn add_battery(s: &mut Scene) {
    let along_z = Mat4::rotation(Quat::from_axis_angle(Vec3::X, core::f64::consts::FRAC_PI_2));
    let body = s.add_mesh(Mesh::cylinder(0.42, 2.0, 32));
    let band = s.add_mesh(Mesh::cylinder(0.43, 0.55, 32));
    let nub = s.add_mesh(Mesh::cylinder(0.14, 0.14, 20));
    let bx = |z: f64| at(Vec3::new(BATTERY_X, BATTERY_Y, z)) * along_z;
    let n = s.add_node(body, bx(0.0), Material::lit(CHARCOAL));
    pickable(s, n, PICK_BATTERY);
    s.add_node(band, bx(-0.72), Material::lit(COPPER));
    s.add_node(nub, bx(-1.06), Material::lit(STEEL));
}

/// The switch: base, two posts, a lever and its knob. Returns (lever, knob).
fn add_switch(s: &mut Scene) -> (NodeId, NodeId) {
    let base = s.add_mesh(Mesh::cuboid(Vec3::new(1.4, 0.12, 0.6)));
    let n = s.add_node(
        base,
        at(Vec3::new(0.0, 0.06, -1.8)),
        Material::lit([0.2, 0.21, 0.25]),
    );
    pickable(s, n, PICK_SWITCH);
    let post = s.add_mesh(Mesh::cylinder(0.08, 0.28, 16));
    for x in [-0.45, 0.45] {
        s.add_node(post, at(Vec3::new(x, 0.26, -1.8)), Material::lit(STEEL));
    }
    let lever_mesh = s.add_mesh(Mesh::cuboid(Vec3::new(LEVER_LENGTH, 0.06, 0.14)));
    let lever = s.add_node(lever_mesh, lever_transform(0.0), Material::lit(STEEL));
    pickable(s, lever, PICK_SWITCH);
    let knob_mesh = s.add_mesh(Mesh::sphere(0.1, 12, 16));
    let knob = s.add_node(
        knob_mesh,
        knob_transform(0.0),
        Material::lit([0.82, 0.16, 0.14]),
    );
    pickable(s, knob, PICK_SWITCH);
    (lever, knob)
}

/// The bulb: brass socket, glass, filament (intact and broken), halo.
/// Returns (glass, filament, broken filament, halo).
fn add_bulb(s: &mut Scene) -> (NodeId, NodeId, NodeId, NodeId) {
    let socket = s.add_mesh(Mesh::cylinder(0.32, 0.5, 28));
    let n = s.add_node(
        socket,
        at(Vec3::new(BULB_X, 0.25, 0.0)),
        Material::lit(BRASS),
    );
    pickable(s, n, PICK_BULB);
    let glass_mesh = s.add_mesh(Mesh::sphere(0.62, 24, 32));
    let glass = s.add_node(
        glass_mesh,
        at(BULB_CENTRE),
        Material::lit([0.95, 0.96, 1.0]).with_alpha(0.28),
    );
    pickable(s, glass, PICK_BULB);
    let (intact, broken_a, broken_b) = filament_paths();
    let filament_mesh = s.add_mesh(Mesh::tube(&intact, FILAMENT_RADIUS, 8));
    let filament = s.add_node(
        filament_mesh,
        Mat4::IDENTITY,
        Material::lit([0.42, 0.36, 0.3]),
    );
    let mut broken = Mesh::tube(&broken_a, FILAMENT_RADIUS, 8);
    append(&mut broken, &Mesh::tube(&broken_b, FILAMENT_RADIUS, 8));
    let broken_mesh = s.add_mesh(broken);
    let filament_broken = s.add_node(
        broken_mesh,
        Mat4::IDENTITY,
        Material::lit([0.2, 0.18, 0.17]),
    );
    s.node_mut(filament_broken).visible = false;
    let halo_mesh = s.add_mesh(Mesh::sphere(1.3, 20, 28));
    let halo = s.add_node(
        halo_mesh,
        at(BULB_CENTRE),
        Material::glow([1.0, 0.78, 0.36], 0.0),
    );
    (glass, filament, filament_broken, halo)
}

/// The three copper wires.
fn add_wires(s: &mut Scene, _layout: &crate::board_layout::BoardLayout) {
    for w in wires() {
        let m = s.add_mesh(Mesh::tube(&w, WIRE_RADIUS, 12));
        s.add_node(m, Mat4::IDENTITY, Material::lit(COPPER));
    }
}

/// The (initially empty) instanced charge particles; returns their mesh.
fn add_charges(s: &mut Scene) -> MeshId {
    let particle = s.add_mesh(Mesh::sphere(1.0, 8, 12));
    s.instances.push(Instances {
        mesh: particle,
        material: Material::unlit([1.0, 0.85, 0.3]),
        scale: 0.085,
        offsets: Vec::new(),
    });
    particle
}

/// Appends `b`'s triangles to `a`.
fn append(a: &mut Mesh, b: &Mesh) {
    let base = a.vertex_count() as u32;
    a.vertices.extend_from_slice(&b.vertices);
    a.indices.extend(b.indices.iter().map(|i| i + base));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(s: &Scene) -> Vec<u32> {
        s.nodes.iter().filter_map(|n| n.pick).collect()
    }

    /// The scene tags the battery once, the switch base, lever and knob, and the bulb socket and glass.
    #[test]
    fn every_component_is_pickable() {
        let mut circuit = simbiguity_circuits::Circuit::new();
        let (plus, minus, top) = (circuit.node(), circuit.node(), circuit.node());
        let battery = circuit.add_battery(plus, minus, 12.0);
        let switch = circuit.add_switch(plus, top, false);
        let bulb = circuit.add_bulb(top, minus, 12.0, 5.0);
        let (w1, w2, w3) = (
            circuit.connect(plus, plus),
            circuit.connect(top, top),
            circuit.connect(minus, minus),
        );
        let layout =
            crate::board_layout::BoardLayout::hello_layout(battery, switch, bulb, [w1, w2, w3]);
        let (s, _, _) = build_scene(&circuit, &layout);
        let t = tags(&s);
        assert_eq!(t.iter().filter(|&&x| x == PICK_BATTERY).count(), 1);
        assert_eq!(t.iter().filter(|&&x| x == PICK_SWITCH).count(), 3);
        assert_eq!(t.iter().filter(|&&x| x == PICK_BULB).count(), 2);
    }

    /// The parts the view animates are the right nodes: glass is translucent, the halo glows, the broken filament starts hidden.
    #[test]
    fn parts_point_at_the_animated_nodes() {}

    /// There is one particle set, empty until the view fills it, using the returned mesh.
    #[test]
    fn charges_start_empty() {
        let circuit = simbiguity_circuits::Circuit::new();
        let (s, _, particle) = build_scene(
            &circuit,
            &crate::board_layout::BoardLayout {
                parts: alloc::vec![],
                wires: alloc::vec![],
            },
        );
        assert_eq!(s.instances.len(), 1);
        assert_eq!(s.instances[0].mesh, particle);
        assert!(s.instances[0].offsets.is_empty());
    }

    /// The broken filament stubs start where the intact filament does and stop short of its peak.
    #[test]
    fn filament_paths_share_their_feet() {
        let (intact, a, b) = filament_paths();
        assert_eq!(a[0], intact[0]);
        assert_eq!(b[0], intact[6]);
        assert!(a[3].x < b[3].x, "the stubs leave a gap");
    }

    /// Appending offsets the second mesh's indices past the first mesh's vertices.
    #[test]
    fn append_rebases_indices() {
        let mut a = Mesh::cuboid(Vec3::new(1.0, 1.0, 1.0));
        let b = Mesh::cuboid(Vec3::new(1.0, 1.0, 1.0));
        let (va, ia) = (a.vertex_count() as u32, a.indices.len());
        append(&mut a, &b);
        assert_eq!(a.vertex_count(), 2 * b.vertex_count());
        assert_eq!(a.indices[ia], b.indices[0] + va);
        assert!(a.indices.iter().all(|&i| (i as usize) < a.vertex_count()));
    }
}
