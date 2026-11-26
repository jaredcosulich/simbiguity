//! Union-find (disjoint sets) over node indices.

use alloc::vec::Vec;

/// Union-find over node indices. The smaller index always becomes the root,
/// so results never depend on the order unions happen in.
pub(crate) struct Dsu(Vec<usize>);

impl Dsu {
    pub(crate) fn new(n: usize) -> Dsu {
        Dsu((0..n).collect())
    }

    pub(crate) fn find(&mut self, mut x: usize) -> usize {
        while self.0[x] != x {
            self.0[x] = self.0[self.0[x]];
            x = self.0[x];
        }
        x
    }

    /// Joins the sets of `a` and `b`; `false` if they were already joined.
    pub(crate) fn union(&mut self, a: usize, b: usize) -> bool {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return false;
        }
        let (lo, hi) = if ra < rb { (ra, rb) } else { (rb, ra) };
        self.0[hi] = lo;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Dsu;

    /// A fresh set structure has every element as its own root.
    #[test]
    fn fresh_elements_are_their_own_roots() {
        let mut d = Dsu::new(4);
        for i in 0..4 {
            assert_eq!(d.find(i), i);
        }
    }

    /// Joining two sets returns true and makes them share a root.
    #[test]
    fn union_joins_two_sets() {
        let mut d = Dsu::new(3);
        assert!(d.union(0, 2));
        assert_eq!(d.find(0), d.find(2));
        assert_ne!(d.find(1), d.find(0));
    }

    /// Joining already-joined elements returns false, which is how loops of
    /// ideal batteries are detected.
    #[test]
    fn union_of_joined_elements_returns_false() {
        let mut d = Dsu::new(3);
        d.union(0, 1);
        d.union(1, 2);
        assert!(!d.union(0, 2));
    }

    /// The lowest index is always the root, whatever order the unions came
    /// in, so the solver's numbering is deterministic.
    #[test]
    fn lowest_index_is_root_regardless_of_order() {
        let mut forward = Dsu::new(5);
        forward.union(1, 4);
        forward.union(4, 3);
        let mut backward = Dsu::new(5);
        backward.union(3, 4);
        backward.union(4, 1);
        for i in [1, 3, 4] {
            assert_eq!(forward.find(i), 1);
            assert_eq!(backward.find(i), 1);
        }
    }

    /// An empty structure is allowed (a circuit with no nodes).
    #[test]
    fn empty_structure_is_allowed() {
        let _ = Dsu::new(0);
    }
}
