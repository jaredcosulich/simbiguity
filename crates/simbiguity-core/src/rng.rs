//! A small, seeded, reproducible random number generator.
//!
//! Simulations need randomness — noise in a sensor, the arrival of market
//! orders — but a simulation must also be reproducible. So simbiguity has no
//! global or thread-local RNG: every random draw comes from an [`Rng`] that
//! was seeded explicitly and lives inside the simulation's state (and its
//! snapshots). Same seed, same sequence, on every platform.
//!
//! The algorithm is **PCG32** (PCG-XSH-RR 64/32, by Melissa O'Neill): a
//! 64-bit linear congruential generator (`state = state·a + c`) whose output
//! is scrambled by an xorshift and a data-dependent rotation. It is tiny,
//! fast and statistically strong. It is **not** cryptographically secure.

use simbiguity_math::float;

const MULTIPLIER: u64 = 6_364_136_223_846_793_005;
const DEFAULT_STREAM: u64 = 0xda3e_39cb_94b9_5bdb;
/// 2^-53: scales a 53-bit integer into `[0, 1)` (an `f64` has 53 bits of precision).
const TWO_POW_NEG_53: f64 = 1.0 / 9_007_199_254_740_992.0;

/// A seeded PCG32 random number generator.
#[derive(Debug, Clone, PartialEq)]
pub struct Rng {
    state: u64,
    increment: u64,
    /// Box–Muller produces normals in pairs; the second is kept for the next call.
    spare_normal: Option<f64>,
}

impl Rng {
    /// A generator seeded with `seed` on the default stream.
    pub fn new(seed: u64) -> Rng {
        Rng::with_stream(seed, DEFAULT_STREAM)
    }

    /// A generator seeded with `seed` on stream `stream`. Different streams
    /// with the same seed give independent sequences — handy for giving each
    /// engine in a composed simulation its own RNG from one master seed.
    pub fn with_stream(seed: u64, stream: u64) -> Rng {
        let mut rng = Rng {
            state: 0,
            increment: (stream << 1) | 1,
            spare_normal: None,
        };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    /// The next uniformly distributed `u32`.
    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(MULTIPLIER).wrapping_add(self.increment);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rotation = (old >> 59) as u32;
        xorshifted.rotate_right(rotation)
    }

    /// The next uniformly distributed `u64` (two `u32` draws).
    pub fn next_u64(&mut self) -> u64 {
        let hi = u64::from(self.next_u32());
        let lo = u64::from(self.next_u32());
        (hi << 32) | lo
    }

    /// A uniform `f64` in `[0, 1)`, using all 53 bits of float precision.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * TWO_POW_NEG_53
    }

    /// A uniform `f64` in `[lo, hi)`. Returns `lo` if `hi <= lo`.
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        if hi <= lo {
            return lo;
        }
        clamp_below(lo + (hi - lo) * self.next_f64(), lo, hi)
    }

    /// A uniform integer in `[lo, hi)`, without modulo bias. Returns `lo` if
    /// `hi <= lo`.
    ///
    /// `next_u32() % n` would favour small values whenever `n` does not
    /// divide 2^32. Lemire's method multiplies instead and rejects the few
    /// draws that would make the result uneven.
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        if hi <= lo {
            return lo;
        }
        let n = hi - lo;
        let threshold = n.wrapping_neg() % n;
        loop {
            let m = u64::from(self.next_u32()) * u64::from(n);
            if (m as u32) >= threshold {
                return lo + (m >> 32) as u32;
            }
        }
    }

    /// `true` with probability `p` (clamped to `[0, 1]`).
    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }

    /// A standard normal draw (mean 0, standard deviation 1).
    ///
    /// **Box–Muller**: two independent uniforms `u₁, u₂` become two
    /// independent normals `√(−2 ln u₁)·cos(2π u₂)` and `…·sin(2π u₂)`. The
    /// second is saved and returned by the next call.
    pub fn normal(&mut self) -> f64 {
        if let Some(z) = self.spare_normal.take() {
            return z;
        }
        // 1 − [0, 1) = (0, 1], so ln never sees zero.
        let u1 = 1.0 - self.next_f64();
        let u2 = self.next_f64();
        let (first, second) = box_muller(u1, u2);
        self.spare_normal = Some(second);
        first
    }

    /// A normal draw with the given mean and standard deviation.
    pub fn normal_with(&mut self, mean: f64, std_dev: f64) -> f64 {
        mean + std_dev * self.normal()
    }
}

/// Keeps `x` half-open below `hi`: rounding in `lo + span·u` can land
/// exactly on `hi`, which is then replaced by the float just below it (but
/// never below `lo`). Works for any sign of `hi`.
fn clamp_below(x: f64, lo: f64, hi: f64) -> f64 {
    if x >= hi {
        float::next_down(hi).max(lo)
    } else {
        x
    }
}

