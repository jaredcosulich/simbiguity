//! Picking: which object is under the cursor?
//!
//! The camera turns a pixel into a [`Ray`]; the ray is tested against every
//! pickable node's world-space bounding box; the nearest hit wins.

use simbiguity_math::{Mat4, Vec3};

/// A half-line: every point `origin + dir · t` for `t ≥ 0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    /// Where the ray starts (the camera's eye, for picking).
    pub origin: Vec3,
    /// Direction, unit length.
    pub dir: Vec3,
}

impl Ray {
    /// The point at distance `t` along the ray.
    pub fn at(&self, t: f64) -> Vec3 {
        self.origin + self.dir * t
    }
}

/// An axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    /// Minimum corner.
    pub min: Vec3,
    /// Maximum corner.
    pub max: Vec3,
}

impl Aabb {
    /// The box that contains nothing; growing it by a point gives that point.
    pub const EMPTY: Aabb = Aabb {
        min: Vec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
        max: Vec3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
    };

    /// A box around a single point.
    pub fn point(p: Vec3) -> Aabb {
        Aabb { min: p, max: p }
    }

    /// `true` for [`Aabb::EMPTY`] (min above max on some axis).
    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z
    }

    /// The box grown to contain `p`.
    pub fn including(self, p: Vec3) -> Aabb {
        Aabb {
            min: Vec3::new(
                self.min.x.min(p.x),
                self.min.y.min(p.y),
                self.min.z.min(p.z),
            ),
            max: Vec3::new(
                self.max.x.max(p.x),
                self.max.y.max(p.y),
                self.max.z.max(p.z),
            ),
        }
    }

    /// The box grown by `margin` on every side.
    pub fn expanded(self, margin: f64) -> Aabb {
        let m = Vec3::new(margin, margin, margin);
        Aabb {
            min: self.min - m,
            max: self.max + m,
        }
    }

    /// The world-space box around this box after `transform`.
    pub fn transformed(&self, transform: &Mat4) -> Aabb {
        let mut out = Aabb::EMPTY;
        for i in 0..8 {
            let corner = Vec3::new(
                if i & 1 == 0 { self.min.x } else { self.max.x },
                if i & 2 == 0 { self.min.y } else { self.max.y },
                if i & 4 == 0 { self.min.z } else { self.max.z },
            );
            out = out.including(transform.transform_point(corner));
        }
        out
    }

    /// Distance along `ray` to where it enters this box (`0` if it starts
    /// inside), or `None` if it misses. The slab method.
    pub fn intersect(&self, ray: &Ray) -> Option<f64> {
        let mut t_near: f64 = 0.0;
        let mut t_far = f64::INFINITY;
        for (o, d, lo, hi) in [
            (ray.origin.x, ray.dir.x, self.min.x, self.max.x),
            (ray.origin.y, ray.dir.y, self.min.y, self.max.y),
            (ray.origin.z, ray.dir.z, self.min.z, self.max.z),
        ] {
            if d == 0.0 {
                if o < lo || o > hi {
                    return None;
                }
                continue;
            }
            let (a, b) = ((lo - o) / d, (hi - o) / d);
            let (a, b) = if a < b { (a, b) } else { (b, a) };
            t_near = t_near.max(a);
            t_far = t_far.min(b);
            if t_near > t_far {
                return None;
            }
        }
        Some(t_near)
    }
}

/// What a pick ray hit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    /// The node's pick tag (see [`crate::Node::pick`]).
    pub tag: u32,
    /// Distance from the ray origin.
    pub distance: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit() -> Aabb {
        Aabb {
            min: Vec3::new(-1.0, -1.0, -1.0),
            max: Vec3::new(1.0, 1.0, 1.0),
        }
    }

    fn ray(origin: Vec3, dir: Vec3) -> Ray {
        Ray {
            origin,
            dir: dir.normalize().unwrap(),
        }
    }

    /// A ray aimed at a box enters it at the near face.
    #[test]
    fn ray_hits_the_near_face() {
        let r = ray(Vec3::new(0.0, 0.0, 5.0), -Vec3::Z);
        assert_eq!(unit().intersect(&r), Some(4.0));
        assert_eq!(r.at(4.0), Vec3::new(0.0, 0.0, 1.0));
    }

    /// A ray that passes beside or points away from the box misses.
    #[test]
    fn ray_misses() {
        assert_eq!(
            unit().intersect(&ray(Vec3::new(3.0, 0.0, 5.0), -Vec3::Z)),
            None
        );
        assert_eq!(
            unit().intersect(&ray(Vec3::new(0.0, 0.0, 5.0), Vec3::Z)),
            None
        );
        let diagonal = ray(Vec3::new(5.0, 0.0, 5.0), Vec3::new(-1.0, 0.0, 0.2));
        assert_eq!(unit().intersect(&diagonal), None);
    }

    /// A ray starting inside the box hits at distance zero.
    #[test]
    fn ray_from_inside_hits_at_zero() {
        assert_eq!(unit().intersect(&ray(Vec3::ZERO, Vec3::X)), Some(0.0));
    }

    /// Empty grows to a point, then to a box holding every point.
    #[test]
    fn boxes_grow_by_points() {
        assert!(Aabb::EMPTY.is_empty());
        let b = Aabb::EMPTY.including(Vec3::X);
        assert_eq!(b, Aabb::point(Vec3::X));
        let b = b.including(Vec3::new(-1.0, 2.0, 0.0));
        assert_eq!(
            (b.min, b.max),
            (Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 2.0, 0.0))
        );
        assert!(!b.is_empty());
    }

    /// Expanding adds a margin on every side.
    #[test]
    fn expanded_adds_margin() {
        let b = Aabb::point(Vec3::ZERO).expanded(0.5);
        assert_eq!(b.min, Vec3::new(-0.5, -0.5, -0.5));
        assert_eq!(b.max, Vec3::new(0.5, 0.5, 0.5));
    }

    /// A transformed box bounds every transformed corner.
    #[test]
    fn transformed_bounds_the_moved_box() {
        let b = unit().transformed(&Mat4::translation(Vec3::new(5.0, 0.0, 0.0)));
        assert_eq!((b.min.x, b.max.x), (4.0, 6.0));
        let r = Mat4::rotation(simbiguity_math::Quat::from_axis_angle(
            Vec3::Y,
            core::f64::consts::FRAC_PI_4,
        ));
        let rb = unit().transformed(&r);
        assert!((rb.max.x - core::f64::consts::SQRT_2).abs() < 1e-9);
    }
}
