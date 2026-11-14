//! Validation of problem inputs (FR-006; constitution VII-D; data-model.md, Problem) and the
//! canonical row-bound view (data-model.md, Canonical row bounds). Tasks T025, T026.
//!
//! Every check is a single pass over the caller's arrays, O(nnz + m + n) time, with no
//! allocation proportional to the input (allocation happens only to format an error message).
//! The `what` field of every error names the defective input, using the Notation Convention
//! (`A.col_ptrs`, `A.row_indices`, `A.values`, `c`, `sense`, `b`, `b_l`, `b_u`, `l`, `u`,
//! `var_types`, `m`, `n`).

#![forbid(unsafe_code)]

use super::{MAX_DIMENSION, Problem, RowBounds, RowIndex};
use crate::error::{Error, Result};

fn invalid(what: &'static str, detail: String) -> Error {
    Error::Invalid { what, detail }
}

/// Validates a CSC matrix (data-model.md, Problem, field A): m and n at most
/// [`MAX_DIMENSION`]; `col_ptrs` of length n + 1, starting at 0, non-decreasing, ending at nnz;
/// `row_indices` and `values` of length nnz; row indices non-negative, below m and strictly
/// increasing within each column; values finite. One pass over the columns.
pub(super) fn csc<I: RowIndex>(
    m: usize,
    n: usize,
    col_ptrs: &[usize],
    row_indices: &[I],
    values: &[f64],
) -> Result<()> {
    if m > MAX_DIMENSION {
        return Err(Error::TooLarge {
            what: "m",
            value: m,
        });
    }
    if n > MAX_DIMENSION {
        return Err(Error::TooLarge {
            what: "n",
            value: n,
        });
    }
    if col_ptrs.len() != n + 1 {
        return Err(invalid(
            "A.col_ptrs",
            format!("length is {}, expected n + 1 = {}", col_ptrs.len(), n + 1),
        ));
    }
    let nnz = row_indices.len();
    if values.len() != nnz {
        return Err(invalid(
            "A.values",
            format!(
                "length is {}, but A.row_indices has length {nnz}",
                values.len()
            ),
        ));
    }
    if col_ptrs[0] != 0 {
        return Err(invalid(
            "A.col_ptrs",
            format!("col_ptrs[0] is {}, expected 0", col_ptrs[0]),
        ));
    }
    if col_ptrs[n] != nnz {
        return Err(invalid(
            "A.col_ptrs",
            format!(
                "col_ptrs[n] is {}, expected nnz = {nnz} (length of A.row_indices)",
                col_ptrs[n]
            ),
        ));
    }
    for j in 0..n {
        let (s, e) = (col_ptrs[j], col_ptrs[j + 1]);
        if e < s {
            return Err(invalid(
                "A.col_ptrs",
                format!(
                    "decreasing: col_ptrs[{}] = {e} < col_ptrs[{j}] = {s}",
                    j + 1
                ),
            ));
        }
        if e > nnz {
            // Reached only when a later pointer decreases back to nnz.
            return Err(invalid(
                "A.col_ptrs",
                format!(
                    "col_ptrs[{}] = {e} exceeds nnz = {nnz} (pointers not non-decreasing)",
                    j + 1
                ),
            ));
        }
        let mut prev: Option<usize> = None;
        for k in s..e {
            let Some(i) = row_indices[k].index() else {
                return Err(invalid(
                    "A.row_indices",
                    format!("negative row index at position {k} (column {j})"),
                ));
            };
            if i >= m {
                return Err(invalid(
                    "A.row_indices",
                    format!("row index {i} at position {k} (column {j}) out of range for m = {m}"),
                ));
            }
            if let Some(p) = prev {
                if i == p {
                    return Err(invalid(
                        "A.row_indices",
                        format!("duplicate row index {i} in column {j} at position {k}"),
                    ));
                }
                if i < p {
                    return Err(invalid(
                        "A.row_indices",
                        format!(
                            "unsorted row indices in column {j}: {i} follows {p} at position {k}"
                        ),
                    ));
                }
            }
            prev = Some(i);
            let v = values[k];
            if !v.is_finite() {
                return Err(invalid(
                    "A.values",
                    format!("non-finite value {v} at position {k} (row {i}, column {j})"),
                ));
            }
        }
    }
    Ok(())
}

fn check_len(what: &'static str, len: usize, expected: usize, dim: &str) -> Result<()> {
    if len != expected {
        return Err(invalid(
            what,
            format!("length is {len}, expected {dim} = {expected}"),
        ));
    }
    Ok(())
}

