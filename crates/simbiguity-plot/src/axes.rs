pub struct Axes {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub px_x: f64,
    pub px_y: f64,
    pub px_w: f64,
    pub px_h: f64,
}

impl Axes {
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
