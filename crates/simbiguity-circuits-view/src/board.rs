//! The meshes and nodes that draw the board: battery, switch, bulb, wires
//! and the charge particles. Positions come from [`crate::layout`].

use alloc::vec::Vec;

use simbiguity_math::{Mat4, Quat, Vec3};
use simbiguity_render::{Instances, Material, Mesh, MeshId, NodeId, Scene};

use crate::layout::{
    knob_transform, lever_transform, BATTERY_Y, LEVER_LENGTH, WIRE_RADIUS, WIRE_Y,
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
    pub switch_bases: alloc::vec::Vec<Mat4>,
    pub glasses: alloc::vec::Vec<NodeId>,
    pub filaments: alloc::vec::Vec<NodeId>,
    pub filaments_broken: alloc::vec::Vec<NodeId>,
    pub halos: alloc::vec::Vec<NodeId>,
    pub leds: alloc::vec::Vec<(simbiguity_circuits::ComponentId, NodeId, NodeId)>,
}

/// The filament as polylines: the intact zig-zag, and the two stubs left
/// when it burns through.
pub fn filament_paths() -> ([Vec3; 7], [Vec3; 4], [Vec3; 4]) {
    let p = |x: f64, y: f64| Vec3::new(x, y, 0.0);
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
        switch_bases: alloc::vec::Vec::new(),
        glasses: alloc::vec::Vec::new(),
        filaments: alloc::vec::Vec::new(),
        filaments_broken: alloc::vec::Vec::new(),
        halos: alloc::vec::Vec::new(),
        leds: alloc::vec::Vec::new(),
    };
    for p in &layout.parts {
        match circuit.component(p.component) {
            simbiguity_circuits::Component::Battery { .. } => {
                add_battery(&mut s, p);
            }
            simbiguity_circuits::Component::Switch { .. } => {
                let (lever, knob, base_mat) = add_switch(&mut s, p);
                parts.levers.push(lever);
                parts.knobs.push(knob);
                parts.switch_bases.push(base_mat);
            }
            simbiguity_circuits::Component::Bulb { .. } => {
                let (glass, filament, broken, halo) = add_bulb(&mut s, p);
                parts.glasses.push(glass);
                parts.filaments.push(filament);
                parts.filaments_broken.push(broken);
                parts.halos.push(halo);
            }
            simbiguity_circuits::Component::Diode { .. } => {
                add_diode(&mut s, p);
            }
            simbiguity_circuits::Component::Led { color, .. } => {
                let (glass, halo) = add_led(&mut s, p, color);
                parts.leds.push((p.component, glass, halo));
            }
            simbiguity_circuits::Component::SignalSource { .. } => {
                add_signal_source(&mut s, p);
            }
            simbiguity_circuits::Component::Bjt { .. } => {
                add_bjt(&mut s, p);
            }
            simbiguity_circuits::Component::OpAmp { .. } => {
                add_opamp(&mut s, p);
            }
            _ => {}
        }
    }
    add_wires(&mut s, layout);
    let particle = add_charges(&mut s, layout.wires.len());
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
fn base_transform(p: &crate::board_layout::PartPlacement, y: f64) -> Mat4 {
    let rot = match p.rotation {
        crate::board_layout::Rotation::Deg0 => 0.0,
        crate::board_layout::Rotation::Deg90 => core::f64::consts::FRAC_PI_2,
        crate::board_layout::Rotation::Deg180 => core::f64::consts::PI,
        crate::board_layout::Rotation::Deg270 => 1.5 * core::f64::consts::PI,
    };
    Mat4::translation(Vec3::new(p.cell.0 as f64 * 2.0, y, p.cell.1 as f64 * 2.0))
        * Mat4::rotation(Quat::from_axis_angle(Vec3::Y, rot))
}

