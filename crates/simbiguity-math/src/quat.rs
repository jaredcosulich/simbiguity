//! Quaternions: the robust way to store a 3D rotation.
//!
//! A rotation by angle θ about a unit axis `n` is the quaternion
//! `(cos θ/2, n·sin θ/2)`. Compared with three Euler angles, quaternions
//! never hit "gimbal lock" (losing a degree of freedom when two axes line up),
//! compose with a single multiply, and interpolate smoothly with [`Quat::slerp`]
//! — which is why cameras, rigid bodies and animation tracks all use them.

use core::ops::{Mul, Neg};

use crate::float;
use crate::vec::Vec3;

/// A quaternion `w + xi + yj + zk`. Rotations are *unit* quaternions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    /// Scalar (real) part: `cos θ/2` for a rotation.
    pub w: f64,
    /// i component.
    pub x: f64,
    /// j component.
    pub y: f64,
    /// k component.
    pub z: f64,
}

impl Default for Quat {
    fn default() -> Quat {
        Quat::IDENTITY
    }
}

impl Quat {
    /// The "no rotation" quaternion.
    pub const IDENTITY: Quat = Quat::new(1.0, 0.0, 0.0, 0.0);

    /// Builds a quaternion from raw components (not normalized).
    pub const fn new(w: f64, x: f64, y: f64, z: f64) -> Quat {
        Quat { w, x, y, z }
    }

    /// Rotation by `angle` radians about `axis` (right-hand rule). The axis
    /// need not be unit length; a zero axis gives the identity.
    pub fn from_axis_angle(axis: Vec3, angle: f64) -> Quat {
        match axis.normalize() {
            Some(n) => {
                let (s, c) = float::sin_cos(angle * 0.5);
                Quat::new(c, n.x * s, n.y * s, n.z * s)
            }
            None => Quat::IDENTITY,
        }
    }

