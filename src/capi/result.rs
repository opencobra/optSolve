//! Solving and reading results through C (feature 005; FR-005 to FR-007).

use core::ffi::c_char;
use core::mem::size_of;
use std::ffi::CString;
use std::sync::OnceLock;

use super::*;
use crate::error::{Error, Result};
use crate::options::ParamValue;
use crate::result::{Certificate, Solution, SolutionBuffers};

/// A library-owned result. Arrays are read in place; names are converted to NUL-terminated
/// strings on first read.
#[allow(non_camel_case_types)] // C type name (header).
pub struct optsolver_result {
    pub(crate) solution: Solution,
    names: OnceLock<Names>,
}

struct Names {
    solver_name: CString,
    solver_version: CString,
    raw_status: Vec<CString>,
    attributes: Vec<(CString, Option<CString>)>,
    parameters: Vec<(CString, Option<CString>)>,
}

fn cstring(s: &str) -> CString {
    CString::new(s.replace('\0', " ")).unwrap_or_default()
}

fn pairs(v: &[(String, ParamValue)]) -> Vec<(CString, Option<CString>)> {
    v.iter()
        .map(|(n, val)| {
            let s = match val {
                ParamValue::Str(s) => Some(cstring(s)),
                _ => None,
            };
            (cstring(n), s)
        })
        .collect()
}

impl optsolver_result {
    pub(crate) fn new(solution: Solution) -> Self {
        Self {
            solution,
            names: OnceLock::new(),
        }
    }
    fn names(&self) -> &Names {
        self.names.get_or_init(|| {
            let s = &self.solution;
            Names {
                solver_name: cstring(&s.solver.name),
                solver_version: cstring(&s.solver.version),
                raw_status: s.raw_status.iter().map(|(n, _)| cstring(n)).collect(),
                attributes: pairs(&s.attributes),
                parameters: pairs(&s.effective_parameters),
            }
        })
    }
}

/// A caller-owned output array.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)] // C type name (header).
pub struct optsolver_array {
    /// The array, or NULL when not wanted.
    pub data: *mut f64,
    /// Its length.
    pub len: usize,
}

/// Caller-owned output arrays for [`optsolver_solve_into`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)] // C type name (header).
pub struct optsolver_buffers {
    /// `sizeof(optsolver_buffers)` as the caller compiled it.
    pub struct_size: u32,
    /// Primal x (length n, required).
    pub x: optsolver_array,
    /// Row duals y (length m).
    pub y: optsolver_array,
    /// Reduced costs z (length n).
    pub z: optsolver_array,
    /// Row activity (length m).
    pub row_activity: optsolver_array,
    /// Conic duals s (length k).
    pub s: optsolver_array,
    /// Lower-bound multipliers z_l (length n).
    pub z_l: optsolver_array,
    /// Upper-bound multipliers z_u (length n).
    pub z_u: optsolver_array,
}

/// Phase timings in seconds.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[allow(non_camel_case_types)] // C type name (header).
pub struct optsolver_timings {
    /// Validation, conversion and loading.
    pub load: f64,
    /// Optimisation.
    pub solve: f64,
    /// Result extraction.
    pub extraction: f64,
    /// Independent verification.
    pub verification: f64,
}

/// A typed parameter or attribute value.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)] // C type name (header).
pub struct optsolver_value {
    /// `OPTSOLVER_PARAM_*`.
    pub r#type: i32,
    /// Integer value.
    pub int_value: i64,
    /// Double value.
    pub double_value: f64,
    /// String value (owned by the result).
    pub string_value: *const c_char,
}

/// One verification measure.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)] // C type name (header).
pub struct optsolver_measure {
    /// Name (static).
    pub name: *const c_char,
    /// Absolute value.
    pub absolute: f64,
    /// Relative value.
    pub relative: f64,
    /// Tolerance judged against.
    pub tolerance: f64,
    /// `OPTSOLVER_VERDICT_*`.
    pub verdict: i32,
    /// 1 if the status requires this measure.
    pub required: i32,
}

/// Writes a new result to `out`.
///
/// # Safety
/// `out` must be non-NULL and writable.
unsafe fn emit(out: *mut *mut optsolver_result, s: Solution) {
    let r = Box::new(optsolver_result::new(s));
    // SAFETY: by the contract.
    unsafe { out.write(Box::into_raw(r)) };
}

