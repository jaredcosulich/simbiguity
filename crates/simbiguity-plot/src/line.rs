use simbiguity_render::Mesh;

use simbiguity_math::float::sqrt;

/// Builds a line mesh using screen-space normal extrusion with mitre joins.
pub fn build_line_mesh(points: &[(f64, f64)], width: f64) -> Mesh {
    let mut m = Mesh::default();
    let n_points = points.len();
    if n_points < 2 {
        return m;
    }

    let half_w = (width / 2.0) as f32;

    for i in 0..n_points {
        let (x, y) = points[i];

        let (dx, dy) = if i == 0 {
            let (nx, ny) = points[1];
            (nx - x, ny - y)
        } else if i == n_points - 1 {
            let (px, py) = points[i - 1];
            (x - px, y - py)
        } else {
            let (px, py) = points[i - 1];
            let (nx, ny) = points[i + 1];

            let d1x = x - px;
            let d1y = y - py;
            let l1 = sqrt(d1x * d1x + d1y * d1y);
            let (n1x, n1y) = if l1 > 0.0 {
                (d1x / l1, d1y / l1)
            } else {
                (0.0, 0.0)
            };

            let d2x = nx - x;
            let d2y = ny - y;
            let l2 = sqrt(d2x * d2x + d2y * d2y);
            let (n2x, n2y) = if l2 > 0.0 {
                (d2x / l2, d2y / l2)
            } else {
                (0.0, 0.0)
            };

            let tx = n1x + n2x;
            let ty = n1y + n2y;
            let tl = sqrt(tx * tx + ty * ty);
            if tl > 0.0 {
                (tx / tl, ty / tl)
            } else {
                (n1x, n1y)
            }
        };

        let l = sqrt(dx * dx + dy * dy);
        let (nx, ny) = if l > 0.0 {
            (-dy / l, dx / l)
        } else {
            (0.0, 1.0)
        };

        let miter_len = if i > 0 && i < n_points - 1 {
            let (px, py) = points[i - 1];
            let d1x = x - px;
            let d1y = y - py;
            let l1 = sqrt(d1x * d1x + d1y * d1y);
            let (nd1x, nd1y) = if l1 > 0.0 {
                (d1x / l1, d1y / l1)
            } else {
                (0.0, 0.0)
            };

            let dot = nx * (-nd1y) + ny * nd1x;
            if dot.abs() > 0.001 {
                let miter = half_w as f64 / dot;
                miter.clamp(-half_w as f64 * 5.0, half_w as f64 * 5.0)
            } else {
                half_w as f64
            }
        } else {
            half_w as f64
        };

        let off_x = (nx * miter_len) as f32;
        let off_y = (ny * miter_len) as f32;

        m.vertices.extend_from_slice(&[
            (x as f32) + off_x,
            (y as f32) + off_y,
            0.0,
            nx as f32,
            ny as f32,
            1.0,
        ]);
        m.vertices.extend_from_slice(&[
            (x as f32) - off_x,
            (y as f32) - off_y,
            0.0,
            -nx as f32,
            -ny as f32,
            1.0,
        ]);
    }

    for i in 0..(n_points - 1) {
        let base = (i * 2) as u32;
        m.indices
            .extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }

    m
}
