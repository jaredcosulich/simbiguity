use simbiguity_math::vec::Vec3;

#[derive(Debug, Clone)]
pub struct Gravity {
    pub acceleration: Vec3,
}

#[derive(Debug, Clone)]
pub struct Attractor {
    pub position: Vec3,
    pub gm: f64,
    pub softening: f64,
}

#[derive(Debug, Clone)]
pub struct Spring {
    pub p1: usize,
    pub p2: usize,
    pub rest_length: f64,
    pub k: f64,
    pub damping: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriveKind {
    Sine,
    Step,
}

#[derive(Debug, Clone)]
pub struct Drive {
    pub axis: Vec3,
    pub amplitude: f64,
    pub omega: f64,
    pub phase: f64,
    pub kind: DriveKind,
}
