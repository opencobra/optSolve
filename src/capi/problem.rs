//! C problem and options structures, and their translation onto the Rust API (feature 005,
//! data-model.md). Every array is borrowed from the caller in place (constitution IV, FR-003);
//! only the cone descriptors (one `Cone` per cone, with power-cone exponents) and raw-parameter
//! names and string values are copied, as the plan states.

use core::ffi::c_char;
use core::mem::{offset_of, size_of};
use std::ffi::CStr;

use super::*;
use crate::backend::ProblemRef;
use crate::conic::{Cone, ConicRef};
use crate::error::{Error, Result};
use crate::options::{Options, ParamValue};
use crate::problem::{CscRef, MAX_DIMENSION, Problem, RowBounds, Sense};

/// A borrowed compressed-sparse-column matrix with `size_t` row indices.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)] // C type name (header).
pub struct optsolver_csc {
    /// Number of rows.
    pub rows: usize,
    /// Number of columns.
    pub cols: usize,
    /// Column pointers, length `cols + 1`.
    pub col_ptrs: *const usize,
    /// Row indices, length `col_ptrs[cols]`.
    pub row_indices: *const usize,
    /// Values, length `col_ptrs[cols]`.
    pub values: *const f64,
}

/// One cone of the product K.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)] // C type name (header).
pub struct optsolver_cone {
    /// `OPTSOLVER_CONE_*`.
    pub kind: i32,
    /// Dimension; for positive semidefinite, the order.
    pub dim: usize,
    /// Power cones: exponents of the leading coordinates.
    pub exponents: *const f64,
    /// Number of exponents.
    pub exponents_len: usize,
}

/// A borrowed problem.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types, non_snake_case)] // C type name; Notation Convention (A, Q, F).
pub struct optsolver_problem {
    /// `sizeof(optsolver_problem)` as the caller compiled it.
    pub struct_size: u32,
    /// Constraint matrix A (m × n).
    pub A: optsolver_csc,
    /// If non-NULL, A's row indices as `int32_t` (then `A.row_indices` must be NULL).
    pub A_row_indices_i32: *const i32,
    /// Objective c, length n.
    pub c: *const f64,
    /// `OPTSOLVER_SENSE_*`.
    pub sense: i32,
    /// Objective constant γ.
    pub gamma: f64,
    /// Lower variable bounds l, length n.
    pub l: *const f64,
    /// Upper variable bounds u, length n.
    pub u: *const f64,
    /// Row senses `'='`, `'<'`, `'>'`, length m (with `b`).
    pub row_sense: *const c_char,
    /// Right-hand sides b, length m.
    pub b: *const f64,
    /// Lower row bounds b_l, length m (instead of `row_sense` and `b`).
    pub b_l: *const f64,
    /// Upper row bounds b_u, length m.
    pub b_u: *const f64,
    /// NULL, or the lower triangle of Q (n × n).
    pub Q: *const optsolver_csc,
    /// NULL, or F (k × n).
    pub F: *const optsolver_csc,
    /// g, length k.
    pub g: *const f64,
    /// The cones, `num_cones` of them.
    pub cones: *const optsolver_cone,
    /// Number of cones.
    pub num_cones: usize,
}

/// A raw vendor parameter.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)] // C type name (header).
pub struct optsolver_param {
    /// Vendor name (UTF-8, NUL-terminated).
    pub name: *const c_char,
    /// `OPTSOLVER_PARAM_*`.
    pub r#type: i32,
    /// Integer value.
    pub int_value: i64,
    /// Double value.
    pub double_value: f64,
    /// String value (UTF-8, NUL-terminated).
    pub string_value: *const c_char,
}

/// Options; every field may be unset. Fields beyond `struct_size` are treated as unset.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)] // C type name (header).
pub struct optsolver_options {
    /// `sizeof(optsolver_options)` as the caller compiled it.
    pub struct_size: u32,
    /// Seconds; NaN = unset.
    pub time_limit: f64,
    /// Simplex iterations; NaN = unset.
    pub iteration_limit: f64,
    /// `OPTSOLVER_UNSET` or ≥ 0.
    pub threads: i64,
    /// NaN = unset.
    pub primal_feasibility_tolerance: f64,
    /// NaN = unset.
    pub dual_feasibility_tolerance: f64,
    /// `OPTSOLVER_UNSET` or `OPTSOLVER_METHOD_*`.
    pub method: i32,
    /// `OPTSOLVER_UNSET` or `OPTSOLVER_PRESOLVE_*`.
    pub presolve: i32,
    /// Used when `random_seed_set` is non-zero.
    pub random_seed: i64,
    /// Whether `random_seed` is set.
    pub random_seed_set: i32,
    /// `OPTSOLVER_UNSET`, 0 or 1.
    pub solver_log: i32,
    /// `OPTSOLVER_UNSET`, 0 or 1.
    pub array_outputs: i32,
    /// `OPTSOLVER_UNSET`, 0 or 1.
    pub verification: i32,
    /// 0 or 1.
    pub certificates: i32,
    /// Gigabytes (10⁹ bytes); NaN = unset.
    pub memory_limit_gb: f64,
    /// Raw vendor parameters.
    pub raw: *const optsolver_param,
    /// Number of raw parameters.
    pub num_raw: usize,
}

