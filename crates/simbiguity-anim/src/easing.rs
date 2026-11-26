//! Easing curves: how progress `t ∈ [0, 1]` maps to eased progress.
//!
//! Every curve satisfies `f(0) = 0` and `f(1) = 1` exactly. Quad, cubic and
//! expo are monotonic (never go backwards); back overshoots, elastic and
//! bounce oscillate — that is their point.

use simbiguity_math::float;

const PI: f64 = core::f64::consts::PI;
/// The classic "back" overshoot constant (≈10% overshoot).
const BACK_C1: f64 = 1.70158;
const BACK_C2: f64 = BACK_C1 * 1.525;
const BACK_C3: f64 = BACK_C1 + 1.0;

/// An easing curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ease {
    /// Constant speed.
    #[default]
    Linear,
    /// Starts slow (t²).
    QuadIn,
    /// Ends slow.
    QuadOut,
    /// Slow at both ends.
    QuadInOut,
    /// Starts slow (t³).
    CubicIn,
    /// Ends slow.
    CubicOut,
    /// Slow at both ends.
    CubicInOut,
    /// Starts very slow (2^(10t − 10)).
    ExpoIn,
    /// Ends very slow.
    ExpoOut,
    /// Very slow at both ends.
    ExpoInOut,
    /// Pulls back before going forward.
    BackIn,
    /// Overshoots, then settles.
    BackOut,
    /// Pulls back, then overshoots.
    BackInOut,
    /// Winds up with growing wobbles.
    ElasticIn,
    /// Overshoots with decaying wobbles.
    ElasticOut,
    /// Wobbles at both ends.
    ElasticInOut,
    /// Bounces before leaving.
    BounceIn,
    /// Lands like a dropped ball.
    BounceOut,
    /// Bounces at both ends.
    BounceInOut,
}

impl Ease {
    /// Every curve, for tests and pickers.
    pub const ALL: [Ease; 19] = [
        Ease::Linear,
        Ease::QuadIn,
        Ease::QuadOut,
        Ease::QuadInOut,
        Ease::CubicIn,
        Ease::CubicOut,
        Ease::CubicInOut,
        Ease::ExpoIn,
        Ease::ExpoOut,
        Ease::ExpoInOut,
        Ease::BackIn,
        Ease::BackOut,
        Ease::BackInOut,
        Ease::ElasticIn,
        Ease::ElasticOut,
        Ease::ElasticInOut,
        Ease::BounceIn,
        Ease::BounceOut,
        Ease::BounceInOut,
    ];

    /// `true` for curves that never go backwards or past the end.
    pub fn is_monotonic(self) -> bool {
        !matches!(
            self,
            Ease::BackIn
                | Ease::BackOut
                | Ease::BackInOut
                | Ease::ElasticIn
                | Ease::ElasticOut
                | Ease::ElasticInOut
                | Ease::BounceIn
                | Ease::BounceOut
                | Ease::BounceInOut
        )
    }

    /// Eased progress for `t`, which is clamped to `[0, 1]` first.
    pub fn apply(self, t: f64) -> f64 {
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        if t == 0.0 || t == 1.0 {
            return t; // exact endpoints for every curve
        }
        match self {
            Ease::Linear => t,
            Ease::QuadIn => t * t,
            Ease::QuadOut => out(t, |u| u * u),
            Ease::QuadInOut => in_out(t, |u| u * u),
            Ease::CubicIn => t * t * t,
            Ease::CubicOut => out(t, |u| u * u * u),
            Ease::CubicInOut => in_out(t, |u| u * u * u),
            Ease::ExpoIn => expo_in(t),
            Ease::ExpoOut => out(t, expo_in),
            Ease::ExpoInOut => in_out(t, expo_in),
            Ease::BackIn => back_in(t),
            Ease::BackOut => out(t, back_in),
            Ease::BackInOut => back_in_out(t),
            Ease::ElasticIn => elastic_in(t),
            Ease::ElasticOut => out(t, elastic_in),
            Ease::ElasticInOut => in_out(t, elastic_in),
            Ease::BounceIn => 1.0 - bounce_out(1.0 - t),
            Ease::BounceOut => bounce_out(t),
            Ease::BounceInOut => in_out(t, |u| 1.0 - bounce_out(1.0 - u)),
        }
    }
}

/// The "out" version of an "in" curve: mirror it in time and value.
fn out(t: f64, f: impl Fn(f64) -> f64) -> f64 {
    1.0 - f(1.0 - t)
}

/// The "in-out" version: the in curve for the first half, out for the second.
fn in_out(t: f64, f: impl Fn(f64) -> f64) -> f64 {
    if t < 0.5 {
        f(2.0 * t) * 0.5
    } else {
        1.0 - f(2.0 - 2.0 * t) * 0.5
    }
}

fn expo_in(t: f64) -> f64 {
    if t <= 0.0 {
        0.0
    } else {
        // 2^(10t − 10) = e^((10t − 10)·ln 2)
        float::exp((10.0 * t - 10.0) * core::f64::consts::LN_2)
    }
}

fn back_in(t: f64) -> f64 {
    BACK_C3 * t * t * t - BACK_C1 * t * t
}

