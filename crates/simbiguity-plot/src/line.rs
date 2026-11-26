use simbiguity_render::Mesh;

pub fn build_line_mesh(points: &[(f64, f64)], width: f64) -> Mesh {
    let mut m = Mesh::default();

    for p in points {
        m.vertices
            .extend_from_slice(&[p.0 as f32, (p.1 - width / 2.0) as f32, 0.0, 0.0, 0.0, 1.0]);
        m.vertices
            .extend_from_slice(&[p.0 as f32, (p.1 + width / 2.0) as f32, 0.0, 0.0, 0.0, 1.0]);
    }

    if m.vertices.len() >= 24 {
        // at least 4 vertices (each 6 floats)
        let n_points = points.len();
        for i in 0..(n_points - 1) {
            let base = (i * 2) as u32;
            m.indices
                .extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
        }
    }

    m
}
