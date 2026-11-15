//! Bulk loading of a problem into a new MOSEK task (tasks T026, T030, T035; feature 002
//! research R3–R5; constitution I, IV).
//!
//! - **A**: one `MSK_putacolslice64` over all columns with the caller's column pointers
//!   (reinterpreted as `int64`, same width on the supported 64-bit targets) and, for `i32`
//!   row indices, the caller's row indices and values unchanged (zero copy). `usize` row
//!   indices are converted block by block through the fixed-size [`IndexBuffer`], one call per
//!   block.
//! - **Bounds**: MOSEK's bound keys are built (4 B per row and column). The value arrays are
//!   the caller's when they hold no infinite value (for sense rows, b is passed as both lower
//!   and upper bound and the key selects the side); otherwise copies with ±∞ replaced by
//!   ±`MSK_INFINITY` (research R3).
//! - **Q**: `MSK_putqobj` triplets from the lower-triangle CSC; the row and column index arrays
//!   are built (O(nnz(Q)), plan Complexity Tracking), the values borrowed.
//! - **F, g, K**: `MSK_putafefentrylist` per block of at most [`BLOCK_CAPACITY`] entries, with
//!   F's row indices passed as `int64` without copying and the column of each entry built in a
//!   fixed-size buffer; `MSK_putafegslice` with the caller's g; one domain per distinct cone
//!   shape and one `MSK_appendaccsseq` over all cones (rows are consecutive per cone).

#![allow(non_snake_case)] // Notation Convention (XII-C): A, Q, F are matrices.

use std::borrow::Cow;
use std::time::Instant;

use super::env::{Env, Task};
use super::ffi;
use crate::backend::{ProblemClass, ProblemRef};
use crate::conic::{Cone, ConicRef};
use crate::error::{Error, Result};
use crate::problem::convert::{BLOCK_CAPACITY, IndexBuffer, i32_row_indices};
use crate::problem::{CscRef, Problem, RowBounds, RowIndex};

/// How A was loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoadPath {
    /// One `MSK_putacolslice64` with the caller's arrays.
    SingleCall,
    /// Converted blocks.
    Blocked {
        /// Number of `MSK_putacolslice64` calls.
        blocks: usize,
    },
}

/// A loaded task with what the solve and the load report need.
pub(crate) struct Loaded<'e> {
    pub(crate) task: Task<'e>,
    pub(crate) m: usize,
    pub(crate) n: usize,
    pub(crate) k: usize,
    pub(crate) class: ProblemClass,
    pub(crate) path: LoadPath,
    /// Largest number of converted row indices or F column indices held at once.
    pub(crate) buffer_high_water: usize,
    /// Bytes of bound keys and bound-value copies made.
    pub(crate) bound_bytes: usize,
    /// Quadratic objective terms passed to MOSEK (positive control; 0 for an LP).
    pub(crate) q_terms: usize,
    pub(crate) vendor_seconds: f64,
    pub(crate) total_seconds: f64,
}

/// Times vendor calls and turns their response codes into errors.
struct Calls<'t, 'e> {
    task: &'t Task<'e>,
    seconds: f64,
}

impl Calls<'_, '_> {
    fn run(&mut self, f: impl FnOnce(ffi::MSKtask_t) -> i32) -> Result<()> {
        let t = Instant::now();
        let rc = f(self.task.raw());
        self.seconds += t.elapsed().as_secs_f64();
        self.task.check(rc)
    }
}

fn int32(what: &'static str, v: usize) -> Result<i32> {
    i32::try_from(v).map_err(|_| Error::TooLarge { what, value: v })
}

fn int64(what: &'static str, v: usize) -> Result<i64> {
    i64::try_from(v).map_err(|_| Error::TooLarge { what, value: v })
}

/// `usize` slice as a pointer to `int64` (same size and alignment on the supported 64-bit
/// targets, lib.rs assertion; every value here is at most nnz or k, below `i64::MAX`).
fn as_i64_ptr(s: &[usize]) -> *const i64 {
    s.as_ptr().cast::<i64>()
}