fn env_ref<'e>(env: *mut optsolver_env) -> Result<&'e optsolver_env> {
    if env.is_null() {
        return Err(invalid("env", "NULL"));
    }
    // SAFETY: a live handle, used by one thread at a time, by the entry point's contract.
    Ok(unsafe { &*env })
}

/// Solves `problem` with `env`; on success writes a library-owned result to `out` (free it with
/// [`optsolver_result_free`]). A NULL `options` means every option unset.
///
/// # Safety
/// `env` must be a live handle not in use on another thread; `problem` and `options` (if
/// non-NULL) must describe valid arrays for the duration of the call; `out` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_solve(
    env: *mut optsolver_env,
    problem: *const optsolver_problem,
    options: *const optsolver_options,
    out: *mut *mut optsolver_result,
) -> i32 {
    guard(|| {
        if out.is_null() {
            return Err(invalid("out", "NULL"));
        }
        let env = env_ref(env)?;
        // SAFETY: the contract.
        let mut o = unsafe { super::problem::options(options) }?;
        // SAFETY: the contract.
        let s = unsafe { with_problem(problem, |p| env.inner.backend().solve(p, &mut o)) }?;
        // SAFETY: `out` non-null and writable.
        unsafe { emit(out, s) };
        Ok(())
    })
}

/// Solves once without a handle: validates the problem and options, then creates and frees an
/// environment of `backend`.
///
/// # Safety
/// As [`optsolver_solve`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_solve_once(
    backend: i32,
    problem: *const optsolver_problem,
    options: *const optsolver_options,
    out: *mut *mut optsolver_result,
) -> i32 {
    guard(|| {
        if out.is_null() {
            return Err(invalid("out", "NULL"));
        }
        // SAFETY: the contract.
        let mut o = unsafe { super::problem::options(options) }?;
        // SAFETY: the contract. The problem is built and validated before the environment.
        let s = unsafe {
            with_problem(problem, |p| {
                let inner = create(backend)?;
                inner.backend().solve(p, &mut o)
            })
        }?;
        // SAFETY: `out` non-null and writable.
        unsafe { emit(out, s) };
        Ok(())
    })
}

/// Borrows a caller output array mutably (NULL data with length 0 is an empty array).
///
/// # Safety
/// A non-NULL `a.data` must point to `a.len` writable doubles not aliasing any other array.
unsafe fn out_array<'b>(a: optsolver_array, what: &'static str) -> Result<Option<&'b mut [f64]>> {
    if a.data.is_null() {
        return if a.len == 0 {
            Ok(None)
        } else {
            Err(invalid(what, format!("NULL with length {}", a.len)))
        };
    }
    if a.len > (isize::MAX as usize) / size_of::<f64>() {
        return Err(Error::TooLarge { what, value: a.len });
    }
    if !a.data.is_aligned() {
        return Err(invalid(what, "misaligned pointer"));
    }
    // SAFETY: non-null, aligned, within isize::MAX, writable and unaliased by the contract.
    let s = unsafe { core::slice::from_raw_parts_mut(a.data, a.len) };
    s.fill(f64::NAN);
    Ok(Some(s))
}

