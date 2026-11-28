#[derive(Debug, Clone)]
pub struct Rod {
    pub p1: usize,
    pub p2: usize,
    pub length: f64,
}

#[derive(Debug, Clone)]
pub struct Ground {
    pub y: f64,
    pub restitution: f64,
    pub friction: f64,
}
