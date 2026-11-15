//! Gurobi quadratic objective and second-order cones (feature 003, tasks T018–T019; research
//! R1–R3; ADR-0003).
//!
//! - **Q**: one `GRBaddqpterms` with the caller's lower triangle: diagonal entries halved
//!   (Gurobi's objective is xᵀQx, ours ½xᵀQx), off-diagonal entries once. The three triplet
//!   arrays are built: O(nnz(Q)), the plan's Complexity Tracking entry.
//! - **Fx + g ∈ K** (quadratic, rotated quadratic, nonnegative and zero cones only, enforced
//!   by the shared capability check): k rows appended after the m rows with `GRBXaddconstrs`
//!   (no coefficients; sense and right-hand side −g: `>` for nonnegative, `=` otherwise), F's
//!   coefficients set with `GRBXchgcoeffs` per block of at most [`BLOCK_CAPACITY`] entries
//!   through a fixed-size buffer, auxiliary columns u (coefficient −1 in their row, so the row
//!   reads Fx − u = −g) for quadratic and rotated quadratic blocks with u₁ ≥ 0 (and u₂ ≥ 0 for
//!   rotated), and one convex quadratic constraint per such cone:
//!   Σ_{i≥2} u_i² − u₁² ≤ 0 (quadratic) or Σ_{i≥3} u_i² − 2u₁u₂ ≤ 0 (rotated). Cones of
//!   dimension 1 (quadratic) or 2 (rotated) need only the bounds. With `QCPDual` = 1 the conic
//!   dual is s = `Pi` of the k rows (research R3).
//!
//! Every array passed to a non-`const` Gurobi parameter here is owned by this module (no
//! caller slice is passed), so no VII-A exception is relied on.

#![allow(non_snake_case)] // Notation Convention (XII-C): Q, F are matrices.

use core::ffi::c_int;
use core::ptr;
use std::time::Instant;

use super::ffi;
use super::load::{Loaded, model_error, to_int};
use crate::conic::{Cone, ConicRef};
use crate::error::Result;
use crate::problem::CscRef;
use crate::problem::convert::BLOCK_CAPACITY;

/// Adds the quadratic objective ½xᵀQx (Q by its lower triangle).
pub(crate) fn add_quadratic_objective(
    loaded: &mut Loaded<'_>,
    Q: &CscRef<'_, usize>,
) -> Result<()> {
    let nnz = Q.nnz();
    if nnz == 0 {
        return Ok(());
    }
    let count = to_int("nnz(Q)", nnz)?;
    let mut qrow = Vec::with_capacity(nnz);
    let mut qcol = Vec::with_capacity(nnz);
    let mut qval = Vec::with_capacity(nnz);
    for j in 0..Q.n() {
        let (rows, vals) = Q.column(j);
        let jj = to_int("column of Q", j)?;
        for (&i, &v) in rows.iter().zip(vals) {
            qrow.push(to_int("row of Q", i)?);
            qcol.push(jj);
            qval.push(if i == j { 0.5 * v } else { v });
        }
    }
    let t = Instant::now();
    // SAFETY: the model is live (owned by `loaded`); qrow, qcol and qval are owned arrays of
    // `count` entries, the number GRBaddqpterms reads (manual, GRBaddqpterms).
    let code = unsafe {
        ffi::GRBaddqpterms(
            loaded.model.as_ptr(),
            count,
            qrow.as_mut_ptr(),
            qcol.as_mut_ptr(),
            qval.as_mut_ptr(),
        )
    };
    loaded.vendor_seconds += t.elapsed().as_secs_f64();
    if code != 0 {
        return Err(model_error(loaded.model, code));
    }
    loaded.q_terms = nnz;
    Ok(())
}

