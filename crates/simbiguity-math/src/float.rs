//! Deterministic float functions for `no_std`.
//!
//! The Rust `core` library has no `sqrt`, `sin` or `exp` for `f64`: those live
//! in `std`, which calls the platform's C math library. Different platforms'
//! math libraries round differently in the last bit, which is enough to make
//! two runs of the "same" simulation drift apart — one native, one in a
//! browser. So simbiguity computes these itself, using only IEEE-754 `+ - * /`
//! (which every platform rounds identically). The result is bit-identical on
//! x86, ARM and WASM, with or without the `std` feature.
//!
//! Each function documents its accuracy. "ulp" means *unit in the last place*:
//! the gap between a float and the next representable float. An error of
//! 1 ulp is the best you can do without exact arithmetic.
//!
//! The techniques here are the classic ones (see fdlibm, the reference C math
//! library): **range reduction** shrinks the input into a small interval where
//! a short **polynomial** is accurate, then the answer is scaled back out.

// The polynomial and π/2, ln 2 split constants are copied digit-for-digit
// from fdlibm so readers can check them against the reference; the extra
// digits round to the same `f64`.
#![allow(clippy::excessive_precision)]

use core::f64::consts::{FRAC_PI_2, LN_2, PI};

const SIGN_MASK: u64 = 1 << 63;
/// 2^52: every `f64` at or above this magnitude is already an integer.
const TWO_POW_52: f64 = 4_503_599_627_370_496.0;

/// Absolute value (exact).
#[inline]
pub fn abs(x: f64) -> f64 {
    f64::from_bits(x.to_bits() & !SIGN_MASK)
}

/// `x` with the sign of `sign` (exact).
#[inline]
pub fn copysign(x: f64, sign: f64) -> f64 {
    f64::from_bits((x.to_bits() & !SIGN_MASK) | (sign.to_bits() & SIGN_MASK))
}

/// The largest float strictly less than `x` (exact). `next_down(0.0)` is the
/// smallest negative subnormal; NaN and −∞ come back unchanged.
pub fn next_down(x: f64) -> f64 {
    if x.is_nan() || x == f64::NEG_INFINITY {
        return x;
    }
    if x == 0.0 {
        return -f64::from_bits(1);
    }
    let bits = x.to_bits();
    if x > 0.0 {
        f64::from_bits(bits - 1)
    } else {
        f64::from_bits(bits + 1)
    }
}

/// Rounds toward zero (exact).
pub fn trunc(x: f64) -> f64 {
    if !x.is_finite() || abs(x) >= TWO_POW_52 {
        return x;
    }
    copysign((x as i64) as f64, x)
}

/// Rounds toward negative infinity (exact).
pub fn floor(x: f64) -> f64 {
    let t = trunc(x);
    if t > x {
        t - 1.0
    } else {
        t
    }
}

/// Rounds to the nearest integer, halfway cases away from zero (exact).
pub fn round(x: f64) -> f64 {
    let t = trunc(x);
    if abs(x - t) >= 0.5 {
        t + copysign(1.0, x)
    } else {
        t
    }
}

/// Multiplies `x` by 2^`k` by building the power of two from its bits.
fn scale_pow2(x: f64, k: i32) -> f64 {
    // A normal power of two covers exponents -1022..=1023. Split larger
    // shifts into two factors so the factor itself never over/underflows.
    fn pow2(k: i32) -> f64 {
        f64::from_bits(((k + 1023) as u64) << 52)
    }
    if (-1022..=1023).contains(&k) {
        x * pow2(k)
    } else {
        let half = k / 2;
        x * pow2(half) * pow2(k - half)
    }
}

/// Square root, correctly rounded or within 1 ulp.
///
/// Newton's method: if `y` is a guess for √x, then `(y + x/y) / 2` is a
/// better one, and each step roughly doubles the number of correct digits.
/// A good first guess comes from halving the float's exponent bits.
/// Negative inputs return NaN.
pub fn sqrt(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 || x == f64::INFINITY {
        return x;
    }
    if x < f64::MIN_POSITIVE {
        // Subnormal: scale up by 2^104 so the bit trick works, then back by 2^-52.
        return scale_pow2(sqrt(scale_pow2(x, 104)), -52);
    }
    round_down_if_closer(sqrt_newton(x), x)
}