    /// The vector (imaginary) part.
    pub fn xyz(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    /// Four-dimensional dot product; `|dot|` near 1 means nearly the same rotation.
    pub fn dot(self, o: Quat) -> f64 {
        self.w * o.w + self.x * o.x + self.y * o.y + self.z * o.z
    }

    /// Length (norm). Unit quaternions have length 1.
    pub fn length(self) -> f64 {
        float::sqrt(self.dot(self))
    }

    /// Rescales to unit length. Repeated multiplies slowly drift off unit
    /// length through rounding; renormalizing keeps them true rotations.
    /// A zero quaternion gives the identity.
    pub fn normalize(self) -> Quat {
        let len = self.length();
        if len > 0.0 && len.is_finite() {
            Quat::new(self.w / len, self.x / len, self.y / len, self.z / len)
        } else {
            Quat::IDENTITY
        }
    }

    /// The conjugate `(w, −x, −y, −z)`: for a unit quaternion, the inverse rotation.
    pub fn conjugate(self) -> Quat {
        Quat::new(self.w, -self.x, -self.y, -self.z)
    }

    /// Rotates a vector. Equivalent to `q · v · q*`, computed with two cross
    /// products instead of two full quaternion multiplies.
    pub fn rotate(self, v: Vec3) -> Vec3 {
        let u = self.xyz();
        let t = u.cross(v) * 2.0;
        v + t * self.w + u.cross(t)
    }

    /// Spherical linear interpolation: moves from `self` (t = 0) to `o`
    /// (t = 1) at constant angular speed along the shortest arc. Plain lerp
    /// would speed up in the middle and shrink the quaternion.
    pub fn slerp(self, o: Quat, t: f64) -> Quat {
        let mut d = self.dot(o);
        // q and −q are the same rotation; flip to take the short way round.
        let o = if d < 0.0 {
            d = -d;
            -o
        } else {
            o
        };
        let (wa, wb) = slerp_weights(d, t);
        Quat::new(
            self.w * wa + o.w * wb,
            self.x * wa + o.x * wb,
            self.y * wa + o.y * wb,
            self.z * wa + o.z * wb,
        )
        .normalize()
    }
}

/// Composition: `a * b` rotates by `b` first, then by `a`.
impl Mul for Quat {
    type Output = Quat;
    fn mul(self, o: Quat) -> Quat {
        Quat::new(
            self.w * o.w - self.x * o.x - self.y * o.y - self.z * o.z,
            self.w * o.x + self.x * o.w + self.y * o.z - self.z * o.y,
            self.w * o.y - self.x * o.z + self.y * o.w + self.z * o.x,
            self.w * o.z + self.x * o.y - self.y * o.x + self.z * o.w,
        )
    }
}

/// Negates every component. `-q` is the same rotation as `q`.
impl Neg for Quat {
    type Output = Quat;
    fn neg(self) -> Quat {
        Quat::new(-self.w, -self.x, -self.y, -self.z)
    }
}

/// Blend weights `(a, b)` for slerp between two unit quaternions whose dot
/// product is `d ≥ 0`. Nearly identical rotations (sin θ ≈ 0) fall back to
/// plain lerp weights, which `slerp` then renormalizes.
fn slerp_weights(d: f64, t: f64) -> (f64, f64) {
    if d > 0.9995 {
        (1.0 - t, t)
    } else {
        let theta = float::acos(d);
        let s = float::sin(theta);
        (float::sin((1.0 - t) * theta) / s, float::sin(t * theta) / s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

    fn near(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-12
    }

    /// Axis-angle builds unit quaternions that rotate by the right-hand
    /// rule; a zero axis gives the identity.
    #[test]
    fn from_axis_angle_rotates_by_right_hand_rule() {
        let q = Quat::from_axis_angle(Vec3::new(0.0, 0.0, 5.0), FRAC_PI_2);
        assert!((q.length() - 1.0).abs() < 1e-15);
        assert!(near(q.rotate(Vec3::X), Vec3::Y));
        assert_eq!(Quat::from_axis_angle(Vec3::ZERO, 1.0), Quat::IDENTITY);
        assert_eq!(Quat::default(), Quat::IDENTITY);
        assert_eq!(
            Quat::IDENTITY.rotate(Vec3::new(1.0, 2.0, 3.0)),
            Vec3::new(1.0, 2.0, 3.0)
        );
    }

    /// Multiplication composes right-first and the conjugate undoes a
    /// rotation.
    #[test]
    fn mul_composes_and_conjugate_inverts() {
        let a = Quat::from_axis_angle(Vec3::Z, FRAC_PI_2);
        let b = Quat::from_axis_angle(Vec3::X, FRAC_PI_2);
        assert!(near((a * b).rotate(Vec3::Y), a.rotate(b.rotate(Vec3::Y))));
        let v = Vec3::new(0.3, -1.0, 2.0);
        assert!(near(a.conjugate().rotate(a.rotate(v)), v));
        assert_eq!(a.xyz(), Vec3::new(a.x, a.y, a.z));
    }

    /// Normalize restores unit length and maps zero to the identity.
    #[test]
    fn normalize_unit_and_zero() {
        let q = Quat::new(2.0, 0.0, 0.0, 0.0).normalize();
        assert_eq!(q, Quat::IDENTITY);
        assert_eq!(Quat::new(0.0, 0.0, 0.0, 0.0).normalize(), Quat::IDENTITY);
        assert!((Quat::new(1.0, 2.0, 3.0, 4.0).normalize().length() - 1.0).abs() < 1e-15);
    }

    /// Rotating by a unit quaternion never changes a vector's length.
    #[test]
    fn rotate_preserves_length() {
        let q = Quat::from_axis_angle(Vec3::new(0.3, -1.0, 2.0), 2.2);
        let v = Vec3::new(4.0, -1.0, 0.5);
        assert!((q.rotate(v).length() - v.length()).abs() < 1e-12);
    }

    /// Negation flips every component and is the same rotation.
    #[test]
    fn neg_is_same_rotation() {
        let q = Quat::from_axis_angle(Vec3::new(1.0, 1.0, 0.0), 0.9);
        let n = -q;
        assert_eq!(n, Quat::new(-q.w, -q.x, -q.y, -q.z));
        assert!(near(n.rotate(Vec3::Z), q.rotate(Vec3::Z)));
    }

    /// Slerp hits both ends, moves at constant angle, and takes the short
    /// arc even when given the negated target.
    #[test]
    fn slerp_endpoints_midpoint_and_short_arc() {
        let a = Quat::IDENTITY;
        let b = Quat::from_axis_angle(Vec3::Z, FRAC_PI_2);
        assert!((a.slerp(b, 0.0).dot(a) - 1.0).abs() < 1e-15);
        assert!((a.slerp(b, 1.0).dot(b) - 1.0).abs() < 1e-15);
        let mid = a.slerp(b, 0.5);
        assert!(near(
            mid.rotate(Vec3::X),
            Quat::from_axis_angle(Vec3::Z, FRAC_PI_4).rotate(Vec3::X)
        ));
        let mid_neg = a.slerp(-b, 0.5);
        assert!(near(mid_neg.rotate(Vec3::X), mid.rotate(Vec3::X)));
        let tiny = Quat::from_axis_angle(Vec3::Z, 1e-4);
        assert!((a.slerp(tiny, 0.5).length() - 1.0).abs() < 1e-15);
    }

    /// The weights sum to the lerp fallback near d = 1 and follow the
    /// sin-ratio formula otherwise; at θ = π/2 the midpoint weights are √½.
    #[test]
    fn slerp_weights_lerp_fallback_and_sine_ratio() {
        assert_eq!(slerp_weights(0.9999, 0.25), (0.75, 0.25));
        let (wa, wb) = slerp_weights(0.0, 0.5);
        let h = core::f64::consts::FRAC_1_SQRT_2;
        assert!((wa - h).abs() < 1e-15 && (wb - h).abs() < 1e-15);
        let (wa, wb) = slerp_weights((PI / 3.0).cos(), 0.0);
        assert!((wa - 1.0).abs() < 1e-15 && wb.abs() < 1e-15);
    }
}