/// Validates the vectors of a problem against its (already validated) matrix A
/// (data-model.md, Problem, including the rules for infinite bounds).
pub(super) fn problem<I: RowIndex>(p: &Problem<'_, I>) -> Result<()> {
    let (m, n) = (p.A.m, p.A.n);
    check_len("c", p.c.len(), n, "n")?;
    for (j, &cj) in p.c.iter().enumerate() {
        if !cj.is_finite() {
            return Err(invalid("c", format!("non-finite value {cj} at index {j}")));
        }
    }
    if !p.gamma.is_finite() {
        return Err(invalid("gamma", format!("{} is not finite", p.gamma)));
    }
    match p.rows {
        RowBounds::SenseRhs { sense, b } => {
            check_len("sense", sense.len(), m, "m")?;
            check_len("b", b.len(), m, "m")?;
            for (i, (&s, &bi)) in sense.iter().zip(b).enumerate() {
                if bi.is_nan() {
                    return Err(invalid("b", format!("NaN at row {i}")));
                }
                match s {
                    b'=' if bi.is_infinite() => {
                        return Err(invalid(
                            "b",
                            format!("infinite value {bi} with sense '=' at row {i}"),
                        ));
                    }
                    b'<' if bi == f64::NEG_INFINITY => {
                        return Err(invalid("b", format!("-inf with sense '<' at row {i}")));
                    }
                    b'>' if bi == f64::INFINITY => {
                        return Err(invalid("b", format!("+inf with sense '>' at row {i}")));
                    }
                    b'=' | b'<' | b'>' => {}
                    other => {
                        return Err(invalid(
                            "sense",
                            format!(
                                "invalid sense byte {other:#04x} at row {i}; expected '=', '<' or '>'"
                            ),
                        ));
                    }
                }
            }
        }
        RowBounds::Ranges { b_l, b_u } => {
            check_len("b_l", b_l.len(), m, "m")?;
            check_len("b_u", b_u.len(), m, "m")?;
            for (i, (&lo, &hi)) in b_l.iter().zip(b_u).enumerate() {
                if lo.is_nan() {
                    return Err(invalid("b_l", format!("NaN at row {i}")));
                }
                if hi.is_nan() {
                    return Err(invalid("b_u", format!("NaN at row {i}")));
                }
                if lo == f64::INFINITY {
                    return Err(invalid("b_l", format!("+inf at row {i}")));
                }
                if hi == f64::NEG_INFINITY {
                    return Err(invalid("b_u", format!("-inf at row {i}")));
                }
            }
        }
    }
    check_len("l", p.l.len(), n, "n")?;
    check_len("u", p.u.len(), n, "n")?;
    for (j, (&lj, &uj)) in p.l.iter().zip(p.u).enumerate() {
        if lj.is_nan() {
            return Err(invalid("l", format!("NaN at index {j}")));
        }
        if uj.is_nan() {
            return Err(invalid("u", format!("NaN at index {j}")));
        }
        if lj == f64::INFINITY {
            return Err(invalid("l", format!("+inf at index {j}")));
        }
        if uj == f64::NEG_INFINITY {
            return Err(invalid("u", format!("-inf at index {j}")));
        }
    }
    if let Some(vt) = p.var_types {
        var_types(n, vt)?;
    }
    Ok(())
}

/// Validates variable types: length n, each `b'C'`, `b'I'` or `b'B'`.
pub(super) fn var_types(n: usize, var_types: &[u8]) -> Result<()> {
    check_len("var_types", var_types.len(), n, "n")?;
    for (j, &t) in var_types.iter().enumerate() {
        if !matches!(t, b'C' | b'I' | b'B') {
            return Err(invalid(
                "var_types",
                format!("invalid type byte {t:#04x} at index {j}; expected 'C', 'I' or 'B'"),
            ));
        }
    }
    Ok(())
}

/// Canonical bounds (b_l,i, b_u,i) of row i (data-model.md): `=` gives (b, b), `<` gives
/// (−∞, b), `>` gives (b, +∞); ranges are returned as given. Nothing is copied.
///
/// For row bounds that have passed [`problem`] validation every value is well defined. For
/// unvalidated row bounds (the enum is public) the function does not panic on the data: an
/// invalid sense byte, or a row index beyond either array, gives (NaN, NaN).
pub(super) fn canonical_row_bounds(rows: &RowBounds<'_>, i: usize) -> (f64, f64) {
    match *rows {
        RowBounds::SenseRhs { sense, b } => match (sense.get(i), b.get(i)) {
            (Some(b'='), Some(&bi)) => (bi, bi),
            (Some(b'<'), Some(&bi)) => (f64::NEG_INFINITY, bi),
            (Some(b'>'), Some(&bi)) => (bi, f64::INFINITY),
            _ => (f64::NAN, f64::NAN),
        },
        RowBounds::Ranges { b_l, b_u } => match (b_l.get(i), b_u.get(i)) {
            (Some(&lo), Some(&hi)) => (lo, hi),
            _ => (f64::NAN, f64::NAN),
        },
    }
}