/// Newton iterations for √x on a positive normal `x`, from a first guess
/// made by halving the exponent bits.
fn sqrt_newton(x: f64) -> f64 {
    let mut y = f64::from_bits((x.to_bits() >> 1) + 0x1FF8_0000_0000_0000);
    for _ in 0..6 {
        y = 0.5 * (y + x / y);
    }
    y
}

/// Newton can settle one ulp high: returns the float just below `y` when its
/// square is closer to `x`, otherwise `y`.
fn round_down_if_closer(y: f64, x: f64) -> f64 {
    let below = f64::from_bits(y.to_bits() - 1);
    if abs(below * below - x) < abs(y * y - x) {
        below
    } else {
        y
    }
}

// Cody–Waite split of π/2: the first part has trailing zero bits, so
// `q * PIO2_1` is exact for any quadrant count `q` below 2^20.
const PIO2_1: f64 = 1.570_796_326_734_125_614_17e0;
const PIO2_2: f64 = 6.077_100_506_303_965_976_60e-11;
const PIO2_3: f64 = 2.022_266_248_711_166_455_80e-21;

/// Reduces `x` to `r` in about [-π/4, π/4] and a quadrant `q` (mod 4) with
/// `x = q·π/2 + r`. Accurate for |x| < ~1.6e6; beyond that `sin`/`cos`
/// lose digits (simulations never need angles that large).
fn reduce_quadrant(x: f64) -> (f64, i64) {
    let q = round(x * (2.0 / PI));
    let r = ((x - q * PIO2_1) - q * PIO2_2) - q * PIO2_3;
    (r, (q as i64) & 3)
}

/// sin on [-π/4, π/4] (fdlibm `__kernel_sin` coefficients).
fn kernel_sin(x: f64) -> f64 {
    const S1: f64 = -1.666_666_666_666_663_243_48e-1;
    const S2: f64 = 8.333_333_333_322_489_461_24e-3;
    const S3: f64 = -1.984_126_982_985_794_931_34e-4;
    const S4: f64 = 2.755_731_370_707_006_767_89e-6;
    const S5: f64 = -2.505_076_025_340_686_341_95e-8;
    const S6: f64 = 1.589_690_995_211_550_102_21e-10;
    let z = x * x;
    x + x * z * (S1 + z * (S2 + z * (S3 + z * (S4 + z * (S5 + z * S6)))))
}

/// cos on [-π/4, π/4] (fdlibm `__kernel_cos` coefficients).
fn kernel_cos(x: f64) -> f64 {
    const C1: f64 = 4.166_666_666_666_660_190_37e-2;
    const C2: f64 = -1.388_888_888_887_410_957_49e-3;
    const C3: f64 = 2.480_158_728_947_672_941_78e-5;
    const C4: f64 = -2.755_731_435_139_066_330_35e-7;
    const C5: f64 = 2.087_572_321_298_174_827_90e-9;
    const C6: f64 = -1.135_964_755_778_819_482_65e-11;
    let z = x * x;
    1.0 - 0.5 * z + z * z * (C1 + z * (C2 + z * (C3 + z * (C4 + z * (C5 + z * C6)))))
}

/// Sine and cosine together (one range reduction). Within 2e-16 absolute
/// error for |x| < 1.6e6. Non-finite inputs return NaN.
pub fn sin_cos(x: f64) -> (f64, f64) {
    if !x.is_finite() {
        return (f64::NAN, f64::NAN);
    }
    let (r, q) = reduce_quadrant(x);
    let (s, c) = (kernel_sin(r), kernel_cos(r));
    match q {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    }
}

/// Sine. See [`sin_cos`] for accuracy.
pub fn sin(x: f64) -> f64 {
    sin_cos(x).0
}

/// Cosine. See [`sin_cos`] for accuracy.
pub fn cos(x: f64) -> f64 {
    sin_cos(x).1
}

/// Tangent, as `sin / cos` (relative error grows near the poles at ±π/2).
pub fn tan(x: f64) -> f64 {
    let (s, c) = sin_cos(x);
    s / c
}

const LN2_HI: f64 = 6.931_471_803_691_238_164_90e-1;
const LN2_LO: f64 = 1.908_214_929_270_587_700_02e-10;

