//! Where everything sits on the board, and the meshes that draw it.
//!
//! The board lies in the XZ plane with +Y up; one unit is about 4 cm.
//! The loop runs battery `+` → switch → bulb → battery `−`, which is also
//! the direction conventional current flows.

use alloc::vec;
use alloc::vec::Vec;

use simbiguity_math::{Mat4, Quat, Vec3};
use simbiguity_render::path::{path_length, point_along};

/// Pick tag of the battery.
pub const PICK_BATTERY: u32 = 1;
/// Pick tag of the switch (base and lever).
pub const PICK_SWITCH: u32 = 2;
/// Pick tag of the bulb.
pub const PICK_BULB: u32 = 3;

/// The switch lever's pivot, on top of the left post.
pub const LEVER_PIVOT: Vec3 = Vec3::new(-0.45, 0.4, -1.8);
/// Lever length, pivot to tip.
pub const LEVER_LENGTH: f64 = 0.95;
/// How far an open lever is lifted, in radians.
pub const LEVER_OPEN_ANGLE: f64 = 0.8;
/// Centre of the bulb's glass.
pub const BULB_CENTRE: Vec3 = Vec3::new(2.6, 1.05, 0.0);

pub(crate) const WIRE_RADIUS: f64 = 0.06;
const WIRE_Y: f64 = 0.12;
pub(crate) const BATTERY_X: f64 = -2.6;
pub(crate) const BATTERY_Y: f64 = 0.42;
pub(crate) const BULB_X: f64 = 2.6;

/// One stretch of the current loop; particles are drawn only on wires.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// The stretch as a polyline.
    pub path: Vec<Vec3>,
    /// `true` on a visible wire; `false` inside a component.
    pub wire: bool,
}

/// The closed path charge follows, battery `+` around to battery `+`.
#[derive(Debug, Clone, PartialEq)]
pub struct CurrentLoop {
    /// Sections in flow order.
    pub sections: Vec<Section>,
}

impl CurrentLoop {
    /// Total loop length.
    pub fn length(&self) -> f64 {
        self.sections.iter().map(|s| path_length(&s.path)).sum()
    }

    /// The point `s` along the loop (wrapped), and whether it is on a wire.
    pub fn at(&self, s: f64) -> (Vec3, bool) {
        let total = self.length();
        let mut left = if total > 0.0 { wrap(s, total) } else { 0.0 };
        for sec in &self.sections {
            let len = path_length(&sec.path);
            if left <= len {
                return (point_along(&sec.path, left), sec.wire);
            }
            left -= len;
        }
        (Vec3::ZERO, false)
    }
}

/// The three wires, in flow order.
pub(crate) fn wires() -> [Vec<Vec3>; 3] {
    let y = WIRE_Y;
    [
        vec![
            Vec3::new(BATTERY_X, BATTERY_Y, -1.12),
            Vec3::new(BATTERY_X, BATTERY_Y, -1.4),
            Vec3::new(BATTERY_X, y, -1.8),
            Vec3::new(-0.53, y, -1.8),
        ],
        vec![
            Vec3::new(0.53, y, -1.8),
            Vec3::new(BULB_X, y, -1.8),
            Vec3::new(BULB_X, y, -0.34),
        ],
        vec![
            Vec3::new(BULB_X, y, 0.34),
            Vec3::new(BULB_X, y, 1.8),
            Vec3::new(BATTERY_X, y, 1.8),
            Vec3::new(BATTERY_X, BATTERY_Y, 1.4),
            Vec3::new(BATTERY_X, BATTERY_Y, 1.0),
        ],
    ]
}

/// The loop charge follows: wires, plus the hidden stretches through the
/// switch, the bulb and the battery.
pub fn current_loop() -> CurrentLoop {
    let [w1, w2, w3] = wires();
    let hidden = |path: Vec<Vec3>| Section { path, wire: false };
    let wire = |path: Vec<Vec3>| Section { path, wire: true };
    CurrentLoop {
        sections: vec![
            wire(w1),
            hidden(vec![
                Vec3::new(-0.53, WIRE_Y, -1.8),
                Vec3::new(0.53, WIRE_Y, -1.8),
            ]),
            wire(w2),
            hidden(vec![
                Vec3::new(BULB_X, WIRE_Y, -0.34),
                Vec3::new(BULB_X, 0.6, 0.0),
                Vec3::new(BULB_X, WIRE_Y, 0.34),
            ]),
            wire(w3),
            hidden(vec![
                Vec3::new(BATTERY_X, BATTERY_Y, 1.0),
                Vec3::new(BATTERY_X, BATTERY_Y, -1.12),
            ]),
        ],
    }
}