/// Adds Fx + g ∈ K for quadratic, rotated quadratic, nonnegative and zero cones.
pub(crate) fn add_conic(loaded: &mut Loaded<'_>, conic: &ConicRef<'_>) -> Result<()> {
    let model = loaded.model;
    let (m, k) = (loaded.m, conic.k());
    let F = conic.F();
    let g = conic.g();
    let mut vendor = 0.0;
    let mut call = |f: &mut dyn FnMut() -> c_int| -> Result<()> {
        let t = Instant::now();
        let code = f();
        vendor += t.elapsed().as_secs_f64();
        if code != 0 {
            Err(model_error(model, code))
        } else {
            Ok(())
        }
    };

    // Rows m .. m + k: sense and right-hand side −g, no coefficients yet. O(k).
    let mut sense: Vec<core::ffi::c_char> = Vec::with_capacity(k);
    for (_, cone) in conic.blocks() {
        let s = match cone {
            Cone::Nonnegative(_) => ffi::GRB_GREATER_EQUAL,
            _ => ffi::GRB_EQUAL,
        };
        sense.extend(std::iter::repeat_n(s, cone.dim()));
    }
    let mut rhs: Vec<f64> = g.iter().map(|v| -v).collect();
    let mut cbeg = vec![0usize; k];
    let (mut cind, mut cval): (Vec<c_int>, Vec<f64>) = (Vec::new(), Vec::new());
    let k_int = to_int("k", k)?;
    // SAFETY: the model is live; cbeg, sense and rhs have k entries, cind and cval none (numnz
    // = 0, so Gurobi reads none of them; their dangling non-null pointers are never read);
    // constrnames NULL is documented as allowed (manual, GRBXaddconstrs).
    call(&mut || unsafe {
        ffi::GRBXaddconstrs(
            model.as_ptr(),
            k_int,
            0,
            cbeg.as_mut_ptr(),
            cind.as_mut_ptr(),
            cval.as_mut_ptr(),
            sense.as_mut_ptr(),
            rhs.as_mut_ptr(),
            ptr::null_mut(),
        )
    })?;

    // F's coefficients, block by block through fixed-size buffers.
    let cp = F.col_ptrs();
    let (ri, va) = (F.row_indices(), F.values());
    let cap = BLOCK_CAPACITY.min(F.nnz().max(1));
    let mut rows_buf: Vec<c_int> = Vec::with_capacity(cap);
    let mut cols_buf: Vec<c_int> = Vec::with_capacity(cap);
    let mut vals_buf: Vec<f64> = Vec::with_capacity(cap);
    let mut j = 0;
    let mut s = 0;
    while s < F.nnz() {
        let e = (s + cap).min(F.nnz());
        rows_buf.clear();
        cols_buf.clear();
        vals_buf.clear();
        for t in s..e {
            while cp[j + 1] <= t {
                j += 1;
            }
            rows_buf.push(to_int("conic row", m + ri[t])?);
            cols_buf.push(to_int("column", j)?);
            vals_buf.push(va[t]);
        }
        loaded.buffer_high_water = loaded.buffer_high_water.max(rows_buf.len());
        let cnt = rows_buf.len();
        // SAFETY: the model is live; the three owned buffers have `cnt` entries, the number
        // GRBXchgcoeffs reads (manual, GRBXchgcoeffs).
        call(&mut || unsafe {
            ffi::GRBXchgcoeffs(
                model.as_ptr(),
                cnt,
                rows_buf.as_mut_ptr(),
                cols_buf.as_mut_ptr(),
                vals_buf.as_mut_ptr(),
            )
        })?;
        s = e;
    }

    // Auxiliary columns for quadratic and rotated quadratic blocks: −1 in their row. O(k).
    let first_aux = loaded.n + loaded.aux_columns;
    let mut vind: Vec<c_int> = Vec::new();
    let mut lb: Vec<f64> = Vec::new();
    let mut cones: Vec<(usize, bool, usize)> = Vec::new(); // (first aux column, rotated, dim)
    for (start, cone) in conic.blocks() {
        let (rotated, d) = match cone {
            Cone::Quadratic(d) => (false, *d),
            Cone::RotatedQuadratic(d) => (true, *d),
            _ => continue,
        };
        cones.push((first_aux + vind.len(), rotated, d));
        for i in 0..d {
            vind.push(to_int("conic row", m + start + i)?);
            let nonnegative = i == 0 || (rotated && i == 1);
            lb.push(if nonnegative { 0.0 } else { -ffi::GRB_INFINITY });
        }
    }
    let aux = vind.len();
    if aux > 0 {
        let aux_int = to_int("auxiliary columns", aux)?;
        let mut vbeg: Vec<usize> = (0..aux).collect();
        let mut vval = vec![-1.0_f64; aux];
        let mut ub = vec![ffi::GRB_INFINITY; aux];
        // SAFETY: the model is live; vbeg, vind, vval, lb and ub are owned arrays of `aux`
        // entries (aux columns, aux nonzeros); obj, vtype and varnames NULL are documented as
        // allowed (manual, GRBXaddvars).
        call(&mut || unsafe {
            ffi::GRBXaddvars(
                model.as_ptr(),
                aux_int,
                aux,
                vbeg.as_mut_ptr(),
                vind.as_mut_ptr(),
                vval.as_mut_ptr(),
                ptr::null_mut(),
                lb.as_mut_ptr(),
                ub.as_mut_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        })?;
        // New columns must exist before quadratic constraints refer to them.
        // SAFETY: the model is live (manual, GRBupdatemodel).
        call(&mut || unsafe { ffi::GRBupdatemodel(model.as_ptr()) })?;
    }

    // One convex quadratic constraint per quadratic (d ≥ 2) or rotated (d ≥ 3) cone.
    for &(first, rotated, d) in &cones {
        if (!rotated && d < 2) || (rotated && d < 3) {
            continue; // the bounds already define the cone
        }
        let col = |i: usize| to_int("auxiliary column", first + i);
        let (mut qrow, mut qcol, mut qval) = (Vec::new(), Vec::new(), Vec::new());
        let tail = if rotated { 2 } else { 1 };
        for i in tail..d {
            qrow.push(col(i)?);
            qcol.push(col(i)?);
            qval.push(1.0);
        }
        if rotated {
            qrow.push(col(0)?);
            qcol.push(col(1)?);
            qval.push(-2.0);
        } else {
            qrow.push(col(0)?);
            qcol.push(col(0)?);
            qval.push(-1.0);
        }
        let nq = to_int("quadratic terms", qval.len())?;
        // SAFETY: the model is live; qrow, qcol and qval are owned arrays of `nq` entries; no
        // linear part (numlnz 0, lind and lval NULL, documented as allowed); QCname NULL is
        // allowed (manual, GRBaddqconstr).
        call(&mut || unsafe {
            ffi::GRBaddqconstr(
                model.as_ptr(),
                0,
                ptr::null_mut(),
                ptr::null_mut(),
                nq,
                qrow.as_mut_ptr(),
                qcol.as_mut_ptr(),
                qval.as_mut_ptr(),
                ffi::GRB_LESS_EQUAL,
                0.0,
                ptr::null(),
            )
        })?;
    }
    loaded.vendor_seconds += vendor;
    loaded.k = k;
    loaded.conic_aux = aux;
    Ok(())
}
