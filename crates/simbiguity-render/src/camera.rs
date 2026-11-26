//! A perspective camera that orbits a target point.

use simbiguity_math::{float, Mat4, Vec3};

use crate::pick::Ray;

/// A camera on a sphere around `target`: drag to orbit, scroll to zoom.
///
/// `yaw` turns around the vertical axis (0 looks from +Z toward −Z);
/// `pitch` is elevation above the target's horizon. Both are radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitCamera {
    /// The point looked at.
    pub target: Vec3,
    /// Angle around +Y, radians.
    pub yaw: f64,
    /// Elevation, radians; kept within `[min_pitch, max_pitch]`.
    pub pitch: f64,
    /// Distance from the target; kept within `[min_distance, max_distance]`.
    pub distance: f64,
    /// Vertical field of view, radians.
    pub fov_y: f64,
    /// Near clip plane distance.
    pub near: f64,
    /// Far clip plane distance.
    pub far: f64,
    /// Lowest allowed pitch.
    pub min_pitch: f64,
    /// Highest allowed pitch (just short of straight down, where yaw is undefined).
    pub max_pitch: f64,
    /// Closest allowed zoom.
    pub min_distance: f64,
    /// Farthest allowed zoom.
    pub max_distance: f64,
}

impl Default for OrbitCamera {
    fn default() -> OrbitCamera {
        OrbitCamera {
            target: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.5,
            distance: 10.0,
            fov_y: 45f64.to_radians(),
            near: 0.1,
            far: 200.0,
            min_pitch: 0.05,
            max_pitch: 1.5,
            min_distance: 2.0,
            max_distance: 40.0,
        }
    }
}

impl OrbitCamera {
    /// Where the camera is.
    pub fn eye(&self) -> Vec3 {
        let (sy, cy) = float::sin_cos(self.yaw);
        let (sp, cp) = float::sin_cos(self.pitch);
        self.target + Vec3::new(cp * sy, sp, cp * cy) * self.distance
    }

    /// World → view matrix.
    pub fn view(&self) -> Mat4 {
        Mat4::look_at(self.eye(), self.target, Vec3::Y)
    }

    /// View → clip matrix for a viewport of `aspect` = width / height.
    pub fn projection(&self, aspect: f64) -> Mat4 {
        Mat4::perspective(self.fov_y, aspect, self.near, self.far)
    }

    /// World → clip, the product the vertex shader uses.
    pub fn view_projection(&self, aspect: f64) -> Mat4 {
        self.projection(aspect) * self.view()
    }

    /// Re-applies the pitch and distance limits.
    pub fn clamp(&mut self) {
        self.pitch = self.pitch.clamp(self.min_pitch, self.max_pitch);
        self.distance = self.distance.clamp(self.min_distance, self.max_distance);
    }

    /// Turns by `d_yaw` and tilts by `d_pitch` (radians), within limits.
    pub fn orbit(&mut self, d_yaw: f64, d_pitch: f64) {
        self.yaw += d_yaw;
        self.pitch += d_pitch;
        self.clamp();
    }

    /// Multiplies the distance by `factor` (below 1 moves closer), within limits.
    pub fn zoom(&mut self, factor: f64) {
        if factor.is_finite() && factor > 0.0 {
            self.distance *= factor;
        }
        self.clamp();
    }

    /// Where `p` lands on a `width × height` viewport, in pixels from the
    /// top-left corner; `None` if it is behind the camera.
    pub fn project(&self, p: Vec3, width: f64, height: f64) -> Option<(f64, f64)> {
        let view = self.view().transform_point(p);
        if view.z >= 0.0 {
            return None; // the camera looks down −Z
        }
        let ndc = self.projection(width / height).transform_point(view);
        Some(((ndc.x + 1.0) * 0.5 * width, (1.0 - ndc.y) * 0.5 * height))
    }

