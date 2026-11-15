//! Gurobi bulk loading (task T042; research R2, R3, R17.1; FR-007; SC-004; constitution IV,
//! VII-A, VII-B, VII-D).
//!
//! Two paths, chosen by the caller's row-index type:
//!
//! - **`i32` indices** ([`LoadPath::SingleCall`]): one `GRBXloadmodel` call with every column.
//!   `col_ptrs`, row indices and values are passed straight from the caller's slices. The
//!   manual requires `vlen` (column lengths; it documents no `NULL` for it), so `vlen` is
//!   computed into a `Vec<c_int>` of length n: O(n), never O(nnz).
//! - **`usize` indices** ([`LoadPath::Blocked`]): `GRBXloadmodel` with zero columns (rows'
//!   senses and right-hand sides only), then one `GRBXaddvars` per [`Block`] of an
//!   [`IndexBuffer`], whose fixed buffer holds the block's converted `i32` row indices. Gurobi
//!   copies the arrays at call time (research R17.1), so the buffer is reused between blocks.
//!
//! Row bounds (research R3; data-model.md): sense rows are passed as they are (optSolver's
//! sense bytes are Gurobi's), with b copied O(m) only when it holds an infinite value. Ranges
//! are mapped to sense + right-hand side, O(m); a row with b_l < b_u (both finite), or with
//! b_l > b_u (both finite), becomes `=` 0 with one auxiliary column (coefficient −1 in that
//! row, bounds [b_l, b_u], objective 0) appended after the caller's n columns. Gurobi accepts a
//! variable with lb > ub and reports the model infeasible (checked by
//! `tests::ranged_rows_auxiliary_columns_and_infinite_bounds`), which is how b_l > b_u reaches
//! the status (data-model.md).
//!
//! Variable bounds are passed directly unless one is infinite; then l and u are copied O(n)
//! with ±∞ mapped to ±`GRB_INFINITY`.
//!
//! **Non-`const` array parameters (constitution 2.4.1, VII-A).** Every pointer below that is
//! derived from a shared slice and passed to a non-`const` parameter of `GRBXloadmodel` or
//! `GRBXaddvars` relies on: the Gurobi 13.0 reference manual, Model Creation and Modification
//! (<https://docs.gurobi.com/projects/optimizer/en/13.0/reference/c/model.html>), which
//! describes these arrays only as data used "to initialize the model data" / values "for the
//! new variables" and never as written by Gurobi; and the recorded probe (research R17.1),
//! kept as the licensed test `tests::caller_arrays_unchanged_after_load_probe`. Below this is
//! cited as "[manual + probe]".

#![allow(non_snake_case)] // Notation Convention (XII-C): A is a matrix.

use core::ffi::{c_char, c_int};
use core::marker::PhantomData;
use core::ptr::{self, NonNull};
use std::borrow::Cow;
use std::time::Instant;

use super::env::{self, Env};
use super::ffi;
use crate::backend::ProblemRef;
use crate::error::{Error, Result};
use crate::problem::convert::{IndexBuffer, i32_row_indices};
use crate::problem::{CscRef, MAX_DIMENSION, Problem, RowBounds, RowIndex, Sense};

/// Which loading path ran (recorded for the benchmark positive control, research R2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoadPath {
    /// One `GRBXloadmodel` call with the caller's `i32` row indices.
    SingleCall,
    /// `GRBXloadmodel` with zero columns, then `blocks` calls of `GRBXaddvars`.
    Blocked {
        /// Number of column blocks added.
        blocks: usize,
    },
}

