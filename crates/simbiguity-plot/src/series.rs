use alloc::vec::Vec;

#[derive(Clone, Debug)]
pub struct Series {
    capacity: usize,
    samples: Vec<(f64, f64)>, // (time, value)
    head: usize,
}

impl Series {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            samples: Vec::with_capacity(capacity),
            head: 0,
        }
    }

    pub fn push(&mut self, time: f64, value: f64) {
        if self.samples.len() < self.capacity {
            self.samples.push((time, value));
        } else {
            self.samples[self.head] = (time, value);
            self.head = (self.head + 1) % self.capacity;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &(f64, f64)> {
        let (first, second) = if self.samples.len() < self.capacity {
            (self.samples.as_slice(), &[][..])
        } else {
            let (a, b) = self.samples.split_at(self.head);
            (b, a)
        };
        first.iter().chain(second.iter())
    }

    pub fn window(&self, duration: f64) -> impl Iterator<Item = &(f64, f64)> {
        let max_time = self.iter().last().map(|&(t, _)| t).unwrap_or(0.0);
        let min_time = max_time - duration;
        self.iter().filter(move |&&(t, _)| t >= min_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// pushing capacity + 3 keeps the newest values in order
    #[test]
    fn pushing_over_capacity_keeps_newest_in_order() {
        let mut s = Series::new(3);
        for i in 0..6 {
            s.push(i as f64, i as f64);
        }
        let values: alloc::vec::Vec<_> = s.iter().map(|&(_, v)| v).collect();
        assert_eq!(values, alloc::vec![3.0, 4.0, 5.0]);
    }

    /// window
    #[test]
    fn window_returns_recent_values() {
        let mut s = Series::new(10);
        s.push(1.0, 1.0);
        s.push(2.0, 2.0);
        s.push(3.0, 3.0);
        let values: alloc::vec::Vec<_> = s.window(1.5).map(|&(_, v)| v).collect();
        assert_eq!(values, alloc::vec![2.0, 3.0]);
    }
}