fn add_battery(s: &mut Scene, p: &crate::board_layout::PartPlacement) {
    let along_z = Mat4::rotation(Quat::from_axis_angle(Vec3::X, core::f64::consts::FRAC_PI_2));
    let body = s.add_mesh(Mesh::cylinder(0.42, 2.0, 32));
    let band = s.add_mesh(Mesh::cylinder(0.43, 0.55, 32));
    let nub = s.add_mesh(Mesh::cylinder(0.14, 0.14, 20));
    let base = base_transform(p, BATTERY_Y);
    let bx = |z: f64| base * Mat4::translation(Vec3::new(0.0, 0.0, z)) * along_z;
    let n = s.add_node(body, bx(0.0), Material::lit(CHARCOAL));
    pickable(s, n, p.component.index() as u32 + 1);
    s.add_node(band, bx(-0.72), Material::lit(COPPER));
    s.add_node(nub, bx(-1.06), Material::lit(STEEL));
}

/// The switch: base, two posts, a lever and its knob. Returns (lever, knob).
fn add_switch(s: &mut Scene, p: &crate::board_layout::PartPlacement) -> (NodeId, NodeId, Mat4) {
    let base = s.add_mesh(Mesh::cuboid(Vec3::new(1.4, 0.12, 0.6)));
    let base_mat = base_transform(p, 0.06);
    let n = s.add_node(base, base_mat, Material::lit([0.2, 0.21, 0.25]));
    pickable(s, n, p.component.index() as u32 + 1);
    let post = s.add_mesh(Mesh::cylinder(0.08, 0.28, 16));
    for x in [-0.45, 0.45] {
        s.add_node(
            post,
            base_mat * at(Vec3::new(x, 0.2, 0.0)),
            Material::lit(STEEL),
        );
    }
    let lever_mesh = s.add_mesh(Mesh::cuboid(Vec3::new(LEVER_LENGTH, 0.06, 0.14)));
    let lever = s.add_node(
        lever_mesh,
        lever_transform(base_mat, 0.0),
        Material::lit(STEEL),
    );
    pickable(s, lever, p.component.index() as u32 + 1);
    let knob_mesh = s.add_mesh(Mesh::sphere(0.1, 12, 16));
    let knob = s.add_node(
        knob_mesh,
        knob_transform(base_mat, 0.0),
        Material::lit([0.82, 0.16, 0.14]),
    );
    pickable(s, knob, p.component.index() as u32 + 1);
    (lever, knob, base_mat)
}

/// The bulb: brass socket, glass, filament (intact and broken), halo.
/// Returns (glass, filament, broken filament, halo).
fn add_bulb(
    s: &mut Scene,
    p: &crate::board_layout::PartPlacement,
) -> (NodeId, NodeId, NodeId, NodeId) {
    let base_mat = base_transform(p, 0.0);
    let socket = s.add_mesh(Mesh::cylinder(0.32, 0.5, 28));
    let n = s.add_node(
        socket,
        base_mat * at(Vec3::new(0.0, 0.25, 0.0)),
        Material::lit(BRASS),
    );
    pickable(s, n, p.component.index() as u32 + 1);
    let glass_mesh = s.add_mesh(Mesh::sphere(0.62, 24, 32));
    let bulb_centre = Vec3::new(0.0, 1.05, 0.0);
    let glass = s.add_node(
        glass_mesh,
        base_mat * at(bulb_centre),
        Material::lit([0.95, 0.96, 1.0]).with_alpha(0.28),
    );
    pickable(s, glass, p.component.index() as u32 + 1);
    let (intact, broken_a, broken_b) = filament_paths();
    let filament_mesh = s.add_mesh(Mesh::tube(&intact, FILAMENT_RADIUS, 8));
    let filament = s.add_node(filament_mesh, base_mat, Material::lit([0.42, 0.36, 0.3]));
    let mut broken = Mesh::tube(&broken_a, FILAMENT_RADIUS, 8);
    append(&mut broken, &Mesh::tube(&broken_b, FILAMENT_RADIUS, 8));
    let broken_mesh = s.add_mesh(broken);
    let filament_broken = s.add_node(broken_mesh, base_mat, Material::lit([0.2, 0.18, 0.17]));
    s.node_mut(filament_broken).visible = false;
    let halo_mesh = s.add_mesh(Mesh::sphere(1.3, 20, 28));
    let halo = s.add_node(
        halo_mesh,
        base_mat * at(bulb_centre),
        Material::glow([1.0, 0.78, 0.36], 0.0),
    );
    (glass, filament, filament_broken, halo)
}