/// Fills `options` with every field unset (`struct_size` = `sizeof(optsolver_options)`).
///
/// # Safety
/// `options` must be NULL or point to a writable `optsolver_options`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_options_init(options: *mut optsolver_options) {
    contain((), || {
        if options.is_null() {
            return;
        }
        let o = optsolver_options {
            struct_size: u32::try_from(size_of::<optsolver_options>()).unwrap_or(u32::MAX),
            time_limit: f64::NAN,
            iteration_limit: f64::NAN,
            threads: i64::from(OPTSOLVER_UNSET),
            primal_feasibility_tolerance: f64::NAN,
            dual_feasibility_tolerance: f64::NAN,
            method: OPTSOLVER_UNSET,
            presolve: OPTSOLVER_UNSET,
            random_seed: 0,
            random_seed_set: 0,
            solver_log: OPTSOLVER_UNSET,
            array_outputs: OPTSOLVER_UNSET,
            verification: OPTSOLVER_UNSET,
            certificates: 0,
            memory_limit_gb: f64::NAN,
            raw: core::ptr::null(),
            num_raw: 0,
        };
        // SAFETY: non-null and writable by the contract.
        unsafe { options.write(o) };
    })
}

/// Borrows `len` elements at `ptr` (constitution IV: no copy). `len` 0 gives an empty slice
/// whatever `ptr` is; NULL with a nonzero length is invalid; a length whose byte size exceeds
/// `isize::MAX` is too large; a misaligned pointer is invalid.
///
/// # Safety
/// A non-NULL `ptr` with nonzero `len` must point to `len` initialised elements that stay valid
/// and unmodified for `'a`.
pub(crate) unsafe fn borrow<'a, T>(
    ptr: *const T,
    len: usize,
    what: &'static str,
) -> Result<&'a [T]> {
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        return Err(invalid(what, format!("NULL with length {len}")));
    }
    if len > (isize::MAX as usize) / size_of::<T>().max(1) {
        return Err(Error::TooLarge { what, value: len });
    }
    if !ptr.is_aligned() {
        return Err(invalid(what, "misaligned pointer"));
    }
    // SAFETY: non-null, aligned, size within isize::MAX, and by this function's contract valid
    // for `len` elements for 'a.
    Ok(unsafe { core::slice::from_raw_parts(ptr, len) })
}

/// Validates a matrix's dimensions before any of its arrays is touched (contract: A's
/// dimensions first), with the Rust API's error.
fn dimensions(rows: usize, cols: usize) -> Result<()> {
    if rows > MAX_DIMENSION {
        return Err(Error::TooLarge {
            what: "m",
            value: rows,
        });
    }
    if cols > MAX_DIMENSION {
        return Err(Error::TooLarge {
            what: "n",
            value: cols,
        });
    }
    Ok(())
}

/// Column pointers and nnz of a C matrix, after its dimensions.
///
/// # Safety
/// As [`borrow`], for `col_ptrs` of length `cols + 1`.
unsafe fn col_ptrs<'a>(m: &optsolver_csc, what: &'static str) -> Result<(&'a [usize], usize)> {
    dimensions(m.rows, m.cols)?;
    let len = m.cols.checked_add(1).ok_or(Error::TooLarge {
        what,
        value: m.cols,
    })?;
    // SAFETY: caller's contract.
    let cp = unsafe { borrow(m.col_ptrs, len, what) }?;
    Ok((cp, cp[m.cols]))
}

/// A `size_t`-indexed C matrix as a validated [`CscRef`].
///
/// # Safety
/// The matrix's arrays must be valid as the header describes, for `'a`.
unsafe fn csc_usize<'a>(m: &optsolver_csc, names: [&'static str; 3]) -> Result<CscRef<'a, usize>> {
    // SAFETY: caller's contract.
    let (cp, nnz) = unsafe { col_ptrs(m, names[0]) }?;
    // SAFETY: caller's contract.
    let ri = unsafe { borrow(m.row_indices, nnz, names[1]) }?;
    // SAFETY: caller's contract.
    let va = unsafe { borrow(m.values, nnz, names[2]) }?;
    CscRef::new(m.rows, m.cols, cp, ri, va)
}

