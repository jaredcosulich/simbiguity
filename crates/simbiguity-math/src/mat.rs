//! Matrices: the fixed 4×4 transform matrix used by cameras and renderers,
//! and a heap-allocated dense matrix for solvers.

use alloc::vec;
use alloc::vec::Vec;
use core::ops::{Index, IndexMut, Mul};

use crate::float;
use crate::quat::Quat;
use crate::vec::Vec3;

/// A 4×4 matrix in **column-major** order (the layout WebGL and OpenGL
/// expect), so `m[col * 4 + row]` is row `row`, column `col`.
///
/// 4×4 matrices act on 3D points written as `(x, y, z, 1)`. The extra
/// coordinate lets a single matrix multiply express translation as well as
/// rotation and scale, and lets a perspective projection divide by depth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    /// Elements, column-major.
    pub m: [f64; 16],
}

impl Default for Mat4 {
    fn default() -> Mat4 {
        Mat4::IDENTITY
    }
}

impl Mat4 {
    /// The identity: leaves every point where it is.
    pub const IDENTITY: Mat4 = Mat4 {
        m: [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ],
    };

    /// Element at `row`, `col`.
    pub fn get(&self, row: usize, col: usize) -> f64 {
        self.m[col * 4 + row]
    }

    /// Swaps rows and columns.
    pub fn transpose(&self) -> Mat4 {
        let mut out = [0.0; 16];
        for (i, v) in out.iter_mut().enumerate() {
            *v = self.m[(i % 4) * 4 + i / 4];
        }
        Mat4 { m: out }
    }

    /// Moves points by `t`.
    pub fn translation(t: Vec3) -> Mat4 {
        let mut r = Mat4::IDENTITY;
        r.m[12] = t.x;
        r.m[13] = t.y;
        r.m[14] = t.z;
        r
    }

    /// Scales each axis independently.
    pub fn scale(s: Vec3) -> Mat4 {
        let mut r = Mat4::IDENTITY;
        r.m[0] = s.x;
        r.m[5] = s.y;
        r.m[10] = s.z;
        r
    }

