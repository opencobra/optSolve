//! MOSEK parameters (task T023; feature 002 research R9; constitution III, VII-G).
//!
//! This is the only code in the crate that calls MOSEK's parameter setters (enforced by
//! `tests/standards.rs`). Every default optSolver imposes comes from
//! [`crate::options::defaults`]; this module only maps options to MOSEK parameters.

use core::ffi::{CStr, c_char, c_void};
use std::ffi::CString;
use std::panic::{AssertUnwindSafe, catch_unwind};

use super::env::{Task, task_error};
use super::ffi;
use crate::backend::{OptionMapping, ProblemClass};
use crate::error::{Error, Result};
use crate::options::{Method, Options, ParamValue, Presolve};
use crate::verify::Tolerances;

/// MOSEK mapping of the shared options fields it honours (research R9). `random_seed` is
/// absent (unsupported for continuous problems), so setting it is an `Unsupported` error.
/// `certificates` sets nothing: MOSEK always returns them. `cancel` and `progress` are served
/// by the callback.
pub(crate) const MAPPINGS: &[OptionMapping] = &[
    OptionMapping {
        field: "time_limit",
        vendor_parameters: &["MSK_DPAR_OPTIMIZER_MAX_TIME"],
    },
    OptionMapping {
        field: "iteration_limit",
        vendor_parameters: &["MSK_IPAR_SIM_MAX_ITERATIONS"],
    },
    OptionMapping {
        field: "threads",
        vendor_parameters: &["MSK_IPAR_NUM_THREADS"],
    },
    OptionMapping {
        field: "primal_feasibility_tolerance",
        vendor_parameters: &[
            "MSK_DPAR_INTPNT_TOL_PFEAS",
            "MSK_DPAR_INTPNT_CO_TOL_PFEAS",
            "MSK_DPAR_INTPNT_QO_TOL_PFEAS",
        ],
    },
    OptionMapping {
        field: "dual_feasibility_tolerance",
        vendor_parameters: &[
            "MSK_DPAR_INTPNT_TOL_DFEAS",
            "MSK_DPAR_INTPNT_CO_TOL_DFEAS",
            "MSK_DPAR_INTPNT_QO_TOL_DFEAS",
        ],
    },
    OptionMapping {
        field: "method",
        vendor_parameters: &["MSK_IPAR_OPTIMIZER"],
    },
    OptionMapping {
        field: "presolve",
        vendor_parameters: &["MSK_IPAR_PRESOLVE_USE"],
    },
    OptionMapping {
        field: "solver_log",
        vendor_parameters: &["MSK_IPAR_LOG"],
    },
    OptionMapping {
        field: "certificates",
        vendor_parameters: &[],
    },
    OptionMapping {
        field: "cancel",
        vendor_parameters: &[],
    },
    OptionMapping {
        field: "progress",
        vendor_parameters: &[],
    },
];

/// Records parameters in the order they are set, under MOSEK's names.
struct Recorder<'t, 'e> {
    task: &'t Task<'e>,
    set: Vec<(String, ParamValue)>,
}

impl Recorder<'_, '_> {
    fn int(&mut self, name: &'static str, param: i32, value: i32) -> Result<()> {
        // SAFETY: the task is live (borrowed); `param` is an MSKiparame value from ffi.rs.
        let rc = unsafe { ffi::MSK_putintparam(self.task.raw(), param, value) };
        self.task.check(rc).map_err(|e| named(e, name))?;
        self.set
            .push((name.to_string(), ParamValue::Int(i64::from(value))));
        Ok(())
    }
    fn dbl(&mut self, name: &'static str, param: i32, value: f64) -> Result<()> {
        // SAFETY: as in `int`, for an MSKdparame value.
        let rc = unsafe { ffi::MSK_putdouparam(self.task.raw(), param, value) };
        self.task.check(rc).map_err(|e| named(e, name))?;
        self.set.push((name.to_string(), ParamValue::Double(value)));
        Ok(())
    }
}

