//! Tests of [`CircuitView`]: animation follows the solved circuit.

use super::*;
use crate::layout::{LEVER_OPEN_ANGLE, PICK_SWITCH};

/// Particle speed is proportional to current, signed, and zero for no current.
#[test]
fn particle_speed_is_proportional_to_current() {
    assert_eq!(particle_speed(0.0), 0.0);
    assert_eq!(particle_speed(0.5), 0.5 * SPEED_PER_AMP);
    assert_eq!(particle_speed(1.0), 2.0 * particle_speed(0.5));
    assert_eq!(particle_speed(-0.5), -particle_speed(0.5));
}

/// An open switch means no current, so the charges stand still and the bulb stays dark.
#[test]
fn open_switch_leaves_charges_still() {
    let mut v = CircuitView::new(12.0, false);
    let before = v.scene().instances[0].offsets.clone();
    v.seek(3.0);
    let r = v.readouts();
    assert_eq!((r.current, r.particle_speed, r.glow), (0.0, 0.0, 0.0));
    assert_eq!(v.scene().instances[0].offsets, before);
}

/// A closed switch at rated voltage drives half an amp and moves the charges.
#[test]
fn closed_switch_moves_charges() {
    let mut v = CircuitView::new(12.0, true);
    let before = v.scene().instances[0].offsets.clone();
    v.seek(0.5);
    let r = v.readouts();
    assert!(r.closed && r.solved && !r.blown);
    assert!((r.current - 0.5).abs() < 1e-9);
    assert!((r.particle_speed - 0.5 * SPEED_PER_AMP).abs() < 1e-9);
    assert_ne!(v.scene().instances[0].offsets, before);
}

/// The glow warms up through its spring instead of snapping to the brightness.
#[test]
fn glow_follows_brightness_through_the_spring() {
    let mut v = CircuitView::new(12.0, true);
    v.seek(0.0);
    assert_eq!(v.readouts().glow, 0.0, "a bulb starts cold");
    v.seek(0.1);
    let early = v.readouts().glow;
    v.seek(2.5);
    let late = v.readouts().glow;
    assert!(early > 0.0 && early < late, "{early} then {late}");
    assert!((late - v.readouts().brightness).abs() < 0.01);
}

/// Seeking is deterministic: the same inputs and time give the same frame.
#[test]
fn seek_is_deterministic() {
    let mut a = CircuitView::new(16.0, true);
    let mut b = CircuitView::new(16.0, true);
    b.advance(0.37);
    a.seek(2.5);
    b.seek(2.5);
    assert_eq!(a.encode(640, 400), b.encode(640, 400));
    assert_eq!(a.time(), b.time());
}

/// Advance runs whole fixed steps of wall-clock time.
#[test]
fn advance_runs_fixed_steps() {
    let mut v = CircuitView::new(12.0, true);
    v.advance(STEP * 10.0 + STEP * 0.5);
    assert!((v.time() - STEP * 10.0).abs() < 1e-9);
    v.advance(STEP * 0.5);
    assert!((v.time() - STEP * 11.0).abs() < 1e-9);
}

/// Toggling reports the new state and swings the lever toward it.
#[test]
fn toggle_swings_the_lever() {
    let mut v = CircuitView::new(12.0, false);
    v.seek(0.0);
    let open_tip = v.scene().node(v.parts.knobs[0]).transform.get(1, 3);
    assert!(v.toggle_switch());
    assert!(v.is_closed());
    v.advance(1.0);
    let closed_tip = v.scene().node(v.parts.knobs[0]).transform.get(1, 3);
    assert!(closed_tip < open_tip, "the knob drops as the lever closes");
    assert!(!v.toggle_switch());
    assert_eq!(v.lever_target(0), LEVER_OPEN_ANGLE);
}

