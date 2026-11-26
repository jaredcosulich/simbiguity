//! Triangle meshes and the primitives every scene is built from.
//!
//! Vertices are interleaved `[px, py, pz, nx, ny, nz]` (`f32`, stride 24
//! bytes); indices are `u32` triangles wound **counter-clockwise** when seen
//! from outside, the WebGL default front face.

use alloc::vec::Vec;

use simbiguity_math::{float, Vec3};

use crate::pick::Aabb;

/// `f32`s per vertex: position then normal.
pub const VERTEX_STRIDE: usize = 6;

const TAU: f64 = core::f64::consts::TAU;
const PI: f64 = core::f64::consts::PI;

/// An indexed triangle mesh.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mesh {
    /// Interleaved positions and normals, [`VERTEX_STRIDE`] floats each.
    pub vertices: Vec<f32>,
    /// Triangle corners, three per triangle.
    pub indices: Vec<u32>,
}

impl Mesh {
    /// Number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len() / VERTEX_STRIDE
    }

    /// Number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Position of vertex `i`.
    pub fn position(&self, i: usize) -> Vec3 {
        let v = &self.vertices[i * VERTEX_STRIDE..];
        Vec3::new(f64::from(v[0]), f64::from(v[1]), f64::from(v[2]))
    }

    /// Normal of vertex `i`.
    pub fn normal(&self, i: usize) -> Vec3 {
        let v = &self.vertices[i * VERTEX_STRIDE + 3..];
        Vec3::new(f64::from(v[0]), f64::from(v[1]), f64::from(v[2]))
    }

    /// The smallest box holding every vertex (empty mesh → a point at 0).
    pub fn bounds(&self) -> Aabb {
        let mut b = Aabb::EMPTY;
        for i in 0..self.vertex_count() {
            b = b.including(self.position(i));
        }
        if b.is_empty() {
            Aabb::point(Vec3::ZERO)
        } else {
            b
        }
    }

    fn push(&mut self, p: Vec3, n: Vec3) -> u32 {
        let i = self.vertex_count() as u32;
        self.vertices.extend_from_slice(&[
            p.x as f32, p.y as f32, p.z as f32, n.x as f32, n.y as f32, n.z as f32,
        ]);
        i
    }

    fn tri(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend_from_slice(&[a, b, c]);
    }

    /// Two triangles for the quad `a b c d` (counter-clockwise).
    fn quad(&mut self, a: u32, b: u32, c: u32, d: u32) {
        self.tri(a, b, c);
        self.tri(a, c, d);
    }

    /// A grid of `rows × cols` vertices (already pushed from `base`, row by
    /// row) stitched into quads. `flip` reverses the winding.
    fn stitch(&mut self, base: u32, rows: u32, cols: u32, flip: bool) {
        for r in 0..rows - 1 {
            for c in 0..cols - 1 {
                let a = base + r * cols + c;
                let b = a + cols;
                if flip {
                    self.quad(a, a + 1, b + 1, b);
                } else {
                    self.quad(a, b, b + 1, a + 1);
                }
            }
        }
    }

    /// An axis-aligned box of `size`, centred on the origin. Each face has
    /// its own four vertices so its normal is flat.
    pub fn cuboid(size: Vec3) -> Mesh {
        let h = size * 0.5;
        let mut m = Mesh::default();
        // (normal, u axis, v axis) with u × v = normal, so the quad is CCW.
        let faces = [
            (Vec3::X, Vec3::new(0.0, 0.0, -1.0), Vec3::Y),
            (-Vec3::X, Vec3::Z, Vec3::Y),
            (Vec3::Y, Vec3::X, Vec3::new(0.0, 0.0, -1.0)),
            (-Vec3::Y, Vec3::X, Vec3::Z),
            (Vec3::Z, Vec3::X, Vec3::Y),
            (-Vec3::Z, -Vec3::X, Vec3::Y),
        ];
        for (n, u, v) in faces {
            let c = mul(n, h);
            let (u, v) = (mul(u, h), mul(v, h));
            let a = m.push(c - u - v, n);
            let b = m.push(c + u - v, n);
            let cc = m.push(c + u + v, n);
            let d = m.push(c - u + v, n);
            m.quad(a, b, cc, d);
        }
        m
    }

    /// A closed cylinder along +Y, centred on the origin.
    pub fn cylinder(radius: f64, height: f64, segments: u32) -> Mesh {
        let segments = segments.max(3);
        let mut m = Mesh::default();
        let hy = height * 0.5;
        // Side: two rows of segments + 1 (the seam is duplicated for normals).
        for y in [-hy, hy] {
            for i in 0..=segments {
                let (s, c) = float::sin_cos(TAU * f64::from(i) / f64::from(segments));
                m.push(Vec3::new(radius * s, y, radius * c), Vec3::new(s, 0.0, c));
            }
        }
        m.stitch(0, 2, segments + 1, true);
        // Caps: a centre vertex and a ring, facing ±Y.
        for (y, n) in [(hy, Vec3::Y), (-hy, -Vec3::Y)] {
            let centre = m.push(Vec3::new(0.0, y, 0.0), n);
            for i in 0..segments {
                let (s, c) = float::sin_cos(TAU * f64::from(i) / f64::from(segments));
                m.push(Vec3::new(radius * s, y, radius * c), n);
            }
            for i in 0..segments {
                let a = centre + 1 + i;
                let b = centre + 1 + (i + 1) % segments;
                if n.y > 0.0 {
                    m.tri(centre, a, b);
                } else {
                    m.tri(centre, b, a);
                }
            }
        }
        m
    }

    /// A UV sphere centred on the origin.
    pub fn sphere(radius: f64, rings: u32, segments: u32) -> Mesh {
        let mut m = Mesh::default();
        let (rings, segments) = (rings.max(2), segments.max(3));
        for r in 0..=rings {
            let phi = PI * f64::from(r) / f64::from(rings);
            m.ring(phi, radius, 0.0, segments);
        }
        m.stitch(0, rings + 1, segments + 1, false);
        m
    }

    /// A capsule along +Y: a cylinder of `length` with hemispherical ends,
    /// centred on the origin (total height `length + 2·radius`).
    pub fn capsule(radius: f64, length: f64, rings: u32, segments: u32) -> Mesh {
        let mut m = Mesh::default();
        let half = rings.max(1);
        let segments = segments.max(3);
        let hy = length * 0.5;
        // Top hemisphere down to the equator, then the equator again (shifted
        // down) to the bottom pole: the stitch between the two equators is
        // the cylinder.
        for r in 0..=half {
            m.ring(
                0.5 * PI * f64::from(r) / f64::from(half),
                radius,
                hy,
                segments,
            );
        }
        for r in 0..=half {
            m.ring(
                0.5 * PI * (1.0 + f64::from(r) / f64::from(half)),
                radius,
                -hy,
                segments,
            );
        }
        m.stitch(0, 2 * (half + 1), segments + 1, false);
        m
    }

    /// One latitude ring of a sphere at polar angle `phi`, lifted by `dy`.
    fn ring(&mut self, phi: f64, radius: f64, dy: f64, segments: u32) {
        let (sp, cp) = float::sin_cos(phi);
        for i in 0..=segments {
            let (st, ct) = float::sin_cos(TAU * f64::from(i) / f64::from(segments));
            let n = Vec3::new(sp * st, cp, sp * ct);
            self.push(n * radius + Vec3::new(0.0, dy, 0.0), n);
        }
    }

    /// A tube of `radius` following `path` (open ends). Rings are oriented
    /// by parallel transport, so the tube does not twist around bends.
    /// Fewer than two distinct points give an empty mesh.
    pub fn tube(path: &[Vec3], radius: f64, segments: u32) -> Mesh {
        let mut pts: Vec<Vec3> = Vec::with_capacity(path.len());
        for &p in path {
            if pts.last().map_or(true, |&q: &Vec3| q.distance(p) > 1e-9) {
                pts.push(p);
            }
        }
        let mut m = Mesh::default();
        if pts.len() < 2 {
            return m;
        }
        let segments = segments.max(3);
        let n = pts.len();
        let tangent = |i: usize| -> Vec3 {
            let d = if i == 0 {
                pts[1] - pts[0]
            } else if i == n - 1 {
                pts[n - 1] - pts[n - 2]
            } else {
                // Average the two directions so corners are mitred.
                let a = (pts[i] - pts[i - 1]).normalize().unwrap_or(Vec3::X);
                let b = (pts[i + 1] - pts[i]).normalize().unwrap_or(Vec3::X);
                a + b
            };
            d.normalize().unwrap_or(Vec3::X)
        };
        let t0 = tangent(0);
        let seed = if t0.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
        let mut normal = t0.cross(seed).normalize().unwrap_or(Vec3::Z);
        let mut prev_t = t0;
        for (i, &p) in pts.iter().enumerate() {
            let t = tangent(i);
            // Parallel transport: rotate the frame by the change in tangent.
            normal = (normal - t * normal.dot(t))
                .normalize()
                .unwrap_or_else(|| prev_t.cross(seed).normalize().unwrap_or(Vec3::Z));
            prev_t = t;
            let binormal = t.cross(normal);
            // A mitred corner's ring lies in the bisecting plane; stretching
            // it along the bend by 1 / cos(half-angle) keeps the tube's
            // radius on both legs.
            let (bend, stretch) = if i > 0 && i < n - 1 {
                let a = (pts[i] - pts[i - 1]).normalize().unwrap_or(t);
                let bend = (a - t * a.dot(t)).normalize().unwrap_or(Vec3::ZERO);
                (bend, 1.0 / a.dot(t).max(0.5))
            } else {
                (Vec3::ZERO, 1.0)
            };
            for k in 0..=segments {
                let (s, c) = float::sin_cos(TAU * f64::from(k) / f64::from(segments));
                let dir = normal * c + binormal * s;
                let off = dir + bend * (dir.dot(bend) * (stretch - 1.0));
                m.push(p + off * radius, dir);
            }
        }
        m.stitch(0, n as u32, segments + 1, true);
        m
    }
}