fn named(e: Error, name: &str) -> Error {
    match e {
        Error::Solver { code, message } => Error::Solver {
            code,
            message: format!("parameter {name}: {message}"),
        },
        other => other,
    }
}

fn unsupported(field: &'static str) -> Error {
    Error::Unsupported {
        field,
        backend: "mosek",
    }
}

fn invalid_raw(detail: String) -> Error {
    Error::Invalid {
        what: "options.raw",
        detail,
    }
}

/// Forwards MOSEK's log stream to the `log` facade (constitution III). Never unwinds.
unsafe extern "C" fn log_stream(_handle: *mut c_void, text: *const c_char) {
    if text.is_null() {
        return;
    }
    // SAFETY: MOSEK passes a NUL-terminated string valid for the duration of the call.
    let line = unsafe { CStr::from_ptr(text) }.to_string_lossy();
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let line = line.trim_end();
        if !line.is_empty() {
            log::info!(target: "optsolver::mosek", "{line}");
        }
    }));
}

/// Applies `options` to `task` for a problem of class `class` and returns every parameter
/// set, with its value, under MOSEK's names (research R9).
///
/// Order: `MSK_IPAR_LOG` (0 unless the solver log is requested, in which case the log stream
/// is linked to the `log` facade and MOSEK's default level is kept); then each set shared
/// field; then each raw parameter, type-checked with `MSK_whichparam`. Unsupported values of
/// `method` (Concurrent) and `presolve` (Conservative, Aggressive) are
/// [`Error::Unsupported`].
pub(crate) fn apply(
    task: &Task<'_>,
    options: &Options<'_>,
    class: ProblemClass,
) -> Result<Vec<(String, ParamValue)>> {
    let mut r = Recorder {
        task,
        set: Vec::new(),
    };
    if options.effective_solver_log() {
        // SAFETY: the task is live; `log_stream` has the MSKstreamfunc signature and uses no
        // handle; MOSEK calls it only during calls on this task.
        let rc = unsafe {
            ffi::MSK_linkfunctotaskstream(
                task.raw(),
                ffi::MSK_STREAM_LOG,
                core::ptr::null_mut(),
                Some(log_stream),
            )
        };
        task.check(rc)?;
    } else {
        r.int("MSK_IPAR_LOG", ffi::MSK_IPAR_LOG, 0)?;
    }
    if let Some(v) = options.time_limit {
        r.dbl(
            "MSK_DPAR_OPTIMIZER_MAX_TIME",
            ffi::MSK_DPAR_OPTIMIZER_MAX_TIME,
            v,
        )?;
    }
    if let Some(v) = options.iteration_limit {
        if v.is_nan() || v < 0.0 {
            return Err(Error::Invalid {
                what: "options.iteration_limit",
                detail: format!("{v} is not a non-negative number"),
            });
        }
        // Saturating conversion of a non-negative count (MOSEK takes an int).
        #[allow(clippy::cast_possible_truncation)] // 0 ≤ floor(v) < i32::MAX here.
        let n = if v >= f64::from(i32::MAX) {
            i32::MAX
        } else {
            v.floor() as i32
        };
        r.int(
            "MSK_IPAR_SIM_MAX_ITERATIONS",
            ffi::MSK_IPAR_SIM_MAX_ITERATIONS,
            n,
        )?;
    }
    if let Some(v) = options.threads {
        let v = i32::try_from(v).map_err(|_| Error::Invalid {
            what: "options.threads",
            detail: format!("{v} exceeds the largest value MOSEK accepts ({})", i32::MAX),
        })?;
        r.int("MSK_IPAR_NUM_THREADS", ffi::MSK_IPAR_NUM_THREADS, v)?;
    }
    if let Some(v) = options.primal_feasibility_tolerance {
        r.dbl(
            "MSK_DPAR_INTPNT_TOL_PFEAS",
            ffi::MSK_DPAR_INTPNT_TOL_PFEAS,
            v,
        )?;
        r.dbl(
            "MSK_DPAR_INTPNT_CO_TOL_PFEAS",
            ffi::MSK_DPAR_INTPNT_CO_TOL_PFEAS,
            v,
        )?;
        r.dbl(
            "MSK_DPAR_INTPNT_QO_TOL_PFEAS",
            ffi::MSK_DPAR_INTPNT_QO_TOL_PFEAS,
            v,
        )?;
    }
    if let Some(v) = options.dual_feasibility_tolerance {
        r.dbl(
            "MSK_DPAR_INTPNT_TOL_DFEAS",
            ffi::MSK_DPAR_INTPNT_TOL_DFEAS,
            v,
        )?;
        r.dbl(
            "MSK_DPAR_INTPNT_CO_TOL_DFEAS",
            ffi::MSK_DPAR_INTPNT_CO_TOL_DFEAS,
            v,
        )?;
        r.dbl(
            "MSK_DPAR_INTPNT_QO_TOL_DFEAS",
            ffi::MSK_DPAR_INTPNT_QO_TOL_DFEAS,
            v,
        )?;
    }
    if let Some(m) = options.method {
        let code = match m {
            Method::Auto => ffi::MSK_OPTIMIZER_FREE,
            Method::PrimalSimplex => ffi::MSK_OPTIMIZER_PRIMAL_SIMPLEX,
            Method::DualSimplex => ffi::MSK_OPTIMIZER_DUAL_SIMPLEX,
            Method::Barrier if class == ProblemClass::Conic => ffi::MSK_OPTIMIZER_CONIC,
            Method::Barrier => ffi::MSK_OPTIMIZER_INTPNT,
            Method::Concurrent => return Err(unsupported("method")),
        };
        r.int("MSK_IPAR_OPTIMIZER", ffi::MSK_IPAR_OPTIMIZER, code)?;
    }
    if let Some(p) = options.presolve {
        let code = match p {
            Presolve::Auto => ffi::MSK_PRESOLVE_MODE_FREE,
            Presolve::Off => ffi::MSK_PRESOLVE_MODE_OFF,
            Presolve::Conservative | Presolve::Aggressive => return Err(unsupported("presolve")),
        };
        r.int("MSK_IPAR_PRESOLVE_USE", ffi::MSK_IPAR_PRESOLVE_USE, code)?;
    }
    for (name, value) in &options.raw {
        apply_raw(&mut r, name, value)?;
    }
    Ok(r.set)
}