/// The three copper wires.
fn add_wires(s: &mut Scene, layout: &crate::board_layout::BoardLayout) {
    for w in &layout.wires {
        let pts: alloc::vec::Vec<Vec3> = w
            .points
            .iter()
            .map(|&(x, z)| Vec3::new(x as f64 * 2.0, WIRE_Y, z as f64 * 2.0))
            .collect();
        let m = s.add_mesh(Mesh::tube(&pts, WIRE_RADIUS, 12));
        s.add_node(m, Mat4::IDENTITY, Material::lit(COPPER));
    }
}

/// The (initially empty) instanced charge particles; returns their mesh.
fn add_charges(s: &mut Scene, wire_count: usize) -> MeshId {
    let particle = s.add_mesh(Mesh::sphere(1.0, 8, 12));
    for _ in 0..wire_count {
        s.instances.push(Instances {
            mesh: particle,
            material: Material::unlit([1.0, 0.85, 0.3]),
            scale: 0.085,
            offsets: Vec::new(),
        });
    }
    particle
}

/// Appends `b`'s triangles to `a`.
fn append(a: &mut Mesh, b: &Mesh) {
    let base = a.vertex_count() as u32;
    a.vertices.extend_from_slice(&b.vertices);
    a.indices.extend(b.indices.iter().map(|i| i + base));
}

fn add_diode(s: &mut Scene, p: &crate::board_layout::PartPlacement) {
    let along_z = Mat4::rotation(Quat::from_axis_angle(Vec3::X, core::f64::consts::FRAC_PI_2));
    let base = base_transform(p, BATTERY_Y);
    let bx = |z: f64| base * Mat4::translation(Vec3::new(0.0, 0.0, z)) * along_z;
    let body = s.add_mesh(Mesh::cylinder(0.2, 1.2, 16));
    let n = s.add_node(body, bx(0.0), Material::lit(CHARCOAL));
    pickable(s, n, p.component.index() as u32 + 1);

    let band = s.add_mesh(Mesh::cylinder(0.21, 0.2, 16));
    s.add_node(band, bx(-0.4), Material::lit(STEEL)); // Cathode end
}

fn add_led(s: &mut Scene, p: &crate::board_layout::PartPlacement, color: &str) -> (NodeId, NodeId) {
    let base_mat = base_transform(p, 0.0);
    let rgb = match color {
        "red" => [1.0, 0.2, 0.2],
        "green" => [0.2, 1.0, 0.2],
        "blue" => [0.2, 0.2, 1.0],
        "yellow" => [1.0, 1.0, 0.2],
        _ => [1.0, 1.0, 1.0],
    };

    let glass_mesh = s.add_mesh(Mesh::sphere(0.3, 16, 16));
    let bulb_centre = Vec3::new(0.0, 0.5, 0.0);
    let glass = s.add_node(
        glass_mesh,
        base_mat * at(bulb_centre),
        Material::lit(rgb).with_alpha(0.8),
    );
    pickable(s, glass, p.component.index() as u32 + 1);

    let halo_mesh = s.add_mesh(Mesh::sphere(1.0, 16, 16));
    let halo = s.add_node(
        halo_mesh,
        base_mat * at(bulb_centre),
        Material::glow(rgb, 0.0),
    );

    (glass, halo)
}

fn add_signal_source(s: &mut Scene, p: &crate::board_layout::PartPlacement) {
    let base_mat = base_transform(p, 0.1);
    let disc = s.add_mesh(Mesh::cylinder(0.8, 0.1, 32));
    let n = s.add_node(disc, base_mat, Material::lit(STEEL));
    pickable(s, n, p.component.index() as u32 + 1);
}

fn add_bjt(s: &mut Scene, p: &crate::board_layout::PartPlacement) {
    let base_mat = base_transform(p, 0.15);
    let body = s.add_mesh(Mesh::cuboid(Vec3::new(0.4, 0.4, 0.2)));
    let n = s.add_node(body, base_mat, Material::lit(CHARCOAL));
    pickable(s, n, p.component.index() as u32 + 1);
    for x in [-0.15, 0.0, 0.15] {
        let leg = s.add_mesh(Mesh::cylinder(0.02, 0.15, 8));
        let leg_mat = base_mat * at(Vec3::new(x, -0.075, 0.0));
        s.add_node(leg, leg_mat, Material::lit(STEEL));
    }
}

