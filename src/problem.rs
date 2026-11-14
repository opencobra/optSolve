//! The solver-independent problem (data-model.md, Problem; constitution IV, VII-D, XVI-A).
//!
//! A linear programme: optimise σ-sense of cᵀx + γ subject to row bounds on Ax and
//! l ≤ x ≤ u. Every array is borrowed from the caller. A [`Problem`] can only be built by
//! its validating constructor, so holding one is proof that every invariant the C
//! libraries assume has been checked (VII-D). Field names follow the Notation Convention.

#![forbid(unsafe_code)]
#![allow(non_snake_case)] // Notation Convention (XII-C): A, Q are matrices.

use crate::error::{Error, Result};

mod sealed {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for i32 {}
}

/// Integer type of the row indices of A: `usize` (converted for solvers needing 32 bits) or
/// `i32` (passed straight through with no copy). Sealed: no other implementations.
pub trait RowIndex: sealed::Sealed + Copy + Send + Sync + 'static {
    /// The index as `usize`, or `None` if it is negative.
    fn index(self) -> Option<usize>;
    /// The slice itself when it already is `i32`, so a backend can pass it without copying.
    fn as_i32_slice(slice: &[Self]) -> Option<&[i32]>;
}

impl RowIndex for usize {
    #[inline]
    fn index(self) -> Option<usize> {
        Some(self)
    }
    #[inline]
    fn as_i32_slice(_: &[Self]) -> Option<&[i32]> {
        None
    }
}

impl RowIndex for i32 {
    #[inline]
    fn index(self) -> Option<usize> {
        usize::try_from(self).ok()
    }
    #[inline]
    fn as_i32_slice(slice: &[Self]) -> Option<&[i32]> {
        Some(slice)
    }
}

/// Largest row or column count the supported solvers accept (Gurobi and MOSEK `int`).
pub const MAX_DIMENSION: usize = i32::MAX as usize;

/// A borrowed m-by-n matrix in compressed sparse column form, validated on construction.
#[derive(Debug, Clone, Copy)]
pub struct CscRef<'a, I: RowIndex> {
    pub(crate) m: usize,
    pub(crate) n: usize,
    pub(crate) col_ptrs: &'a [usize],
    pub(crate) row_indices: &'a [I],
    pub(crate) values: &'a [f64],
}

impl<'a, I: RowIndex> CscRef<'a, I> {
    /// Validates and borrows a CSC matrix: `col_ptrs` has length n + 1, starts at 0, is
    /// non-decreasing and ends at nnz; `row_indices` and `values` have length nnz; row indices
    /// are in range and strictly increasing within each column; values are finite; m and n are
    /// at most [`MAX_DIMENSION`].
    pub fn new(
        m: usize,
        n: usize,
        col_ptrs: &'a [usize],
        row_indices: &'a [I],
        values: &'a [f64],
    ) -> Result<Self> {
        crate::problem::validate::csc(m, n, col_ptrs, row_indices, values)?;
        Ok(Self {
            m,
            n,
            col_ptrs,
            row_indices,
            values,
        })
    }
    /// Number of rows.
    pub fn m(&self) -> usize {
        self.m
    }
    /// Number of columns.
    pub fn n(&self) -> usize {
        self.n
    }
    /// Number of stored entries.
    pub fn nnz(&self) -> usize {
        self.values.len()
    }
    /// Column pointers, length n + 1.
    pub fn col_ptrs(&self) -> &'a [usize] {
        self.col_ptrs
    }
    /// Row indices, length nnz.
    pub fn row_indices(&self) -> &'a [I] {
        self.row_indices
    }
    /// Values, length nnz.
    pub fn values(&self) -> &'a [f64] {
        self.values
    }
    /// Row indices and values of column `j`.
    pub fn column(&self, j: usize) -> (&'a [I], &'a [f64]) {
        let (s, e) = (self.col_ptrs[j], self.col_ptrs[j + 1]);
        (&self.row_indices[s..e], &self.values[s..e])
    }
}

/// Row bounds, in either of the two native forms (constitution IV).
#[derive(Debug, Clone, Copy)]
pub enum RowBounds<'a> {
    /// Sense per row (`b'='`, `b'<'`, `b'>'`) with right-hand side b.
    SenseRhs {
        /// Row senses, length m.
        sense: &'a [u8],
        /// Right-hand sides, length m.
        b: &'a [f64],
    },
    /// Lower and upper row bounds: b_l ≤ Ax ≤ b_u.
    Ranges {
        /// Lower row bounds, length m.
        b_l: &'a [f64],
        /// Upper row bounds, length m.
        b_u: &'a [f64],
    },
}