/// A loaded Gurobi model. Owns the model and frees it in `Drop`; borrows the [`Env`] it was
/// created in so that it cannot outlive it (constitution VII-B).
#[derive(Debug)]
pub(crate) struct Loaded<'e> {
    /// The model (exclusively owned).
    pub(crate) model: NonNull<ffi::GRBmodel>,
    /// The caller's number of columns.
    pub(crate) n: usize,
    /// The number of rows.
    pub(crate) m: usize,
    /// Columns appended after the caller's n for finite ranged rows (research R3).
    pub(crate) aux_columns: usize,
    /// Row index of each auxiliary column, in order.
    pub(crate) ranged_rows: Vec<usize>,
    /// Conic rows appended after the m rows (feature 003; 0 without conic constraints).
    pub(crate) k: usize,
    /// Auxiliary columns of quadratic and rotated quadratic cones, appended after the ranged-row
    /// auxiliary columns (feature 003 research R2).
    pub(crate) conic_aux: usize,
    /// Quadratic objective terms passed to Gurobi (positive control; 0 for an LP).
    pub(crate) q_terms: usize,
    /// Which loading path ran.
    pub(crate) path: LoadPath,
    /// Largest number of `i32` entries held by the conversion buffer (0 for `SingleCall`).
    pub(crate) buffer_high_water: usize,
    /// Wall-clock seconds spent inside Gurobi's load calls (`GRBXloadmodel`, `GRBXaddvars`,
    /// `GRBupdatemodel`) (task T055).
    pub(crate) vendor_seconds: f64,
    /// Wall-clock seconds of the whole `load` call; `total_seconds - vendor_seconds` is
    /// optSolver's own loading work (conversion, sense and infinity mapping, auxiliary columns).
    pub(crate) total_seconds: f64,
    _env: PhantomData<&'e Env>,
}

impl Drop for Loaded<'_> {
    fn drop(&mut self) {
        // SAFETY: `self.model` was created by GRBXloadmodel, is exclusively owned by this
        // value and is freed exactly once here, before the borrowed `Env` (manual, GRBfreemodel).
        unsafe { ffi::GRBfreemodel(self.model.as_ptr()) };
    }
}

/// [`Error::Solver`] for a failed call on `model`, with the message of the model's own
/// environment (the manual: "the new model gets a copy of this environment").
pub(super) fn model_error(model: NonNull<ffi::GRBmodel>, code: c_int) -> Error {
    // SAFETY: `model` is live; GRBgetenv only returns the model's environment pointer (or
    // null, which `env::error` handles).
    let menv = unsafe { ffi::GRBgetenv(model.as_ptr()) };
    env::error(menv, code)
}

pub(super) fn to_int(what: &'static str, value: usize) -> Result<c_int> {
    c_int::try_from(value).map_err(|_| Error::TooLarge { what, value })
}

/// ±∞ → ±`GRB_INFINITY`; finite values unchanged.
fn grb(v: f64) -> f64 {
    if v == f64::INFINITY {
        ffi::GRB_INFINITY
    } else if v == f64::NEG_INFINITY {
        -ffi::GRB_INFINITY
    } else {
        v
    }
}

/// The slice itself when it has no infinite value, else an O(len) copy with ±∞ mapped.
fn finite_or_mapped(v: &[f64]) -> Cow<'_, [f64]> {
    if v.iter().any(|x| x.is_infinite()) {
        Cow::Owned(v.iter().map(|&x| grb(x)).collect())
    } else {
        Cow::Borrowed(v)
    }
}

/// Rows in Gurobi's sense + right-hand side form, plus the auxiliary columns of ranged rows.
struct Rows<'a> {
    sense: Cow<'a, [u8]>,
    rhs: Cow<'a, [f64]>,
    ranged_rows: Vec<usize>,
    aux_lb: Vec<f64>,
    aux_ub: Vec<f64>,
}

/// Maps validated row bounds to Gurobi rows (research R3; data-model.md). O(m) at most.
fn rows<'a>(bounds: &RowBounds<'a>) -> Rows<'a> {
    match *bounds {
        RowBounds::SenseRhs { sense, b } => {
            if b.iter().any(|x| x.is_infinite()) {
                // Validated: an infinite b is only '<' +inf or '>' -inf, both a free row.
                let (s, r) = sense
                    .iter()
                    .zip(b)
                    .map(|(&s, &bi)| {
                        if bi.is_infinite() {
                            (b'<', ffi::GRB_INFINITY)
                        } else {
                            (s, bi)
                        }
                    })
                    .unzip();
                Rows {
                    sense: Cow::Owned(s),
                    rhs: Cow::Owned(r),
                    ranged_rows: Vec::new(),
                    aux_lb: Vec::new(),
                    aux_ub: Vec::new(),
                }
            } else {
                Rows {
                    sense: Cow::Borrowed(sense),
                    rhs: Cow::Borrowed(b),
                    ranged_rows: Vec::new(),
                    aux_lb: Vec::new(),
                    aux_ub: Vec::new(),
                }
            }
        }
        RowBounds::Ranges { b_l, b_u } => {
            let m = b_l.len();
            let mut sense = Vec::with_capacity(m);
            let mut rhs = Vec::with_capacity(m);
            let (mut ranged_rows, mut aux_lb, mut aux_ub) = (Vec::new(), Vec::new(), Vec::new());
            for (i, (&lo, &hi)) in b_l.iter().zip(b_u).enumerate() {
                // Validated: lo is never +inf, hi never -inf.
                let (s, r) = match (lo.is_infinite(), hi.is_infinite()) {
                    _ if lo == hi => (b'=', lo),
                    (true, true) => (b'<', ffi::GRB_INFINITY),
                    (true, false) => (b'<', hi),
                    (false, true) => (b'>', lo),
                    (false, false) => {
                        ranged_rows.push(i);
                        aux_lb.push(lo);
                        aux_ub.push(hi);
                        (b'=', 0.0)
                    }
                };
                sense.push(s);
                rhs.push(r);
            }
            Rows {
                sense: Cow::Owned(sense),
                rhs: Cow::Owned(rhs),
                ranged_rows,
                aux_lb,
                aux_ub,
            }
        }
    }
}