/// Loads `problem` into a new task of `env`.
pub(crate) fn load<'e>(env: &'e Env, problem: ProblemRef<'_, '_>) -> Result<Loaded<'e>> {
    match problem {
        ProblemRef::Usize(p) => load_problem(env, p, Some(*p.A())),
        ProblemRef::I32(p) => load_problem(env, p, None),
    }
}

/// The class a problem is solved as.
pub(crate) fn class_of<I: RowIndex>(p: &Problem<'_, I>) -> ProblemClass {
    if p.conic().is_some_and(|c| c.k() > 0) {
        ProblemClass::Conic
    } else if p.Q().is_some() {
        ProblemClass::Qp
    } else {
        ProblemClass::Lp
    }
}

/// `A_usize` is A itself when its row indices are `usize` (the blocked path needs it typed).
fn load_problem<'e, I: RowIndex>(
    env: &'e Env,
    p: &Problem<'_, I>,
    A_usize: Option<CscRef<'_, usize>>,
) -> Result<Loaded<'e>> {
    let start = Instant::now();
    let task = env.task()?;
    let (m, n) = (p.m(), p.n());
    let (m32, n32) = (int32("m", m)?, int32("n", n)?);
    let A = p.A();
    let nnz = A.nnz();
    let mut calls = Calls {
        task: &task,
        seconds: 0.0,
    };
    let nnz64 = int64("nnz(A)", nnz)?;
    // SAFETY: (all closures passed to `calls.run` in this function): the task is live (owned
    // by `task`, borrowed by `calls`); every pointer passed comes from a slice that outlives
    // the call and has at least the number of entries MOSEK reads, as stated at each call.
    calls.run(|t| unsafe { ffi::MSK_putmaxnumvar(t, n32) })?;
    // SAFETY: as stated in the SAFETY note for this block of calls above.
    calls.run(|t| unsafe { ffi::MSK_putmaxnumcon(t, m32) })?;
    // SAFETY: as stated in the SAFETY note for this block of calls above.
    calls.run(|t| unsafe { ffi::MSK_putmaxnumanz(t, nnz64) })?;
    // SAFETY: as stated in the SAFETY note for this block of calls above.
    calls.run(|t| unsafe { ffi::MSK_appendcons(t, m32) })?;
    // SAFETY: as stated in the SAFETY note for this block of calls above.
    calls.run(|t| unsafe { ffi::MSK_appendvars(t, n32) })?;

    // A.
    let mut high_water = 0;
    let path = if nnz == 0 {
        LoadPath::SingleCall
    } else if let Some(rows) = i32_row_indices(A) {
        let cp = A.col_ptrs();
        // ptrb = col_ptrs[0..n], ptre = col_ptrs[1..=n] (n entries each); asub and aval have
        // nnz entries, the largest index ptre reaches.
        // SAFETY: as stated in the SAFETY note for this block of calls above.
        calls.run(|t| unsafe {
            ffi::MSK_putacolslice64(
                t,
                0,
                n32,
                as_i64_ptr(&cp[..n]),
                as_i64_ptr(&cp[1..]),
                rows.as_ptr(),
                A.values().as_ptr(),
            )
        })?;
        LoadPath::SingleCall
    } else {
        let A_usize = A_usize.ok_or_else(|| Error::Invalid {
            what: "A.row_indices",
            detail: "row indices are neither i32 nor usize".into(),
        })?;
        let mut buffer = IndexBuffer::new();
        let mut blocks = buffer.blocks(&A_usize);
        let mut count = 0;
        while let Some(block) = blocks.next_block() {
            let (first, last) = (
                int32("column", block.cols.start)?,
                int32("column", block.cols.end)?,
            );
            let w = block.cols.len();
            high_water = high_water.max(block.row_indices.len());
            // Block-relative starts (w + 1 entries): ptrb = starts[0..w], ptre =
            // starts[1..=w]; asub and aval hold the block's entries.
            // SAFETY: as stated in the SAFETY note for this block of calls above.
            calls.run(|t| unsafe {
                ffi::MSK_putacolslice64(
                    t,
                    first,
                    last,
                    as_i64_ptr(&block.starts[..w]),
                    as_i64_ptr(&block.starts[1..]),
                    block.row_indices.as_ptr(),
                    block.values.as_ptr(),
                )
            })?;
            count += 1;
        }
        LoadPath::Blocked { blocks: count }
    };

    // Objective.
    if n > 0 {
        // c has n entries.
        // SAFETY: as stated in the SAFETY note for this block of calls above.
        calls.run(|t| unsafe { ffi::MSK_putcslice(t, 0, n32, p.c().as_ptr()) })?;
    }
    let gamma = p.gamma();
    // SAFETY: as stated in the SAFETY note for this block of calls above.
    calls.run(|t| unsafe { ffi::MSK_putcfix(t, gamma) })?;
    let sense = match p.sense() {
        crate::problem::Sense::Minimise => ffi::MSK_OBJECTIVE_SENSE_MINIMIZE,
        crate::problem::Sense::Maximise => ffi::MSK_OBJECTIVE_SENSE_MAXIMIZE,
    };
    // SAFETY: as stated in the SAFETY note for this block of calls above.
    calls.run(|t| unsafe { ffi::MSK_putobjsense(t, sense) })?;

    // Bounds.
    let mut bound_bytes = 0;
    if n > 0 {
        let (keys, lo, hi) = bounds(p.l(), p.u());
        bound_bytes += 4 * keys.len() + copied(&lo) + copied(&hi);
        // keys, lo and hi have n entries.
        // SAFETY: as stated in the SAFETY note for this block of calls above.
        calls.run(|t| unsafe {
            ffi::MSK_putvarboundslice(t, 0, n32, keys.as_ptr(), lo.as_ptr(), hi.as_ptr())
        })?;
    }
    if m > 0 {
        let (keys, lo, hi) = row_bounds(p.rows());
        bound_bytes += 4 * keys.len() + copied(&lo) + copied(&hi);
        // keys, lo and hi have m entries.
        // SAFETY: as stated in the SAFETY note for this block of calls above.
        calls.run(|t| unsafe {
            ffi::MSK_putconboundslice(t, 0, m32, keys.as_ptr(), lo.as_ptr(), hi.as_ptr())
        })?;
    }

    // Q.
    let mut q_terms = 0;
    if let Some(Q) = p.Q().filter(|q| q.nnz() > 0) {
        q_terms = Q.nnz();
        let nq = int32("nnz(Q)", Q.nnz())?;
        let mut qi = Vec::with_capacity(Q.nnz());
        let mut qj = Vec::with_capacity(Q.nnz());
        for j in 0..Q.n() {
            let (rows, _) = Q.column(j);
            let jj = int32("column", j)?;
            for &i in rows {
                qi.push(int32("row of Q", i)?);
                qj.push(jj);
            }
        }
        // qi, qj and Q's values have nnz(Q) entries.
        // SAFETY: as stated in the SAFETY note for this block of calls above.
        calls.run(|t| unsafe {
            ffi::MSK_putqobj(t, nq, qi.as_ptr(), qj.as_ptr(), Q.values().as_ptr())
        })?;
    }

    // F, g, K.
    let mut k = 0;
    if let Some(conic) = p.conic().filter(|c| c.k() > 0) {
        k = conic.k();
        let hw = load_conic(&mut calls, conic)?;
        high_water = high_water.max(hw);
    }

    let vendor_seconds = calls.seconds;
    Ok(Loaded {
        class: class_of(p),
        task,
        m,
        n,
        k,
        path,
        buffer_high_water: high_water,
        bound_bytes,
        q_terms,
        vendor_seconds,
        total_seconds: start.elapsed().as_secs_f64(),
    })
}