    /// The rotation a unit quaternion represents. Each column is where that
    /// axis (X, Y, Z) ends up after the rotation.
    pub fn rotation(q: Quat) -> Mat4 {
        let x = q.rotate(Vec3::X);
        let y = q.rotate(Vec3::Y);
        let z = q.rotate(Vec3::Z);
        Mat4 {
            m: [
                x.x, x.y, x.z, 0.0, //
                y.x, y.y, y.z, 0.0, //
                z.x, z.y, z.z, 0.0, //
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    /// OpenGL/WebGL perspective projection (right-handed view space looking
    /// down −Z, clip-space depth in [−1, 1]). `fov_y` is the vertical field of
    /// view in radians. Distant things shrink because the result's `w` is the
    /// point's depth, and the GPU divides by `w`.
    pub fn perspective(fov_y: f64, aspect: f64, near: f64, far: f64) -> Mat4 {
        let f = 1.0 / float::tan(fov_y * 0.5);
        let nf = 1.0 / (near - far);
        let mut m = [0.0; 16];
        m[0] = f / aspect;
        m[5] = f;
        m[10] = (far + near) * nf;
        m[11] = -1.0;
        m[14] = 2.0 * far * near * nf;
        Mat4 { m }
    }

    /// A view matrix for a camera at `eye` looking at `target`, with `up`
    /// roughly upward. Returns the identity if `eye == target` or `up` is
    /// parallel to the view direction (no unique orientation exists).
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Mat4 {
        let Some(f) = (target - eye).normalize() else {
            return Mat4::IDENTITY;
        };
        let Some(s) = f.cross(up).normalize() else {
            return Mat4::IDENTITY;
        };
        let u = s.cross(f);
        Mat4 {
            m: [
                s.x,
                u.x,
                -f.x,
                0.0,
                s.y,
                u.y,
                -f.y,
                0.0,
                s.z,
                u.z,
                -f.z,
                0.0,
                -s.dot(eye),
                -u.dot(eye),
                f.dot(eye),
                1.0,
            ],
        }
    }

    /// Transforms a point (`w = 1`), dividing by the resulting `w`.
    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        let w = self.row_dot(3, p, 1.0);
        Vec3::new(
            self.row_dot(0, p, 1.0) / w,
            self.row_dot(1, p, 1.0) / w,
            self.row_dot(2, p, 1.0) / w,
        )
    }

    /// Transforms a direction (`w = 0`): rotation and scale apply,
    /// translation does not.
    pub fn transform_vector(&self, v: Vec3) -> Vec3 {
        Vec3::new(
            self.row_dot(0, v, 0.0),
            self.row_dot(1, v, 0.0),
            self.row_dot(2, v, 0.0),
        )
    }

    /// Row `row` of this matrix dotted with the 4-vector `(v, w)`.
    fn row_dot(&self, row: usize, v: Vec3, w: f64) -> f64 {
        let m = &self.m;
        m[row] * v.x + m[4 + row] * v.y + m[8 + row] * v.z + m[12 + row] * w
    }

    /// The inverse matrix (undoes this transform), or `None` when the
    /// matrix is singular — e.g. a scale of zero squashes space flat and
    /// cannot be undone. Uses the adjugate (cofactor) formula.
    pub fn inverse(&self) -> Option<Mat4> {
        let a = &self.m;
        let b00 = a[0] * a[5] - a[1] * a[4];
        let b01 = a[0] * a[6] - a[2] * a[4];
        let b02 = a[0] * a[7] - a[3] * a[4];
        let b03 = a[1] * a[6] - a[2] * a[5];
        let b04 = a[1] * a[7] - a[3] * a[5];
        let b05 = a[2] * a[7] - a[3] * a[6];
        let b06 = a[8] * a[13] - a[9] * a[12];
        let b07 = a[8] * a[14] - a[10] * a[12];
        let b08 = a[8] * a[15] - a[11] * a[12];
        let b09 = a[9] * a[14] - a[10] * a[13];
        let b10 = a[9] * a[15] - a[11] * a[13];
        let b11 = a[10] * a[15] - a[11] * a[14];
        let det = b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06;
        if det == 0.0 || !det.is_finite() {
            return None;
        }
        let d = 1.0 / det;
        Some(Mat4 {
            m: [
                (a[5] * b11 - a[6] * b10 + a[7] * b09) * d,
                (a[2] * b10 - a[1] * b11 - a[3] * b09) * d,
                (a[13] * b05 - a[14] * b04 + a[15] * b03) * d,
                (a[10] * b04 - a[9] * b05 - a[11] * b03) * d,
                (a[6] * b08 - a[4] * b11 - a[7] * b07) * d,
                (a[0] * b11 - a[2] * b08 + a[3] * b07) * d,
                (a[14] * b02 - a[12] * b05 - a[15] * b01) * d,
                (a[8] * b05 - a[10] * b02 + a[11] * b01) * d,
                (a[4] * b10 - a[5] * b08 + a[7] * b06) * d,
                (a[1] * b08 - a[0] * b10 - a[3] * b06) * d,
                (a[12] * b04 - a[13] * b02 + a[15] * b00) * d,
                (a[9] * b02 - a[8] * b04 - a[11] * b00) * d,
                (a[5] * b07 - a[4] * b09 - a[6] * b06) * d,
                (a[0] * b09 - a[1] * b07 + a[2] * b06) * d,
                (a[13] * b01 - a[12] * b03 - a[14] * b00) * d,
                (a[8] * b03 - a[9] * b01 + a[10] * b00) * d,
            ],
        })
    }
}

/// Composition: `a * b` applies `b` first, then `a`.
impl Mul for Mat4 {
    type Output = Mat4;
    fn mul(self, o: Mat4) -> Mat4 {
        let mut m = [0.0; 16];
        for col in 0..4 {
            for row in 0..4 {
                m[col * 4 + row] = (0..4).map(|k| self.get(row, k) * o.get(k, col)).sum();
            }
        }
        Mat4 { m }
    }
}

/// A heap-allocated `rows × cols` matrix in **row-major** order, sized at
/// runtime. Solvers use it: a circuit with `n` unknowns builds an `n × n`
/// system whose size is only known once the circuit is.
#[derive(Debug, Clone, PartialEq)]
pub struct DenseMatrix {
    rows: usize,
    cols: usize,
    data: Vec<f64>,
}

impl DenseMatrix {
    /// A `rows × cols` matrix of zeros.
    pub fn zeros(rows: usize, cols: usize) -> DenseMatrix {
        DenseMatrix {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }

    /// The `n × n` identity.
    pub fn identity(n: usize) -> DenseMatrix {
        let mut m = DenseMatrix::zeros(n, n);
        for i in 0..n {
            m[(i, i)] = 1.0;
        }
        m
    }

    /// Builds from row-major data; `None` if `data.len() != rows * cols`.
    pub fn from_row_major(rows: usize, cols: usize, data: Vec<f64>) -> Option<DenseMatrix> {
        (data.len() == rows * cols).then_some(DenseMatrix { rows, cols, data })
    }

    /// Number of rows.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Number of columns.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Row `r` as a slice.
    pub fn row(&self, r: usize) -> &[f64] {
        &self.data[r * self.cols..(r + 1) * self.cols]
    }

    /// Swaps two rows in place.
    pub fn swap_rows(&mut self, a: usize, b: usize) {
        if a == b {
            return;
        }
        for c in 0..self.cols {
            self.data.swap(a * self.cols + c, b * self.cols + c);
        }
    }

    /// Matrix–vector product `A·x`; `None` if `x.len() != cols`.
    pub fn mul_vec(&self, x: &[f64]) -> Option<Vec<f64>> {
        (x.len() == self.cols).then(|| {
            (0..self.rows)
                .map(|r| self.row(r).iter().zip(x).map(|(a, b)| a * b).sum())
                .collect()
        })
    }
}

impl DenseMatrix {
    /// Position of `(r, c)` in `data`.
    ///
    /// # Panics
    /// If `r` or `c` is out of range (a column overflow would otherwise
    /// silently read the next row).
    fn offset(&self, r: usize, c: usize) -> usize {
        assert!(
            r < self.rows && c < self.cols,
            "DenseMatrix index out of bounds"
        );
        r * self.cols + c
    }
}

impl Index<(usize, usize)> for DenseMatrix {
    type Output = f64;
    fn index(&self, (r, c): (usize, usize)) -> &f64 {
        &self.data[self.offset(r, c)]
    }
}

impl IndexMut<(usize, usize)> for DenseMatrix {
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut f64 {
        let i = self.offset(r, c);
        &mut self.data[i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::FRAC_PI_2;

    fn near(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-12
    }

    fn mat_near(a: &Mat4, b: &Mat4) -> bool {
        a.m.iter().zip(&b.m).all(|(x, y)| (x - y).abs() < 1e-12)
    }

    /// The identity is the default and leaves points unchanged; element
    /// access follows column-major layout.
    #[test]
    fn identity_default_and_get_layout() {
        assert_eq!(Mat4::default(), Mat4::IDENTITY);
        let p = Vec3::new(1.0, -2.0, 3.0);
        assert_eq!(Mat4::IDENTITY.transform_point(p), p);
        let t = Mat4::translation(Vec3::new(5.0, 6.0, 7.0));
        assert_eq!(t.get(0, 3), 5.0);
        assert_eq!(t.get(3, 0), 0.0);
    }

    /// Translation moves points but not directions; scale stretches both.
    #[test]
    fn translation_and_scale_on_points_and_vectors() {
        let t = Mat4::translation(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(t.transform_point(Vec3::ZERO), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(t.transform_vector(Vec3::X), Vec3::X);
        let s = Mat4::scale(Vec3::new(2.0, 3.0, 4.0));
        assert_eq!(
            s.transform_point(Vec3::new(1.0, 1.0, 1.0)),
            Vec3::new(2.0, 3.0, 4.0)
        );
        assert_eq!(s.transform_vector(Vec3::Y), Vec3::new(0.0, 3.0, 0.0));
    }

    /// The row helper must dot the right column-major entries and include
    /// the translation column only in proportion to w.
    #[test]
    fn row_dot_reads_column_major_rows() {
        let t = Mat4::translation(Vec3::new(10.0, 20.0, 30.0));
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(t.row_dot(0, v, 1.0), 11.0);
        assert_eq!(t.row_dot(1, v, 0.0), 2.0);
        assert_eq!(t.row_dot(2, v, 2.0), 63.0);
        assert_eq!(t.row_dot(3, v, 1.0), 1.0);
    }

    /// The rotation matrix must agree with rotating by the quaternion.
    #[test]
    fn rotation_matches_quaternion() {
        let q = Quat::from_axis_angle(Vec3::new(1.0, 2.0, 3.0), 0.7);
        let r = Mat4::rotation(q);
        let v = Vec3::new(-1.0, 0.5, 2.0);
        assert!(near(r.transform_vector(v), q.rotate(v)));
        assert!(near(
            Mat4::rotation(Quat::from_axis_angle(Vec3::Y, FRAC_PI_2)).transform_vector(Vec3::X),
            -Vec3::Z
        ));
    }

    /// Composition applies the right operand first, and transpose swaps
    /// rows and columns.
    #[test]
    fn mul_order_and_transpose() {
        let t = Mat4::translation(Vec3::new(1.0, 0.0, 0.0));
        let s = Mat4::scale(Vec3::new(2.0, 2.0, 2.0));
        assert_eq!((t * s).transform_point(Vec3::X), Vec3::new(3.0, 0.0, 0.0));
        assert_eq!((s * t).transform_point(Vec3::X), Vec3::new(4.0, 0.0, 0.0));
        assert_eq!(t.transpose().get(3, 0), 1.0);
        assert_eq!(t.transpose().transpose(), t);
    }

    /// The inverse must undo a general transform and refuse singular input.
    #[test]
    fn inverse_undoes_and_rejects_singular() {
        let a = Mat4::translation(Vec3::new(1.0, -2.0, 3.0))
            * Mat4::rotation(Quat::from_axis_angle(Vec3::new(0.0, 1.0, 1.0), 1.1))
            * Mat4::scale(Vec3::new(2.0, 0.5, 3.0));
        let inv = a.inverse().unwrap();
        assert!(mat_near(&(a * inv), &Mat4::IDENTITY));
        assert!(Mat4::scale(Vec3::new(1.0, 0.0, 1.0)).inverse().is_none());
    }

    /// Perspective puts the near and far planes at clip depth −1 and +1 and
    /// shrinks distant points; look_at maps the eye to the origin and the
    /// target onto −Z, returning identity for degenerate input.
    #[test]
    fn perspective_and_look_at() {
        let p = Mat4::perspective(FRAC_PI_2, 2.0, 1.0, 10.0);
        assert!((p.transform_point(Vec3::new(0.0, 0.0, -1.0)).z + 1.0).abs() < 1e-12);
        assert!((p.transform_point(Vec3::new(0.0, 0.0, -10.0)).z - 1.0).abs() < 1e-12);
        let near_x = p.transform_point(Vec3::new(1.0, 0.0, -2.0)).x;
        let far_x = p.transform_point(Vec3::new(1.0, 0.0, -4.0)).x;
        assert!(far_x < near_x);
        let eye = Vec3::new(0.0, 0.0, 5.0);
        let v = Mat4::look_at(eye, Vec3::ZERO, Vec3::Y);
        assert!(near(v.transform_point(eye), Vec3::ZERO));
        assert!(near(
            v.transform_point(Vec3::ZERO),
            Vec3::new(0.0, 0.0, -5.0)
        ));
        assert_eq!(Mat4::look_at(eye, eye, Vec3::Y), Mat4::IDENTITY);
        assert_eq!(Mat4::look_at(Vec3::ZERO, Vec3::Y, Vec3::Y), Mat4::IDENTITY);
    }

    /// Dense constructors, row access, row swaps and mat-vec products.
    #[test]
    fn dense_matrix_basics() {
        assert!(DenseMatrix::from_row_major(2, 2, alloc::vec![1.0; 3]).is_none());
        let mut a =
            DenseMatrix::from_row_major(2, 3, alloc::vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
        assert_eq!((a.rows(), a.cols()), (2, 3));
        assert_eq!(a.mul_vec(&[1.0, 0.0, 1.0]), Some(alloc::vec![4.0, 10.0]));
        assert_eq!(a.mul_vec(&[1.0]), None);
        a.swap_rows(0, 1);
        assert_eq!(a.row(0), &[4.0, 5.0, 6.0]);
        a.swap_rows(1, 1);
        assert_eq!(a.row(1), &[1.0, 2.0, 3.0]);
        let id = DenseMatrix::identity(3);
        assert_eq!(id[(1, 1)], 1.0);
        assert_eq!(id[(0, 1)], 0.0);
    }

    /// The offset helper maps row-major positions and IndexMut writes there.
    #[test]
    fn offset_maps_row_major_positions() {
        let mut a = DenseMatrix::zeros(2, 3);
        assert_eq!(a.offset(0, 0), 0);
        assert_eq!(a.offset(1, 2), 5);
        a[(1, 0)] = 7.0;
        assert_eq!(a.row(1), &[7.0, 0.0, 0.0]);
    }

    /// A column past the end must panic instead of reading the next row.
    #[test]
    #[should_panic(expected = "DenseMatrix index out of bounds")]
    fn offset_rejects_column_overflow() {
        let a = DenseMatrix::zeros(2, 2);
        let _ = a[(0, 2)];
    }
}