/// Reads `struct_size` and checks it covers at least `min` bytes.
///
/// # Safety
/// `p` must be non-NULL and point to at least a `u32`.
unsafe fn struct_size<T>(p: *const T, min: usize, what: &'static str) -> Result<usize> {
    // SAFETY: every C input structure begins with a u32 `struct_size`; `p` is non-null.
    let size = unsafe { p.cast::<u32>().read() } as usize;
    if size < min {
        return Err(invalid(
            what,
            format!("struct_size {size} is below the minimum {min} of interface version 1.0"),
        ));
    }
    Ok(size)
}

/// The cones as Rust values (one per cone; power-cone exponents copied).
fn cones(c: &[optsolver_cone]) -> Result<Vec<Cone>> {
    c.iter()
        .map(|c| {
            let kind = cone_kind(c.kind).ok_or_else(|| {
                invalid("cones.kind", format!("unknown cone kind code {}", c.kind))
            })?;
            Ok(match kind {
                crate::conic::ConeKind::Quadratic => Cone::Quadratic(c.dim),
                crate::conic::ConeKind::RotatedQuadratic => Cone::RotatedQuadratic(c.dim),
                crate::conic::ConeKind::Nonnegative => Cone::Nonnegative(c.dim),
                crate::conic::ConeKind::Zero => Cone::Zero(c.dim),
                crate::conic::ConeKind::PositiveSemidefinite => {
                    Cone::PositiveSemidefinite { order: c.dim }
                }
                crate::conic::ConeKind::PrimalExponential
                | crate::conic::ConeKind::DualExponential => {
                    if c.dim != 3 && c.dim != 0 {
                        return Err(invalid(
                            "cones.dim",
                            format!(
                                "an exponential cone has dimension 3 (or 0 for default), got {}",
                                c.dim
                            ),
                        ));
                    }
                    if kind == crate::conic::ConeKind::PrimalExponential {
                        Cone::PrimalExponential
                    } else {
                        Cone::DualExponential
                    }
                }
                crate::conic::ConeKind::PrimalPower | crate::conic::ConeKind::DualPower => {
                    // SAFETY: the caller's exponent array is valid for exponents_len elements.
                    let alpha = unsafe { borrow(c.exponents, c.exponents_len, "cones.exponents") }?
                        .to_vec();
                    if kind == crate::conic::ConeKind::PrimalPower {
                        Cone::PrimalPower { dim: c.dim, alpha }
                    } else {
                        Cone::DualPower { dim: c.dim, alpha }
                    }
                }
            })
        })
        .collect()
}