/// Solves with `env`, the backend writing x (and y, z, row activity, s, z_l, z_u where
/// supplied) into the caller's arrays. Every supplied array is first filled with NaN, so an
/// entry the backend does not write is NaN, also on an error return. A wrong length is
/// `OPTSOLVER_ERR_INVALID` naming the array, before any solver call. The result holds the scalar
/// and descriptive fields; its arrays for buffered quantities are empty.
///
/// # Safety
/// As [`optsolver_solve`]; `buffers` must be non-NULL and its arrays writable, of the stated
/// lengths, and not overlapping each other or any input array (overlap is undefined behaviour).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_solve_into(
    env: *mut optsolver_env,
    problem: *const optsolver_problem,
    options: *const optsolver_options,
    buffers: *const optsolver_buffers,
    out: *mut *mut optsolver_result,
) -> i32 {
    guard(|| {
        if out.is_null() {
            return Err(invalid("out", "NULL"));
        }
        if buffers.is_null() {
            return Err(invalid("buffers", "NULL"));
        }
        // SAFETY: non-null; begins with struct_size.
        let size = unsafe { buffers.cast::<u32>().read() } as usize;
        if size < size_of::<optsolver_buffers>() {
            return Err(invalid(
                "buffers.struct_size",
                format!(
                    "struct_size {size} is below the minimum {}",
                    size_of::<optsolver_buffers>()
                ),
            ));
        }
        // SAFETY: complete structure by the size check and the contract.
        let b = unsafe { &*buffers };
        // SAFETY: the contract.
        let x = unsafe { out_array(b.x, "out.x") }?.unwrap_or(&mut []);
        // SAFETY: the contract.
        let y = unsafe { out_array(b.y, "out.y") }?;
        // SAFETY: the contract.
        let z = unsafe { out_array(b.z, "out.z") }?;
        // SAFETY: the contract.
        let row_activity = unsafe { out_array(b.row_activity, "out.row_activity") }?;
        // SAFETY: the contract.
        let s = unsafe { out_array(b.s, "out.s") }?;
        // SAFETY: the contract.
        let z_l = unsafe { out_array(b.z_l, "out.z_l") }?;
        // SAFETY: the contract.
        let z_u = unsafe { out_array(b.z_u, "out.z_u") }?;
        let env = env_ref(env)?;
        // SAFETY: the contract.
        let mut o = unsafe { super::problem::options(options) }?;
        let bufs = SolutionBuffers {
            x,
            y,
            z,
            row_activity,
            s,
            z_l,
            z_u,
        };
        // SAFETY: the contract.
        let sol =
            unsafe { with_problem(problem, |p| env.inner.backend().solve_into(p, &mut o, bufs)) }?;
        // SAFETY: `out` non-null and writable.
        unsafe { emit(out, sol) };
        Ok(())
    })
}

/// Frees a result. NULL is a no-op.
///
/// # Safety
/// `result` must be NULL or a result from the library, not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_free(result: *mut optsolver_result) {
    contain((), || {
        if !result.is_null() {
            // SAFETY: from Box::into_raw, freed once, by the contract.
            drop(unsafe { Box::from_raw(result) });
        }
    })
}

/// Runs `f` on a result (NULL → `default`).
fn with<T>(r: *const optsolver_result, default: T, f: impl FnOnce(&optsolver_result) -> T) -> T {
    if r.is_null() {
        return default;
    }
    // SAFETY: a live result by the accessor's contract.
    f(unsafe { &*r })
}

/// Status code (`OPTSOLVER_STATUS_*`; 0 for NULL).
///
/// # Safety
/// `result` must be NULL or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_status(result: *const optsolver_result) -> i32 {
    contain(0, || with(result, 0, |r| status_code(r.solution.status)))
}

/// Limit reason (`OPTSOLVER_LIMIT_*`).
///
/// # Safety
/// `result` must be NULL or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_limit_reason(result: *const optsolver_result) -> i32 {
    contain(0, || {
        with(result, 0, |r| limit_code(r.solution.limit_reason))
    })
}

/// What solution is returned (`OPTSOLVER_RETURNED_*`).
///
/// # Safety
/// `result` must be NULL or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_returned(result: *const optsolver_result) -> i32 {
    contain(0, || {
        with(result, 0, |r| returned_code(r.solution.returned))
    })
}

fn slice_out(v: &[f64], len: *mut usize) -> *const f64 {
    if !len.is_null() {
        // SAFETY: a writable out-pointer by the accessor's contract.
        unsafe { len.write(v.len()) };
    }
    if v.is_empty() {
        core::ptr::null()
    } else {
        v.as_ptr()
    }
}

/// A result array (`OPTSOLVER_ARRAY_*`) read in place; NULL and length 0 when absent.
///
/// # Safety
/// `result` must be NULL or a live result; `len` NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_array(
    result: *const optsolver_result,
    which: i32,
    len: *mut usize,
) -> *const f64 {
    contain(core::ptr::null(), || {
        with(result, slice_out(&[], len), |r| {
            let s = &r.solution;
            let v: &[f64] = match which {
                OPTSOLVER_ARRAY_X => &s.x,
                OPTSOLVER_ARRAY_Y => &s.y,
                OPTSOLVER_ARRAY_Z => &s.z,
                OPTSOLVER_ARRAY_Z_L => &s.z_l,
                OPTSOLVER_ARRAY_Z_U => &s.z_u,
                OPTSOLVER_ARRAY_S => &s.s,
                OPTSOLVER_ARRAY_ROW_ACTIVITY => &s.row_activity,
                _ => &[],
            };
            slice_out(v, len)
        })
    })
}

