//! Solving `A·x = b` with LU decomposition and partial pivoting.
//!
//! Many simulations reduce to "n linear equations in n unknowns": the node
//! voltages of a circuit, the constraint forces holding a ragdoll together.
//! **Gaussian elimination** solves them by subtracting multiples of one row
//! from the rows below until the matrix is upper-triangular, then solving
//! from the bottom up. Recording those multipliers gives `P·A = L·U` (a
//! permutation, a lower- and an upper-triangular matrix), so the same `A`
//! can be re-solved for a new `b` cheaply.
//!
//! **Why pivoting matters.** Elimination divides by the diagonal entry (the
//! *pivot*). A zero pivot is a division by zero even when the system has a
//! perfectly good answer; a tiny pivot multiplies rounding errors by its
//! reciprocal. *Partial pivoting* swaps in the row with the largest entry in
//! the current column before each step, which keeps every multiplier ≤ 1.

use alloc::vec::Vec;
use core::fmt;

use crate::float;
use crate::mat::DenseMatrix;

/// Why a linear system could not be solved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveError {
    /// `A` is not square, so it cannot have a unique solution.
    NotSquare {
        /// Rows of `A`.
        rows: usize,
        /// Columns of `A`.
        cols: usize,
    },
    /// `b` (or the matrix being re-solved) has the wrong length.
    DimensionMismatch {
        /// Expected length (the size of `A`).
        expected: usize,
        /// Length actually given.
        got: usize,
    },
    /// The matrix is singular (or numerically indistinguishable from it): at
    /// elimination step `pivot_row` no remaining row had a usable pivot, so
    /// unknown `pivot_row` is not pinned down by the equations. In a circuit
    /// this is a floating node or a loop of ideal voltage sources.
    Singular {
        /// The elimination step (and unknown index) that failed.
        pivot_row: usize,
    },
}

