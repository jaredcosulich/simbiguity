//! A damped spring that follows a target: the bulb warming up and cooling
//! down, a camera easing to a new angle, anything that should *respond*
//! rather than jump.
//!
//! The spring obeys `x'' = −ω²(x − target) − 2ζω·x'`. [`Spring::step`]
//! uses the exact closed-form solution, not numerical integration, so a
//! step of any size is stable and stepping `dt` twice equals stepping `2·dt`
//! (up to rounding).
//!
//! - `ζ = 1` (**critically damped**) reaches the target as fast as possible
//!   without overshooting.
//! - `ζ < 1` (**under-damped**) overshoots and wobbles.
//! - `ζ > 1` (**over-damped**) creeps in slowly.

use simbiguity_math::float;

/// A one-dimensional damped spring follower.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring {
    value: f64,
    velocity: f64,
    /// Where the spring is pulling toward.
    pub target: f64,
    /// Natural angular frequency ω, in radians per second: stiffness.
    pub omega: f64,
    /// Damping ratio ζ: 1 critical, below 1 bouncy, above 1 sluggish.
    pub zeta: f64,
}

impl Spring {
    /// A spring at rest at `value` (also its target).
    pub fn new(value: f64, omega: f64, zeta: f64) -> Spring {
        Spring {
            value,
            velocity: 0.0,
            target: value,
            omega: omega.max(0.0),
            zeta: zeta.max(0.0),
        }
    }

    /// A critically-damped spring at rest at `value`.
    pub fn critical(value: f64, omega: f64) -> Spring {
        Spring::new(value, omega, 1.0)
    }

    /// The current value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// The current velocity.
    pub fn velocity(&self) -> f64 {
        self.velocity
    }

    /// Teleports to `value` and stops (target unchanged).
    pub fn snap(&mut self, value: f64) {
        self.value = value;
        self.velocity = 0.0;
    }

    /// Advances by `dt` seconds (negative or NaN `dt` does nothing).
    pub fn step(&mut self, dt: f64) {
        if dt.is_nan() || dt <= 0.0 {
            return;
        }
        let (e, v) = evolve(
            self.value - self.target,
            self.velocity,
            self.omega,
            self.zeta,
            dt,
        );
        self.value = self.target + e;
        self.velocity = v;
    }

    /// A time by which this spring, released from rest, is guaranteed to
    /// stay within `tolerance × |initial error|` of its target. `INFINITY`
    /// for a spring with no stiffness or (for ζ = 0) no damping.
    pub fn settling_time(&self, tolerance: f64) -> f64 {
        settling_time(self.omega, self.zeta, tolerance)
    }
}

/// Closed-form evolution of error `e` and velocity `v` over `t`.
fn evolve(e: f64, v: f64, w: f64, z: f64, t: f64) -> (f64, f64) {
    if w == 0.0 {
        return (e + v * t, v); // no spring: coast
    }
    if (z - 1.0).abs() < 1e-9 {
        let decay = float::exp(-w * t);
        let b = v + w * e;
        return ((e + b * t) * decay, (v - w * b * t) * decay);
    }
    if z < 1.0 {
        let wd = w * float::sqrt(1.0 - z * z);
        let decay = float::exp(-z * w * t);
        let (s, c) = float::sin_cos(wd * t);
        let b = (v + z * w * e) / wd;
        return (
            decay * (e * c + b * s),
            decay * (v * c - (e * wd + z * w * b) * s),
        );
    }
    let root = float::sqrt(z * z - 1.0);
    let (r1, r2) = (-w * (z - root), -w * (z + root));
    let c2 = (v - r1 * e) / (r2 - r1);
    let c1 = e - c2;
    let (x1, x2) = (float::exp(r1 * t), float::exp(r2 * t));
    (c1 * x1 + c2 * x2, r1 * c1 * x1 + r2 * c2 * x2)
}