fn opt_out<T: Copy>(v: Option<T>, out: *mut T) -> i32 {
    match v {
        Some(x) => {
            if !out.is_null() {
                // SAFETY: writable out-pointer by the accessor's contract.
                unsafe { out.write(x) };
            }
            1
        }
        None => 0,
    }
}

/// Objective value; returns 1 and writes it when present, else 0.
///
/// # Safety
/// `result` must be NULL or a live result; `value` NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_objective(
    result: *const optsolver_result,
    value: *mut f64,
) -> i32 {
    contain(0, || {
        with(result, 0, |r| opt_out(r.solution.objective, value))
    })
}

/// Best bound; returns 1 and writes it when present, else 0.
///
/// # Safety
/// `result` must be NULL or a live result; `value` NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_bound(
    result: *const optsolver_result,
    value: *mut f64,
) -> i32 {
    contain(0, || with(result, 0, |r| opt_out(r.solution.bound, value)))
}

/// Simplex iterations; 1 when present.
///
/// # Safety
/// `result` must be NULL or a live result; `value` NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_simplex_iterations(
    result: *const optsolver_result,
    value: *mut f64,
) -> i32 {
    contain(0, || {
        with(result, 0, |r| {
            opt_out(r.solution.counts.simplex_iterations, value)
        })
    })
}

/// Barrier iterations; 1 when present.
///
/// # Safety
/// `result` must be NULL or a live result; `value` NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_barrier_iterations(
    result: *const optsolver_result,
    value: *mut i64,
) -> i32 {
    contain(0, || {
        with(result, 0, |r| {
            opt_out(r.solution.counts.barrier_iterations, value)
        })
    })
}

/// Phase timings (zeros for NULL).
///
/// # Safety
/// `result` must be NULL or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_timings(
    result: *const optsolver_result,
) -> optsolver_timings {
    contain(optsolver_timings::default(), || {
        with(result, optsolver_timings::default(), |r| {
            let t = r.solution.timings;
            optsolver_timings {
                load: t.load,
                solve: t.solve,
                extraction: t.extraction,
                verification: t.verification,
            }
        })
    })
}

/// 1 if a basis is returned.
///
/// # Safety
/// `result` must be NULL or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_has_basis(result: *const optsolver_result) -> i32 {
    contain(0, || {
        with(result, 0, |r| i32::from(r.solution.basis.is_some()))
    })
}

/// Basis status of variable `j` (`OPTSOLVER_BASIS_*`), −1 when absent or out of range.
///
/// # Safety
/// `result` must be NULL or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_basis_variable(
    result: *const optsolver_result,
    j: usize,
) -> i32 {
    contain(-1, || {
        with(result, -1, |r| {
            r.solution
                .basis
                .as_ref()
                .and_then(|b| b.variables.get(j))
                .map_or(-1, |&s| basis_code(s))
        })
    })
}

/// Basis status of row `i`, −1 when absent or out of range.
///
/// # Safety
/// `result` must be NULL or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_basis_row(
    result: *const optsolver_result,
    i: usize,
) -> i32 {
    contain(-1, || {
        with(result, -1, |r| {
            r.solution
                .basis
                .as_ref()
                .and_then(|b| b.rows.get(i))
                .map_or(-1, |&s| basis_code(s))
        })
    })
}

/// Certificate kind (`OPTSOLVER_CERTIFICATE_*`).
///
/// # Safety
/// `result` must be NULL or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_certificate(result: *const optsolver_result) -> i32 {
    contain(0, || {
        with(result, 0, |r| match &r.solution.certificate {
            None => OPTSOLVER_CERTIFICATE_NONE,
            Some(Certificate::Farkas { .. }) => OPTSOLVER_CERTIFICATE_FARKAS,
            Some(Certificate::UnboundedRay { .. }) => OPTSOLVER_CERTIFICATE_UNBOUNDED_RAY,
        })
    })
}

