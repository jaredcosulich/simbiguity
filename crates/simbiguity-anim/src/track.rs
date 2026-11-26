//! Keyframe tracks: a value pinned at several times, eased in between.

use alloc::vec::Vec;

use crate::easing::Ease;
use crate::lerp::Lerp;

/// A value at a moment, and the easing used to arrive at it from the
/// previous keyframe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Keyframe<T: Lerp> {
    /// Time in seconds.
    pub time: f64,
    /// The value at `time`.
    pub value: T,
    /// The curve of the segment that *ends* at this keyframe.
    pub ease: Ease,
}

/// Keyframes sorted by time. Before the first key the track holds the first
/// value; after the last it holds the last.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Track<T: Lerp> {
    keys: Vec<Keyframe<T>>,
}

impl<T: Lerp> Track<T> {
    /// An empty track (sampling it returns `None`).
    pub fn new() -> Track<T> {
        Track { keys: Vec::new() }
    }

    /// Adds a keyframe arriving with `ease`. Keys may be added in any order;
    /// a key at an existing time replaces it.
    pub fn key(mut self, time: f64, value: T, ease: Ease) -> Track<T> {
        self.insert(Keyframe { time, value, ease });
        self
    }

    /// Inserts a keyframe, keeping the track sorted.
    pub fn insert(&mut self, k: Keyframe<T>) {
        match self.keys.iter().position(|e| e.time >= k.time) {
            Some(i) if self.keys[i].time == k.time => self.keys[i] = k,
            Some(i) => self.keys.insert(i, k),
            None => self.keys.push(k),
        }
    }

    /// The keyframes, sorted by time.
    pub fn keys(&self) -> &[Keyframe<T>] {
        &self.keys
    }

    /// Time of the last keyframe (0 for an empty track).
    pub fn duration(&self) -> f64 {
        self.keys.last().map_or(0.0, |k| k.time)
    }

    /// The value at time `t`, or `None` if the track has no keys.
    pub fn sample(&self, t: f64) -> Option<T> {
        let first = self.keys.first()?;
        if t <= first.time {
            return Some(first.value);
        }
        // First key strictly after t: the segment is (i-1, i).
        match self.keys.iter().position(|k| k.time > t) {
            None => self.keys.last().map(|k| k.value),
            Some(i) => {
                let (a, b) = (&self.keys[i - 1], &self.keys[i]);
                let u = (t - a.time) / (b.time - a.time);
                Some(a.value.lerp(b.value, b.ease.apply(u)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An empty track samples to None and has zero duration.
    #[test]
    fn empty_track_has_nothing_to_sample() {
        let t: Track<f64> = Track::new();
        assert_eq!(t.sample(0.0), None);
        assert_eq!(t.duration(), 0.0);
    }

    /// Before the first key the track holds it, after the last it holds that.
    #[test]
    fn holds_first_and_last_values() {
        let t = Track::new()
            .key(1.0, 10.0, Ease::Linear)
            .key(3.0, 30.0, Ease::Linear);
        assert_eq!(t.sample(-5.0), Some(10.0));
        assert_eq!(t.sample(1.0), Some(10.0));
        assert_eq!(t.sample(3.0), Some(30.0));
        assert_eq!(t.sample(99.0), Some(30.0));
        assert_eq!(t.duration(), 3.0);
    }

    /// Between keys the segment uses the easing of the key it arrives at.
    #[test]
    fn segments_use_the_arriving_keys_ease() {
        let t = Track::new()
            .key(0.0, 0.0, Ease::Linear)
            .key(1.0, 1.0, Ease::QuadIn)
            .key(2.0, 2.0, Ease::Linear);
        assert_eq!(t.sample(0.5), Some(0.25));
        assert_eq!(t.sample(1.5), Some(1.5));
    }

    /// Keys added out of order are sorted, and a key at an existing time replaces it.
    #[test]
    fn keys_sort_and_replace() {
        let t = Track::new()
            .key(2.0, 2.0, Ease::Linear)
            .key(0.0, 0.0, Ease::Linear)
            .key(1.0, 1.0, Ease::Linear)
            .key(1.0, 5.0, Ease::Linear);
        let times: std::vec::Vec<f64> = t.keys().iter().map(|k| k.time).collect();
        assert_eq!(times, [0.0, 1.0, 2.0]);
        assert_eq!(t.sample(1.0), Some(5.0));
    }
}