/// The lever's transform when lifted by `angle` radians.
pub fn lever_transform(angle: f64) -> Mat4 {
    Mat4::translation(LEVER_PIVOT)
        * Mat4::rotation(Quat::from_axis_angle(Vec3::Z, angle))
        * Mat4::translation(Vec3::new(LEVER_LENGTH * 0.5, 0.0, 0.0))
}

/// The knob's transform when the lever is lifted by `angle`.
pub fn knob_transform(angle: f64) -> Mat4 {
    Mat4::translation(LEVER_PIVOT)
        * Mat4::rotation(Quat::from_axis_angle(Vec3::Z, angle))
        * Mat4::translation(Vec3::new(LEVER_LENGTH, 0.06, 0.0))
}

/// `x` wrapped into `[0, period)`.
pub fn wrap(x: f64, period: f64) -> f64 {
    let r = x % period;
    if r < 0.0 {
        r + period
    } else {
        r
    }
}

/// The `count` evenly spaced charge positions on `path` that are on a
/// visible wire, with the first at `phase` along the loop.
pub fn particle_offsets(path: &CurrentLoop, phase: f64, count: usize) -> Vec<Vec3> {
    let len = path.length();
    (0..count)
        .filter_map(|k| {
            let (pos, on_wire) = path.at(phase + k as f64 * len / count as f64);
            on_wire.then_some(pos)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use simbiguity_render::path::path_length;

    /// The loop is the three wires plus three hidden stretches, and its length is their sum.
    #[test]
    fn loop_length_is_the_sum_of_its_sections() {
        let l = current_loop();
        assert_eq!(l.sections.len(), 6);
        assert_eq!(l.sections.iter().filter(|s| s.wire).count(), 3);
        let sum: f64 = l.sections.iter().map(|s| path_length(&s.path)).sum();
        assert_eq!(l.length(), sum);
        assert!(l.length() > 10.0 && l.length() < 20.0, "{}", l.length());
    }

    /// The loop is closed: each section starts where the previous one ends.
    #[test]
    fn loop_is_closed() {
        let l = current_loop();
        for (i, s) in l.sections.iter().enumerate() {
            let next = &l.sections[(i + 1) % l.sections.len()];
            assert!(
                s.path.last().unwrap().distance(next.path[0]) < 1e-12,
                "gap after {i}"
            );
        }
    }

    /// at wraps around the loop and reports whether a point is on a wire.
    #[test]
    fn at_wraps_and_reports_wires() {
        let l = current_loop();
        let len = l.length();
        assert_eq!(l.at(0.0), (l.sections[0].path[0], true));
        assert_eq!(l.at(len + 0.25), l.at(0.25));
        assert_eq!(l.at(-0.25), l.at(len - 0.25));
        let w1 = path_length(&l.sections[0].path);
        assert!(!l.at(w1 + 0.1).1, "inside the switch is hidden");
    }

    /// An empty loop has no length and returns the origin off-wire.
    #[test]
    fn empty_loop_is_inert() {
        let l = CurrentLoop { sections: vec![] };
        assert_eq!(l.length(), 0.0);
        assert_eq!(l.at(3.0), (Vec3::ZERO, false));
    }

    /// wrap maps any value into the half-open period.
    #[test]
    fn wrap_maps_into_the_period() {
        assert_eq!(wrap(5.0, 4.0), 1.0);
        assert_eq!(wrap(-1.0, 4.0), 3.0);
        assert_eq!(wrap(0.0, 4.0), 0.0);
        assert_eq!(wrap(4.0, 4.0), 0.0);
    }

    /// The lever swings about its pivot, and the knob sits at its tip.
    #[test]
    fn lever_and_knob_follow_the_angle() {
        let closed = lever_transform(0.0).transform_point(Vec3::ZERO);
        assert!(
            (closed - (LEVER_PIVOT + Vec3::new(LEVER_LENGTH * 0.5, 0.0, 0.0))).length() < 1e-12
        );
        let open = knob_transform(LEVER_OPEN_ANGLE).transform_point(Vec3::ZERO);
        assert!(open.y > LEVER_PIVOT.y + 0.5, "an open lever lifts its knob");
    }

    /// Only particles on wires are drawn, every one of them on the loop.
    #[test]
    fn particle_offsets_skip_hidden_stretches() {
        let l = current_loop();
        let all = particle_offsets(&l, 0.0, 36);
        assert!(!all.is_empty() && all.len() < 36);
        assert_eq!(particle_offsets(&l, 0.0, 0), Vec::<Vec3>::new());
        let moved = particle_offsets(&l, 0.3, 36);
        assert_ne!(all, moved, "phase moves the charges");
    }
}