/// A certificate array (`OPTSOLVER_CERT_*`) read in place; NULL and length 0 when absent.
///
/// # Safety
/// `result` must be NULL or a live result; `len` NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_certificate_array(
    result: *const optsolver_result,
    which: i32,
    len: *mut usize,
) -> *const f64 {
    contain(core::ptr::null(), || {
        with(result, slice_out(&[], len), |r| {
            let v: &[f64] = match (&r.solution.certificate, which) {
                (Some(Certificate::Farkas { y, .. }), OPTSOLVER_CERT_Y) => y,
                (Some(Certificate::Farkas { z, .. }), OPTSOLVER_CERT_Z) => z,
                (Some(Certificate::Farkas { s, .. }), OPTSOLVER_CERT_S) => s,
                (Some(Certificate::UnboundedRay { r }), OPTSOLVER_CERT_RAY) => r,
                _ => &[],
            };
            slice_out(v, len)
        })
    })
}

/// Number of raw status codes.
///
/// # Safety
/// `result` must be NULL or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_raw_status_count(
    result: *const optsolver_result,
) -> usize {
    contain(0, || with(result, 0, |r| r.solution.raw_status.len()))
}

/// Raw status `i`: returns its name and writes its code (NULL when out of range).
///
/// # Safety
/// `result` must be NULL or a live result; `code` NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_raw_status(
    result: *const optsolver_result,
    i: usize,
    code: *mut i64,
) -> *const c_char {
    contain(core::ptr::null(), || {
        with(result, core::ptr::null(), |r| {
            match r.solution.raw_status.get(i) {
                None => core::ptr::null(),
                Some((_, c)) => {
                    opt_out(Some(*c), code);
                    r.names().raw_status[i].as_ptr()
                }
            }
        })
    })
}

/// Solver name.
///
/// # Safety
/// `result` must be NULL or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_solver_name(
    result: *const optsolver_result,
) -> *const c_char {
    contain(c"".as_ptr(), || {
        with(result, c"".as_ptr(), |r| r.names().solver_name.as_ptr())
    })
}

/// Solver version.
///
/// # Safety
/// `result` must be NULL or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_solver_version(
    result: *const optsolver_result,
) -> *const c_char {
    contain(c"".as_ptr(), || {
        with(result, c"".as_ptr(), |r| r.names().solver_version.as_ptr())
    })
}

fn value_out(v: &ParamValue, s: Option<&CString>, out: *mut optsolver_value) {
    if out.is_null() {
        return;
    }
    let value = optsolver_value {
        r#type: param_type(v),
        int_value: match v {
            ParamValue::Int(i) => *i,
            _ => 0,
        },
        double_value: match v {
            ParamValue::Double(d) => *d,
            _ => 0.0,
        },
        string_value: s.map_or(core::ptr::null(), |c| c.as_ptr()),
    };
    // SAFETY: writable out-pointer by the accessor's contract.
    unsafe { out.write(value) };
}

/// Number of extra solver attributes.
///
/// # Safety
/// `result` must be NULL or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_attribute_count(
    result: *const optsolver_result,
) -> usize {
    contain(0, || with(result, 0, |r| r.solution.attributes.len()))
}

/// Attribute `i`: returns its name and writes its value (NULL when out of range).
///
/// # Safety
/// `result` must be NULL or a live result; `value` NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_attribute(
    result: *const optsolver_result,
    i: usize,
    value: *mut optsolver_value,
) -> *const c_char {
    contain(core::ptr::null(), || {
        with(result, core::ptr::null(), |r| {
            match r.solution.attributes.get(i) {
                None => core::ptr::null(),
                Some((_, v)) => {
                    let (n, s) = &r.names().attributes[i];
                    value_out(v, s.as_ref(), value);
                    n.as_ptr()
                }
            }
        })
    })
}

/// Number of effective parameters.
///
/// # Safety
/// `result` must be NULL or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_parameter_count(
    result: *const optsolver_result,
) -> usize {
    contain(0, || {
        with(result, 0, |r| r.solution.effective_parameters.len())
    })
}

/// Effective parameter `i`: returns its name and writes its value (NULL when out of range).
///
/// # Safety
/// `result` must be NULL or a live result; `value` NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_parameter(
    result: *const optsolver_result,
    i: usize,
    value: *mut optsolver_value,
) -> *const c_char {
    contain(core::ptr::null(), || {
        with(result, core::ptr::null(), |r| {
            match r.solution.effective_parameters.get(i) {
                None => core::ptr::null(),
                Some((_, v)) => {
                    let (n, s) = &r.names().parameters[i];
                    value_out(v, s.as_ref(), value);
                    n.as_ptr()
                }
            }
        })
    })
}

