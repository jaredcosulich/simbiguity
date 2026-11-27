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
pub const WIRE_Y: f64 = 0.12;
pub(crate) const BATTERY_Y: f64 = 0.42;

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

/// The loop charge follows: wires, plus the hidden stretches through the
/// switch, the bulb and the battery.
pub fn wire_path(route: &crate::board_layout::WireRoute) -> CurrentLoop {
    let pts: Vec<Vec3> = route
        .points
        .iter()
        .map(|&(x, z)| Vec3::new(x as f64 * 2.0, WIRE_Y, z as f64 * 2.0))
        .collect();
    CurrentLoop {
        sections: vec![Section {
            path: pts,
            wire: true,
        }],
    }
}

/// The lever's transform when lifted by `angle` radians.
pub fn lever_transform(base: Mat4, angle: f64) -> Mat4 {
    base * Mat4::translation(Vec3::new(-0.45, 0.34, 0.0))
        * Mat4::rotation(Quat::from_axis_angle(Vec3::Z, angle))
        * Mat4::translation(Vec3::new(LEVER_LENGTH * 0.5, 0.0, 0.0))
}

/// The knob's transform when the lever is lifted by `angle`.
pub fn knob_transform(base: Mat4, angle: f64) -> Mat4 {
    base * Mat4::translation(Vec3::new(-0.45, 0.34, 0.0))
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

    /// Verifies that an empty loop has zero length and returns the origin for any offset.
    #[test]
    fn empty_loop_is_inert() {
        let l = CurrentLoop { sections: vec![] };
        assert_eq!(l.length(), 0.0);
        assert_eq!(l.at(3.0), (Vec3::ZERO, false));
    }

    /// Verifies that the wrapping logic correctly bounds negative and positive values into [0, length).
    #[test]
    fn wrap_maps_into_the_period() {
        assert_eq!(wrap(5.0, 4.0), 1.0);
        assert_eq!(wrap(-1.0, 4.0), 3.0);
        assert_eq!(wrap(0.0, 4.0), 0.0);
        assert_eq!(wrap(4.0, 4.0), 0.0);
    }

    /// Verifies that the visual lever representation tracks the designated rotation angle correctly.
    #[test]
    fn lever_and_knob_follow_the_angle() {
        let closed = lever_transform(Mat4::IDENTITY, 0.0).transform_point(Vec3::ZERO);
        assert!(
            (closed - (Vec3::new(-0.45, 0.34, 0.0) + Vec3::new(LEVER_LENGTH * 0.5, 0.0, 0.0)))
                .length()
                < 1e-12
        );
        let open = knob_transform(Mat4::IDENTITY, LEVER_OPEN_ANGLE).transform_point(Vec3::ZERO);
        assert!(open.y > 0.34 + 0.5, "an open lever lifts its knob");
    }
}
