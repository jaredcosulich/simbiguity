//! A single value moving from `from` to `to` over a duration.

use crate::easing::Ease;
use crate::lerp::Lerp;

/// One eased transition. Sampling is a pure function of time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tween<T: Lerp> {
    /// Value at (and before) time 0.
    pub from: T,
    /// Value at (and after) `duration`.
    pub to: T,
    /// Length in seconds. Zero or negative means "jump straight to `to`".
    pub duration: f64,
    /// How progress is shaped.
    pub ease: Ease,
}

impl<T: Lerp> Tween<T> {
    /// A linear tween.
    pub fn new(from: T, to: T, duration: f64) -> Tween<T> {
        Tween {
            from,
            to,
            duration,
            ease: Ease::Linear,
        }
    }

    /// The same tween with a different easing curve.
    pub fn with_ease(mut self, ease: Ease) -> Tween<T> {
        self.ease = ease;
        self
    }

    /// Linear progress `0..=1` at time `t` (seconds since the tween started).
    pub fn progress(&self, t: f64) -> f64 {
        if self.duration <= 0.0 {
            return if t >= 0.0 { 1.0 } else { 0.0 };
        }
        (t / self.duration).clamp(0.0, 1.0)
    }

    /// The value at time `t`; clamped to `from` before 0 and `to` after the end.
    pub fn sample(&self, t: f64) -> T {
        self.from.lerp(self.to, self.ease.apply(self.progress(t)))
    }

    /// `true` once `t` has reached the end.
    pub fn is_done(&self, t: f64) -> bool {
        self.progress(t) >= 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A linear tween is from at 0, halfway at the midpoint and to at the end.
    #[test]
    fn linear_tween_samples_evenly() {
        let tw = Tween::new(2.0, 6.0, 4.0);
        assert_eq!(tw.sample(0.0), 2.0);
        assert_eq!(tw.sample(2.0), 4.0);
        assert_eq!(tw.sample(4.0), 6.0);
    }

    /// Sampling clamps before the start and after the end.
    #[test]
    fn samples_clamp_outside_the_duration() {
        let tw = Tween::new(0.0, 1.0, 1.0);
        assert_eq!(tw.sample(-1.0), 0.0);
        assert_eq!(tw.sample(3.0), 1.0);
        assert!(!tw.is_done(0.5));
        assert!(tw.is_done(1.0));
    }

    /// with_ease shapes the progress.
    #[test]
    fn with_ease_shapes_progress() {
        let tw = Tween::new(0.0, 1.0, 2.0).with_ease(Ease::QuadIn);
        assert_eq!(tw.sample(1.0), 0.25);
        assert_eq!(tw.progress(1.0), 0.5);
    }

    /// A zero-length tween jumps to its end value at time 0.
    #[test]
    fn zero_duration_jumps() {
        let tw = Tween::new(3.0, 7.0, 0.0);
        assert_eq!(tw.progress(-0.1), 0.0);
        assert_eq!(tw.sample(0.0), 7.0);
        assert!(tw.is_done(0.0));
    }
}