/// Overall verification verdict (`OPTSOLVER_VERIFICATION_*`).
///
/// # Safety
/// `result` must be NULL or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_verification(result: *const optsolver_result) -> i32 {
    contain(0, || {
        with(result, 0, |r| {
            overall_code(r.solution.verification.as_ref().map(|v| v.overall))
        })
    })
}

/// Number of verification measures.
///
/// # Safety
/// `result` must be NULL or a live result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_measure_count(result: *const optsolver_result) -> usize {
    contain(0, || {
        with(result, 0, |r| {
            r.solution
                .verification
                .as_ref()
                .map_or(0, |v| v.measures.len())
        })
    })
}

/// Measure `i`: writes it and returns `OPTSOLVER_OK`, or `OPTSOLVER_ERR_INVALID` when out of
/// range or NULL.
///
/// # Safety
/// `result` must be NULL or a live result; `out` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_result_measure(
    result: *const optsolver_result,
    i: usize,
    out: *mut optsolver_measure,
) -> i32 {
    guard(|| {
        if result.is_null() || out.is_null() {
            return Err(invalid(
                if result.is_null() { "result" } else { "out" },
                "NULL",
            ));
        }
        // SAFETY: a live result by the contract.
        let r = unsafe { &*result };
        let m = r
            .solution
            .verification
            .as_ref()
            .and_then(|v| v.measures.get(i))
            .ok_or_else(|| invalid("i", format!("no measure {i}")))?;
        let value = optsolver_measure {
            name: static_name(m.name),
            absolute: m.absolute,
            relative: m.relative,
            tolerance: m.tolerance,
            verdict: verdict_code(m.verdict),
            required: i32::from(m.required),
        };
        // SAFETY: writable by the contract.
        unsafe { out.write(value) };
        Ok(())
    })
}

// ---------------------------------------------------------------------------------------------
// A safe Rust helper reaching the C path in-process, for the benchmark (feature 005 task T017):
// no `unsafe` is needed outside this module.

/// An owned C-interface environment handle, freed on drop.
pub struct CEnv(*mut optsolver_env);

impl CEnv {
    /// Creates a handle through `optsolver_env_create`.
    pub fn new(backend: i32) -> std::result::Result<CEnv, String> {
        let mut out = core::ptr::null_mut();
        // SAFETY: `out` is a writable local.
        let rc = unsafe { optsolver_env_create(backend, &mut out) };
        if rc != OPTSOLVER_OK {
            return Err(last_message());
        }
        Ok(CEnv(out))
    }
}

impl Drop for CEnv {
    fn drop(&mut self) {
        // SAFETY: the handle came from optsolver_env_create and is freed once.
        unsafe { optsolver_env_free(self.0) };
    }
}

fn last_message() -> String {
    let m = optsolver_last_error_message();
    if m.is_null() {
        return String::new();
    }
    // SAFETY: a NUL-terminated string from the library, freed once below.
    let s = unsafe { std::ffi::CStr::from_ptr(m) }
        .to_string_lossy()
        .into_owned();
    // SAFETY: from optsolver_last_error_message.
    unsafe { optsolver_string_free(m) };
    s
}

/// A borrowed LP in ranges form, for [`solve_lp_through_c`].
#[derive(Debug, Clone, Copy)]
#[allow(non_snake_case)] // Notation Convention (A).
pub struct CPathLp<'a> {
    /// m.
    pub m: usize,
    /// n.
    pub n: usize,
    /// A's column pointers.
    pub A_col_ptrs: &'a [usize],
    /// A's row indices.
    pub A_row_indices: &'a [usize],
    /// A's values.
    pub A_values: &'a [f64],
    /// c.
    pub c: &'a [f64],
    /// l.
    pub l: &'a [f64],
    /// u.
    pub u: &'a [f64],
    /// b_l.
    pub b_l: &'a [f64],
    /// b_u.
    pub b_u: &'a [f64],
    /// Objective sense.
    pub sense: crate::problem::Sense,
}

