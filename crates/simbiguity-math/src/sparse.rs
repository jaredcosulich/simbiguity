#![allow(missing_docs)]
use crate::linsolve::SolveError;
use alloc::vec;
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq)]
pub struct CscMatrix {
    pub n: usize,
    pub col_ptr: Vec<usize>,
    pub row_idx: Vec<usize>,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SparseMatrix {
    pub rows: usize,
    pub cols: usize,
    pub triplets: Vec<(usize, usize, f64)>,
}

impl SparseMatrix {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            triplets: Vec::new(),
        }
    }

    pub fn add(&mut self, row: usize, col: usize, val: f64) {
        if val != 0.0 {
            self.triplets.push((row, col, val));
        }
    }

    pub fn to_csc(&self) -> CscMatrix {
        let n = self.cols;
        let mut t = self.triplets.clone();
        t.sort_unstable_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
        let mut col_ptr = vec![0; n + 1];
        let mut row_idx = Vec::new();
        let mut values = Vec::new();
        let mut nnz = 0;
        let mut last_c = usize::MAX;
        let mut last_r = usize::MAX;

        for &(r, c, v) in &t {
            if c != last_c {
                for ptr in col_ptr.iter_mut().take(c + 1).skip(last_c.wrapping_add(1)) {
                    *ptr = nnz;
                }
                last_c = c;
                last_r = r;
                row_idx.push(r);
                values.push(v);
                nnz += 1;
            } else if r != last_r {
                last_r = r;
                row_idx.push(r);
                values.push(v);
                nnz += 1;
            } else {
                let last_idx = nnz - 1;
                values[last_idx] += v;
            }
        }
        for ptr in col_ptr.iter_mut().take(n + 1).skip(last_c.wrapping_add(1)) {
            *ptr = nnz;
        }
        CscMatrix {
            n,
            col_ptr,
            row_idx,
            values,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SymbolicLu {
    pub n: usize,
    pub pinv: Vec<usize>,
    pub p: Vec<usize>,
}

impl SymbolicLu {
    pub fn new(a: &SparseMatrix) -> Self {
        let csc = a.to_csc();
        let n = a.cols;
        let mut deg = vec![0; n];
        let mut edges = vec![Vec::new(); n];

        for c in 0..n {
            for p in csc.col_ptr[c]..csc.col_ptr[c + 1] {
                let r = csc.row_idx[p];
                if r != c {
                    edges[r].push(c);
                    edges[c].push(r);
                }
            }
        }
        for i in 0..n {
            edges[i].sort_unstable();
            edges[i].dedup();
            deg[i] = edges[i].len();
        }

        let mut active = vec![true; n];
        let mut p = Vec::with_capacity(n);
        for _ in 0..n {
            let mut min_deg = usize::MAX;
            let mut best = 0;
            for i in 0..n {
                if active[i] && deg[i] < min_deg {
                    min_deg = deg[i];
                    best = i;
                }
            }
            p.push(best);
            active[best] = false;
            let neighbors: Vec<usize> =
                edges[best].iter().copied().filter(|&x| active[x]).collect();
            for &u in &neighbors {
                for &v in &neighbors {
                    if u != v && !edges[u].contains(&v) {
                        edges[u].push(v);
                    }
                }
            }
            for &u in &neighbors {
                deg[u] = edges[u].iter().filter(|&&x| active[x]).count();
            }
        }

        let mut pinv = vec![0; n];
        for (i, &v) in p.iter().enumerate() {
            pinv[v] = i;
        }
        Self { n, p, pinv }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SparseLu {
    pub n: usize,
    pub l_col_ptr: Vec<usize>,
    pub l_row_idx: Vec<usize>,
    pub l_values: Vec<f64>,
    pub u_col_ptr: Vec<usize>,
    pub u_row_idx: Vec<usize>,
    pub u_values: Vec<f64>,
    pub p: Vec<usize>,
    pub q: Vec<usize>,
    pub pinv: Vec<usize>,
    pub qinv: Vec<usize>,
}

impl SparseLu {
    pub fn factor(sym: &SymbolicLu, a: &SparseMatrix) -> Result<SparseLu, SolveError> {
        let n = a.cols;
        let csc = a.to_csc();
        let mut l_col_ptr = vec![0; n + 1];
        let mut l_row_idx = Vec::new();
        let mut l_values = Vec::new();
        let mut u_col_ptr = vec![0; n + 1];
        let mut u_row_idx = Vec::new();
        let mut u_values = Vec::new();
        let mut p = vec![usize::MAX; n];
        let mut pinv = vec![usize::MAX; n];
        let q = &sym.p;
        let mut x = vec![0.0; n];

        let mut max_abs_a = 0.0;
        for &v in &csc.values {
            if v.abs() > max_abs_a {
                max_abs_a = v.abs();
            }
        }
        let tolerance = max_abs_a * n as f64 * f64::EPSILON;

        for k in 0..n {
            let col_orig = q[k];
            for p_idx in csc.col_ptr[col_orig]..csc.col_ptr[col_orig + 1] {
                let r = csc.row_idx[p_idx];
                x[r] = csc.values[p_idx];
            }

            for j in 0..k {
                let pivot_row = p[j];
                if x[pivot_row] != 0.0 {
                    let y_j = x[pivot_row];
                    for idx in l_col_ptr[j]..l_col_ptr[j + 1] {
                        let r = l_row_idx[idx];
                        x[r] -= y_j * l_values[idx];
                    }
                }
            }

            let mut pivot = usize::MAX;
            let mut max_val = 0.0;
            let mut nonzeros = Vec::new();
            for r in 0..n {
                if x[r] != 0.0 {
                    nonzeros.push(r);
                    if pinv[r] == usize::MAX && x[r].abs() > max_val {
                        max_val = x[r].abs();
                        pivot = r;
                    }
                }
            }
            if max_val <= tolerance || max_abs_a == 0.0 {
                return Err(SolveError::Singular { pivot_row: k });
            }

            p[k] = pivot;
            pinv[pivot] = k;
            let pivot_val = x[pivot];
            u_col_ptr[k] = u_row_idx.len();
            l_col_ptr[k] = l_row_idx.len();

            for &r in &nonzeros {
                let v = x[r];
                x[r] = 0.0;
                if v != 0.0 {
                    if pinv[r] <= k {
                        u_row_idx.push(r);
                        u_values.push(v);
                    } else {
                        l_row_idx.push(r);
                        l_values.push(v / pivot_val);
                    }
                }
            }
            u_col_ptr[k + 1] = u_row_idx.len();
            l_col_ptr[k + 1] = l_row_idx.len();
        }

        Ok(SparseLu {
            n,
            l_col_ptr,
            l_row_idx,
            l_values,
            u_col_ptr,
            u_row_idx,
            u_values,
            p,
            q: q.clone(),
            pinv,
            qinv: sym.pinv.clone(),
        })
    }

    pub fn refactor(&mut self, a: &SparseMatrix) -> Result<(), SolveError> {
        let n = self.n;
        let csc = a.to_csc();
        let mut x = vec![0.0; n];

        let mut max_abs_a = 0.0;
        for &v in &csc.values {
            if v.abs() > max_abs_a {
                max_abs_a = v.abs();
            }
        }
        let tolerance = max_abs_a * n as f64 * f64::EPSILON;

        self.l_values.clear();
        self.u_values.clear();

        for k in 0..n {
            let col_orig = self.q[k];
            for p_idx in csc.col_ptr[col_orig]..csc.col_ptr[col_orig + 1] {
                let r = csc.row_idx[p_idx];
                x[r] = csc.values[p_idx];
            }
            for j in 0..k {
                let pivot_row = self.p[j];
                if x[pivot_row] != 0.0 {
                    let y_j = x[pivot_row];
                    for idx in self.l_col_ptr[j]..self.l_col_ptr[j + 1] {
                        let r = self.l_row_idx[idx];
                        x[r] -= y_j * self.l_values[idx];
                    }
                }
            }

            let pivot = self.p[k];
            let pivot_val = x[pivot];
            if pivot_val.abs() <= tolerance || max_abs_a == 0.0 {
                return Err(SolveError::Singular { pivot_row: k });
            }

            for idx in self.u_col_ptr[k]..self.u_col_ptr[k + 1] {
                let r = self.u_row_idx[idx];
                self.u_values.push(x[r]);
                x[r] = 0.0;
            }
            for idx in self.l_col_ptr[k]..self.l_col_ptr[k + 1] {
                let r = self.l_row_idx[idx];
                self.l_values.push(x[r] / pivot_val);
                x[r] = 0.0;
            }

            for v in x.iter_mut() {
                *v = 0.0;
            }
        }
        Ok(())
    }

    pub fn solve(&self, b: &[f64]) -> Result<Vec<f64>, SolveError> {
        if b.len() != self.n {
            return Err(SolveError::DimensionMismatch {
                expected: self.n,
                got: b.len(),
            });
        }

        let mut z = vec![0.0; self.n];
        let mut b_copy = b.to_vec();

        for k in 0..self.n {
            let zk = b_copy[self.p[k]];
            z[k] = zk;
            for idx in self.l_col_ptr[k]..self.l_col_ptr[k + 1] {
                let r = self.l_row_idx[idx];
                b_copy[r] -= zk * self.l_values[idx];
            }
        }

        let mut y = vec![0.0; self.n];
        for k in (0..self.n).rev() {
            let mut piv = 0.0;
            for idx in self.u_col_ptr[k]..self.u_col_ptr[k + 1] {
                let r = self.u_row_idx[idx];
                if r == self.p[k] {
                    piv = self.u_values[idx];
                    break;
                }
            }
            let y_val = z[k] / piv;
            y[k] = y_val;
            for idx in self.u_col_ptr[k]..self.u_col_ptr[k + 1] {
                let r = self.u_row_idx[idx];
                if r != self.p[k] {
                    let i = self.pinv[r];
                    z[i] -= y_val * self.u_values[idx];
                }
            }
        }

        let mut x = vec![0.0; self.n];
        for k in 0..self.n {
            x[self.q[k]] = y[k];
        }
        Ok(x)
    }
}