fn back_in_out(t: f64) -> f64 {
    let u = 2.0 * t;
    if t < 0.5 {
        u * u * ((BACK_C2 + 1.0) * u - BACK_C2) * 0.5
    } else {
        let v = u - 2.0;
        (v * v * ((BACK_C2 + 1.0) * v + BACK_C2) + 2.0) * 0.5
    }
}

fn elastic_in(t: f64) -> f64 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let c4 = 2.0 * PI / 3.0;
    -float::exp((10.0 * t - 10.0) * core::f64::consts::LN_2) * float::sin((t * 10.0 - 10.75) * c4)
}

fn bounce_out(t: f64) -> f64 {
    const N: f64 = 7.5625;
    const D: f64 = 2.75;
    if t < 1.0 / D {
        N * t * t
    } else if t < 2.0 / D {
        let u = t - 1.5 / D;
        N * u * u + 0.75
    } else if t < 2.5 / D {
        let u = t - 2.25 / D;
        N * u * u + 0.9375
    } else {
        let u = t - 2.625 / D;
        N * u * u + 0.984375
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steps(n: usize) -> impl Iterator<Item = f64> {
        (0..=n).map(move |i| i as f64 / n as f64)
    }

    /// Every curve starts at exactly 0 and ends at exactly 1.
    #[test]
    fn every_curve_hits_both_endpoints_exactly() {
        for e in Ease::ALL {
            assert_eq!(e.apply(0.0), 0.0, "{e:?} at 0");
            assert_eq!(e.apply(1.0), 1.0, "{e:?} at 1");
        }
    }

    /// Inputs outside the unit interval, and NaN, are clamped first.
    #[test]
    fn out_of_range_and_nan_inputs_are_clamped() {
        for e in Ease::ALL {
            assert_eq!(e.apply(-3.0), 0.0, "{e:?}");
            assert_eq!(e.apply(7.5), 1.0, "{e:?}");
            assert_eq!(e.apply(f64::NAN), 0.0, "{e:?}");
        }
    }

    /// Curves that claim to be monotonic never go backwards or leave the unit interval.
    #[test]
    fn monotonic_curves_never_go_backwards() {
        for e in Ease::ALL.into_iter().filter(|e| e.is_monotonic()) {
            let mut prev = 0.0;
            for t in steps(400) {
                let y = e.apply(t);
                assert!(y >= prev - 1e-12, "{e:?} went back at {t}");
                assert!((0.0..=1.0).contains(&y), "{e:?} left [0,1] at {t}");
                prev = y;
            }
        }
    }

    /// Back and elastic curves really do overshoot, which is why they are not monotonic.
    #[test]
    fn non_monotonic_curves_leave_the_unit_interval_or_reverse() {
        for e in Ease::ALL.into_iter().filter(|e| !e.is_monotonic()) {
            let ys: std::vec::Vec<f64> = steps(400).map(|t| e.apply(t)).collect();
            let outside = ys.iter().any(|y| !(0.0..=1.0).contains(y));
            let reverses = ys.windows(2).any(|w| w[1] < w[0] - 1e-9);
            assert!(outside || reverses, "{e:?} behaves monotonically");
        }
    }

    /// The in-out curves are symmetric about the midpoint.
    #[test]
    fn in_out_curves_are_symmetric() {
        for e in [
            Ease::QuadInOut,
            Ease::CubicInOut,
            Ease::ExpoInOut,
            Ease::BounceInOut,
        ] {
            assert!((e.apply(0.5) - 0.5).abs() < 1e-12, "{e:?} midpoint");
            for t in steps(20) {
                let s = e.apply(t) + e.apply(1.0 - t);
                assert!((s - 1.0).abs() < 1e-9, "{e:?} at {t}");
            }
        }
    }

    /// Known values pin the formulas: quad-in is t squared, cubic-out mirrors cubic-in.
    #[test]
    fn known_values_match_the_formulas() {
        assert_eq!(Ease::Linear.apply(0.3), 0.3);
        assert!((Ease::QuadIn.apply(0.5) - 0.25).abs() < 1e-15);
        assert!((Ease::QuadOut.apply(0.5) - 0.75).abs() < 1e-15);
        assert!((Ease::CubicIn.apply(0.5) - 0.125).abs() < 1e-15);
        assert!((Ease::CubicOut.apply(0.5) - 0.875).abs() < 1e-15);
        assert!((Ease::ExpoIn.apply(0.9) - 0.5).abs() < 1e-12);
        assert!((Ease::BounceOut.apply(1.0 / 2.75) - 1.0).abs() < 1e-12);
        assert!(Ease::BackIn.apply(0.2) < 0.0, "back pulls back first");
        assert!(Ease::BackOut.apply(0.8) > 1.0, "back overshoots");
    }

    /// Linear is the default curve.
    #[test]
    fn default_is_linear() {
        assert_eq!(Ease::default(), Ease::Linear);
        assert!(Ease::Linear.is_monotonic());
        assert!(!Ease::ElasticOut.is_monotonic());
    }

    /// The private helpers handle their own edge inputs.
    #[test]
    fn helpers_guard_their_edges() {
        assert_eq!(expo_in(0.0), 0.0);
        assert_eq!(elastic_in(0.0), 0.0);
        assert_eq!(elastic_in(1.0), 1.0);
        assert!((back_in_out(0.25) + back_in_out(0.75) - 1.0).abs() < 1e-12);
    }
}