/// Box–Muller: uniforms `u1` in (0, 1] and `u2` in [0, 1) → two independent
/// standard normals `(r·cos 2πu₂, r·sin 2πu₂)` with `r = √(−2 ln u₁)`.
fn box_muller(u1: f64, u2: f64) -> (f64, f64) {
    let radius = float::sqrt(-2.0 * float::ln(u1));
    let (s, c) = float::sin_cos(core::f64::consts::TAU * u2);
    (radius * c, radius * s)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The PCG32 reference sequence for seed 42, stream 54 (from the
    /// pcg-c demo) pins the algorithm bit for bit across platforms.
    #[test]
    fn pcg32_matches_reference_sequence() {
        let mut rng = Rng::with_stream(42, 54);
        let got: [u32; 6] = core::array::from_fn(|_| rng.next_u32());
        assert_eq!(
            got,
            [0xa15c02b7, 0x7b47f409, 0xba1d3330, 0x83d2f293, 0xbfa4784b, 0xcbed606e]
        );
    }

    /// Same seed gives the same sequence; different seeds or streams differ;
    /// clones continue identically (snapshot safety).
    #[test]
    fn seeding_and_streams_are_reproducible() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        assert_eq!(a.next_u64(), b.next_u64());
        assert_ne!(Rng::new(7).next_u32(), Rng::new(8).next_u32());
        assert_ne!(
            Rng::with_stream(7, 1).next_u32(),
            Rng::with_stream(7, 2).next_u32()
        );
        let mut c = a.clone();
        assert_eq!(a.normal(), c.normal());
        assert_eq!(a.normal(), c.normal());
    }

    /// next_u64 concatenates two u32 draws high-first.
    #[test]
    fn next_u64_is_two_u32_draws() {
        let mut a = Rng::new(3);
        let mut b = a.clone();
        let hi = u64::from(b.next_u32());
        let lo = u64::from(b.next_u32());
        assert_eq!(a.next_u64(), (hi << 32) | lo);
    }

    /// Floats stay in [0, 1) and ranges stay half-open; empty ranges
    /// return lo.
    #[test]
    fn floats_and_ranges_stay_in_bounds() {
        let mut rng = Rng::new(1);
        for _ in 0..10_000 {
            let f = rng.next_f64();
            assert!((0.0..1.0).contains(&f));
            let r = rng.range(-2.0, 3.0);
            assert!((-2.0..3.0).contains(&r));
            let i = rng.range_u32(5, 12);
            assert!((5..12).contains(&i));
        }
        assert_eq!(rng.range(4.0, 4.0), 4.0);
        assert_eq!(rng.range(4.0, 1.0), 4.0);
        assert_eq!(rng.range_u32(9, 9), 9);
        assert_eq!(rng.range_u32(9, 2), 9);
    }

    /// range_u32 must reach every value with no visible bias, including a
    /// span that does not divide 2^32.
    #[test]
    fn range_u32_is_unbiased_and_covers_span() {
        let mut rng = Rng::new(99);
        let mut counts = [0u32; 3];
        for _ in 0..30_000 {
            counts[rng.range_u32(0, 3) as usize] += 1;
        }
        for c in counts {
            assert!((9_400..10_600).contains(&c), "{counts:?}");
        }
        assert!(rng.range_u32(0, u32::MAX) < u32::MAX);
    }

    /// chance honours its probability, including the 0 and 1 extremes.
    #[test]
    fn chance_honours_probability() {
        let mut rng = Rng::new(5);
        assert!((0..1000).all(|_| !rng.chance(0.0)));
        assert!((0..1000).all(|_| rng.chance(1.0)));
        let hits = (0..10_000).filter(|_| rng.chance(0.25)).count();
        assert!((2_300..2_700).contains(&hits));
    }

    /// Normals have mean ≈ 0 and variance ≈ 1; normal_with shifts and scales.
    #[test]
    fn normal_has_unit_moments() {
        let mut rng = Rng::new(11);
        let n = 20_000;
        let draws: [f64; 20_000] = core::array::from_fn(|_| rng.normal());
        let mean = draws.iter().sum::<f64>() / n as f64;
        let var = draws.iter().map(|z| (z - mean) * (z - mean)).sum::<f64>() / n as f64;
        assert!(mean.abs() < 0.03, "mean {mean}");
        assert!((var - 1.0).abs() < 0.05, "var {var}");
        let mut a = Rng::new(2);
        let mut b = Rng::new(2);
        assert_eq!(a.normal_with(10.0, 3.0), 10.0 + 3.0 * b.normal());
    }

    /// A value that rounded onto hi is pulled just below it, but never
    /// below lo; values already below hi pass through.
    #[test]
    fn clamp_below_keeps_range_half_open() {
        assert_eq!(clamp_below(0.5, 0.0, 1.0), 0.5);
        let below_one = f64::from_bits(1.0f64.to_bits() - 1);
        assert_eq!(clamp_below(1.0, 0.0, 1.0), below_one);
        assert!(clamp_below(1.0, 0.0, 1.0) < 1.0);
        let lo = below_one;
        assert_eq!(clamp_below(1.0, lo, 1.0), lo);
        // hi = 0 used to underflow the bit pattern; negative hi used to
        // step upward past hi.
        assert!(clamp_below(0.0, -1.0, 0.0) < 0.0);
        assert!(clamp_below(-1.0, -2.0, -1.0) < -1.0);
        assert!(clamp_below(-1.0, -2.0, -1.0) >= -2.0);
    }

    /// Box–Muller with u2 = 0 puts everything in the cosine output; u1 = 1
    /// gives radius 0; a known u1 gives the expected radius.
    #[test]
    fn box_muller_known_values() {
        assert_eq!(box_muller(1.0, 0.3), (0.0, 0.0));
        let r = (-2.0 * (0.5f64).ln()).sqrt();
        let (c, s) = box_muller(0.5, 0.0);
        assert!((c - r).abs() < 1e-15 && s.abs() < 1e-15);
        let (c, s) = box_muller(0.5, 0.25);
        assert!(c.abs() < 1e-15 && (s - r).abs() < 1e-15);
    }
}