/// e^x, within 2 ulp.
///
/// Writes `x = k·ln2 + r` with |r| ≤ ln2/2, so `e^x = 2^k · e^r`. The power
/// of two is exact (it only sets exponent bits) and `e^r` is a short Taylor
/// series because `r` is small.
pub fn exp(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x > 709.782_712_893_384 {
        return f64::INFINITY;
    }
    if x < -745.133_219_101_941_1 {
        return 0.0;
    }
    let k = round(x / LN_2);
    let r = (x - k * LN2_HI) - k * LN2_LO;
    scale_pow2(exp_taylor(r), k as i32)
}

/// e^r for small |r| (≤ ln2/2): the Taylor series to r^14/14!, evaluated by
/// Horner's rule from the top.
fn exp_taylor(r: f64) -> f64 {
    let mut sum = 1.0;
    let mut n = 14.0;
    while n >= 1.0 {
        sum = 1.0 + r * sum / n;
        n -= 1.0;
    }
    sum
}

/// `Σ ±zʲ / (2j+1)` over odd denominators `1, 3, …, top` (Horner's rule from
/// the top). With `alternating` the signs go `+ − + …` (the atan series);
/// without, all terms are positive (the atanh series behind `ln`).
fn odd_series(z: f64, top: u32, alternating: bool) -> f64 {
    let mut series = 0.0;
    for k in (1..=top).rev().step_by(2) {
        let sign = if alternating && (k / 2) % 2 == 1 {
            -1.0
        } else {
            1.0
        };
        series = sign / k as f64 + z * series;
    }
    series
}

/// Splits a positive normal `x` into `(e, m)` with `x = 2^e · m` and
/// `m` in [√½, √2], so `ln m` is small and its series converges fast.
fn split_mantissa(x: f64) -> (i64, f64) {
    let bits = x.to_bits();
    let mut e = ((bits >> 52) & 0x7FF) as i64 - 1023;
    let mut m = f64::from_bits((bits & 0x000F_FFFF_FFFF_FFFF) | 0x3FF0_0000_0000_0000);
    if m > core::f64::consts::SQRT_2 {
        m *= 0.5;
        e += 1;
    }
    (e, m)
}

/// Natural logarithm, within 2 ulp. `ln(0) = -∞`; negative inputs give NaN.
///
/// Splits `x = 2^e · m` with m in [√½, √2], so `ln x = e·ln2 + ln m`, then
/// uses `ln m = 2·atanh(s)` with `s = (m−1)/(m+1)`, whose odd series
/// `2(s + s³/3 + s⁵/5 + …)` converges fast because |s| ≤ 0.172.
pub fn ln(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return f64::NEG_INFINITY;
    }
    if x == f64::INFINITY {
        return x;
    }
    if x < f64::MIN_POSITIVE {
        return ln(scale_pow2(x, 54)) - 54.0 * LN_2;
    }
    let (e, m) = split_mantissa(x);
    let s = (m - 1.0) / (m + 1.0);
    let series = odd_series(s * s, 23, false);
    let e = e as f64;
    e * LN2_HI + (e * LN2_LO + 2.0 * s * series)
}

/// Arctangent, in [-π/2, π/2], within 4 ulp.
///
/// Uses `atan(t) = π/2 − atan(1/t)` to bring |t| ≤ 1, then the half-angle
/// identity `atan(t) = 2·atan(t / (1 + √(1+t²)))` twice to bring |t| ≤ 0.2,
/// where the series `t − t³/3 + t⁵/5 − …` converges quickly.
pub fn atan(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    let (u, flip) = atan_reduce(abs(x));
    let small = 4.0 * u * odd_series(u * u, 25, true);
    let result = if flip { FRAC_PI_2 - small } else { small };
    copysign(result, x)
}

/// Range reduction for `atan` on `t ≥ 0`: returns `(u, flip)` with
/// `atan(t) = 4·atan(u)` when `!flip` and `π/2 − 4·atan(u)` when `flip`,
/// and `u ≤ tan(π/16) ≈ 0.199`.
fn atan_reduce(t: f64) -> (f64, bool) {
    let (mut u, flip) = if t > 1.0 { (1.0 / t, true) } else { (t, false) };
    for _ in 0..2 {
        u /= 1.0 + sqrt(1.0 + u * u);
    }
    (u, flip)
}