fn apply_raw(r: &mut Recorder<'_, '_>, name: &str, value: &ParamValue) -> Result<()> {
    let cname = CString::new(name)
        .map_err(|_| invalid_raw(format!("parameter name {name:?} contains a NUL character")))?;
    let task = r.task.raw();
    let (mut kind, mut index) = (0, 0);
    // SAFETY: the task is live; `cname` is NUL-terminated and outlives the call; `kind` and
    // `index` are writable locals.
    let rc = unsafe { ffi::MSK_whichparam(task, cname.as_ptr(), &mut kind, &mut index) };
    if rc != ffi::MSK_RES_OK || kind == ffi::MSK_PAR_INVALID_TYPE {
        return Err(invalid_raw(format!("unknown MOSEK parameter {name}")));
    }
    let wrong_type = |expected: &str| {
        invalid_raw(format!(
            "MOSEK parameter {name} takes {expected}, got {value:?}"
        ))
    };
    let (rc, set_value) = match (kind, value) {
        (ffi::MSK_PAR_INT_TYPE, ParamValue::Int(i)) => {
            let v = i32::try_from(*i).map_err(|_| {
                invalid_raw(format!(
                    "value {i} of MOSEK parameter {name} is outside the int range"
                ))
            })?;
            // SAFETY: live task, NUL-terminated name outliving the call.
            (
                // SAFETY: as stated in the SAFETY note for this block of calls above.
                unsafe { ffi::MSK_putnaintparam(task, cname.as_ptr(), v) },
                ParamValue::Int(*i),
            )
        }
        (ffi::MSK_PAR_INT_TYPE, _) => return Err(wrong_type("an integer")),
        (ffi::MSK_PAR_DOU_TYPE, ParamValue::Double(d)) => (
            // SAFETY: as above.
            unsafe { ffi::MSK_putnadouparam(task, cname.as_ptr(), *d) },
            ParamValue::Double(*d),
        ),
        (ffi::MSK_PAR_DOU_TYPE, ParamValue::Int(i)) => {
            let t = i.unsigned_abs();
            if t != 0 && (t >> t.trailing_zeros()) >= (1_u64 << 53) {
                return Err(invalid_raw(format!(
                    "value {i} of MOSEK parameter {name} is not exactly representable as a double"
                )));
            }
            let d = *i as f64;
            // SAFETY: as above.
            (
                // SAFETY: as stated in the SAFETY note for this block of calls above.
                unsafe { ffi::MSK_putnadouparam(task, cname.as_ptr(), d) },
                ParamValue::Double(d),
            )
        }
        (ffi::MSK_PAR_DOU_TYPE, _) => return Err(wrong_type("a double")),
        (ffi::MSK_PAR_STR_TYPE, ParamValue::Str(s)) => {
            let cs = CString::new(s.as_str()).map_err(|_| {
                invalid_raw(format!(
                    "value of MOSEK parameter {name} contains a NUL character"
                ))
            })?;
            // SAFETY: as above; MOSEK copies the value.
            (
                // SAFETY: as stated in the SAFETY note for this block of calls above.
                unsafe { ffi::MSK_putnastrparam(task, cname.as_ptr(), cs.as_ptr()) },
                ParamValue::Str(s.clone()),
            )
        }
        (ffi::MSK_PAR_STR_TYPE, _) => return Err(wrong_type("a string")),
        (other, _) => {
            return Err(invalid_raw(format!(
                "MOSEK parameter {name} has unrecognised type code {other}"
            )));
        }
    };
    if rc != ffi::MSK_RES_OK {
        return Err(named(task_error(task, rc), name));
    }
    r.set.push((name.to_owned(), set_value));
    Ok(())
}

