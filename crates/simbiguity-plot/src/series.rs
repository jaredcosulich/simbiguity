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