/// Overvoltage blows the bulb, which breaks the loop, shows the broken filament and flashes; replacing it restores the circuit.
#[test]
fn blowing_and_replacing_the_bulb() {
    let mut v = CircuitView::new(12.0, true);
    v.set_volts(24.0);
    assert!(v.is_blown());
    let r = v.readouts();
    assert_eq!((r.current, r.particle_speed), (0.0, 0.0));
    let s = v.scene();
    assert!(!s.node(v.parts.filaments[0]).visible);
    assert!(s.node(v.parts.filaments_broken[0]).visible);
    let flash = s.node(v.parts.halos[0]).material.emissive;
    assert!(flash > 1.0, "the blow flashes: {flash}");
    v.advance(2.0);
    assert!(
        v.scene().node(v.parts.halos[0]).material.emissive < flash,
        "the flash decays"
    );
    v.set_volts(12.0);
    v.replace_bulb();
    assert!(!v.is_blown());
    assert!((v.readouts().current - 0.5).abs() < 1e-9);
}

/// The switch is under the pixel its base projects to; empty sky picks nothing.
#[test]
fn pick_finds_the_switch() {
    let v = CircuitView::new(12.0, false);
    let (x, y) = v
        .camera
        .project(Vec3::new(0.3, 0.1, -1.8), 960.0, 600.0)
        .unwrap();
    assert_eq!(v.pick(x, y, 960.0, 600.0), Some(PICK_SWITCH));
    assert_eq!(v.pick(2.0, 2.0, 960.0, 600.0), None);
}

/// encode and commands return the same frame.
#[test]
fn commands_returns_the_last_frame() {
    let mut v = CircuitView::new(12.0, true);
    let words = v.encode(320, 200).to_vec();
    assert_eq!(v.commands(), &words[..]);
}

/// The default camera opens on a three-quarter view within its zoom limits.
#[test]
fn default_camera_is_three_quarter() {
    let c = default_camera();
    assert!((c.yaw.to_degrees() - 25.0).abs() < 1e-9);
    assert!(c.distance >= c.min_distance && c.distance <= c.max_distance);
}

/// A layout is a promise about where things go: two bulbs placed in
/// different grid cells must be drawn in different places, or a learner
/// can't see that the circuit is parallel at all.
#[test]
fn parallel_bulbs_are_drawn_in_their_own_cells() {
    let (circuit, layout, bulb_a, bulb_b) = parallel_fixture();
    let view = CircuitView::from_layout(circuit, layout);
    assert_ne!(view.part_position(bulb_a), view.part_position(bulb_b));
}

fn parallel_fixture() -> (Circuit, BoardLayout, ComponentId, ComponentId) {
    let mut c = Circuit::new();
    let n0 = c.node();
    let n1 = c.node();
    let n2 = c.node();
    let n3 = c.node();
    let battery = c.add_battery(n1, n0, 6.0);
    let switch = c.add_switch(n1, n2, true);
    let bulb_a = c.add_bulb(n2, n0, 6.0, 3.0);
    let bulb_b = c.add_bulb(n2, n0, 6.0, 3.0);
    let w1 = c.connect(n2, n3);

    use crate::board_layout::{PartPlacement, Rotation, WireRoute};
    let layout = BoardLayout {
        parts: alloc::vec![
            PartPlacement {
                component: battery,
                cell: (0, 0),
                rotation: Rotation::Deg0
            },
            PartPlacement {
                component: switch,
                cell: (1, 0),
                rotation: Rotation::Deg0
            },
            PartPlacement {
                component: bulb_a,
                cell: (2, 0),
                rotation: Rotation::Deg0
            },
            PartPlacement {
                component: bulb_b,
                cell: (2, 2),
                rotation: Rotation::Deg0
            },
        ],
        wires: alloc::vec![WireRoute {
            component: w1,
            points: alloc::vec![(2, 0), (2, 2)]
        }],
    };
    (c, layout, bulb_a, bulb_b)
}

impl CircuitView {
    pub fn part_position(&self, id: ComponentId) -> Vec3 {
        for (i, p) in self.layout.parts.iter().enumerate() {
            if p.component == id {
                if let Component::Bulb { .. } = self.sim.circuit().component(id) {
                    // find which bulb in glasses array
                    // wait, glasses is just in order of bulbs.
                    let mut b_idx = 0;
                    for part in &self.layout.parts {
                        if part.component == id {
                            break;
                        }
                        if let Component::Bulb { .. } = self.sim.circuit().component(part.component)
                        {
                            b_idx += 1;
                        }
                    }
                    return self
                        .scene
                        .node(self.parts.glasses[b_idx])
                        .transform
                        .transform_point(Vec3::ZERO);
                }
            }
        }
        Vec3::ZERO
    }
}