fn get_dbl(task: &Task<'_>, param: i32) -> Result<f64> {
    let mut v = 0.0;
    // SAFETY: live task, MSKdparame value, writable local.
    let rc = unsafe { ffi::MSK_getdouparam(task.raw(), param, &mut v) };
    task.check(rc)?;
    Ok(v)
}

/// Effective verification tolerances read back from `task` after [`apply`] (research R9):
/// for a basic LP solution `MSK_DPAR_BASIS_TOL_X`/`_S`; otherwise the interior-point
/// `PFEAS`/`DFEAS` of the class solved (LP, QP or conic). Integrality is not used by MOSEK's
/// continuous classes and is reported as 1e-5.
pub(crate) fn effective_tolerances(
    task: &Task<'_>,
    class: ProblemClass,
    basic: bool,
) -> Result<Tolerances> {
    let (p, d) = match (class, basic) {
        (_, true) => (ffi::MSK_DPAR_BASIS_TOL_X, ffi::MSK_DPAR_BASIS_TOL_S),
        (ProblemClass::Conic, false) => (
            ffi::MSK_DPAR_INTPNT_CO_TOL_PFEAS,
            ffi::MSK_DPAR_INTPNT_CO_TOL_DFEAS,
        ),
        (ProblemClass::Qp, false) => (
            ffi::MSK_DPAR_INTPNT_QO_TOL_PFEAS,
            ffi::MSK_DPAR_INTPNT_QO_TOL_DFEAS,
        ),
        _ => (
            ffi::MSK_DPAR_INTPNT_TOL_PFEAS,
            ffi::MSK_DPAR_INTPNT_TOL_DFEAS,
        ),
    };
    Ok(Tolerances {
        primal: get_dbl(task, p)?,
        dual: get_dbl(task, d)?,
        integrality: 1e-5,
    })
}
