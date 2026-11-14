//! Row-index conversion to 32 bits through a fixed-size reusable buffer (research R2;
//! FR-007; SC-004). Implemented by task T025.
//!
//! Two paths for a backend that needs `i32` row indices:
//!
//! - **`i32` indices**: [`i32_row_indices`] returns the caller's own slice (no copy).
//! - **`usize` indices**: an [`IndexBuffer`] owns one `i32` buffer of [`BLOCK_CAPACITY`]
//!   entries and one buffer of at most [`BLOCK_CAPACITY`] + 1 rebased column starts. Its
//!   [`Blocks`] cursor walks A in consecutive column blocks; each [`Block`] gives the block's
//!   column range, rebased column starts, the converted row indices and the caller's values
//!   (borrowed, not copied). A block holds at most [`BLOCK_CAPACITY`] nonzeros and at most
//!   [`BLOCK_CAPACITY`] columns, so the memory used is independent of nnz.
//!
//! **A single column with more nonzeros than the capacity** is returned alone in a block, and
//! for that block the index buffer grows to the column's nonzero count; it is shrunk back to
//! the capacity before the next block. Memory is therefore bounded by
//! max([`BLOCK_CAPACITY`], largest column nnz) `i32` entries, and the largest column nnz is at
//! most m: never a term proportional to the total nnz. This choice keeps every valid problem
//! loadable rather than rejecting tall dense columns.
//!
//! Conversion never fails: a [`CscRef`] has been validated, so every row index is below
//! m ≤ [`crate::problem::MAX_DIMENSION`] = `i32::MAX`.

#![forbid(unsafe_code)]
#![allow(non_snake_case)] // Notation Convention (XII-C): A is a matrix.

use std::ops::Range;

use super::{CscRef, RowIndex};

/// Capacity of the conversion buffer in entries (2^18 = 1 MiB of `i32`).
pub const BLOCK_CAPACITY: usize = 1 << 18;

/// The caller's row indices as `i32` without copying, when they already are `i32`; `None`
/// for `usize` indices (use an [`IndexBuffer`]).
pub fn i32_row_indices<'a, I: RowIndex>(A: &CscRef<'a, I>) -> Option<&'a [i32]> {
    I::as_i32_slice(A.row_indices)
}

/// Reusable fixed-capacity buffers for converting `usize` row indices to `i32` block by
/// block. Create once and reuse across problems.
#[derive(Debug, Clone)]
pub struct IndexBuffer {
    capacity: usize,
    indices: Vec<i32>,
    starts: Vec<usize>,
}

impl Default for IndexBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl IndexBuffer {
    /// Buffers of [`BLOCK_CAPACITY`] entries.
    pub fn new() -> Self {
        Self::with_capacity(BLOCK_CAPACITY)
    }

    /// Buffers of at most `capacity` entries (at least 1). Intended for tests of the blocking
    /// logic; backends use [`IndexBuffer::new`]. Memory is reserved lazily, only as far as the
    /// largest block actually needs, so a small problem never pays for the full capacity
    /// (SC-004).
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            capacity,
            indices: Vec::new(),
            starts: Vec::new(),
        }
    }

    /// Block capacity in nonzeros (and in columns).
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// A cursor over the column blocks of A, starting at column 0.
    pub fn blocks<'b, 'a>(&'b mut self, A: &CscRef<'a, usize>) -> Blocks<'b, 'a> {
        Blocks {
            buf: self,
            A: *A,
            next_col: 0,
        }
    }
}

/// One block of consecutive columns of A, with row indices converted to `i32`.
#[derive(Debug, Clone)]
pub struct Block<'b, 'a> {
    /// Columns of A in this block.
    pub cols: Range<usize>,
    /// Column starts rebased to the block: length `cols.len() + 1`, first 0, last nnz of the
    /// block. Column `cols.start + k` occupies entries `starts[k]..starts[k + 1]`.
    pub starts: &'b [usize],
    /// Row indices of the block, converted to `i32`.
    pub row_indices: &'b [i32],
    /// Values of the block: a sub-slice of the caller's values (no copy).
    pub values: &'a [f64],
}

/// Cursor over the column blocks of A (a lending iterator: each block borrows the buffer
/// until the next call).
#[derive(Debug)]
pub struct Blocks<'b, 'a> {
    buf: &'b mut IndexBuffer,
    A: CscRef<'a, usize>,
    next_col: usize,
}

impl<'a> Blocks<'_, 'a> {
    /// The next block, or `None` after the last column. A matrix with n = 0 yields no block.
    pub fn next_block(&mut self) -> Option<Block<'_, 'a>> {
        let n = self.A.n;
        let first = self.next_col;
        if first >= n {
            return None;
        }
        let cap = self.buf.capacity;
        let col_ptrs = self.A.col_ptrs;
        let base = col_ptrs[first];

        // Greedily take columns while the block's nnz and column count fit the capacity; the
        // first column is always taken (it may exceed the capacity on its own).
        let mut end = first + 1;
        while end < n && end - first < cap && col_ptrs[end + 1] - base <= cap {
            end += 1;
        }
        let top = col_ptrs[end];

        let buf = &mut *self.buf;
        if buf.indices.capacity() > cap {
            // A previous oversized column grew the buffer: return to the fixed capacity.
            buf.indices.clear();
            buf.indices.shrink_to(cap);
        }
        buf.indices.clear();
        buf.indices.reserve_exact(top - base);
        buf.indices
            .extend(self.A.row_indices[base..top].iter().map(|&i| {
                // Validated: i < m <= i32::MAX, so the conversion cannot fail.
                i32::try_from(i).unwrap_or(i32::MAX)
            }));
        buf.starts.clear();
        buf.starts.reserve_exact(end - first + 1);
        buf.starts
            .extend(col_ptrs[first..=end].iter().map(|&p| p - base));

        self.next_col = end;
        Some(Block {
            cols: first..end,
            starts: &buf.starts,
            row_indices: &buf.indices,
            values: &self.A.values[base..top],
        })
    }
}
