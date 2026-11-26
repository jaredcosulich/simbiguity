//! Polylines: measuring a path and walking along it.

use simbiguity_math::Vec3;

/// Total length of a polyline.
pub fn path_length(path: &[Vec3]) -> f64 {
    path.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// The point `distance` along a polyline (clamped to its ends).
pub fn point_along(path: &[Vec3], distance: f64) -> Vec3 {
    let mut left = distance.max(0.0);
    for w in path.windows(2) {
        let len = w[0].distance(w[1]);
        if left <= len && len > 0.0 {
            return w[0].lerp(w[1], left / len);
        }
        left -= len;
    }
    path.last().copied().unwrap_or(Vec3::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;

    const L: [Vec3; 3] = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(3.0, 0.0, 0.0),
        Vec3::new(3.0, 4.0, 0.0),
    ];

    /// The length of a polyline is the sum of its segments.
    #[test]
    fn length_sums_segments() {
        assert_eq!(path_length(&L), 7.0);
        assert_eq!(path_length(&L[..1]), 0.0);
        assert_eq!(path_length(&[]), 0.0);
    }

    /// Walking a distance lands on the right segment at the right place.
    #[test]
    fn point_along_walks_segments() {
        assert_eq!(point_along(&L, 1.5), Vec3::new(1.5, 0.0, 0.0));
        assert_eq!(point_along(&L, 3.0), Vec3::new(3.0, 0.0, 0.0));
        assert_eq!(point_along(&L, 5.0), Vec3::new(3.0, 2.0, 0.0));
    }

    /// Distances before the start or past the end clamp to the ends.
    #[test]
    fn point_along_clamps_to_ends() {
        assert_eq!(point_along(&L, -2.0), L[0]);
        assert_eq!(point_along(&L, 99.0), L[2]);
    }

    /// An empty path gives the origin, and zero-length segments are skipped.
    #[test]
    fn degenerate_paths() {
        assert_eq!(point_along(&[], 1.0), Vec3::ZERO);
        let dup = [Vec3::X, Vec3::X, Vec3::new(2.0, 0.0, 0.0)];
        assert_eq!(point_along(&dup, 0.5), Vec3::new(1.5, 0.0, 0.0));
    }
}