/// Loads `problem` into a new Gurobi model in `env` (research R2, R3).
///
/// Variable types are rejected by the backend before this is called; Q and conic data are
/// added after the linear part (feature 003, `gurobi::load_qc`).
pub(crate) fn load<'e>(env: &'e Env, problem: ProblemRef<'_, '_>) -> Result<Loaded<'e>> {
    let started = Instant::now();
    let mut loaded = match problem {
        ProblemRef::I32(p) => load_generic(env, p),
        ProblemRef::Usize(p) => load_generic(env, p),
    }?;
    loaded.total_seconds = started.elapsed().as_secs_f64();
    Ok(loaded)
}

fn load_generic<'e, I: LoadIndex>(env: &'e Env, p: &Problem<'_, I>) -> Result<Loaded<'e>> {
    let (m, n) = (p.m(), p.n());
    let m_int = to_int("m", m)?;
    let n_int = to_int("n", n)?;
    let rows = rows(p.rows());
    let k = rows.ranged_rows.len();
    if n + k > MAX_DIMENSION {
        return Err(Error::TooLarge {
            what: "n + ranged rows",
            value: n + k,
        });
    }
    let k_int = to_int("ranged rows", k)?;
    let objsense = match p.sense() {
        Sense::Minimise => ffi::GRB_MINIMIZE,
        Sense::Maximise => ffi::GRB_MAXIMIZE,
    };
    let lb = finite_or_mapped(p.l());
    let ub = finite_or_mapped(p.u());
    let c = p.c();

    let mut loaded = I::load_columns(
        env,
        p.A(),
        Model {
            m_int,
            n_int,
            objsense,
            objcon: p.gamma(),
            c,
            lb: &lb,
            ub: &ub,
            sense: &rows.sense,
            rhs: &rows.rhs,
        },
    )?;
    loaded.n = n;
    loaded.m = m;

    if k > 0 {
        // Auxiliary columns of ranged rows: column t has −1 in row ranged_rows[t]. O(m).
        let mut vbeg: Vec<usize> = (0..k).collect();
        let mut vind: Vec<c_int> = rows
            .ranged_rows
            .iter()
            .map(|&i| to_int("m", i))
            .collect::<Result<_>>()?;
        let mut vval = vec![-1.0_f64; k];
        let (mut aux_lb, mut aux_ub) = (rows.aux_lb, rows.aux_ub);
        let vendor = Instant::now();
        // SAFETY: `loaded.model` is live. vbeg, vind, vval, lb, ub are locally owned arrays
        // of k entries (k columns, k nonzeros), as GRBXaddvars reads (manual, GRBXaddvars);
        // obj, vtype and varnames are NULL, documented as allowed.
        let code = unsafe {
            ffi::GRBXaddvars(
                loaded.model.as_ptr(),
                k_int,
                k,
                vbeg.as_mut_ptr(),
                vind.as_mut_ptr(),
                vval.as_mut_ptr(),
                ptr::null_mut(),
                aux_lb.as_mut_ptr(),
                aux_ub.as_mut_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        loaded.vendor_seconds += vendor.elapsed().as_secs_f64();
        if code != 0 {
            return Err(model_error(loaded.model, code));
        }
        loaded.aux_columns = k;
        loaded.ranged_rows = rows.ranged_rows;
    }

    // Feature 003: quadratic objective and second-order cones (research R1, R2).
    if let Some(q) = p.Q() {
        super::load_qc::add_quadratic_objective(&mut loaded, q)?;
    }
    if let Some(conic) = p.conic().filter(|c| c.k() > 0) {
        super::load_qc::add_conic(&mut loaded, conic)?;
    }

    let vendor = Instant::now();
    // Apply the pending additions so attributes can be queried before optimising.
    // SAFETY: `loaded.model` is live (manual, GRBupdatemodel).
    let code = unsafe { ffi::GRBupdatemodel(loaded.model.as_ptr()) };
    loaded.vendor_seconds += vendor.elapsed().as_secs_f64();
    if code != 0 {
        return Err(model_error(loaded.model, code));
    }
    Ok(loaded)
}

/// Model-level data shared by both paths.
struct Model<'x> {
    m_int: c_int,
    n_int: c_int,
    objsense: c_int,
    objcon: f64,
    c: &'x [f64],
    lb: &'x [f64],
    ub: &'x [f64],
    sense: &'x [u8],
    rhs: &'x [f64],
}

/// Creates the model with `GRBXloadmodel`; `numvars` columns described by the CSC arrays
/// (zero columns: all four empty). Returns the owned model.
#[allow(clippy::too_many_arguments)]
fn xloadmodel<'e>(
    env: &'e Env,
    d: &Model<'_>,
    numvars: c_int,
    obj: &[f64],
    vbeg: &[usize],
    vlen: &mut [c_int],
    vind: &[i32],
    vval: &[f64],
    lb: &[f64],
    ub: &[f64],
) -> Result<Loaded<'e>> {
    let mut raw: *mut ffi::GRBmodel = ptr::null_mut();
    let vendor = Instant::now();
    // SAFETY: `env.raw()` is a live, started environment borrowed for 'e; `raw` is a writable
    // local. Lengths: obj, lb, ub, vlen have `numvars` entries; vbeg at least `numvars`; vind
    // and vval cover vbeg[j] + vlen[j] for every j (validated CSC); sense and rhs have
    // m = `numconstrs` entries. Pname, vtype and the name arrays are NULL, which the manual
    // allows (vtype: continuous; names: defaults). obj, sense, rhs, vbeg, vind, vval, lb, ub
    // may point into the caller's shared slices although the parameters are not `const`:
    // [manual + probe] (module docs; constitution 2.4.1 VII-A). `vlen` is locally owned.
    let code = unsafe {
        ffi::GRBXloadmodel(
            env.raw(),
            &mut raw,
            ptr::null(),
            numvars,
            d.m_int,
            d.objsense,
            d.objcon,
            obj.as_ptr().cast_mut(),
            d.sense.as_ptr().cast::<c_char>().cast_mut(),
            d.rhs.as_ptr().cast_mut(),
            vbeg.as_ptr().cast_mut(),
            vlen.as_mut_ptr(),
            vind.as_ptr().cast_mut(),
            vval.as_ptr().cast_mut(),
            lb.as_ptr().cast_mut(),
            ub.as_ptr().cast_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    if code != 0 {
        // Read the message first; GRBXloadmodel creates no model on failure, but free one
        // defensively if it did.
        let err = env::error(env.raw(), code);
        if let Some(model) = NonNull::new(raw) {
            drop(owned(model, LoadPath::SingleCall));
        }
        return Err(err);
    }
    let vendor_seconds = vendor.elapsed().as_secs_f64();
    let model = NonNull::new(raw).ok_or_else(|| Error::Solver {
        code: 0,
        message: String::from("GRBXloadmodel returned success but no model"),
    })?;
    let mut loaded = owned(model, LoadPath::SingleCall);
    loaded.vendor_seconds = vendor_seconds;
    Ok(loaded)
}

fn owned<'e>(model: NonNull<ffi::GRBmodel>, path: LoadPath) -> Loaded<'e> {
    Loaded {
        model,
        n: 0,
        m: 0,
        aux_columns: 0,
        ranged_rows: Vec::new(),
        path,
        vendor_seconds: 0.0,
        total_seconds: 0.0,
        buffer_high_water: 0,
        k: 0,
        conic_aux: 0,
        q_terms: 0,
        _env: PhantomData,
    }
}

/// The loading path for each row-index type.
trait LoadIndex: RowIndex {
    fn load_columns<'e>(env: &'e Env, A: &CscRef<'_, Self>, d: Model<'_>) -> Result<Loaded<'e>>;
}