/// `√(1 − x²)`, computed as `√((1 − x)(1 + x))` to keep precision near ±1:
/// the leg opposite `x` in a unit right triangle.
fn unit_complement(x: f64) -> f64 {
    sqrt((1.0 - x) * (1.0 + x))
}

/// Angle of the point `(x, y)` from the positive x axis, in [-π, π].
pub fn atan2(y: f64, x: f64) -> f64 {
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    if x == 0.0 {
        return if y == 0.0 {
            // Matches IEEE: atan2(±0, +0) = ±0, atan2(±0, −0) = ±π.
            if x.is_sign_negative() {
                copysign(PI, y)
            } else {
                y
            }
        } else {
            copysign(FRAC_PI_2, y)
        };
    }
    let a = atan(y / x);
    if x > 0.0 {
        a
    } else {
        a + copysign(PI, y)
    }
}

/// Arccosine, in [0, π]. Inputs outside [-1, 1] give NaN.
pub fn acos(x: f64) -> f64 {
    if !(-1.0..=1.0).contains(&x) {
        return f64::NAN;
    }
    atan2(unit_complement(x), x)
}

/// Arcsine, in [-π/2, π/2]. Inputs outside [-1, 1] give NaN.
pub fn asin(x: f64) -> f64 {
    if !(-1.0..=1.0).contains(&x) {
        return f64::NAN;
    }
    atan2(x, unit_complement(x))
}

