#[derive(Debug, Clone, Copy, Default)]
pub struct Energy {
    pub kinetic: f64,
    pub potential_gravity: f64,
    pub potential_spring: f64,
}
impl Energy {
    pub fn total(&self) -> f64 {
        self.kinetic + self.potential_gravity + self.potential_spring
    }
}
