use alloc::vec::Vec;
use core::f64::consts::LN_10;
use simbiguity_math::float::{exp, floor, ln};

pub struct NiceTicks;

impl NiceTicks {
    pub fn compute(min: f64, max: f64, count: usize) -> (Vec<f64>, f64) {
        if count == 0 || min >= max {
            return (alloc::vec![min], 0.0);
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

        let decimals = if mag < 0.0 { -mag as i32 } else { 0 };
        let mut rounding_factor = 1.0;
        for _ in 0..decimals {
            rounding_factor *= 10.0;
        }

        for i in 0..100 {
            let t_raw = start + (i as f64) * nice_step;
            let t = simbiguity_math::float::round(t_raw * rounding_factor) / rounding_factor;

            if t > max + nice_step * 0.5 {
                break;
            }
            if t >= min {
                ticks.push(t);
            }
        }
        if ticks.is_empty() {
            ticks.push(min);
        }
        (ticks, nice_step)
    }

    pub fn format_tick(tick: f64, step: f64) -> alloc::string::String {
        use alloc::format;
        if step >= 1.0 {
            format!("{:.0}", tick)
        } else if step >= 0.1 {
            format!("{:.1}", tick)
        } else if step >= 0.01 {
            format!("{:.2}", tick)
        } else {
            format!("{:.3}", tick)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 1-2-5 rule: asking for about 5 ticks over [0, 10] gives a step of 2.
    /// So the ticks are exactly 0, 2, 4, 6, 8, 10, a hand-checkable example of
    /// choosing "nice" axis labels.
    #[test]
    fn ticks_for_zero_to_ten_step_by_two() {
        let (ticks, _) = NiceTicks::compute(0.0, 10.0, 5);
        assert_eq!(ticks, alloc::vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
    }

    /// Verifies that tick generation doesn't loop infinitely or produce excessive ticks for very large magnitudes.
    #[test]
    fn ticks_terminate_at_huge_magnitudes() {
        let (ticks, _) = NiceTicks::compute(1e17, 1e17 + 16.0, 5);
        assert!(ticks.len() <= 20);
    }
}