fn add_opamp(s: &mut Scene, p: &crate::board_layout::PartPlacement) {
    let base_mat = base_transform(p, 0.15);
    let body = s.add_mesh(Mesh::cylinder(0.4, 0.2, 3));
    let n = s.add_node(
        body,
        base_mat * Mat4::rotation(Quat::from_axis_angle(Vec3::Y, core::f64::consts::FRAC_PI_2)),
        Material::lit(CHARCOAL),
    );
    pickable(s, n, p.component.index() as u32 + 1);

    let plus_h = s.add_mesh(Mesh::cuboid(Vec3::new(0.08, 0.02, 0.02)));
    let plus_v = s.add_mesh(Mesh::cuboid(Vec3::new(0.02, 0.02, 0.08)));
    s.add_node(
        plus_h,
        base_mat * at(Vec3::new(-0.2, 0.11, -0.15)),
        Material::lit([1.0, 1.0, 1.0]),
    );
    s.add_node(
        plus_v,
        base_mat * at(Vec3::new(-0.2, 0.11, -0.15)),
        Material::lit([1.0, 1.0, 1.0]),
    );

    s.add_node(
        plus_h,
        base_mat * at(Vec3::new(-0.2, 0.11, 0.15)),
        Material::lit([1.0, 1.0, 1.0]),
    );

    for (x, z) in [(-0.15, -0.2), (-0.15, 0.2), (0.15, 0.0)] {
        let leg = s.add_mesh(Mesh::cylinder(0.02, 0.15, 8));
        let leg_mat = base_mat * at(Vec3::new(x, -0.075, z));
        s.add_node(leg, leg_mat, Material::lit(STEEL));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{PICK_BATTERY, PICK_BULB, PICK_SWITCH};

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
    fn parts_point_at_the_animated_nodes() {
        let mut circuit = simbiguity_circuits::Circuit::new();
        let (n1, n2, n3, n4, n5, n6) = (
            circuit.node(),
            circuit.node(),
            circuit.node(),
            circuit.node(),
            circuit.node(),
            circuit.node(),
        );
        let battery = circuit.add_battery(n1, n6, 12.0);
        let switch = circuit.add_switch(n2, n3, false);
        let bulb = circuit.add_bulb(n4, n5, 12.0, 5.0);
        let (w1, w2, w3) = (
            circuit.connect(n1, n2),
            circuit.connect(n3, n4),
            circuit.connect(n5, n6),
        );
        let layout =
            crate::board_layout::BoardLayout::hello_layout(battery, switch, bulb, [w1, w2, w3]);
        let (s, parts, _) = build_scene(&circuit, &layout);

        let glass_mat = s.node(parts.glasses[0]).material;
        assert!(glass_mat.alpha < 1.0);

        let halo_mat = s.node(parts.halos[0]).material;
        assert_eq!(halo_mat.emissive, 0.0);

        assert!(!s.node(parts.filaments_broken[0]).visible);
    }

    /// There is one particle set, empty until the view fills it, using the returned mesh.
    #[test]
    fn charges_start_empty() {
        let mut circuit = simbiguity_circuits::Circuit::new();
        let plus = circuit.node();
        let minus = circuit.node();
        let top = circuit.node();
        let battery = circuit.add_battery(plus, minus, 12.0);
        let switch = circuit.add_switch(plus, top, false);
        let bulb = circuit.add_bulb(top, minus, 12.0, 5.0);
        let w1 = circuit.connect(plus, plus);
        let w2 = circuit.connect(top, top);
        let w3 = circuit.connect(minus, minus);
        let layout =
            crate::board_layout::BoardLayout::hello_layout(battery, switch, bulb, [w1, w2, w3]);

        let (s, _, particle) = build_scene(&circuit, &layout);
        assert_eq!(s.instances.len(), 3);
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