#[allow(clippy::ptr_arg)] // Whether the slice was copied is the point.
fn copied(v: &Cow<'_, [f64]>) -> usize {
    match v {
        Cow::Borrowed(_) => 0,
        Cow::Owned(o) => 8 * o.len(),
    }
}

/// MOSEK bound key of [lo, hi].
fn key(lo: f64, hi: f64) -> i32 {
    match (lo.is_finite(), hi.is_finite()) {
        (true, true) if lo == hi => ffi::MSK_BK_FX,
        (true, true) => ffi::MSK_BK_RA,
        (true, false) => ffi::MSK_BK_LO,
        (false, true) => ffi::MSK_BK_UP,
        (false, false) => ffi::MSK_BK_FR,
    }
}

/// ±∞ replaced by ±`MSK_INFINITY`; borrowed when nothing is infinite.
fn finite(v: &[f64]) -> Cow<'_, [f64]> {
    if v.iter().all(|x| x.is_finite()) {
        Cow::Borrowed(v)
    } else {
        Cow::Owned(
            v.iter()
                .map(|&x| x.clamp(-ffi::MSK_INFINITY, ffi::MSK_INFINITY))
                .collect(),
        )
    }
}

fn bounds<'a>(lo: &'a [f64], hi: &'a [f64]) -> (Vec<i32>, Cow<'a, [f64]>, Cow<'a, [f64]>) {
    let keys = lo.iter().zip(hi).map(|(&l, &u)| key(l, u)).collect();
    (keys, finite(lo), finite(hi))
}