fn mul(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x * b.x, a.y * b.y, a.z * b.z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn every_primitive() -> Vec<(&'static str, Mesh)> {
        let bend = [
            Vec3::ZERO,
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 2.0),
            Vec3::new(2.0, 2.0, 2.0),
        ];
        vec![
            ("cuboid", Mesh::cuboid(Vec3::new(2.0, 1.0, 3.0))),
            ("cylinder", Mesh::cylinder(0.5, 2.0, 16)),
            ("sphere", Mesh::sphere(1.0, 8, 12)),
            ("capsule", Mesh::capsule(0.5, 1.0, 4, 12)),
            ("tube", Mesh::tube(&bend, 0.1, 8)),
        ]
    }

    /// Vertex and triangle counts follow from the primitive's resolution.
    #[test]
    fn counts_match_the_resolution() {
        let b = Mesh::cuboid(Vec3::new(1.0, 1.0, 1.0));
        assert_eq!((b.vertex_count(), b.triangle_count()), (24, 12));
        let c = Mesh::cylinder(1.0, 1.0, 16);
        assert_eq!(c.vertex_count(), 2 * 17 + 2 * 17);
        assert_eq!(c.triangle_count(), 2 * 16 + 2 * 16);
        let s = Mesh::sphere(1.0, 8, 12);
        assert_eq!((s.vertex_count(), s.triangle_count()), (9 * 13, 2 * 8 * 12));
        let t = Mesh::tube(&[Vec3::ZERO, Vec3::X, Vec3::new(1.0, 1.0, 0.0)], 0.1, 6);
        assert_eq!((t.vertex_count(), t.triangle_count()), (3 * 7, 2 * 2 * 6));
    }

    /// Every vertex normal is unit length and every index is in range.
    #[test]
    fn normals_are_unit_and_indices_in_range() {
        for (name, m) in every_primitive() {
            for i in 0..m.vertex_count() {
                assert!(
                    (m.normal(i).length() - 1.0).abs() < 1e-5,
                    "{name} vertex {i}"
                );
            }
            assert!(
                m.indices.iter().all(|&i| (i as usize) < m.vertex_count()),
                "{name}"
            );
            assert_eq!(m.indices.len() % 3, 0, "{name}");
        }
    }

    /// Every non-degenerate triangle is wound counter-clockwise seen from outside.
    #[test]
    fn triangles_wind_counter_clockwise_from_outside() {
        for (name, m) in every_primitive() {
            for t in m.indices.chunks(3) {
                let [a, b, c] = [t[0], t[1], t[2]].map(|i| i as usize);
                let face = (m.position(b) - m.position(a)).cross(m.position(c) - m.position(a));
                if face.length() < 1e-9 {
                    continue; // a pole's zero-area sliver
                }
                let n = m.normal(a) + m.normal(b) + m.normal(c);
                assert!(face.dot(n) > 0.0, "{name} triangle {t:?} faces inward");
            }
        }
    }

    /// Primitives are centred on the origin with the size asked for.
    #[test]
    fn bounds_match_the_requested_size() {
        let b = Mesh::cuboid(Vec3::new(2.0, 1.0, 3.0)).bounds();
        assert_eq!(
            (b.min, b.max),
            (Vec3::new(-1.0, -0.5, -1.5), Vec3::new(1.0, 0.5, 1.5))
        );
        let c = Mesh::capsule(0.5, 1.0, 4, 12).bounds();
        assert!((c.max.y - 1.0).abs() < 1e-6 && (c.min.y + 1.0).abs() < 1e-6);
        let s = Mesh::sphere(2.0, 8, 12).bounds();
        assert!((s.max.y - 2.0).abs() < 1e-6);
    }

    /// Every ring of a tube stays its radius away from the path it follows.
    #[test]
    fn tube_follows_its_path() {
        let path = [
            Vec3::ZERO,
            Vec3::new(3.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 3.0),
        ];
        let m = Mesh::tube(&path, 0.2, 8);
        for i in 0..m.vertex_count() {
            let p = m.position(i);
            let d = path
                .windows(2)
                .map(|w| {
                    // Distance to the leg's whole line: a mitred corner's
                    // outer vertices sit past the leg's end, on its line.
                    let (a, b) = (w[0], w[1]);
                    let t = (p - a).dot(b - a) / (b - a).length_squared();
                    p.distance(a.lerp(b, t))
                })
                .fold(f64::INFINITY, f64::min);
            assert!((d - 0.2).abs() < 1e-5, "vertex {i} is {d} from the path");
        }
    }

    /// A tube needs two distinct points; duplicates are dropped and too few points give an empty mesh.
    #[test]
    fn degenerate_tubes_are_empty() {
        assert_eq!(Mesh::tube(&[], 0.1, 8), Mesh::default());
        assert_eq!(Mesh::tube(&[Vec3::X, Vec3::X], 0.1, 8), Mesh::default());
        let dup = Mesh::tube(&[Vec3::ZERO, Vec3::ZERO, Vec3::X], 0.1, 3);
        assert_eq!(dup.vertex_count(), 2 * 4);
    }

    /// An empty mesh has a point box at the origin.
    #[test]
    fn empty_mesh_bounds_are_a_point() {
        let b = Mesh::default().bounds();
        assert_eq!((b.min, b.max), (Vec3::ZERO, Vec3::ZERO));
    }

    /// Low resolutions are raised to the minimum that still makes a solid.
    #[test]
    fn resolutions_have_a_floor() {
        assert_eq!(Mesh::cylinder(1.0, 1.0, 0), Mesh::cylinder(1.0, 1.0, 3));
        assert_eq!(Mesh::sphere(1.0, 0, 0), Mesh::sphere(1.0, 2, 3));
        assert_eq!(Mesh::capsule(1.0, 1.0, 0, 0), Mesh::capsule(1.0, 1.0, 1, 3));
    }
}
