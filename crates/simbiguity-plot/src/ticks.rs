use alloc::vec::Vec;
use core::f64::consts::LN_10;
use simbiguity_math::float::{exp, floor, ln};

pub struct NiceTicks;

impl NiceTicks {
    pub fn compute(min: f64, max: f64, count: usize) -> Vec<f64> {
        if count == 0 || min >= max {
            return alloc::vec![min];
        }
        let range = max - min;
        let exact_step = range / (count as f64 - 1.0);

        let mag = floor(ln(exact_step) / LN_10);
        let mag_pow = exp(mag * LN_10);
        let norm_step = exact_step / mag_pow;

        let nice_step = if norm_step < 1.5 {
            1.0
        } else if norm_step < 3.0 {
            2.0
        } else if norm_step < 7.0 {
            5.0
        } else {
            10.0
        } * mag_pow;

        let start = floor(min / nice_step) * nice_step;
        let mut ticks = Vec::new();
        let mut t = start;
        while t <= max + nice_step * 0.5 {
            if t >= min {
                ticks.push(t);
            }
            t += nice_step;
        }
        if ticks.is_empty() {
            ticks.push(min);
        }
        ticks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that tick generation works correctly for a basic range.
    #[test]
    fn test_compute_ticks() {
        let ticks = NiceTicks::compute(0.0, 10.0, 5);
        assert!(!ticks.is_empty());
        assert_eq!(ticks[0], 0.0);
    }
}