impl LoadIndex for i32 {
    fn load_columns<'e>(env: &'e Env, A: &CscRef<'_, i32>, d: Model<'_>) -> Result<Loaded<'e>> {
        let vind = i32_row_indices(A).unwrap_or(&[]);
        let col_ptrs = A.col_ptrs();
        // vlen is required by GRBXloadmodel (manual: "Each variable in the model has a vbeg
        // and vlen value"; no NULL documented). O(n); a column has at most m ≤ i32::MAX
        // entries.
        let mut vlen: Vec<c_int> = col_ptrs
            .windows(2)
            .map(|w| to_int("column nonzeros", w[1] - w[0]))
            .collect::<Result<_>>()?;
        xloadmodel(
            env,
            &d,
            d.n_int,
            d.c,
            col_ptrs,
            &mut vlen,
            vind,
            A.values(),
            d.lb,
            d.ub,
        )
    }
}

impl LoadIndex for usize {
    fn load_columns<'e>(env: &'e Env, A: &CscRef<'_, usize>, d: Model<'_>) -> Result<Loaded<'e>> {
        let mut loaded = xloadmodel(env, &d, 0, &[], &[], &mut [], &[], &[], &[], &[])?;
        let mut buffer = IndexBuffer::new();
        let mut blocks = buffer.blocks(A);
        let (mut count, mut high_water) = (0_usize, 0_usize);
        while let Some(block) = blocks.next_block() {
            let numvars = to_int("block columns", block.cols.len())?;
            let cols = block.cols.clone();
            high_water = high_water.max(block.row_indices.len());
            let vendor = Instant::now();
            // SAFETY: `loaded.model` is live. For the block's k = cols.len() columns: vbeg
            // (`block.starts`, k + 1 rebased starts, the first k read), obj, lb and ub (k
            // entries each, sub-slices for `cols`); vind and vval have `numnz` entries.
            // vtype and varnames are NULL (continuous, default names). vbeg, vind point
            // into the IndexBuffer and vval, obj, lb, ub may point into the caller's shared
            // slices although the parameters are not `const`: [manual + probe] (module docs;
            // constitution 2.4.1 VII-A); the probe also shows the arrays are copied at call
            // time, so the buffer may be reused for the next block.
            let code = unsafe {
                ffi::GRBXaddvars(
                    loaded.model.as_ptr(),
                    numvars,
                    block.row_indices.len(),
                    block.starts.as_ptr().cast_mut(),
                    block.row_indices.as_ptr().cast_mut(),
                    block.values.as_ptr().cast_mut(),
                    d.c[cols.clone()].as_ptr().cast_mut(),
                    d.lb[cols.clone()].as_ptr().cast_mut(),
                    d.ub[cols].as_ptr().cast_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };
            loaded.vendor_seconds += vendor.elapsed().as_secs_f64();
            if code != 0 {
                return Err(model_error(loaded.model, code));
            }
            count += 1;
        }
        loaded.path = LoadPath::Blocked { blocks: count };
        loaded.buffer_high_water = high_water;
        Ok(loaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::convert::BLOCK_CAPACITY;
    use core::ffi::{CStr, c_double};

    fn env_or_skip() -> Option<Env> {
        match Env::new() {
            Ok(env) => Some(env),
            Err(e) => {
                eprintln!("skipping licensed test: no Gurobi environment ({e})");
                None
            }
        }
    }

    fn int_attr(model: NonNull<ffi::GRBmodel>, name: &CStr) -> i32 {
        let mut v: c_int = 0;
        // SAFETY: live model, NUL-terminated name, writable int.
        let code = unsafe { ffi::GRBgetintattr(model.as_ptr(), name.as_ptr(), &mut v) };
        assert_eq!(code, 0, "{name:?}");
        v
    }

    fn dbl_attr(model: NonNull<ffi::GRBmodel>, name: &CStr) -> f64 {
        let mut v: c_double = 0.0;
        // SAFETY: live model, NUL-terminated name, writable double.
        let code = unsafe { ffi::GRBgetdblattr(model.as_ptr(), name.as_ptr(), &mut v) };
        assert_eq!(code, 0, "{name:?}");
        v
    }

    fn dbl_array(model: NonNull<ffi::GRBmodel>, name: &CStr, len: usize) -> Vec<f64> {
        let mut v = vec![0.0; len];
        // SAFETY: live model, NUL-terminated name, `v` has `len` writable doubles.
        let code = unsafe {
            ffi::GRBgetdblattrarray(
                model.as_ptr(),
                name.as_ptr(),
                0,
                c_int::try_from(len).unwrap(),
                v.as_mut_ptr(),
            )
        };
        assert_eq!(code, 0, "{name:?}");
        v
    }

    fn optimize(model: NonNull<ffi::GRBmodel>) -> i32 {
        // SAFETY: live model.
        assert_eq!(unsafe { ffi::GRBoptimize(model.as_ptr()) }, 0);
        int_attr(model, c"Status")
    }

    /// Dense-ish m-by-n CSC with every column holding rows `j % 3 ..` stepping by 1 or 2.
    struct Data {
        m: usize,
        n: usize,
        col_ptrs: Vec<usize>,
        rows_usize: Vec<usize>,
        rows_i32: Vec<i32>,
        values: Vec<f64>,
    }

    fn data(m: usize, n: usize, step: usize) -> Data {
        let mut col_ptrs = vec![0];
        let (mut rows_usize, mut values) = (Vec::new(), Vec::new());
        for j in 0..n {
            for i in ((j % 3).min(m.saturating_sub(1))..m).step_by(step) {
                rows_usize.push(i);
                values.push(1.0 + ((i * 7 + j * 13) % 5) as f64);
            }
            col_ptrs.push(rows_usize.len());
        }
        let rows_i32 = rows_usize
            .iter()
            .map(|&i| i32::try_from(i).unwrap())
            .collect();
        Data {
            m,
            n,
            col_ptrs,
            rows_usize,
            rows_i32,
            values,
        }
    }

    struct Snapshot(Vec<Vec<u64>>);

    fn snap(fs: &[&[f64]], others: &[Vec<u64>]) -> Snapshot {
        let mut v: Vec<Vec<u64>> = fs
            .iter()
            .map(|s| s.iter().map(|x| x.to_bits()).collect())
            .collect();
        v.extend(others.iter().cloned());
        Snapshot(v)
    }

    /// VII-A probe (constitution 2.4.1; research R17.1), kept as a licensed test: every caller
    /// array passed by pointer is bit-for-bit unchanged after loading through both paths.
    #[test]
    fn caller_arrays_unchanged_after_load_probe() {
        let Some(env) = env_or_skip() else { return };
        let d = data(4, 6, 1);
        let c: Vec<f64> = (0..d.n).map(|j| j as f64 - 2.5).collect();
        let l = vec![-1.0; d.n];
        let u: Vec<f64> = (0..d.n).map(|j| 3.0 + j as f64).collect();
        let sense = vec![b'<', b'>', b'=', b'<'];
        let b = vec![10.0, -1.0, 2.0, 7.0];
        let all = |d: &Data| {
            snap(
                &[&c, &l, &u, &b, &d.values],
                &[
                    sense.iter().map(|&s| u64::from(s)).collect(),
                    d.col_ptrs.iter().map(|&p| p as u64).collect(),
                    d.rows_usize.iter().map(|&p| p as u64).collect(),
                    d.rows_i32
                        .iter()
                        .map(|&p| u64::from(u32::from_ne_bytes(p.to_ne_bytes())))
                        .collect(),
                ],
            )
            .0
        };
        let before = all(&d);
        let rows = RowBounds::SenseRhs {
            sense: &sense,
            b: &b,
        };
        let Ai = CscRef::new(d.m, d.n, &d.col_ptrs, &d.rows_i32, &d.values).unwrap();
        let pi = Problem::new(Ai, &c, Sense::Minimise, rows, &l, &u).unwrap();
        let li = load(&env, (&pi).into()).unwrap();
        assert_eq!(li.path, LoadPath::SingleCall);
        let Au = CscRef::new(d.m, d.n, &d.col_ptrs, &d.rows_usize, &d.values).unwrap();
        let pu = Problem::new(Au, &c, Sense::Minimise, rows, &l, &u).unwrap();
        let lu = load(&env, (&pu).into()).unwrap();
        assert_eq!(lu.path, LoadPath::Blocked { blocks: 1 });
        drop((li, lu));
        assert_eq!(
            all(&d),
            before,
            "a caller array changed during GRBXloadmodel/GRBXaddvars"
        );
    }

    fn compare_paths(env: &Env, d: &Data, expect_multi_block: bool) {
        let c: Vec<f64> = (0..d.n).map(|j| (j % 11) as f64 - 5.0).collect();
        let l: Vec<f64> = (0..d.n).map(|j| -((j % 4) as f64)).collect();
        let u: Vec<f64> = (0..d.n).map(|j| 1.0 + (j % 9) as f64).collect();
        let sense: Vec<u8> = (0..d.m).map(|i| [b'<', b'>', b'='][i % 3]).collect();
        let b: Vec<f64> = (0..d.m).map(|i| (i % 13) as f64).collect();
        let rows = RowBounds::SenseRhs {
            sense: &sense,
            b: &b,
        };
        let Ai = CscRef::new(d.m, d.n, &d.col_ptrs, &d.rows_i32, &d.values).unwrap();
        let pi = Problem::new(Ai, &c, Sense::Maximise, rows, &l, &u)
            .unwrap()
            .with_gamma(1.5)
            .unwrap();
        let Au = CscRef::new(d.m, d.n, &d.col_ptrs, &d.rows_usize, &d.values).unwrap();
        let pu = Problem::new(Au, &c, Sense::Maximise, rows, &l, &u)
            .unwrap()
            .with_gamma(1.5)
            .unwrap();
        let li = load(env, (&pi).into()).unwrap();
        let lu = load(env, (&pu).into()).unwrap();
        assert_eq!(li.path, LoadPath::SingleCall);
        assert_eq!(li.buffer_high_water, 0);
        let LoadPath::Blocked { blocks } = lu.path else {
            panic!("expected Blocked")
        };
        assert!(lu.buffer_high_water <= BLOCK_CAPACITY);
        if expect_multi_block {
            assert!(d.values.len() > BLOCK_CAPACITY);
            assert!(blocks > 1, "blocks = {blocks}");
        }
        for ld in [&li, &lu] {
            // Task T055: the vendor share is measured and cannot exceed the whole load.
            assert!(ld.vendor_seconds > 0.0, "vendor time measured");
            assert!(
                ld.vendor_seconds <= ld.total_seconds,
                "vendor time within total"
            );
            assert_eq!((ld.m, ld.n, ld.aux_columns), (d.m, d.n, 0));
            assert_eq!(int_attr(ld.model, c"NumVars") as usize, d.n);
            assert_eq!(int_attr(ld.model, c"NumConstrs") as usize, d.m);
            assert_eq!(int_attr(ld.model, c"NumNZs") as usize, d.values.len());
            assert_eq!(int_attr(ld.model, c"ModelSense"), ffi::GRB_MAXIMIZE);
            assert_eq!(dbl_attr(ld.model, c"ObjCon"), 1.5);
        }
        for (name, len) in [(c"Obj", d.n), (c"LB", d.n), (c"UB", d.n), (c"RHS", d.m)] {
            assert_eq!(
                dbl_array(li.model, name, len),
                dbl_array(lu.model, name, len),
                "{name:?}"
            );
        }
        assert_eq!(dbl_array(li.model, c"Obj", d.n), c);
        assert_eq!(dbl_array(li.model, c"LB", d.n), l);
        assert_eq!(dbl_array(li.model, c"UB", d.n), u);
        assert_eq!(dbl_array(li.model, c"RHS", d.m), b);
    }

    #[test]
    fn both_paths_give_the_same_model() {
        let Some(env) = env_or_skip() else { return };
        compare_paths(&env, &data(7, 9, 2), false);
    }

    #[test]
    fn both_paths_same_model_with_many_blocks() {
        let Some(env) = env_or_skip() else { return };
        // 600 x 500 dense-ish: ~300 000 nonzeros > 2^18.
        compare_paths(&env, &data(600, 500, 1), true);
    }

    #[test]
    fn ranged_rows_auxiliary_columns_and_infinite_bounds() {
        let Some(env) = env_or_skip() else { return };
        let inf = f64::INFINITY;
        // 6 rows, 2 columns, every entry 1.
        let (m, n) = (6, 2);
        let col_ptrs = vec![0, 6, 12];
        let rows_i32: Vec<i32> = (0..2).flat_map(|_| 0..6).collect();
        let rows_usize: Vec<usize> = (0..2).flat_map(|_| 0..6).collect();
        let values = vec![1.0; 12];
        let b_l = vec![1.0, -inf, 2.0, -inf, 1.0, 4.0];
        let b_u = vec![1.0, 5.0, inf, inf, 3.0, 2.0];
        let c = vec![1.0, 0.0];
        let l = vec![-inf, 0.0];
        let u = vec![inf, 10.0];
        let rows = RowBounds::Ranges {
            b_l: &b_l,
            b_u: &b_u,
        };
        let Ai = CscRef::new(m, n, &col_ptrs, &rows_i32, &values).unwrap();
        let Au = CscRef::new(m, n, &col_ptrs, &rows_usize, &values).unwrap();
        let pi = Problem::new(Ai, &c, Sense::Minimise, rows, &l, &u).unwrap();
        let pu = Problem::new(Au, &c, Sense::Minimise, rows, &l, &u).unwrap();
        for ld in [
            load(&env, (&pi).into()).unwrap(),
            load(&env, (&pu).into()).unwrap(),
        ] {
            assert_eq!(ld.aux_columns, 2);
            assert_eq!(ld.ranged_rows, vec![4, 5]);
            assert_eq!(int_attr(ld.model, c"NumVars"), 4);
            assert_eq!(int_attr(ld.model, c"NumConstrs"), 6);
            assert_eq!(int_attr(ld.model, c"NumNZs"), 14);
            assert_eq!(
                dbl_array(ld.model, c"RHS", 6),
                vec![1.0, 5.0, 2.0, 1e100, 0.0, 0.0]
            );
            assert_eq!(dbl_array(ld.model, c"LB", 4), vec![-1e100, 0.0, 1.0, 4.0]);
            assert_eq!(dbl_array(ld.model, c"UB", 4), vec![1e100, 10.0, 3.0, 2.0]);
            assert_eq!(dbl_array(ld.model, c"Obj", 4), vec![1.0, 0.0, 0.0, 0.0]);
            // Row 5 has b_l = 4 > b_u = 2: Gurobi accepted lb > ub and reports infeasibility.
            let status = optimize(ld.model);
            assert!(
                status == ffi::GRB_INFEASIBLE || status == ffi::GRB_INF_OR_UNBD,
                "status {status}"
            );
        }
    }

    /// Senses are honoured: min and max of x0 + x1 under 1 ≤ x0 + x1 ≤ 3 (a ranged row), with
    /// '<' and '>' rows and a free sense row.
    #[test]
    fn ranged_and_sense_rows_solve_correctly() {
        let Some(env) = env_or_skip() else { return };
        let inf = f64::INFINITY;
        let col_ptrs = vec![0, 1, 2];
        let ri = vec![0_i32, 0];
        let vals = vec![1.0, 1.0];
        let c = vec![1.0, 1.0];
        let (l, u) = (vec![0.0, 0.0], vec![inf, inf]);
        let (b_l, b_u) = (vec![1.0], vec![3.0]);
        for (sense, want) in [(Sense::Minimise, 1.0), (Sense::Maximise, 3.0)] {
            let A = CscRef::new(1, 2, &col_ptrs, &ri, &vals).unwrap();
            let p = Problem::new(
                A,
                &c,
                sense,
                RowBounds::Ranges {
                    b_l: &b_l,
                    b_u: &b_u,
                },
                &l,
                &u,
            )
            .unwrap();
            let ld = load(&env, (&p).into()).unwrap();
            assert_eq!(optimize(ld.model), ffi::GRB_OPTIMAL);
            assert!((dbl_attr(ld.model, c"ObjVal") - want).abs() < 1e-9);
        }
        // Rows: x0 + x1 >= 2, x0 + x1 <= 5, a free '<' +inf row and a free '>' -inf row.
        let col_ptrs = vec![0, 4, 8];
        let ri: Vec<i32> = (0..2).flat_map(|_| 0..4).collect();
        let vals = vec![1.0; 8];
        let s = vec![b'>', b'<', b'<', b'>'];
        let b = vec![2.0, 5.0, inf, -inf];
        for (sense, want) in [(Sense::Minimise, 2.0), (Sense::Maximise, 5.0)] {
            let A = CscRef::new(4, 2, &col_ptrs, &ri, &vals).unwrap();
            let p = Problem::new(
                A,
                &c,
                sense,
                RowBounds::SenseRhs { sense: &s, b: &b },
                &l,
                &u,
            )
            .unwrap();
            let ld = load(&env, (&p).into()).unwrap();
            assert_eq!(dbl_array(ld.model, c"RHS", 4), vec![2.0, 5.0, 1e100, 1e100]);
            assert_eq!(optimize(ld.model), ffi::GRB_OPTIMAL);
            assert!((dbl_attr(ld.model, c"ObjVal") - want).abs() < 1e-9);
        }
    }
}
