//! Blending between two values.

use simbiguity_math::{Quat, Vec2, Vec3};

/// A value that can be blended: `a.lerp(b, 0) == a`, `a.lerp(b, 1) == b`.
///
/// `t` outside `[0, 1]` extrapolates, which is what lets "back" and
/// "elastic" easing overshoot.
pub trait Lerp: Copy {
    /// The blend of `self` and `to` at `t`.
    fn lerp(self, to: Self, t: f64) -> Self;
}

impl Lerp for f64 {
    fn lerp(self, to: f64, t: f64) -> f64 {
        if t == 1.0 {
            to // exact end value, whatever the rounding
        } else {
            self + (to - self) * t
        }
    }
}

impl Lerp for Vec2 {
    fn lerp(self, to: Vec2, t: f64) -> Vec2 {
        Vec2::new(self.x.lerp(to.x, t), self.y.lerp(to.y, t))
    }
}

impl Lerp for Vec3 {
    fn lerp(self, to: Vec3, t: f64) -> Vec3 {
        Vec3::new(
            self.x.lerp(to.x, t),
            self.y.lerp(to.y, t),
            self.z.lerp(to.z, t),
        )
    }
}

/// Rotations blend along the shortest arc (spherical interpolation).
impl Lerp for Quat {
    fn lerp(self, to: Quat, t: f64) -> Quat {
        self.slerp(to, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// f64 blends linearly and lands on the exact end value at t = 1.
    #[test]
    fn f64_lerp_hits_ends_exactly() {
        assert_eq!(0.1f64.lerp(0.7, 0.0), 0.1);
        assert_eq!(0.1f64.lerp(0.7, 1.0), 0.7);
        assert!((2.0f64.lerp(4.0, 0.25) - 2.5).abs() < 1e-15);
    }

    /// t outside the unit interval extrapolates, so overshooting easings work.
    #[test]
    fn f64_lerp_extrapolates() {
        assert_eq!(0.0f64.lerp(10.0, 1.5), 15.0);
        assert_eq!(0.0f64.lerp(10.0, -0.5), -5.0);
    }

    /// Vectors blend per component.
    #[test]
    fn vectors_blend_per_component() {
        assert_eq!(
            Lerp::lerp(Vec2::new(0.0, 2.0), Vec2::new(4.0, 6.0), 0.5),
            Vec2::new(2.0, 4.0)
        );
        assert_eq!(
            Lerp::lerp(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 4.0, 8.0), 0.25),
            Vec3::new(0.5, 1.0, 2.0)
        );
        let end = Vec3::new(0.1, 0.2, 0.3);
        assert_eq!(Lerp::lerp(Vec3::new(9.0, 9.0, 9.0), end, 1.0), end);
    }

    /// Quaternions blend by slerp, so halfway between 0 and 90 degrees is 45 degrees.
    #[test]
    fn quaternions_blend_along_the_arc() {
        let a = Quat::IDENTITY;
        let b = Quat::from_axis_angle(Vec3::Y, core::f64::consts::FRAC_PI_2);
        let mid = a.lerp(b, 0.5);
        let want = Quat::from_axis_angle(Vec3::Y, core::f64::consts::FRAC_PI_4);
        let v = mid.rotate(Vec3::X) - want.rotate(Vec3::X);
        assert!(v.length() < 1e-9);
    }
}