/// Builds the borrowed Rust problem from a C problem and runs `f` with it. Order: the
/// structure size; A's dimensions; A's arrays; then every other array (contract).
///
/// # Safety
/// `p` must be non-NULL and point to an `optsolver_problem` whose arrays are valid as the header
/// describes for the duration of the call.
pub(crate) unsafe fn with_problem<R>(
    p: *const optsolver_problem,
    f: impl FnOnce(ProblemRef<'_, '_>) -> Result<R>,
) -> Result<R> {
    if p.is_null() {
        return Err(invalid("problem", "NULL"));
    }
    // SAFETY: non-null; the contract.
    unsafe { struct_size(p, size_of::<optsolver_problem>(), "problem.struct_size") }?;
    // SAFETY: the structure is complete (size checked) and valid by the contract.
    let p = unsafe { &*p };
    let (m, n) = (p.A.rows, p.A.cols);
    // SAFETY: the contract (A's arrays valid).
    let (cp, nnz) = unsafe { col_ptrs(&p.A, "A.col_ptrs") }?;
    // SAFETY: the contract.
    let values = unsafe { borrow(p.A.values, nnz, "A.values") }?;
    let use_i32 = !p.A_row_indices_i32.is_null();
    if use_i32 && !p.A.row_indices.is_null() {
        return Err(invalid(
            "A.row_indices",
            "both A.row_indices and A_row_indices_i32 are given; give one",
        ));
    }
    if nnz > 0 && !use_i32 && p.A.row_indices.is_null() {
        return Err(invalid(
            "A.row_indices",
            "neither A.row_indices nor A_row_indices_i32 is given",
        ));
    }
    let sense = match p.sense {
        OPTSOLVER_SENSE_MINIMISE => Sense::Minimise,
        OPTSOLVER_SENSE_MAXIMISE => Sense::Maximise,
        other => {
            return Err(invalid(
                "sense",
                format!("{other} is not 1 (minimise) or -1 (maximise)"),
            ));
        }
    };
    // SAFETY: the contract (the caller's c is valid for n elements).
    let c = unsafe { borrow(p.c, n, "c") }?;
    // SAFETY: the contract.
    let l = unsafe { borrow(p.l, n, "l") }?;
    // SAFETY: the contract.
    let u = unsafe { borrow(p.u, n, "u") }?;
    let sense_form = !p.row_sense.is_null() || !p.b.is_null();
    let range_form = !p.b_l.is_null() || !p.b_u.is_null();
    let rows = match (sense_form, range_form) {
        (true, true) => {
            return Err(invalid(
                "row bounds",
                "both forms given (row_sense and b, and b_l and b_u); give one",
            ));
        }
        (true, false) => RowBounds::SenseRhs {
            // SAFETY: the contract; `char` and `u8` have the same size and alignment.
            sense: unsafe { borrow(p.row_sense.cast::<u8>(), m, "row_sense") }?,
            // SAFETY: the contract.
            b: unsafe { borrow(p.b, m, "b") }?,
        },
        (false, true) => RowBounds::Ranges {
            // SAFETY: the contract.
            b_l: unsafe { borrow(p.b_l, m, "b_l") }?,
            // SAFETY: the contract.
            b_u: unsafe { borrow(p.b_u, m, "b_u") }?,
        },
        (false, false) if m == 0 => RowBounds::Ranges { b_l: &[], b_u: &[] },
        (false, false) => {
            return Err(invalid(
                "row bounds",
                "neither form given (row_sense and b, or b_l and b_u)",
            ));
        }
    };
    // Q and conic data.
    let q = if p.Q.is_null() {
        None
    } else {
        // SAFETY: non-null and valid by the contract.
        let q = unsafe { &*p.Q };
        // SAFETY: Q's arrays are valid by the contract.
        Some(unsafe { csc_usize(q, ["Q.col_ptrs", "Q.row_indices", "Q.values"]) }?)
    };
    let cone_list = if p.F.is_null() {
        Vec::new()
    } else {
        // SAFETY: the contract.
        cones(unsafe { borrow(p.cones, p.num_cones, "cones") }?)?
    };
    let conic = if p.F.is_null() {
        None
    } else {
        // SAFETY: non-null and valid by the contract.
        let fm = unsafe { &*p.F };
        // SAFETY: F's arrays are valid by the contract.
        let f = unsafe { csc_usize(fm, ["F.col_ptrs", "F.row_indices", "F.values"]) }?;
        // SAFETY: the contract.
        let g = unsafe { borrow(p.g, f.m(), "g") }?;
        Some((f, g))
    };
    macro_rules! finish {
        ($A:expr) => {{
            let mut problem = Problem::new($A, c, sense, rows, l, u)?.with_gamma(p.gamma)?;
            if let Some(q) = q {
                problem = problem.with_Q(q)?;
            }
            if let Some((f, g)) = conic {
                problem = problem.with_conic(ConicRef::new(f, g, &cone_list)?)?;
            }
            f((&problem).into())
        }};
    }
    if use_i32 {
        // SAFETY: the contract.
        let ri = unsafe { borrow(p.A_row_indices_i32, nnz, "A.row_indices") }?;
        finish!(CscRef::new(m, n, cp, ri, values)?)
    } else {
        // SAFETY: the contract.
        let ri = unsafe { borrow(p.A.row_indices, nnz, "A.row_indices") }?;
        finish!(CscRef::new(m, n, cp, ri, values)?)
    }
}

fn utf8(p: *const c_char, what: &'static str) -> Result<String> {
    if p.is_null() {
        return Err(invalid(what, "NULL"));
    }
    // SAFETY: non-null and NUL-terminated by the contract.
    unsafe { CStr::from_ptr(p) }
        .to_str()
        .map(str::to_string)
        .map_err(|_| invalid(what, "not valid UTF-8"))
}

fn flag(v: i32, what: &'static str) -> Result<Option<bool>> {
    match v {
        OPTSOLVER_UNSET => Ok(None),
        0 => Ok(Some(false)),
        1 => Ok(Some(true)),
        other => Err(invalid(what, format!("{other} is not -1 (unset), 0 or 1"))),
    }
}

fn dbl(v: f64) -> Option<f64> {
    (!v.is_nan()).then_some(v)
}