/// Reads every accessor of a live result (names, raw statuses, parameters, attributes,
/// measures, basis, certificate arrays, iteration counts), discarding the values.
///
/// # Safety
/// `out` must be a live result from the library.
unsafe fn read_every_accessor(out: *const optsolver_result, m: usize, n: usize) {
    let mut code = 0i64;
    let mut v = optsolver_value {
        r#type: 0,
        int_value: 0,
        double_value: 0.0,
        string_value: core::ptr::null(),
    };
    let mut ms = optsolver_measure {
        name: core::ptr::null(),
        absolute: 0.0,
        relative: 0.0,
        tolerance: 0.0,
        verdict: 0,
        required: 0,
    };
    let (mut d, mut it, mut len) = (0.0f64, 0i64, 0usize);
    // Each accessor call: `out` is a live result by this function's contract, and every
    // out-pointer refers to a writable local.
    macro_rules! acc {
        ($e:expr) => {{
            // SAFETY: see the comment above the macro.
            unsafe { $e }
        }};
    }
    std::hint::black_box(acc!(optsolver_result_solver_name(out)));
    std::hint::black_box(acc!(optsolver_result_solver_version(out)));
    for i in 0..acc!(optsolver_result_raw_status_count(out)) {
        std::hint::black_box(acc!(optsolver_result_raw_status(out, i, &mut code)));
    }
    for i in 0..acc!(optsolver_result_parameter_count(out)) {
        std::hint::black_box(acc!(optsolver_result_parameter(out, i, &mut v)));
    }
    for i in 0..acc!(optsolver_result_attribute_count(out)) {
        std::hint::black_box(acc!(optsolver_result_attribute(out, i, &mut v)));
    }
    for i in 0..acc!(optsolver_result_measure_count(out)) {
        std::hint::black_box(acc!(optsolver_result_measure(out, i, &mut ms)));
    }
    if acc!(optsolver_result_has_basis(out)) == 1 {
        for j in 0..n {
            std::hint::black_box(acc!(optsolver_result_basis_variable(out, j)));
        }
        for i in 0..m {
            std::hint::black_box(acc!(optsolver_result_basis_row(out, i)));
        }
    }
    for w in 0..4 {
        std::hint::black_box(acc!(optsolver_result_certificate_array(out, w, &mut len)));
    }
    std::hint::black_box(acc!(optsolver_result_bound(out, &mut d)));
    std::hint::black_box(acc!(optsolver_result_simplex_iterations(out, &mut d)));
    std::hint::black_box(acc!(optsolver_result_barrier_iterations(out, &mut it)));
    std::hint::black_box(acc!(optsolver_result_verification(out)));
    std::hint::black_box(acc!(optsolver_result_limit_reason(out)));
    std::hint::black_box(acc!(optsolver_result_returned(out)));
    std::hint::black_box(acc!(optsolver_result_certificate(out)));
}

/// Caller buffers for [`solve_lp_through_c`]: x, y, z and row activity.
pub type CPathBuffers<'a> = (&'a mut [f64], &'a mut [f64], &'a mut [f64], &'a mut [f64]);

/// What the C path returned, read through the accessors.
#[derive(Debug, Clone, PartialEq)]
pub struct CPathOutcome {
    /// Status code.
    pub status: i32,
    /// Objective, if present.
    pub objective: Option<f64>,
    /// Phase timings.
    pub timings: optsolver_timings,
    /// Sum of the lengths of every array read.
    pub array_lengths: usize,
}

