//! A fixed-timestep clock.
//!
//! Browsers deliver animation frames at uneven intervals (16 ms, then 17,
//! then 50 when a tab stutters). If a simulation stepped by whatever time
//! passed, its results would depend on the frame rate: a fast machine and a
//! slow one would disagree, and big steps can make physics explode.
//!
//! The fix is the **accumulator** pattern: real time is banked, and the
//! simulation is stepped by a constant `dt` as many whole times as the bank
//! allows. The leftover fraction becomes `alpha`, which a renderer uses to
//! interpolate between the last two states so motion still looks smooth.
//!
//! Time is banked in integer **nanoseconds**, not `f64` seconds, so step
//! counts are exact: sixty frames of 1/60 s always make exactly sixty steps,
//! with no rounding drift over a long session.

use crate::simulation::Seconds;

const NANOS_PER_SECOND: f64 = 1e9;

/// What one [`FixedClock::advance`] call produced.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepCount {
    /// Whole fixed steps to run now.
    pub steps: u32,
    /// Fraction of a step left in the bank, in `[0, 1)`: how far the
    /// displayed moment is between the last state and the next one.
    pub alpha: f64,
    /// Steps that were owed but skipped because of the per-call cap.
    pub dropped: u64,
}

/// Turns uneven real-time frames into a whole number of fixed steps.
#[derive(Debug, Clone, PartialEq)]
pub struct FixedClock {
    step_nanos: u64,
    max_steps: u32,
    accumulator_nanos: u64,
    total_steps: u64,
}

impl FixedClock {
    /// The default cap on steps per [`FixedClock::advance`] call.
    pub const DEFAULT_MAX_STEPS: u32 = 8;

    /// A clock that steps by `step` (rounded to the nearest nanosecond).
    ///
    /// # Panics
    /// If `step` is not finite or rounds to less than one nanosecond.
    pub fn new(step: Seconds) -> FixedClock {
        let step_nanos = to_nanos(step);
        assert!(
            step.0.is_finite() && step_nanos >= 1,
            "FixedClock step must be a finite duration of at least 1 ns"
        );
        FixedClock {
            step_nanos,
            max_steps: Self::DEFAULT_MAX_STEPS,
            accumulator_nanos: 0,
            total_steps: 0,
        }
    }

    /// Caps how many steps one `advance` may return (minimum 1).
    ///
    /// This guards against the **spiral of death**: if stepping takes longer
    /// than the time it simulates, each frame owes more steps than the last
    /// and the app freezes. Past the cap, owed time is dropped instead — the
    /// simulation runs slow for a moment rather than locking up.
    pub fn with_max_steps(mut self, max_steps: u32) -> FixedClock {
        self.max_steps = max_steps.max(1);
        self
    }

    /// The fixed step, as the simulation should receive it.
    pub fn step(&self) -> Seconds {
        nanos_to_seconds(self.step_nanos as f64)
    }

    /// Total steps produced since creation (or the last [`FixedClock::reset`]).
    pub fn total_steps(&self) -> u64 {
        self.total_steps
    }

    /// Simulated time so far: `total_steps × step`.
    pub fn elapsed(&self) -> Seconds {
        nanos_to_seconds(self.total_steps as f64 * self.step_nanos as f64)
    }

    /// Empties the bank and zeroes the step count.
    pub fn reset(&mut self) {
        self.accumulator_nanos = 0;
        self.total_steps = 0;
    }

    /// Banks `real_dt` of wall-clock time and reports how many fixed steps to
    /// run. Negative or NaN durations bank nothing.
    pub fn advance(&mut self, real_dt: Seconds) -> StepCount {
        let banked = self.accumulator_nanos.saturating_add(to_nanos(real_dt));
        let owed = banked / self.step_nanos;
        let remainder = banked % self.step_nanos;
        let steps = owed.min(u64::from(self.max_steps));
        self.accumulator_nanos = remainder;
        self.total_steps += steps;
        StepCount {
            steps: steps as u32,
            alpha: remainder as f64 / self.step_nanos as f64,
            dropped: owed - steps,
        }
    }
}

/// Seconds → whole nanoseconds, rounded; negative/NaN → 0, huge → saturates.
fn to_nanos(s: Seconds) -> u64 {
    let n = s.0 * NANOS_PER_SECOND + 0.5;
    if n.is_nan() || n < 1.0 {
        0
    } else {
        // `as` saturates at u64::MAX for out-of-range floats.
        n as u64
    }
}