/// Translates C options (NULL: every field unset). Fields beyond `struct_size` are unset.
///
/// # Safety
/// `o` must be NULL or point to at least `struct_size` readable bytes of an
/// `optsolver_options`, with valid raw-parameter arrays and strings.
pub(crate) unsafe fn options<'cb>(o: *const optsolver_options) -> Result<Options<'cb>> {
    let mut out = Options::default();
    if o.is_null() {
        return Ok(out);
    }
    // SAFETY: non-null; at least the u32 size field by the contract.
    let size = unsafe { struct_size(o, size_of::<u32>(), "options.struct_size") }?;
    macro_rules! field {
        ($name:ident) => {{
            let end =
                offset_of!(optsolver_options, $name) + size_of_val(&optsolver_default().$name);
            if end <= size {
                // SAFETY: the caller's structure covers this field (end ≤ struct_size), so the
                // projection stays in bounds of the caller's allocation.
                let field = unsafe { core::ptr::addr_of!((*o).$name) };
                // SAFETY: as above; the field's bytes are initialised by the caller.
                Some(unsafe { field.read_unaligned() })
            } else {
                None
            }
        }};
    }
    if let Some(v) = field!(time_limit) {
        out.time_limit = dbl(v);
    }
    if let Some(v) = field!(iteration_limit) {
        out.iteration_limit = dbl(v);
    }
    if let Some(v) = field!(threads) {
        out.threads = match v {
            -1 => None,
            v => Some(u32::try_from(v).map_err(|_| {
                invalid(
                    "options.threads",
                    format!("{v} is not -1 (unset) or 0..=4294967295"),
                )
            })?),
        };
    }
    if let Some(v) = field!(primal_feasibility_tolerance) {
        out.primal_feasibility_tolerance = dbl(v);
    }
    if let Some(v) = field!(dual_feasibility_tolerance) {
        out.dual_feasibility_tolerance = dbl(v);
    }
    if let Some(v) = field!(method) {
        if v != OPTSOLVER_UNSET {
            out.method = Some(
                method_of(v)
                    .ok_or_else(|| invalid("options.method", format!("unknown code {v}")))?,
            );
        }
    }
    if let Some(v) = field!(presolve) {
        if v != OPTSOLVER_UNSET {
            out.presolve = Some(
                presolve_of(v)
                    .ok_or_else(|| invalid("options.presolve", format!("unknown code {v}")))?,
            );
        }
    }
    if let (Some(seed), Some(set)) = (field!(random_seed), field!(random_seed_set)) {
        if set != 0 {
            out.random_seed = Some(i32::try_from(seed).map_err(|_| {
                invalid("options.random_seed", format!("{seed} is outside int32_t"))
            })?);
        }
    }
    if let Some(v) = field!(solver_log) {
        out.solver_log = flag(v, "options.solver_log")?;
    }
    if let Some(v) = field!(array_outputs) {
        out.array_outputs = flag(v, "options.array_outputs")?;
    }
    if let Some(v) = field!(verification) {
        out.verification = flag(v, "options.verification")?;
    }
    if let Some(v) = field!(certificates) {
        out.certificates = match v {
            0 => false,
            1 => true,
            other => {
                return Err(invalid(
                    "options.certificates",
                    format!("{other} is not 0 or 1"),
                ));
            }
        };
    }
    if let Some(v) = field!(memory_limit_gb) {
        out.memory_limit_gb = dbl(v);
    }
    if let (Some(raw), Some(num)) = (field!(raw), field!(num_raw)) {
        // SAFETY: the contract.
        let params = unsafe { borrow(raw, num, "options.raw") }?;
        for prm in params {
            let name = utf8(prm.name, "options.raw.name")?;
            let value = match prm.r#type {
                OPTSOLVER_PARAM_INT => ParamValue::Int(prm.int_value),
                OPTSOLVER_PARAM_DOUBLE => ParamValue::Double(prm.double_value),
                OPTSOLVER_PARAM_STRING => {
                    ParamValue::Str(utf8(prm.string_value, "options.raw.string_value")?)
                }
                other => {
                    return Err(invalid("options.raw.type", format!("unknown code {other}")));
                }
            };
            out.raw.push((name, value));
        }
    }
    Ok(out)
}