/// Computes the hyperbolic tangent of `x`.
pub fn tanh(x: f64) -> f64 {
    let ax = abs(x);
    if ax > 22.0 {
        return if x > 0.0 { 1.0 } else { -1.0 };
    }
    let e2x = exp(2.0 * ax);
    let t = (e2x - 1.0) / (e2x + 1.0);
    if x < 0.0 {
        -t
    } else {
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    /// Distance in representable floats between two finite same-sign values.
    fn ulps(a: f64, b: f64) -> u64 {
        (a.to_bits() as i64 - b.to_bits() as i64).unsigned_abs()
    }

    /// A deterministic spread of inputs in `[lo, hi]`.
    fn samples(lo: f64, hi: f64, n: usize) -> Vec<f64> {
        (0..=n)
            .map(|i| lo + (hi - lo) * (i as f64 / n as f64))
            .collect()
    }

    /// abs and copysign only touch the sign bit, so they must preserve
    /// signed zero, infinities and the magnitude exactly.
    #[test]
    fn abs_and_copysign_handle_sign_bit_exactly() {
        assert_eq!(abs(-3.5), 3.5);
        assert_eq!(abs(2.0), 2.0);
        assert!(abs(-0.0).is_sign_positive());
        assert_eq!(abs(f64::NEG_INFINITY), f64::INFINITY);
        assert_eq!(copysign(3.0, -1.0), -3.0);
        assert_eq!(copysign(-3.0, 1.0), 3.0);
        assert!(copysign(0.0, -1.0).is_sign_negative());
    }

    /// next_down must step toward −∞ for positive, negative and zero inputs
    /// alike, since a raw bits−1 steps the wrong way for negatives.
    #[test]
    fn next_down_steps_toward_negative_infinity() {
        assert!(next_down(1.0) < 1.0);
        assert_eq!(next_down(1.0).to_bits(), 1.0f64.to_bits() - 1);
        assert!(next_down(-1.0) < -1.0);
        assert_eq!(next_down(0.0), -f64::from_bits(1));
        assert_eq!(next_down(-0.0), -f64::from_bits(1));
        assert_eq!(next_down(f64::INFINITY), f64::MAX);
        assert_eq!(next_down(f64::NEG_INFINITY), f64::NEG_INFINITY);
        assert_eq!(next_down(-f64::MAX), f64::NEG_INFINITY);
        assert!(next_down(f64::NAN).is_nan());
    }

    /// trunc, floor and round are the building blocks of every range
    /// reduction; they must be exact, including negatives and halfway cases.
    #[test]
    fn trunc_floor_round_are_exact() {
        assert_eq!(trunc(2.7), 2.0);
        assert_eq!(trunc(-2.7), -2.0);
        assert!(trunc(-0.3).is_sign_negative());
        assert_eq!(floor(-2.1), -3.0);
        assert_eq!(floor(2.9), 2.0);
        assert_eq!(floor(-3.0), -3.0);
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -3.0);
        assert_eq!(round(2.49), 2.0);
        assert_eq!(trunc(1e300), 1e300);
        assert!(trunc(f64::NAN).is_nan());
        assert_eq!(floor(f64::NEG_INFINITY), f64::NEG_INFINITY);
    }

    /// The power-of-two scaler must stay exact past the normal exponent
    /// range by splitting the shift, which exp and the subnormal paths need.
    #[test]
    fn scale_pow2_splits_large_shifts() {
        assert_eq!(scale_pow2(1.0, 10), 1024.0);
        assert_eq!(scale_pow2(3.0, -1), 1.5);
        assert_eq!(scale_pow2(0.5, 1024), 2f64.powi(1023));
        assert_eq!(scale_pow2(1.0, -1074), f64::from_bits(1));
    }

    /// sqrt must be within 1 ulp of the correctly rounded platform result
    /// across many magnitudes, and exact on perfect squares.
    #[test]
    fn sqrt_matches_platform_within_one_ulp() {
        for x in [0.25, 2.0, 4.0, 9.0, 1e-300, 1e300, 123_456.789, f64::MAX] {
            assert!(ulps(sqrt(x), x.sqrt()) <= 1, "sqrt({x})");
        }
        for x in samples(1e-6, 1e6, 2000) {
            assert!(ulps(sqrt(x), x.sqrt()) <= 1, "sqrt({x})");
        }
        assert_eq!(sqrt(16.0), 4.0);
        assert_eq!(sqrt(1.0), 1.0);
    }

    /// Subnormals take a scaled detour; NaN, negatives, zero and infinity are
    /// special-cased and must come back as IEEE prescribes.
    #[test]
    fn sqrt_special_and_subnormal_inputs() {
        let tiny = f64::from_bits(1);
        assert!(ulps(sqrt(tiny), tiny.sqrt()) <= 1);
        let sub = f64::MIN_POSITIVE / 3.0;
        assert!(ulps(sqrt(sub), sub.sqrt()) <= 1);
        assert!(sqrt(-1.0).is_nan());
        assert!(sqrt(f64::NAN).is_nan());
        assert_eq!(sqrt(0.0), 0.0);
        assert!(sqrt(-0.0).is_sign_negative());
        assert_eq!(sqrt(f64::INFINITY), f64::INFINITY);
    }

    /// The Newton stage alone must already land within one ulp, so the
    /// correction only ever has a single neighbour to consider.
    #[test]
    fn sqrt_newton_converges_to_within_one_ulp() {
        for x in [2.0, 3.0, 10.0, 0.1, 1e10] {
            assert!(ulps(sqrt_newton(x), x.sqrt()) <= 1, "x={x}");
        }
    }

    /// The 1-ulp correction steps down only when the lower neighbour's
    /// square is strictly closer, and otherwise leaves the guess alone.
    #[test]
    fn round_down_if_closer_picks_the_nearer_neighbour() {
        let high = f64::from_bits(2.0f64.to_bits() + 1);
        assert_eq!(round_down_if_closer(high, 4.0), 2.0);
        assert_eq!(round_down_if_closer(2.0, 4.0), 2.0);
        let r = 2.0f64.sqrt();
        assert_eq!(round_down_if_closer(r, 2.0), r);
    }

    /// sin and cos must stay within 2e-16 absolute of the platform across
    /// all four quadrants and moderately large angles.
    #[test]
    fn sin_cos_match_platform() {
        for x in samples(-100.0, 100.0, 5000)
            .into_iter()
            .chain([1e5, -1e5, 1e6])
        {
            let (s, c) = sin_cos(x);
            assert!((s - x.sin()).abs() <= 2.5e-16, "sin({x})");
            assert!((c - x.cos()).abs() <= 2.5e-16, "cos({x})");
            assert_eq!(sin(x), s);
            assert_eq!(cos(x), c);
        }
        assert_eq!(sin_cos(0.0), (0.0, 1.0));
        let (s, c) = sin_cos(f64::INFINITY);
        assert!(s.is_nan() && c.is_nan());
        assert!(sin(f64::NAN).is_nan());
    }

    /// Quadrant reduction must leave a remainder within about π/4 and pick
    /// the quadrant that sin_cos's sign table expects.
    #[test]
    fn reduce_quadrant_lands_in_the_central_interval() {
        for x in samples(-50.0, 50.0, 997) {
            let (r, q) = reduce_quadrant(x);
            assert!(r.abs() <= core::f64::consts::FRAC_PI_4 + 1e-12, "x={x}");
            assert!((0..4).contains(&q));
        }
        assert_eq!(reduce_quadrant(PI).1, 2);
        assert_eq!(reduce_quadrant(FRAC_PI_2).1, 1);
    }

    /// tan is sin/cos, so it must agree with the platform away from poles.
    #[test]
    fn tan_matches_platform_away_from_poles() {
        for x in samples(-1.4, 1.4, 500) {
            let rel = ((tan(x) - x.tan()) / x.tan()).abs();
            assert!(rel < 1e-14 || (tan(x) - x.tan()).abs() < 1e-16, "x={x}");
        }
        assert_eq!(tan(0.0), 0.0);
    }

    /// exp must be within 2 ulp of the platform across its useful range.
    #[test]
    fn exp_matches_platform_within_two_ulp() {
        for x in samples(-700.0, 700.0, 4000)
            .into_iter()
            .chain(samples(-1.0, 1.0, 1000))
        {
            assert!(ulps(exp(x), x.exp()) <= 2, "exp({x})");
        }
        assert_eq!(exp(0.0), 1.0);
    }

    /// The overflow and underflow thresholds and the split scale path just
    /// inside them must give inf, zero, and finite gradual-underflow values.
    #[test]
    fn exp_thresholds_and_extremes() {
        assert_eq!(exp(710.0), f64::INFINITY);
        assert_eq!(exp(-746.0), 0.0);
        assert!(exp(f64::NAN).is_nan());
        assert!(exp(709.7).is_finite());
        assert!(ulps(exp(709.7), 709.7f64.exp()) <= 2);
        let sub = exp(-740.0);
        assert!(sub > 0.0 && sub < f64::MIN_POSITIVE);
        assert!(ulps(sub, (-740.0f64).exp()) <= 2);
    }

    /// The Taylor kernel must be exact at zero and accurate at the edges of
    /// its reduced interval.
    #[test]
    fn exp_taylor_is_accurate_on_reduced_interval() {
        assert_eq!(exp_taylor(0.0), 1.0);
        let h = LN_2 / 2.0;
        assert!(ulps(exp_taylor(h), h.exp()) <= 1);
        assert!(ulps(exp_taylor(-h), (-h).exp()) <= 1);
    }

    /// The odd-denominator series must start at 1, follow the requested sign
    /// pattern, and converge to atan/atanh identities for small z.
    #[test]
    fn odd_series_signs_and_values() {
        assert_eq!(odd_series(0.0, 25, true), 1.0);
        assert_eq!(odd_series(0.0, 23, false), 1.0);
        assert_eq!(odd_series(1.0, 3, true), 1.0 - 1.0 / 3.0);
        assert_eq!(odd_series(1.0, 3, false), 1.0 + 1.0 / 3.0);
        assert_eq!(odd_series(1.0, 5, true), 1.0 - 1.0 / 3.0 + 1.0 / 5.0);
        let u: f64 = 0.15;
        assert!(ulps(u * odd_series(u * u, 25, true), u.atan()) <= 2);
        assert!(ulps(u * odd_series(u * u, 23, false), u.atanh()) <= 2);
    }

    /// The mantissa split must reconstruct x and keep m in [√½, √2].
    #[test]
    fn split_mantissa_reconstructs_input() {
        assert_eq!(split_mantissa(1.0), (0, 1.0));
        assert_eq!(split_mantissa(2.0), (1, 1.0));
        assert_eq!(split_mantissa(1.5), (1, 0.75));
        assert_eq!(split_mantissa(0.5), (-1, 1.0));
        for x in [3.0, 1e-300, 7.25e200, 0.7] {
            let (e, m) = split_mantissa(x);
            assert!((core::f64::consts::FRAC_1_SQRT_2..=core::f64::consts::SQRT_2).contains(&m));
            assert_eq!(scale_pow2(m, e as i32), x);
        }
    }

    /// ln must be within 2 ulp of the platform and handle 0, negatives,
    /// infinity and subnormals.
    #[test]
    fn ln_matches_platform_and_special_cases() {
        for x in samples(1e-3, 1e3, 4000)
            .into_iter()
            .chain([1e-300, 1e300, 0.999_999, 1.000_001])
        {
            assert!(
                ulps(ln(x), x.ln()) <= 2 || (ln(x) - x.ln()).abs() < 1e-16,
                "ln({x})"
            );
        }
        assert_eq!(ln(1.0), 0.0);
        assert_eq!(ln(0.0), f64::NEG_INFINITY);
        assert!(ln(-1.0).is_nan());
        assert_eq!(ln(f64::INFINITY), f64::INFINITY);
        let sub = f64::MIN_POSITIVE / 1024.0;
        assert!(ulps(ln(sub), sub.ln()) <= 2);
    }

    /// atan must hit the known values exactly enough and stay within 4 ulp
    /// of the platform on both sides of the |x| = 1 reciprocal switch.
    #[test]
    fn atan_matches_platform_within_four_ulp() {
        assert!(ulps(atan(1.0), core::f64::consts::FRAC_PI_4) <= 1);
        assert!(ulps(atan(-1.0), -core::f64::consts::FRAC_PI_4) <= 1);
        for x in samples(-20.0, 20.0, 4001)
            .into_iter()
            .chain([1e10, -1e10, 1e-10])
        {
            if x == 0.0 {
                continue;
            }
            assert!(ulps(atan(x), x.atan()) <= 4, "atan({x})");
        }
        assert_eq!(atan(0.0), 0.0);
        assert_eq!(atan(f64::INFINITY), FRAC_PI_2);
        assert!(atan(f64::NAN).is_nan());
    }

    /// Reduction must flip above 1 and shrink u below tan(π/16) so the
    /// series converges.
    #[test]
    fn atan_reduce_flips_and_shrinks() {
        let (u, flip) = atan_reduce(0.5);
        assert!(!flip && u <= 0.2);
        let (u, flip) = atan_reduce(4.0);
        assert!(flip && u <= 0.2);
        assert!((4.0 * u.atan() - 0.25f64.atan()).abs() < 1e-15);
        assert_eq!(atan_reduce(0.0), (0.0, false));
        assert!(atan_reduce(1.0).0 <= 0.2 && !atan_reduce(1.0).1);
    }

    /// atan2 must match the platform in every quadrant and follow IEEE for
    /// signed zeros on the axes.
    #[test]
    fn atan2_quadrants_and_zeros() {
        for (y, x) in [
            (1.0, 1.0),
            (1.0, -1.0),
            (-1.0, -1.0),
            (-1.0, 1.0),
            (3.0, -0.5),
            (0.0, -2.0),
        ] {
            assert!(ulps(atan2(y, x), f64::atan2(y, x)) <= 4, "atan2({y},{x})");
        }
        assert_eq!(atan2(0.0, 0.0), 0.0);
        assert!(atan2(-0.0, 0.0).is_sign_negative());
        assert_eq!(atan2(0.0, -0.0), PI);
        assert_eq!(atan2(-0.0, -0.0), -PI);
        assert_eq!(atan2(2.0, 0.0), FRAC_PI_2);
        assert_eq!(atan2(-2.0, 0.0), -FRAC_PI_2);
        assert!(atan2(f64::NAN, 1.0).is_nan());
    }

    /// acos and asin must agree with the platform on [-1, 1] and reject
    /// out-of-domain inputs.
    #[test]
    fn acos_asin_match_platform_and_domain() {
        for x in samples(-1.0, 1.0, 2000) {
            assert!((acos(x) - x.acos()).abs() < 1e-15, "acos({x})");
            assert!((asin(x) - x.asin()).abs() < 1e-15, "asin({x})");
        }
        assert_eq!(acos(1.0), 0.0);
        assert_eq!(asin(1.0), FRAC_PI_2);
        assert!(acos(1.5).is_nan());
        assert!(asin(-1.01).is_nan());
        assert!(asin(f64::NAN).is_nan());
    }

    /// The unit complement is √(1−x²), exact at the ends and the middle.
    #[test]
    fn unit_complement_values() {
        assert_eq!(unit_complement(0.0), 1.0);
        assert_eq!(unit_complement(1.0), 0.0);
        assert_eq!(unit_complement(-1.0), 0.0);
        assert!((unit_complement(0.6) - 0.8).abs() < 1e-16);
    }
}