/// Nanoseconds (as `f64`, so products of counts fit) → seconds.
fn nanos_to_seconds(nanos: f64) -> Seconds {
    Seconds(nanos / NANOS_PER_SECOND)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sixty frames of 1/60 s must make exactly sixty steps with no drift,
    /// which f64 banking would not guarantee.
    #[test]
    fn sixty_frames_make_exactly_sixty_steps() {
        let mut clock = FixedClock::new(Seconds::from_hz(60.0));
        let total: u32 = (0..60)
            .map(|_| clock.advance(Seconds::from_hz(60.0)).steps)
            .sum();
        assert_eq!(total, 60);
        assert_eq!(clock.total_steps(), 60);
        // The step is 1/60 s rounded to 16_666_667 ns, so elapsed is exactly
        // 60 of those, within 1 ns per step of a true second.
        assert_eq!(clock.elapsed(), Seconds(60.0 * 16_666_667.0 / 1e9));
        assert!((clock.elapsed().get() - 1.0).abs() < 60e-9);
    }

    /// Leftover time is banked as alpha in [0, 1) and paid out on later calls.
    #[test]
    fn remainder_becomes_alpha_and_carries_over() {
        let mut clock = FixedClock::new(Seconds(0.010));
        let c = clock.advance(Seconds(0.025));
        assert_eq!(c.steps, 2);
        assert!((c.alpha - 0.5).abs() < 1e-12);
        assert_eq!(c.dropped, 0);
        let c = clock.advance(Seconds(0.005));
        assert_eq!(c.steps, 1);
        assert_eq!(c.alpha, 0.0);
    }

    /// Past the per-call cap, owed steps are dropped and reported rather
    /// than spiralling; the cap never goes below one.
    #[test]
    fn max_steps_caps_and_reports_dropped() {
        let mut clock = FixedClock::new(Seconds(0.010)).with_max_steps(3);
        let c = clock.advance(Seconds(0.100));
        assert_eq!((c.steps, c.dropped), (3, 7));
        assert_eq!(clock.total_steps(), 3);
        let mut one = FixedClock::new(Seconds(0.010)).with_max_steps(0);
        assert_eq!(one.advance(Seconds(0.050)).steps, 1);
        assert_eq!(
            FixedClock::new(Seconds(1.0)).advance(Seconds(100.0)).steps,
            FixedClock::DEFAULT_MAX_STEPS
        );
    }

    /// Negative and NaN frame times bank nothing; reset empties the bank.
    #[test]
    fn bad_durations_bank_nothing_and_reset_clears() {
        let mut clock = FixedClock::new(Seconds(0.010));
        assert_eq!(clock.advance(Seconds(-1.0)).steps, 0);
        assert_eq!(clock.advance(Seconds(f64::NAN)).steps, 0);
        clock.advance(Seconds(0.015));
        clock.reset();
        assert_eq!(clock.total_steps(), 0);
        assert_eq!(clock.advance(Seconds(0.005)).steps, 0);
    }

    /// A zero, sub-nanosecond or non-finite step is refused at construction.
    #[test]
    #[should_panic(expected = "at least 1 ns")]
    fn new_rejects_zero_step() {
        let _ = FixedClock::new(Seconds(0.0));
    }

    /// Infinite steps would make every advance owe nothing forever.
    #[test]
    #[should_panic(expected = "finite duration")]
    fn new_rejects_infinite_step() {
        let _ = FixedClock::new(Seconds(f64::INFINITY));
    }

    /// The step is reported back rounded to whole nanoseconds.
    #[test]
    fn step_reports_rounded_nanoseconds() {
        assert_eq!(FixedClock::new(Seconds(0.25)).step(), Seconds(0.25));
        assert_eq!(FixedClock::new(Seconds(1.4e-9)).step(), Seconds(1e-9));
    }

    /// to_nanos rounds to nearest, clamps negatives and NaN to 0 and
    /// saturates huge values.
    #[test]
    fn to_nanos_rounds_clamps_and_saturates() {
        assert_eq!(to_nanos(Seconds(1.0)), 1_000_000_000);
        assert_eq!(to_nanos(Seconds(1.6e-9)), 2);
        assert_eq!(to_nanos(Seconds(0.4e-9)), 0);
        assert_eq!(to_nanos(Seconds(-5.0)), 0);
        assert_eq!(to_nanos(Seconds(f64::NAN)), 0);
        assert_eq!(to_nanos(Seconds(1e30)), u64::MAX);
    }

    /// The nanosecond-to-second conversion is the shared inverse of to_nanos.
    #[test]
    fn nanos_to_seconds_converts() {
        assert_eq!(nanos_to_seconds(1e9), Seconds(1.0));
        assert_eq!(nanos_to_seconds(0.0), Seconds::ZERO);
        assert_eq!(nanos_to_seconds(2.5e8), Seconds(0.25));
    }
}