/// A zero-valued options structure, used only for field sizes.
const fn optsolver_default() -> optsolver_options {
    optsolver_options {
        struct_size: 0,
        time_limit: 0.0,
        iteration_limit: 0.0,
        threads: 0,
        primal_feasibility_tolerance: 0.0,
        dual_feasibility_tolerance: 0.0,
        method: 0,
        presolve: 0,
        random_seed: 0,
        random_seed_set: 0,
        solver_log: 0,
        array_outputs: 0,
        verification: 0,
        certificates: 0,
        memory_limit_gb: 0.0,
        raw: core::ptr::null(),
        num_raw: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Lp {
        cp: Vec<usize>,
        ri: Vec<usize>,
        ri32: Vec<i32>,
        va: Vec<f64>,
        c: Vec<f64>,
        l: Vec<f64>,
        u: Vec<f64>,
        b_l: Vec<f64>,
        b_u: Vec<f64>,
    }

    fn lp() -> Lp {
        Lp {
            cp: vec![0, 2, 3],
            ri: vec![0, 1, 1],
            ri32: vec![0, 1, 1],
            va: vec![1.0, 2.0, 3.0],
            c: vec![1.0, 1.0],
            l: vec![0.0, 0.0],
            u: vec![10.0, 10.0],
            b_l: vec![1.0, 1.0],
            b_u: vec![5.0, 5.0],
        }
    }

    fn problem(d: &Lp) -> optsolver_problem {
        optsolver_problem {
            struct_size: u32::try_from(size_of::<optsolver_problem>()).unwrap(),
            A: optsolver_csc {
                rows: 2,
                cols: 2,
                col_ptrs: d.cp.as_ptr(),
                row_indices: d.ri.as_ptr(),
                values: d.va.as_ptr(),
            },
            A_row_indices_i32: core::ptr::null(),
            c: d.c.as_ptr(),
            sense: OPTSOLVER_SENSE_MINIMISE,
            gamma: 0.0,
            l: d.l.as_ptr(),
            u: d.u.as_ptr(),
            row_sense: core::ptr::null(),
            b: core::ptr::null(),
            b_l: d.b_l.as_ptr(),
            b_u: d.b_u.as_ptr(),
            Q: core::ptr::null(),
            F: core::ptr::null(),
            g: core::ptr::null(),
            cones: core::ptr::null(),
            num_cones: 0,
        }
    }

    fn err_of(p: &optsolver_problem) -> Error {
        // SAFETY: `p` points to arrays owned by the test, alive for the call.
        unsafe { with_problem(p, |_| Ok(())) }.expect_err("expected an error")
    }

    fn assert_invalid(e: Error, what: &str) {
        match e {
            Error::Invalid { what: w, .. } => assert_eq!(w, what),
            other => panic!("expected Invalid {what}, got {other:?}"),
        }
    }

    #[test]
    fn every_slice_is_the_callers_array() {
        let d = lp();
        let p = problem(&d);
        // SAFETY: arrays alive for the call.
        unsafe {
            with_problem(&p, |r| {
                let ProblemRef::Usize(q) = r else {
                    panic!("usize problem expected")
                };
                assert_eq!(q.A().col_ptrs().as_ptr(), d.cp.as_ptr());
                assert_eq!(q.A().row_indices().as_ptr(), d.ri.as_ptr());
                assert_eq!(q.A().values().as_ptr(), d.va.as_ptr());
                assert_eq!(q.c().as_ptr(), d.c.as_ptr());
                assert_eq!(q.l().as_ptr(), d.l.as_ptr());
                assert_eq!(q.u().as_ptr(), d.u.as_ptr());
                let RowBounds::Ranges { b_l, b_u } = q.rows() else {
                    panic!("ranges")
                };
                assert_eq!(b_l.as_ptr(), d.b_l.as_ptr());
                assert_eq!(b_u.as_ptr(), d.b_u.as_ptr());
                Ok(())
            })
        }
        .unwrap();
        let mut p = problem(&d);
        p.A.row_indices = core::ptr::null();
        p.A_row_indices_i32 = d.ri32.as_ptr();
        // SAFETY: arrays alive for the call.
        unsafe {
            with_problem(&p, |r| {
                let ProblemRef::I32(q) = r else {
                    panic!("i32 problem expected")
                };
                assert_eq!(q.A().row_indices().as_ptr(), d.ri32.as_ptr());
                Ok(())
            })
        }
        .unwrap();
    }

    #[test]
    fn sense_form_q_and_conic_slices_are_the_callers_arrays() {
        // The remaining borrowed arrays: row sense and b, Q, F and g (the cone descriptors and
        // power-cone exponents are the documented small copies).
        let d = lp();
        let sense = *b"<<";
        let b = [5.0, 5.0];
        let (qcp, qri, qva) = ([0usize, 1, 2], [0usize, 1], [1.0, 1.0]);
        let (fcp, fri, fva, g) = ([0usize, 1, 2], [1usize, 2], [1.0, 1.0], [1.0, 0.0, 0.0]);
        let q = optsolver_csc {
            rows: 2,
            cols: 2,
            col_ptrs: qcp.as_ptr(),
            row_indices: qri.as_ptr(),
            values: qva.as_ptr(),
        };
        let f = optsolver_csc {
            rows: 3,
            cols: 2,
            col_ptrs: fcp.as_ptr(),
            row_indices: fri.as_ptr(),
            values: fva.as_ptr(),
        };
        let cone = [optsolver_cone {
            kind: OPTSOLVER_CONE_QUADRATIC,
            dim: 3,
            exponents: core::ptr::null(),
            exponents_len: 0,
        }];
        let mut p = problem(&d);
        p.b_l = core::ptr::null();
        p.b_u = core::ptr::null();
        p.row_sense = sense.as_ptr().cast();
        p.b = b.as_ptr();
        p.Q = &q;
        p.F = &f;
        p.g = g.as_ptr();
        p.cones = cone.as_ptr();
        p.num_cones = 1;
        // SAFETY: every array is alive for the call.
        unsafe {
            with_problem(&p, |r| {
                let ProblemRef::Usize(pr) = r else {
                    panic!("usize problem expected")
                };
                let RowBounds::SenseRhs { sense: s, b: bb } = pr.rows() else {
                    panic!("sense form expected")
                };
                assert_eq!(s.as_ptr(), sense.as_ptr());
                assert_eq!(bb.as_ptr(), b.as_ptr());
                let qq = pr.Q().expect("Q");
                assert_eq!(qq.col_ptrs().as_ptr(), qcp.as_ptr());
                assert_eq!(qq.row_indices().as_ptr(), qri.as_ptr());
                assert_eq!(qq.values().as_ptr(), qva.as_ptr());
                let k = pr.conic().expect("conic");
                assert_eq!(k.F().col_ptrs().as_ptr(), fcp.as_ptr());
                assert_eq!(k.F().row_indices().as_ptr(), fri.as_ptr());
                assert_eq!(k.F().values().as_ptr(), fva.as_ptr());
                assert_eq!(k.g().as_ptr(), g.as_ptr());
                Ok(())
            })
        }
        .unwrap();
    }

    #[test]
    fn c_layer_checks_name_the_field() {
        let d = lp();
        let mut p = problem(&d);
        p.c = core::ptr::null();
        assert_invalid(err_of(&p), "c");
        let mut p = problem(&d);
        p.b = d.b_l.as_ptr();
        assert_invalid(err_of(&p), "row bounds");
        let mut p = problem(&d);
        p.b_l = core::ptr::null();
        p.b_u = core::ptr::null();
        assert_invalid(err_of(&p), "row bounds");
        let mut p = problem(&d);
        p.A_row_indices_i32 = d.ri32.as_ptr();
        assert_invalid(err_of(&p), "A.row_indices");
        let mut p = problem(&d);
        p.A.row_indices = core::ptr::null();
        assert_invalid(err_of(&p), "A.row_indices");
        let mut p = problem(&d);
        p.sense = 0;
        assert_invalid(err_of(&p), "sense");
        let mut p = problem(&d);
        p.struct_size = 8;
        assert_invalid(err_of(&p), "problem.struct_size");
        let neg = [0i32, -1, 1];
        let mut p = problem(&d);
        p.A.row_indices = core::ptr::null();
        p.A_row_indices_i32 = neg.as_ptr();
        assert_invalid(err_of(&p), "A.row_indices");
        // SAFETY: NULL problem is checked before any read.
        assert_invalid(
            unsafe { with_problem(core::ptr::null(), |_| Ok(())) }.unwrap_err(),
            "problem",
        );
    }

    #[test]
    fn dimensions_are_checked_before_any_array() {
        let d = lp();
        let mut p = problem(&d);
        p.A.rows = MAX_DIMENSION + 1;
        assert!(matches!(err_of(&p), Error::TooLarge { what: "m", .. }));
        let one = [0usize];
        let mut p = problem(&d);
        p.A.cols = 1usize << 31;
        p.A.col_ptrs = one.as_ptr(); // one element only: must not be read beyond it
        assert!(matches!(err_of(&p), Error::TooLarge { what: "n", .. }));
        // SAFETY: a length whose byte size exceeds isize::MAX is rejected before slicing.
        let e = unsafe { borrow(d.c.as_ptr(), usize::MAX / 2, "c") }.unwrap_err();
        assert!(matches!(e, Error::TooLarge { what: "c", .. }));
    }

    #[test]
    fn options_translate_and_reject() {
        let mut o = core::mem::MaybeUninit::<optsolver_options>::uninit();
        // SAFETY: writable.
        unsafe { optsolver_options_init(o.as_mut_ptr()) };
        // SAFETY: initialised above.
        let base = unsafe { o.assume_init() };
        // SAFETY: valid structure.
        let r = unsafe { options(&base) }.unwrap();
        assert!(r.set_shared_fields().is_empty() && r.raw.is_empty());
        let mut x = base;
        x.threads = 1 << 40;
        // SAFETY: valid structure.
        assert_invalid(unsafe { options(&x) }.unwrap_err(), "options.threads");
        let mut x = base;
        x.random_seed_set = 1;
        x.random_seed = 1 << 40;
        // SAFETY: valid structure.
        assert_invalid(unsafe { options(&x) }.unwrap_err(), "options.random_seed");
        let mut x = base;
        x.method = 9;
        // SAFETY: valid structure.
        assert_invalid(unsafe { options(&x) }.unwrap_err(), "options.method");
        let mut x = base;
        x.presolve = 9;
        // SAFETY: valid structure.
        assert_invalid(unsafe { options(&x) }.unwrap_err(), "options.presolve");
        let bad = [0xffu8, 0];
        let prm = [optsolver_param {
            name: bad.as_ptr().cast(),
            r#type: OPTSOLVER_PARAM_INT,
            int_value: 1,
            double_value: 0.0,
            string_value: core::ptr::null(),
        }];
        let mut x = base;
        x.raw = prm.as_ptr();
        x.num_raw = 1;
        // SAFETY: valid structure.
        assert_invalid(unsafe { options(&x) }.unwrap_err(), "options.raw.name");
        let prm = [optsolver_param {
            name: c"Method".as_ptr(),
            r#type: OPTSOLVER_PARAM_STRING,
            int_value: 0,
            double_value: 0.0,
            string_value: bad.as_ptr().cast(),
        }];
        x.raw = prm.as_ptr();
        // SAFETY: valid structure.
        assert_invalid(
            unsafe { options(&x) }.unwrap_err(),
            "options.raw.string_value",
        );
        let prm = [optsolver_param {
            r#type: 7,
            ..prm[0]
        }];
        x.raw = prm.as_ptr();
        // SAFETY: valid structure.
        assert_invalid(unsafe { options(&x) }.unwrap_err(), "options.raw.type");
        // A truncated structure: only up to time_limit; the rest unset.
        let mut x = base;
        x.time_limit = 5.0;
        x.threads = 3;
        x.struct_size = u32::try_from(offset_of!(optsolver_options, time_limit) + 8).unwrap();
        // SAFETY: valid structure (the reader reads only covered fields).
        let r = unsafe { options(&x) }.unwrap();
        assert_eq!(r.time_limit, Some(5.0));
        assert_eq!(r.threads, None);
        let mut x = base;
        x.memory_limit_gb = 2.0;
        // SAFETY: valid structure.
        assert_eq!(unsafe { options(&x) }.unwrap().memory_limit_gb, Some(2.0));
    }

    #[test]
    fn option_flags_and_raw_strings_are_checked() {
        let mut o = core::mem::MaybeUninit::<optsolver_options>::uninit();
        // SAFETY: writable.
        unsafe { optsolver_options_init(o.as_mut_ptr()) };
        // SAFETY: initialised above.
        let base = unsafe { o.assume_init() };
        type Change = fn(&mut optsolver_options);
        let cases: [(Change, &str); 4] = [
            (|x| x.solver_log = 2, "options.solver_log"),
            (|x| x.array_outputs = -2, "options.array_outputs"),
            (|x| x.verification = 5, "options.verification"),
            (|x| x.certificates = -1, "options.certificates"),
        ];
        for (change, what) in cases {
            let mut x = base;
            change(&mut x);
            // SAFETY: valid structure.
            assert_invalid(unsafe { options(&x) }.unwrap_err(), what);
        }
        let prm = [optsolver_param {
            name: core::ptr::null(),
            r#type: OPTSOLVER_PARAM_INT,
            int_value: 1,
            double_value: 0.0,
            string_value: core::ptr::null(),
        }];
        let mut x = base;
        x.raw = prm.as_ptr();
        x.num_raw = 1;
        // SAFETY: valid structure.
        assert_invalid(unsafe { options(&x) }.unwrap_err(), "options.raw.name");
        let prm = [optsolver_param {
            name: c"Method".as_ptr(),
            r#type: OPTSOLVER_PARAM_STRING,
            ..prm[0]
        }];
        x.raw = prm.as_ptr();
        // SAFETY: valid structure.
        assert_invalid(
            unsafe { options(&x) }.unwrap_err(),
            "options.raw.string_value",
        );
    }

    #[test]
    fn misaligned_input_array_is_rejected() {
        let words = [0u64; 4];
        let misaligned = words.as_ptr().cast::<u8>().wrapping_add(1).cast::<f64>();
        // SAFETY: the pointer is rejected before any read.
        let e = unsafe { borrow(misaligned, 2, "c") }.unwrap_err();
        assert_invalid(e, "c");
    }

    #[test]
    fn exponential_cone_dimension_is_checked() {
        let c = [optsolver_cone {
            kind: OPTSOLVER_CONE_PRIMAL_EXPONENTIAL,
            dim: 4,
            exponents: core::ptr::null(),
            exponents_len: 0,
        }];
        assert_invalid(cones(&c).unwrap_err(), "cones.dim");
        let c = [optsolver_cone { kind: 42, ..c[0] }];
        assert_invalid(cones(&c).unwrap_err(), "cones.kind");
    }
}
