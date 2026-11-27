#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicsError {
    NonPositiveMass,
    SelfConnectedSpring,
    SelfConnectedRod,
    IndexOutOfBounds,
    InvalidParameter,
    InvalidTimeStep,
}
