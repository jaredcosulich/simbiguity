//! 2D and 3D vectors.
//!
//! A vector is an arrow: a direction and a length. Positions, velocities,
//! forces and surface normals are all vectors. The operations here are the
//! handful every simulation and renderer reaches for.

use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

use crate::float;

macro_rules! impl_vector_ops {
    ($t:ident { $($f:ident),+ }) => {
        impl Add for $t {
            type Output = $t;
            fn add(self, o: $t) -> $t { $t { $($f: self.$f + o.$f),+ } }
        }
        impl Sub for $t {
            type Output = $t;
            fn sub(self, o: $t) -> $t { $t { $($f: self.$f - o.$f),+ } }
        }
        impl Mul<f64> for $t {
            type Output = $t;
            fn mul(self, s: f64) -> $t { $t { $($f: self.$f * s),+ } }
        }
        impl Mul<$t> for f64 {
            type Output = $t;
            fn mul(self, v: $t) -> $t { v * self }
        }
        impl Div<f64> for $t {
            type Output = $t;
            fn div(self, s: f64) -> $t { $t { $($f: self.$f / s),+ } }
        }
        impl Neg for $t {
            type Output = $t;
            fn neg(self) -> $t { $t { $($f: -self.$f),+ } }
        }
        impl AddAssign for $t {
            fn add_assign(&mut self, o: $t) { *self = *self + o; }
        }
        impl SubAssign for $t {
            fn sub_assign(&mut self, o: $t) { *self = *self - o; }
        }
        impl $t {
            /// Dot product: `|a|·|b|·cos θ`. Zero when the vectors are perpendicular.
            pub fn dot(self, o: $t) -> f64 { 0.0 $(+ self.$f * o.$f)+ }
            /// Squared length (cheaper than [`Self::length`]; fine for comparisons).
            pub fn length_squared(self) -> f64 { self.dot(self) }
            /// Length (Euclidean norm).
            pub fn length(self) -> f64 { float::sqrt(self.length_squared()) }
            /// Unit vector in the same direction, or `None` for the zero vector
            /// (which has no direction).
            pub fn normalize(self) -> Option<$t> {
                let len = self.length();
                if len > 0.0 && len.is_finite() { Some(self / len) } else { None }
            }
            /// Linear interpolation: `t = 0` gives `self`, `t = 1` gives `o`.
            pub fn lerp(self, o: $t, t: f64) -> $t { self + (o - self) * t }
            /// Distance between two points.
            pub fn distance(self, o: $t) -> f64 { (o - self).length() }
        }
    };
}

/// A 2D vector.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    /// X component.
    pub x: f64,
    /// Y component.
    pub y: f64,
}

impl Vec2 {
    /// The zero vector.
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    /// Builds a vector from components.
    pub const fn new(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }

    /// The 2D "cross product": the z component of the 3D cross product of
    /// `(x, y, 0)` vectors. Positive when `o` is counter-clockwise from `self`.
    pub fn perp_dot(self, o: Vec2) -> f64 {
        self.x * o.y - self.y * o.x
    }
}

impl_vector_ops!(Vec2 { x, y });

/// A 3D vector.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    /// X component.
    pub x: f64,
    /// Y component.
    pub y: f64,
    /// Z component.
    pub z: f64,
}

impl Vec3 {
    /// The zero vector.
    pub const ZERO: Vec3 = Vec3::new(0.0, 0.0, 0.0);
    /// Unit X axis.
    pub const X: Vec3 = Vec3::new(1.0, 0.0, 0.0);
    /// Unit Y axis (simbiguity treats +Y as "up").
    pub const Y: Vec3 = Vec3::new(0.0, 1.0, 0.0);
    /// Unit Z axis.
    pub const Z: Vec3 = Vec3::new(0.0, 0.0, 1.0);

    /// Builds a vector from components.
    pub const fn new(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3 { x, y, z }
    }

    /// Cross product: a vector perpendicular to both inputs, with length
    /// `|a|·|b|·sin θ`, pointing by the right-hand rule. Torque and surface
    /// normals are cross products.
    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
}

impl_vector_ops!(Vec3 { x, y, z });

#[cfg(test)]
mod tests {
    use super::*;

    /// The macro-generated operators must act componentwise on both types.
    #[test]
    fn arithmetic_is_componentwise() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, -5.0, 6.0);
        assert_eq!(a + b, Vec3::new(5.0, -3.0, 9.0));
        assert_eq!(a - b, Vec3::new(-3.0, 7.0, -3.0));
        assert_eq!(a * 2.0, Vec3::new(2.0, 4.0, 6.0));
        assert_eq!(2.0 * a, a * 2.0);
        assert_eq!(a / 2.0, Vec3::new(0.5, 1.0, 1.5));
        assert_eq!(-a, Vec3::new(-1.0, -2.0, -3.0));
        let mut c = a;
        c += b;
        c -= a;
        assert_eq!(c, b);
        assert_eq!(
            Vec2::new(1.0, 2.0) + Vec2::new(3.0, 4.0),
            Vec2::new(4.0, 6.0)
        );
    }

    /// Dot, length, distance and lerp follow their textbook definitions.
    #[test]
    fn dot_length_distance_lerp() {
        let v = Vec2::new(3.0, 4.0);
        assert_eq!(v.length_squared(), 25.0);
        assert_eq!(v.length(), 5.0);
        assert_eq!(Vec2::ZERO.distance(v), 5.0);
        assert_eq!(Vec3::X.dot(Vec3::Y), 0.0);
        assert_eq!(Vec2::ZERO.lerp(v, 0.5), Vec2::new(1.5, 2.0));
        assert_eq!(Vec3::ZERO.lerp(Vec3::X, 1.0), Vec3::X);
    }

    /// Normalize gives unit vectors and refuses the zero vector.
    #[test]
    fn normalize_unit_or_none() {
        assert_eq!(Vec2::new(0.0, 5.0).normalize(), Some(Vec2::new(0.0, 1.0)));
        assert_eq!(Vec3::ZERO.normalize(), None);
        assert_eq!(Vec3::new(f64::INFINITY, 0.0, 0.0).normalize(), None);
    }

    /// The cross product is perpendicular to both inputs, as the plan's
    /// identity requires.
    #[test]
    fn cross_is_perpendicular_to_both_inputs() {
        let a = Vec3::new(1.5, -2.0, 0.25);
        let b = Vec3::new(-0.5, 3.0, 4.0);
        let c = a.cross(b);
        assert!(c.dot(a).abs() < 1e-12);
        assert!(c.dot(b).abs() < 1e-12);
    }

    /// Cross products follow the right-hand rule; perp_dot is its 2D z part.
    #[test]
    fn cross_and_perp_dot_right_hand_rule() {
        assert_eq!(Vec3::X.cross(Vec3::Y), Vec3::Z);
        assert_eq!(Vec3::Y.cross(Vec3::X), -Vec3::Z);
        assert_eq!(Vec2::new(1.0, 0.0).perp_dot(Vec2::new(0.0, 1.0)), 1.0);
        assert_eq!(Vec2::new(0.0, 1.0).perp_dot(Vec2::new(1.0, 0.0)), -1.0);
    }
}