impl RowBounds<'_> {
    /// Number of rows described.
    pub fn len(&self) -> usize {
        match self {
            RowBounds::SenseRhs { b, .. } => b.len(),
            RowBounds::Ranges { b_l, .. } => b_l.len(),
        }
    }
    /// Whether no rows are described.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Canonical bounds (b_l,i, b_u,i) of row i, computed without copying (data-model.md).
    pub fn bounds(&self, i: usize) -> (f64, f64) {
        crate::problem::validate::canonical_row_bounds(self, i)
    }
}

/// Objective sense.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sense {
    /// Minimise.
    Minimise,
    /// Maximise.
    Maximise,
}

impl Sense {
    /// σ: +1 for minimise, −1 for maximise.
    pub fn sigma(self) -> f64 {
        match self {
            Sense::Minimise => 1.0,
            Sense::Maximise => -1.0,
        }
    }
}

/// A validated, borrowed linear programme (data-model.md). Fields are private so that a
/// `Problem` is always valid; read them through the accessors.
#[derive(Debug, Clone, Copy)]
pub struct Problem<'a, I: RowIndex> {
    pub(crate) A: CscRef<'a, I>,
    pub(crate) c: &'a [f64],
    pub(crate) gamma: f64,
    pub(crate) sense: Sense,
    pub(crate) rows: RowBounds<'a>,
    pub(crate) l: &'a [f64],
    pub(crate) u: &'a [f64],
    pub(crate) var_types: Option<&'a [u8]>,
    pub(crate) Q: Option<CscRef<'a, usize>>,
}

impl<'a, I: RowIndex> Problem<'a, I> {
    /// Validates and borrows a linear programme (FR-006, data-model.md rules).
    pub fn new(
        A: CscRef<'a, I>,
        c: &'a [f64],
        sense: Sense,
        rows: RowBounds<'a>,
        l: &'a [f64],
        u: &'a [f64],
    ) -> Result<Self> {
        let p = Self {
            A,
            c,
            gamma: 0.0,
            sense,
            rows,
            l,
            u,
            var_types: None,
            Q: None,
        };
        crate::problem::validate::problem(&p)?;
        Ok(p)
    }
    /// Sets the objective constant γ (must be finite).
    pub fn with_gamma(mut self, gamma: f64) -> Result<Self> {
        if !gamma.is_finite() {
            return Err(Error::Invalid {
                what: "gamma",
                detail: format!("{gamma} is not finite"),
            });
        }
        self.gamma = gamma;
        Ok(self)
    }
    /// Sets variable types (reserved for MILP; `b'C'`, `b'I'`, `b'B'`), length n.
    pub fn with_var_types(mut self, var_types: &'a [u8]) -> Result<Self> {
        crate::problem::validate::var_types(self.A.n, var_types)?;
        self.var_types = Some(var_types);
        Ok(self)
    }
    /// Sets the quadratic objective matrix Q (reserved for QP), n by n.
    pub fn with_Q(mut self, Q: CscRef<'a, usize>) -> Result<Self> {
        if Q.m != self.A.n || Q.n != self.A.n {
            return Err(Error::Invalid {
                what: "Q",
                detail: format!("Q is {}x{}, expected {}x{}", Q.m, Q.n, self.A.n, self.A.n),
            });
        }
        self.Q = Some(Q);
        Ok(self)
    }
    /// Constraint matrix A.
    pub fn A(&self) -> &CscRef<'a, I> {
        &self.A
    }
    /// Objective coefficients c.
    pub fn c(&self) -> &'a [f64] {
        self.c
    }
    /// Objective constant γ.
    pub fn gamma(&self) -> f64 {
        self.gamma
    }
    /// Objective sense.
    pub fn sense(&self) -> Sense {
        self.sense
    }
    /// Row bounds.
    pub fn rows(&self) -> &RowBounds<'a> {
        &self.rows
    }
    /// Variable lower bounds l.
    pub fn l(&self) -> &'a [f64] {
        self.l
    }
    /// Variable upper bounds u.
    pub fn u(&self) -> &'a [f64] {
        self.u
    }
    /// Variable types, if set.
    pub fn var_types(&self) -> Option<&'a [u8]> {
        self.var_types
    }
    /// Quadratic objective matrix, if set.
    pub fn Q(&self) -> Option<&CscRef<'a, usize>> {
        self.Q.as_ref()
    }
    /// Number of rows m.
    pub fn m(&self) -> usize {
        self.A.m
    }
    /// Number of columns n.
    pub fn n(&self) -> usize {
        self.A.n
    }
}

pub mod convert;
mod validate;