impl fmt::Display for SolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolveError::NotSquare { rows, cols } => {
                write!(
                    f,
                    "matrix is {rows}×{cols}, but solving needs a square matrix"
                )
            }
            SolveError::DimensionMismatch { expected, got } => {
                write!(f, "right-hand side has length {got}, expected {expected}")
            }
            SolveError::Singular { pivot_row } => write!(
                f,
                "matrix is singular: unknown {pivot_row} is not determined by the equations"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SolveError {}

/// An LU decomposition `P·A = L·U`, stored compactly: `U` on and above the
/// diagonal, the multipliers of `L` (whose diagonal is all ones) below it.
#[derive(Debug, Clone, PartialEq)]
pub struct Lu {
    lu: DenseMatrix,
    /// `perm[i]` is the row of the original `A` that ended up in row `i`.
    perm: Vec<usize>,
}

impl Lu {
    /// Factors a square matrix, or reports why it cannot be.
    ///
    /// A pivot counts as zero when it is below `n · ε · max|A|`, i.e. smaller
    /// than the rounding noise the elimination itself could have produced.
    pub fn decompose(a: &DenseMatrix) -> Result<Lu, SolveError> {
        let n = a.rows();
        if a.cols() != n {
            return Err(SolveError::NotSquare {
                rows: n,
                cols: a.cols(),
            });
        }
        let max_abs = max_abs_entry(a);
        let tolerance = max_abs * n as f64 * f64::EPSILON;
        let mut lu = a.clone();
        let mut perm: Vec<usize> = (0..n).collect();
        for k in 0..n {
            let (best, best_abs) = choose_pivot(&lu, k);
            if best_abs <= tolerance || max_abs == 0.0 {
                return Err(SolveError::Singular { pivot_row: k });
            }
            lu.swap_rows(k, best);
            perm.swap(k, best);
            eliminate_below(&mut lu, k);
        }
        Ok(Lu { lu, perm })
    }

    /// Size `n` of the factored `n × n` system.
    pub fn size(&self) -> usize {
        self.perm.len()
    }

    /// Solves `A·x = b` for the factored `A`.
    pub fn solve(&self, b: &[f64]) -> Result<Vec<f64>, SolveError> {
        check_rhs_len(self.size(), b)?;
        let mut x = forward_substitute(&self.lu, &self.perm, b);
        back_substitute(&self.lu, &mut x);
        Ok(x)
    }
}

/// `Ok` when `b` has length `n`, else the mismatch.
fn check_rhs_len(n: usize, b: &[f64]) -> Result<(), SolveError> {
    if b.len() == n {
        Ok(())
    } else {
        Err(SolveError::DimensionMismatch {
            expected: n,
            got: b.len(),
        })
    }
}

/// The largest `|entry|` of `a` (0 for an empty matrix): the scale against
/// which a pivot counts as "zero".
fn max_abs_entry(a: &DenseMatrix) -> f64 {
    (0..a.rows())
        .flat_map(|r| a.row(r).iter().copied())
        .map(float::abs)
        .fold(0.0, f64::max)
}

/// Partial pivoting: the row at or below `k` with the largest `|entry|` in
/// column `k`, and that magnitude. Ties keep the earliest row.
fn choose_pivot(lu: &DenseMatrix, k: usize) -> (usize, f64) {
    (k..lu.rows())
        .map(|r| (r, float::abs(lu[(r, k)])))
        .fold((k, -1.0), |acc, cur| if cur.1 > acc.1 { cur } else { acc })
}

/// One elimination step: zeroes column `k` below the pivot by subtracting
/// multiples of row `k`, storing each multiplier where it zeroed an entry.
fn eliminate_below(lu: &mut DenseMatrix, k: usize) {
    let n = lu.rows();
    let pivot = lu[(k, k)];
    for r in k + 1..n {
        let factor = lu[(r, k)] / pivot;
        lu[(r, k)] = factor;
        for c in k + 1..n {
            let delta = factor * lu[(k, c)];
            lu[(r, c)] -= delta;
        }
    }
}

/// Forward substitution: solves `L·y = P·b` (L's unit diagonal implied).
fn forward_substitute(lu: &DenseMatrix, perm: &[usize], b: &[f64]) -> Vec<f64> {
    let mut y: Vec<f64> = perm.iter().map(|&p| b[p]).collect();
    for r in 0..y.len() {
        let s: f64 = (0..r).map(|c| lu[(r, c)] * y[c]).sum();
        y[r] -= s;
    }
    y
}

/// Back substitution: overwrites `y` with the solution of `U·x = y`.
fn back_substitute(lu: &DenseMatrix, y: &mut [f64]) {
    let n = y.len();
    for r in (0..n).rev() {
        let s: f64 = (r + 1..n).map(|c| lu[(r, c)] * y[c]).sum();
        y[r] = (y[r] - s) / lu[(r, r)];
    }
}

/// Solves `A·x = b` in one call. Use [`Lu`] directly to re-solve the same
/// `A` for several right-hand sides.
///
/// An empty (0 × 0) system has the empty solution.
pub fn lu_solve(a: &DenseMatrix, b: &[f64]) -> Result<Vec<f64>, SolveError> {
    if a.rows() == a.cols() {
        check_rhs_len(a.rows(), b)?;
    }
    Lu::decompose(a)?.solve(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;

    fn m(rows: usize, cols: usize, data: &[f64]) -> DenseMatrix {
        DenseMatrix::from_row_major(rows, cols, data.to_vec()).unwrap()
    }

    fn close(a: &[f64], b: &[f64]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-12)
    }

    /// A well-posed 3×3 system must solve to its known answer, and the
    /// result must reproduce b when multiplied back.
    #[test]
    fn lu_solve_solves_a_known_system() {
        let a = m(3, 3, &[2.0, 1.0, -1.0, -3.0, -1.0, 2.0, -2.0, 1.0, 2.0]);
        let b = [8.0, -11.0, -3.0];
        let x = lu_solve(&a, &b).unwrap();
        assert!(close(&x, &[2.0, 3.0, -1.0]));
        assert!(close(&a.mul_vec(&x).unwrap(), &b));
    }

    /// A zero in the leading position must be pivoted around rather than
    /// reported as singular.
    #[test]
    fn lu_solve_pivots_past_a_zero_leading_entry() {
        let a = m(2, 2, &[0.0, 1.0, 1.0, 0.0]);
        assert!(close(&lu_solve(&a, &[3.0, 4.0]).unwrap(), &[4.0, 3.0]));
    }

    /// A factored matrix must be reusable for several right-hand sides.
    #[test]
    fn lu_resolves_for_several_rhs() {
        let a = m(2, 2, &[4.0, 3.0, 6.0, 3.0]);
        let lu = Lu::decompose(&a).unwrap();
        assert_eq!(lu.size(), 2);
        for b in [[10.0, 12.0], [1.0, 0.0], [0.0, 0.0]] {
            let x = lu.solve(&b).unwrap();
            assert!(close(&a.mul_vec(&x).unwrap(), &b));
        }
    }

    /// Singular, non-square and wrongly sized systems must report the
    /// specific error variant, not panic.
    #[test]
    fn lu_solve_reports_each_error_variant() {
        let singular = m(2, 2, &[1.0, 2.0, 2.0, 4.0]);
        assert_eq!(
            lu_solve(&singular, &[1.0, 2.0]),
            Err(SolveError::Singular { pivot_row: 1 })
        );
        assert_eq!(
            lu_solve(&DenseMatrix::zeros(2, 2), &[0.0, 0.0]),
            Err(SolveError::Singular { pivot_row: 0 })
        );
        assert_eq!(
            lu_solve(&m(2, 3, &[1.0; 6]), &[1.0, 2.0]),
            Err(SolveError::NotSquare { rows: 2, cols: 3 })
        );
        assert_eq!(
            lu_solve(&DenseMatrix::identity(3), &[1.0]),
            Err(SolveError::DimensionMismatch {
                expected: 3,
                got: 1
            })
        );
        let lu = Lu::decompose(&DenseMatrix::identity(2)).unwrap();
        assert_eq!(
            lu.solve(&[1.0, 2.0, 3.0]),
            Err(SolveError::DimensionMismatch {
                expected: 2,
                got: 3
            })
        );
    }

    /// A 1×1 system is plain division.
    #[test]
    fn lu_solve_one_by_one_system() {
        assert_eq!(lu_solve(&m(1, 1, &[4.0]), &[2.0]), Ok(vec![0.5]));
    }

    /// The empty system has the empty solution.
    #[test]
    fn lu_solve_empty_system() {
        assert_eq!(lu_solve(&DenseMatrix::zeros(0, 0), &[]), Ok(vec![]));
    }

    /// Error messages must name the problem in plain words.
    #[test]
    fn solve_error_display_messages() {
        assert!(SolveError::Singular { pivot_row: 2 }
            .to_string()
            .contains("unknown 2"));
        assert!(SolveError::NotSquare { rows: 2, cols: 3 }
            .to_string()
            .contains("2×3"));
        assert!(SolveError::DimensionMismatch {
            expected: 3,
            got: 1
        }
        .to_string()
        .contains("length 1"));
    }

    /// The length check passes on a match and reports both sizes otherwise.
    #[test]
    fn check_rhs_len_accepts_and_rejects() {
        assert_eq!(check_rhs_len(2, &[1.0, 2.0]), Ok(()));
        assert_eq!(check_rhs_len(0, &[]), Ok(()));
        assert_eq!(
            check_rhs_len(2, &[1.0]),
            Err(SolveError::DimensionMismatch {
                expected: 2,
                got: 1
            })
        );
    }

    /// The largest magnitude must ignore sign and be 0 for empty/zero input.
    #[test]
    fn max_abs_entry_ignores_sign() {
        assert_eq!(max_abs_entry(&m(2, 2, &[1.0, -7.0, 3.0, 2.0])), 7.0);
        assert_eq!(max_abs_entry(&DenseMatrix::zeros(3, 3)), 0.0);
        assert_eq!(max_abs_entry(&DenseMatrix::zeros(0, 0)), 0.0);
    }

    /// The pivot search must look only at or below row k, compare
    /// magnitudes, and keep the earliest row on ties.
    #[test]
    fn choose_pivot_picks_largest_magnitude_at_or_below_k() {
        let a = m(3, 3, &[9.0, 0.0, 0.0, -5.0, 1.0, 0.0, 2.0, -8.0, 0.0]);
        assert_eq!(choose_pivot(&a, 0), (0, 9.0));
        assert_eq!(choose_pivot(&a, 1), (2, 8.0));
        let ties = m(2, 2, &[3.0, 0.0, -3.0, 0.0]);
        assert_eq!(choose_pivot(&ties, 0), (0, 3.0));
    }

    /// Elimination must zero the column below the pivot (storing the
    /// multipliers there) and update the trailing block.
    #[test]
    fn eliminate_below_stores_multipliers_and_updates_rows() {
        let mut a = m(2, 2, &[2.0, 1.0, 4.0, 5.0]);
        eliminate_below(&mut a, 0);
        assert_eq!(a.row(0), &[2.0, 1.0]);
        assert_eq!(a.row(1), &[2.0, 3.0]);
    }

    /// Forward substitution permutes b and applies the unit-lower factor.
    #[test]
    fn forward_substitute_applies_perm_and_lower_factor() {
        let lu = m(2, 2, &[2.0, 1.0, 0.5, 3.0]);
        assert_eq!(
            forward_substitute(&lu, &[1, 0], &[4.0, 6.0]),
            vec![6.0, 1.0]
        );
        assert_eq!(
            forward_substitute(&lu, &[0, 1], &[4.0, 6.0]),
            vec![4.0, 4.0]
        );
    }

    /// Back substitution solves the upper factor from the bottom up.
    #[test]
    fn back_substitute_solves_upper_factor() {
        let lu = m(2, 2, &[2.0, 1.0, 0.5, 4.0]);
        let mut y = [5.0, 8.0];
        back_substitute(&lu, &mut y);
        assert_eq!(y, [1.5, 2.0]);
    }
}
