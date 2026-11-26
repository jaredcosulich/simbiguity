/// Defines the mapping from data space to pixel space.
///
/// `data_to_px` normalizes data coordinates `(x, y)` to `[0, 1]` based on
/// `[x_min, x_max]` and `[y_min, y_max]`, then scales and translates them
/// into the pixel space rectangle defined by `(px_x, px_y, px_w, px_h)`.
pub struct Axes {
    /// Minimum X in data space.
    pub x_min: f64,
    /// Maximum X in data space.
    pub x_max: f64,
    /// Minimum Y in data space.
    pub y_min: f64,
    /// Maximum Y in data space.
    pub y_max: f64,
    /// Origin X in pixel space (usually left).
    pub px_x: f64,
    /// Origin Y in pixel space (usually top).
    pub px_y: f64,
    /// Width in pixels.
    pub px_w: f64,
    /// Height in pixels.
    pub px_h: f64,
}

impl Axes {
    /// Converts a point from data space to pixel space.
    pub fn data_to_px(&self, x: f64, y: f64) -> (f64, f64) {
        let nx = if self.x_max > self.x_min {
            (x - self.x_min) / (self.x_max - self.x_min)
        } else {
            0.5
        };
        let ny = if self.y_max > self.y_min {
            (y - self.y_min) / (self.y_max - self.y_min)
        } else {
            0.5
        };
        (
            self.px_x + nx * self.px_w,
            self.px_y + (1.0 - ny) * self.px_h,
        )
    }
}