    /// The ray from the eye through pixel `(px, py)` (top-left origin).
    pub fn ray(&self, px: f64, py: f64, width: f64, height: f64) -> Ray {
        let ndc_x = 2.0 * px / width - 1.0;
        let ndc_y = 1.0 - 2.0 * py / height;
        let inv = self
            .view_projection(width / height)
            .inverse()
            .unwrap_or(Mat4::IDENTITY);
        let near = inv.transform_point(Vec3::new(ndc_x, ndc_y, -1.0));
        let far = inv.transform_point(Vec3::new(ndc_x, ndc_y, 1.0));
        let eye = self.eye();
        Ray {
            origin: eye,
            dir: (far - near)
                .normalize()
                .unwrap_or((self.target - eye).normalize().unwrap_or(-Vec3::Z)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cam() -> OrbitCamera {
        OrbitCamera {
            yaw: 0.0,
            pitch: 0.0,
            min_pitch: -1.0,
            ..OrbitCamera::default()
        }
    }

    /// With zero yaw and pitch the eye sits on +Z at the orbit distance.
    #[test]
    fn eye_sits_on_the_orbit_sphere() {
        assert!((cam().eye() - Vec3::new(0.0, 0.0, 10.0)).length() < 1e-12);
        let c = OrbitCamera::default();
        assert!((c.eye().distance(c.target) - c.distance).abs() < 1e-9);
    }

    /// The target projects to the viewport centre, and a point to its right lands right of centre.
    #[test]
    fn project_lands_known_points_on_known_pixels() {
        let c = cam();
        let (x, y) = c.project(Vec3::ZERO, 800.0, 600.0).unwrap();
        assert!((x - 400.0).abs() < 1e-6 && (y - 300.0).abs() < 1e-6);
        let (rx, ry) = c.project(Vec3::new(1.0, 1.0, 0.0), 800.0, 600.0).unwrap();
        assert!(
            rx > 400.0 && ry < 300.0,
            "right and up on screen: {rx},{ry}"
        );
    }

    /// Points behind the camera do not project.
    #[test]
    fn points_behind_the_camera_do_not_project() {
        assert_eq!(cam().project(Vec3::new(0.0, 0.0, 20.0), 800.0, 600.0), None);
    }

    /// A ray through a projected pixel passes back through the point.
    #[test]
    fn ray_inverts_project() {
        let c = OrbitCamera::default();
        let p = Vec3::new(0.7, 0.2, -0.4);
        let (x, y) = c.project(p, 640.0, 400.0).unwrap();
        let r = c.ray(x, y, 640.0, 400.0);
        assert!((r.dir.length() - 1.0).abs() < 1e-9);
        let t = (p - r.origin).dot(r.dir);
        assert!(r.at(t).distance(p) < 1e-6);
    }

    /// Orbiting clamps the pitch to its limits.
    #[test]
    fn orbit_clamps_pitch() {
        let mut c = OrbitCamera::default();
        c.orbit(0.3, 10.0);
        assert_eq!(c.pitch, c.max_pitch);
        assert!((c.yaw - 0.3).abs() < 1e-12);
        c.orbit(0.0, -10.0);
        assert_eq!(c.pitch, c.min_pitch);
    }

    /// Zoom scales distance within limits and ignores nonsense factors.
    #[test]
    fn zoom_scales_within_limits() {
        let mut c = OrbitCamera::default();
        c.zoom(0.5);
        assert_eq!(c.distance, 5.0);
        c.zoom(0.0);
        c.zoom(f64::NAN);
        c.zoom(-2.0);
        assert_eq!(c.distance, 5.0);
        c.zoom(100.0);
        assert_eq!(c.distance, c.max_distance);
        c.zoom(0.0001);
        assert_eq!(c.distance, c.min_distance);
    }

    /// The view-projection is the projection times the view.
    #[test]
    fn view_projection_composes() {
        let c = OrbitCamera::default();
        assert_eq!(c.view_projection(1.5), c.projection(1.5) * c.view());
    }
}
