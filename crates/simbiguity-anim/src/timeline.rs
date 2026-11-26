//! Timelines: clips placed in sequence or in parallel, then seeked.
//!
//! A timeline does not own values; it owns *timing*. Each clip has a start
//! and a duration, and [`Timeline::progress`] says how far through a clip a
//! moment is. Feed that progress to a [`crate::Tween`] or [`crate::Track`].
//! Seeking is exact: `progress(id, t)` depends on nothing but `t`.

use alloc::vec::Vec;

/// A clip on a timeline, in the order clips were added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClipId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq)]
struct Clip {
    start: f64,
    duration: f64,
}

/// A set of timed clips.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Timeline {
    clips: Vec<Clip>,
    looping: bool,
}

impl Timeline {
    /// An empty timeline.
    pub fn new() -> Timeline {
        Timeline::default()
    }

    /// Makes [`Timeline::progress`] wrap around [`Timeline::duration`].
    pub fn looping(mut self, looping: bool) -> Timeline {
        self.looping = looping;
        self
    }

    /// Places a clip at an absolute `start`.
    pub fn at(&mut self, start: f64, duration: f64) -> ClipId {
        self.clips.push(Clip {
            start,
            duration: duration.max(0.0),
        });
        ClipId(self.clips.len() - 1)
    }

    /// Places a clip right after everything so far (sequence).
    pub fn then(&mut self, duration: f64) -> ClipId {
        let start = self.duration();
        self.at(start, duration)
    }

    /// Places a clip starting with clip `with` (parallel).
    pub fn with(&mut self, with: ClipId, duration: f64) -> ClipId {
        let start = self.clips[with.0].start;
        self.at(start, duration)
    }

    /// When a clip starts.
    pub fn start(&self, id: ClipId) -> f64 {
        self.clips[id.0].start
    }

    /// When the last clip ends.
    pub fn duration(&self) -> f64 {
        self.clips
            .iter()
            .map(|c| c.start + c.duration)
            .fold(0.0, f64::max)
    }

    /// The timeline time a moment `t` maps to: `t` itself, or wrapped into
    /// `[0, duration)` when looping.
    pub fn local_time(&self, t: f64) -> f64 {
        let d = self.duration();
        if !self.looping || d <= 0.0 {
            return t;
        }
        let wrapped = t % d;
        if wrapped < 0.0 {
            wrapped + d
        } else {
            wrapped
        }
    }

    /// Linear progress `0..=1` through clip `id` at time `t`.
    pub fn progress(&self, id: ClipId, t: f64) -> f64 {
        let c = self.clips[id.0];
        let local = self.local_time(t) - c.start;
        if c.duration <= 0.0 {
            return if local >= 0.0 { 1.0 } else { 0.0 };
        }
        (local / c.duration).clamp(0.0, 1.0)
    }

    /// `true` while `t` is inside clip `id`.
    pub fn is_active(&self, id: ClipId, t: f64) -> bool {
        let c = self.clips[id.0];
        let local = self.local_time(t) - c.start;
        local >= 0.0 && local < c.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// then places clips back to back and with starts a clip alongside another.
    #[test]
    fn then_sequences_and_with_runs_in_parallel() {
        let mut tl = Timeline::new();
        let a = tl.then(1.0);
        let b = tl.then(2.0);
        let c = tl.with(a, 0.5);
        assert_eq!((tl.start(a), tl.start(b), tl.start(c)), (0.0, 1.0, 0.0));
        assert_eq!(tl.duration(), 3.0);
    }

    /// Seeking is exact: progress is a pure function of time and clamps to the clip.
    #[test]
    fn seek_gives_exact_progress() {
        let mut tl = Timeline::new();
        let a = tl.at(1.0, 2.0);
        assert_eq!(tl.progress(a, 0.0), 0.0);
        assert_eq!(tl.progress(a, 2.0), 0.5);
        assert_eq!(tl.progress(a, 3.0), 1.0);
        assert_eq!(tl.progress(a, 9.0), 1.0);
        assert_eq!(tl.progress(a, 2.0), tl.progress(a, 2.0));
    }

    /// A looping timeline wraps time, including negative times.
    #[test]
    fn looping_wraps_time() {
        let mut tl = Timeline::new().looping(true);
        let a = tl.then(2.0);
        assert_eq!(tl.local_time(5.0), 1.0);
        assert_eq!(tl.local_time(-0.5), 1.5);
        assert_eq!(tl.progress(a, 4.5), 0.25);
    }

    /// A non-looping or empty timeline leaves time alone.
    #[test]
    fn non_looping_and_empty_timelines_pass_time_through() {
        assert_eq!(Timeline::new().looping(true).local_time(7.0), 7.0);
        let mut tl = Timeline::new();
        tl.then(1.0);
        assert_eq!(tl.local_time(7.0), 7.0);
    }

    /// Zero-length clips jump from 0 to 1 at their start; negative durations become zero.
    #[test]
    fn zero_length_clips_jump() {
        let mut tl = Timeline::new();
        let a = tl.at(1.0, -4.0);
        assert_eq!(tl.progress(a, 0.99), 0.0);
        assert_eq!(tl.progress(a, 1.0), 1.0);
        assert!(!tl.is_active(a, 1.0));
    }

    /// is_active is true inside a clip, false at its end and outside it.
    #[test]
    fn is_active_covers_the_half_open_clip() {
        let mut tl = Timeline::new();
        let a = tl.at(1.0, 1.0);
        assert!(!tl.is_active(a, 0.5));
        assert!(tl.is_active(a, 1.0));
        assert!(tl.is_active(a, 1.5));
        assert!(!tl.is_active(a, 2.0));
    }
}