fn settling_time(w: f64, z: f64, tol: f64) -> f64 {
    if w <= 0.0 || z <= 0.0 || tol <= 0.0 {
        return f64::INFINITY;
    }
    if tol >= 1.0 {
        return 0.0;
    }
    if (z - 1.0).abs() < 1e-9 {
        // |e(t)| = (1 + x)·e^(−x) with x = ωt, decreasing: solve (1+x)e^(−x) = tol
        // by Newton on g(x) = ln(1+x) − x − ln(tol).
        let target = float::ln(tol);
        let mut x = 1.0 - target;
        for _ in 0..64 {
            let g = float::ln(1.0 + x) - x - target;
            let dg = 1.0 / (1.0 + x) - 1.0;
            x -= g / dg;
        }
        return x / w;
    }
    if z < 1.0 {
        // Envelope: e^(−ζωt) / √(1 − ζ²) bounds |e(t)| from rest.
        let k = 1.0 / float::sqrt(1.0 - z * z);
        return float::ln(k / tol) / (z * w);
    }
    let root = float::sqrt(z * z - 1.0);
    let (r1, r2) = (-w * (z - root), -w * (z + root));
    let c2 = -r1 / (r2 - r1);
    let c1 = 1.0 - c2;
    float::ln((c1.abs() + c2.abs()) / tol) / -r1
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Releases a spring from rest at 1 toward 0 and samples its error every `dt`.
    fn trace(omega: f64, zeta: f64, dt: f64, n: usize) -> std::vec::Vec<f64> {
        let mut s = Spring::new(1.0, omega, zeta);
        s.target = 0.0;
        (0..n)
            .map(|_| {
                s.step(dt);
                s.value()
            })
            .collect()
    }

    /// For every damping regime, the spring is within tolerance once its settling time has passed.
    #[test]
    fn settles_by_its_settling_time() {
        for zeta in [0.3, 1.0, 2.5] {
            let mut s = Spring::new(1.0, 6.0, zeta);
            s.target = 0.0;
            let t = s.settling_time(0.01);
            assert!(t.is_finite() && t > 0.0, "zeta {zeta}: {t}");
            let mut elapsed = 0.0;
            while elapsed < t {
                s.step(1.0 / 240.0);
                elapsed += 1.0 / 240.0;
            }
            for _ in 0..2000 {
                assert!(s.value().abs() <= 0.01 + 1e-9, "zeta {zeta}: {}", s.value());
                s.step(1.0 / 240.0);
            }
        }
    }

    /// A critically damped spring never crosses its target.
    #[test]
    fn critical_spring_never_overshoots() {
        for v in trace(9.0, 1.0, 1.0 / 120.0, 2000) {
            assert!(v >= 0.0, "overshot to {v}");
        }
    }

    /// An under-damped spring does overshoot.
    #[test]
    fn under_damped_spring_overshoots() {
        assert!(trace(9.0, 0.2, 1.0 / 120.0, 400).iter().any(|v| *v < -0.1));
    }

    /// Closed form: one step of 2 dt equals two steps of dt.
    #[test]
    fn stepping_is_exact_regardless_of_step_size() {
        for zeta in [0.4, 1.0, 1.8] {
            let mut a = Spring::new(2.0, 4.0, zeta);
            let mut b = a;
            a.target = -1.0;
            b.target = -1.0;
            a.step(0.6);
            b.step(0.3);
            b.step(0.3);
            assert!((a.value() - b.value()).abs() < 1e-9, "zeta {zeta}");
            assert!((a.velocity() - b.velocity()).abs() < 1e-9, "zeta {zeta}");
        }
    }

    /// Zero, negative and NaN steps leave the spring untouched.
    #[test]
    fn non_positive_and_nan_steps_do_nothing() {
        let mut s = Spring::critical(1.0, 5.0);
        s.target = 0.0;
        let before = s;
        s.step(0.0);
        s.step(-1.0);
        s.step(f64::NAN);
        assert_eq!(s, before);
    }

    /// Snap teleports and stops without changing the target.
    #[test]
    fn snap_stops_the_spring_in_place() {
        let mut s = Spring::critical(0.0, 5.0);
        s.target = 1.0;
        s.step(0.1);
        assert!(s.velocity() > 0.0);
        s.snap(0.4);
        assert_eq!((s.value(), s.velocity(), s.target), (0.4, 0.0, 1.0));
    }

    /// A spring with no stiffness coasts at its current velocity.
    #[test]
    fn zero_omega_coasts() {
        assert_eq!(evolve(1.0, 2.0, 0.0, 1.0, 0.5), (2.0, 2.0));
    }

    /// Settling time is infinite without stiffness or damping and zero at full tolerance.
    #[test]
    fn settling_time_edge_cases() {
        assert_eq!(
            Spring::new(0.0, 0.0, 1.0).settling_time(0.01),
            f64::INFINITY
        );
        assert_eq!(
            Spring::new(0.0, 3.0, 0.0).settling_time(0.01),
            f64::INFINITY
        );
        assert_eq!(Spring::new(0.0, 3.0, 1.0).settling_time(0.0), f64::INFINITY);
        assert_eq!(Spring::new(0.0, 3.0, 1.0).settling_time(1.0), 0.0);
        let s = Spring::new(0.0, -2.0, -1.0);
        assert_eq!((s.omega, s.zeta), (0.0, 0.0));
    }

    /// A stiffer spring settles faster.
    #[test]
    fn stiffer_springs_settle_sooner() {
        let slow = Spring::critical(0.0, 2.0).settling_time(0.01);
        let fast = Spring::critical(0.0, 8.0).settling_time(0.01);
        assert!((slow / fast - 4.0).abs() < 1e-9);
    }
}
