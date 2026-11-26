use crate::{axes::Axes, series::Series};
use alloc::string::String;
use alloc::vec::Vec;
use simbiguity_render::CommandBuffer;

pub struct Chart<'a> {
    pub axes: Axes,
    pub series: Vec<(&'a Series, [f32; 4])>,
}

impl<'a> Chart<'a> {
    pub fn draw(&self, _cmd: &mut CommandBuffer) -> Vec<(String, f64, f64)> {
        Vec::new()
    }
}