fn row_bounds<'a>(rows: &RowBounds<'a>) -> (Vec<i32>, Cow<'a, [f64]>, Cow<'a, [f64]>) {
    match *rows {
        RowBounds::Ranges { b_l, b_u } => bounds(b_l, b_u),
        RowBounds::SenseRhs { b, .. } => {
            let keys = (0..b.len())
                .map(|i| {
                    let (lo, hi) = rows.bounds(i);
                    key(lo, hi)
                })
                .collect();
            // The key selects the side; b serves as both lower and upper value.
            let v = finite(b);
            (keys, v.clone(), v)
        }
    }
}

/// Loads F, g and the cones; returns the largest number of column indices buffered at once.
fn load_conic(calls: &mut Calls<'_, '_>, conic: &ConicRef<'_>) -> Result<usize> {
    let F = conic.F();
    let k = conic.k();
    let k64 = int64("k", k)?;
    // SAFETY: (all closures in this function): as in `load_problem`.
    calls.run(|t| unsafe { ffi::MSK_putmaxnumafe(t, k64) })?;
    // SAFETY: as stated in the SAFETY note for this block of calls above.
    calls.run(|t| unsafe { ffi::MSK_appendafes(t, k64) })?;

    // F by blocks of entries: afeidx is F's own row indices (usize as int64), varidx the
    // column of each entry, built in a fixed-size buffer.
    let nnz = F.nnz();
    let cp = F.col_ptrs();
    let mut varidx: Vec<i32> = Vec::new();
    let mut high_water = 0;
    let mut j = 0;
    let mut s = 0;
    while s < nnz {
        let e = (s + BLOCK_CAPACITY).min(nnz);
        varidx.clear();
        for t in s..e {
            while cp[j + 1] <= t {
                j += 1;
            }
            varidx.push(int32("column", j)?);
        }
        high_water = high_water.max(varidx.len());
        let len = int64("block", e - s)?;
        let rows = &F.row_indices()[s..e];
        let vals = &F.values()[s..e];
        // rows, varidx and vals have e − s entries.
        // SAFETY: as stated in the SAFETY note for this block of calls above.
        calls.run(|t| unsafe {
            ffi::MSK_putafefentrylist(t, len, as_i64_ptr(rows), varidx.as_ptr(), vals.as_ptr())
        })?;
        s = e;
    }
    // g has k entries.
    let g = conic.g();
    // SAFETY: as stated in the SAFETY note for this block of calls above.
    calls.run(|t| unsafe { ffi::MSK_putafegslice(t, 0, k64, g.as_ptr()) })?;

    // One domain per distinct cone shape.
    let mut shapes: Vec<(&Cone, i64)> = Vec::new();
    let mut domidxs = Vec::with_capacity(conic.cones().len());
    for cone in conic.cones() {
        let idx = match shapes.iter().find(|(c, _)| *c == cone) {
            Some(&(_, d)) => d,
            None => {
                let d = append_domain(calls, cone)?;
                shapes.push((cone, d));
                d
            }
        };
        domidxs.push(idx);
    }
    let numaccs = int64("number of cones", domidxs.len())?;
    // domidxs has numaccs entries; b = NULL (zero offset; g already holds it).
    // SAFETY: as stated in the SAFETY note for this block of calls above.
    calls.run(|t| unsafe {
        ffi::MSK_appendaccsseq(t, numaccs, domidxs.as_ptr(), k64, 0, core::ptr::null())
    })?;
    Ok(high_water)
}

