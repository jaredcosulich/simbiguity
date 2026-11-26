#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Integrator {
    ExplicitEuler,
    SymplecticEuler,
    VelocityVerlet,
}