/// Solves an LP through the C entry points with `threads` and `method` codes
/// (`OPTSOLVER_UNSET` for unset): the owned path reading every array, or (`buffers` set) the
/// caller-buffer path writing x, y, z and row activity into the given vectors.
pub fn solve_lp_through_c(
    env: &CEnv,
    lp: &CPathLp<'_>,
    threads: i64,
    method: i32,
    buffers: Option<CPathBuffers<'_>>,
) -> std::result::Result<CPathOutcome, String> {
    // Every slice must have the length the C structures will declare for it, so that the C
    // layer's reads stay inside the slices (the SAFETY comments below rely on this check).
    let nnz = lp.A_col_ptrs.last().copied();
    let lengths_ok = lp.A_col_ptrs.len() == lp.n + 1
        && nnz == Some(lp.A_row_indices.len())
        && lp.A_values.len() == lp.A_row_indices.len()
        && [lp.c.len(), lp.l.len(), lp.u.len()]
            .iter()
            .all(|&k| k == lp.n)
        && lp.b_l.len() == lp.m
        && lp.b_u.len() == lp.m
        && buffers.as_ref().is_none_or(|(x, y, z, a)| {
            x.len() == lp.n && z.len() == lp.n && y.len() == lp.m && a.len() == lp.m
        });
    if !lengths_ok {
        return Err("solve_lp_through_c: slice lengths do not match m and n".into());
    }
    let problem = optsolver_problem {
        struct_size: u32::try_from(size_of::<optsolver_problem>()).unwrap_or(u32::MAX),
        A: optsolver_csc {
            rows: lp.m,
            cols: lp.n,
            col_ptrs: lp.A_col_ptrs.as_ptr(),
            row_indices: lp.A_row_indices.as_ptr(),
            values: lp.A_values.as_ptr(),
        },
        A_row_indices_i32: core::ptr::null(),
        c: lp.c.as_ptr(),
        sense: match lp.sense {
            crate::problem::Sense::Minimise => OPTSOLVER_SENSE_MINIMISE,
            crate::problem::Sense::Maximise => OPTSOLVER_SENSE_MAXIMISE,
        },
        gamma: 0.0,
        l: lp.l.as_ptr(),
        u: lp.u.as_ptr(),
        row_sense: core::ptr::null(),
        b: core::ptr::null(),
        b_l: lp.b_l.as_ptr(),
        b_u: lp.b_u.as_ptr(),
        Q: core::ptr::null(),
        F: core::ptr::null(),
        g: core::ptr::null(),
        cones: core::ptr::null(),
        num_cones: 0,
    };
    let mut o = core::mem::MaybeUninit::<optsolver_options>::uninit();
    // SAFETY: writable.
    unsafe { optsolver_options_init(o.as_mut_ptr()) };
    // SAFETY: initialised above.
    let mut o = unsafe { o.assume_init() };
    o.threads = threads;
    o.method = method;
    let mut out = core::ptr::null_mut();
    let rc = match buffers {
        None => {
            // SAFETY: every pointer refers to live Rust data whose length matches the structure
            // (checked above) for the call.
            unsafe { optsolver_solve(env.0, &problem, &o, &mut out) }
        }
        Some((x, y, z, act)) => {
            let arr = |v: &mut [f64]| optsolver_array {
                data: v.as_mut_ptr(),
                len: v.len(),
            };
            let none = optsolver_array {
                data: core::ptr::null_mut(),
                len: 0,
            };
            let b = optsolver_buffers {
                struct_size: u32::try_from(size_of::<optsolver_buffers>()).unwrap_or(u32::MAX),
                x: arr(x),
                y: arr(y),
                z: arr(z),
                row_activity: arr(act),
                s: none,
                z_l: none,
                z_u: none,
            };
            // SAFETY: every pointer refers to live, distinct Rust data whose length matches the
            // structure (checked above) for the call.
            unsafe { optsolver_solve_into(env.0, &problem, &o, &b, &mut out) }
        }
    };
    if rc != OPTSOLVER_OK {
        return Err(last_message());
    }
    let mut objective = 0.0;
    let mut lengths = 0;
    for which in 0..=6 {
        let mut len = 0;
        // SAFETY: `out` is a live result until freed below; `len` is writable.
        unsafe { optsolver_result_array(out, which, &mut len) };
        lengths += len;
    }
    // SAFETY: `out` is a live result.
    let status = unsafe { optsolver_result_status(out) };
    // SAFETY: `out` is a live result; `objective` is writable.
    let has_objective = unsafe { optsolver_result_objective(out, &mut objective) } == 1;
    // SAFETY: `out` is a live result.
    let timings = unsafe { optsolver_result_timings(out) };
    // Every other accessor, so the benchmark measures the C layer's lazy name conversions too.
    // SAFETY: `out` is a live result until freed below.
    unsafe { read_every_accessor(out, lp.m, lp.n) };
    let outcome = CPathOutcome {
        status,
        objective: has_objective.then_some(objective),
        timings,
        array_lengths: lengths,
    };
    // SAFETY: freed once.
    unsafe { optsolver_result_free(out) };
    Ok(outcome)
}