fn append_domain(calls: &mut Calls<'_, '_>, cone: &Cone) -> Result<i64> {
    let mut d: i64 = -1;
    let dim = int64("cone dimension", cone.dim())?;
    let dp: *mut i64 = &mut d;
    // SAFETY: (all closures): live task; `dp` points to the writable local `d`; for power
    // cones `alpha` has `nleft` entries.
    match cone {
        Cone::Quadratic(_) => {
            // SAFETY: as stated in the SAFETY note for this block of calls above.
            calls.run(|t| unsafe { ffi::MSK_appendquadraticconedomain(t, dim, dp) })?
        }
        Cone::RotatedQuadratic(_) => {
            // SAFETY: as stated in the SAFETY note for this block of calls above.
            calls.run(|t| unsafe { ffi::MSK_appendrquadraticconedomain(t, dim, dp) })?
        }
        Cone::PrimalExponential => {
            // SAFETY: as stated in the SAFETY note for this block of calls above.
            calls.run(|t| unsafe { ffi::MSK_appendprimalexpconedomain(t, dp) })?
        }
        Cone::DualExponential => {
            // SAFETY: as stated in the SAFETY note for this block of calls above.
            calls.run(|t| unsafe { ffi::MSK_appenddualexpconedomain(t, dp) })?
        }
        Cone::PrimalPower { alpha, .. } => {
            let nleft = int64("power cone exponents", alpha.len())?;
            // SAFETY: as stated in the SAFETY note for this block of calls above.
            calls.run(|t| unsafe {
                ffi::MSK_appendprimalpowerconedomain(t, dim, nleft, alpha.as_ptr(), dp)
            })?
        }
        Cone::DualPower { alpha, .. } => {
            let nleft = int64("power cone exponents", alpha.len())?;
            // SAFETY: as stated in the SAFETY note for this block of calls above.
            calls.run(|t| unsafe {
                ffi::MSK_appenddualpowerconedomain(t, dim, nleft, alpha.as_ptr(), dp)
            })?
        }
        // SAFETY: as stated in the SAFETY note for this block of calls above.
        Cone::Nonnegative(_) => calls.run(|t| unsafe { ffi::MSK_appendrplusdomain(t, dim, dp) })?,
        // SAFETY: as stated in the SAFETY note for this block of calls above.
        Cone::Zero(_) => calls.run(|t| unsafe { ffi::MSK_appendrzerodomain(t, dim, dp) })?,
        Cone::PositiveSemidefinite { .. } => {
            // SAFETY: as stated in the SAFETY note for this block of calls above.
            calls.run(|t| unsafe { ffi::MSK_appendsvecpsdconedomain(t, dim, dp) })?
        }
    }
    Ok(d)
}
