//! The C interface (feature 005, ADR-0004): C entry points over the solver-independent API.
//!
//! This module only translates. Validation, status translation, parameter application and
//! verification stay in the core and the backends. The stable interface is the header
//! `src/capi/optsolver.h`; the Rust path `optsolver::capi` is hidden and exists for tests and
//! benchmarks (feature 005 research R6).
//!
//! Every exported function runs its body inside [`guard`] or [`contain`], which catch unwinding
//! (constitution VII-C) and record a contained panic's message as the calling thread's error;
//! a panic hook installed once makes such panics silent (research R4, constitution III).

#![allow(unsafe_code)] // FFI module: every unsafe block carries a SAFETY comment (VII-A).

use core::ffi::c_char;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, Once, OnceLock};

use crate::backend::ProblemClass;
use crate::conic::ConeKind;
use crate::error::Error;
use crate::options::{Method, ParamValue, Presolve};
use crate::result::{BasisStatus, LimitReason, SolutionReturned, Status};
use crate::verify::{Overall, Verdict};

mod env;
mod problem;
mod result;

pub use env::*;
pub use problem::*;
pub use result::*;

// ---------------------------------------------------------------------------------------------
// Constants (one per header constant; the agreement test compares them with the header).

/// Interface version, major part.
pub const OPTSOLVER_ABI_VERSION_MAJOR: u32 = 1;
/// Interface version, minor part.
pub const OPTSOLVER_ABI_VERSION_MINOR: u32 = 0;

/// Success.
pub const OPTSOLVER_OK: i32 = 0;
/// Invalid input.
pub const OPTSOLVER_ERR_INVALID: i32 = 1;
/// A dimension above what the solvers accept, or an array too large to address.
pub const OPTSOLVER_ERR_TOO_LARGE: i32 = 2;
/// Unsupported field, problem class, cone kind or backend.
pub const OPTSOLVER_ERR_UNSUPPORTED: i32 = 3;
/// The same vendor parameter set by two routes.
pub const OPTSOLVER_ERR_PARAMETER_CONFLICT: i32 = 4;
/// The solver reported an error.
pub const OPTSOLVER_ERR_SOLVER: i32 = 5;
/// A contained internal panic.
pub const OPTSOLVER_ERR_INTERNAL: i32 = 6;

/// Gurobi backend.
pub const OPTSOLVER_BACKEND_GUROBI: i32 = 1;
/// MOSEK backend.
pub const OPTSOLVER_BACKEND_MOSEK: i32 = 2;

/// Minimise.
pub const OPTSOLVER_SENSE_MINIMISE: i32 = 1;
/// Maximise.
pub const OPTSOLVER_SENSE_MAXIMISE: i32 = -1;

/// An integer option left unset.
pub const OPTSOLVER_UNSET: i32 = -1;

/// Statuses.
pub const OPTSOLVER_STATUS_OPTIMAL: i32 = 1;
/// Infeasibility proven.
pub const OPTSOLVER_STATUS_INFEASIBLE: i32 = 2;
/// Unboundedness proven.
pub const OPTSOLVER_STATUS_UNBOUNDED: i32 = 3;
/// Infeasible or unbounded, undetermined.
pub const OPTSOLVER_STATUS_INFEASIBLE_OR_UNBOUNDED: i32 = 4;
/// Stopped at a limit.
pub const OPTSOLVER_STATUS_STOPPED_AT_LIMIT: i32 = 5;
/// Numerical difficulty.
pub const OPTSOLVER_STATUS_NUMERICAL_DIFFICULTY: i32 = 6;
/// Interrupted.
pub const OPTSOLVER_STATUS_INTERRUPTED: i32 = 7;
/// Locally optimal.
pub const OPTSOLVER_STATUS_LOCALLY_OPTIMAL: i32 = 8;
/// Locally infeasible.
pub const OPTSOLVER_STATUS_LOCALLY_INFEASIBLE: i32 = 9;
/// Not solved.
pub const OPTSOLVER_STATUS_NOT_SOLVED: i32 = 10;
/// Unknown solver code.
pub const OPTSOLVER_STATUS_UNKNOWN: i32 = 11;

/// No limit.
pub const OPTSOLVER_LIMIT_NONE: i32 = 0;
/// Time limit.
pub const OPTSOLVER_LIMIT_TIME: i32 = 1;
/// Iteration limit.
pub const OPTSOLVER_LIMIT_ITERATIONS: i32 = 2;
/// Node limit.
pub const OPTSOLVER_LIMIT_NODES: i32 = 3;
/// Solution-count limit.
pub const OPTSOLVER_LIMIT_SOLUTIONS: i32 = 4;
/// Objective cutoff.
pub const OPTSOLVER_LIMIT_OBJECTIVE_CUTOFF: i32 = 5;
/// Objective target.
pub const OPTSOLVER_LIMIT_OBJECTIVE_TARGET: i32 = 6;
/// Work limit.
pub const OPTSOLVER_LIMIT_WORK: i32 = 7;
/// Memory limit.
pub const OPTSOLVER_LIMIT_MEMORY: i32 = 8;

/// No solution returned.
pub const OPTSOLVER_RETURNED_NONE: i32 = 0;
/// A feasible solution returned.
pub const OPTSOLVER_RETURNED_FEASIBLE: i32 = 1;
/// An optimal solution returned.
pub const OPTSOLVER_RETURNED_OPTIMAL: i32 = 2;

/// Cone kinds (the COBRA `coneType` codes).
pub const OPTSOLVER_CONE_QUADRATIC: i32 = 1;
/// Rotated quadratic cone.
pub const OPTSOLVER_CONE_ROTATED_QUADRATIC: i32 = 2;
/// Primal exponential cone.
pub const OPTSOLVER_CONE_PRIMAL_EXPONENTIAL: i32 = 3;
/// Dual exponential cone.
pub const OPTSOLVER_CONE_DUAL_EXPONENTIAL: i32 = 4;
/// Primal power cone.
pub const OPTSOLVER_CONE_PRIMAL_POWER: i32 = 5;
/// Dual power cone.
pub const OPTSOLVER_CONE_DUAL_POWER: i32 = 6;
/// Nonnegative orthant.
pub const OPTSOLVER_CONE_NONNEGATIVE: i32 = 7;
/// Zero cone.
pub const OPTSOLVER_CONE_ZERO: i32 = 8;
/// Positive semidefinite cone (svec layout).
pub const OPTSOLVER_CONE_POSITIVE_SEMIDEFINITE: i32 = 9;

/// Problem-class bits.
pub const OPTSOLVER_CLASS_LP: u32 = 1;
/// Mixed-integer linear programme.
pub const OPTSOLVER_CLASS_MILP: u32 = 2;
/// Quadratic programme.
pub const OPTSOLVER_CLASS_QP: u32 = 4;
/// Conic programme.
pub const OPTSOLVER_CLASS_CONIC: u32 = 8;

/// Methods.
pub const OPTSOLVER_METHOD_AUTO: i32 = 0;
/// Primal simplex.
pub const OPTSOLVER_METHOD_PRIMAL_SIMPLEX: i32 = 1;
/// Dual simplex.
pub const OPTSOLVER_METHOD_DUAL_SIMPLEX: i32 = 2;
/// Barrier.
pub const OPTSOLVER_METHOD_BARRIER: i32 = 3;
/// Concurrent.
pub const OPTSOLVER_METHOD_CONCURRENT: i32 = 4;

/// Presolve levels.
pub const OPTSOLVER_PRESOLVE_AUTO: i32 = 0;
/// No presolve.
pub const OPTSOLVER_PRESOLVE_OFF: i32 = 1;
/// Conservative presolve.
pub const OPTSOLVER_PRESOLVE_CONSERVATIVE: i32 = 2;
/// Aggressive presolve.
pub const OPTSOLVER_PRESOLVE_AGGRESSIVE: i32 = 3;

/// Basis statuses.
pub const OPTSOLVER_BASIS_BASIC: i32 = 0;
/// At lower bound.
pub const OPTSOLVER_BASIS_AT_LOWER: i32 = 1;
/// At upper bound.
pub const OPTSOLVER_BASIS_AT_UPPER: i32 = 2;
/// Superbasic.
pub const OPTSOLVER_BASIS_SUPERBASIC: i32 = 3;
/// Fixed.
pub const OPTSOLVER_BASIS_FIXED: i32 = 4;

/// Verification overall verdicts.
pub const OPTSOLVER_VERIFICATION_NOT_RUN: i32 = 0;
/// Verified.
pub const OPTSOLVER_VERIFICATION_VERIFIED: i32 = 1;
/// Failed.
pub const OPTSOLVER_VERIFICATION_FAILED: i32 = 2;
/// Partially verified.
pub const OPTSOLVER_VERIFICATION_PARTIALLY_VERIFIED: i32 = 3;

/// Measure verdicts.
pub const OPTSOLVER_VERDICT_PASS: i32 = 0;
/// Fail.
pub const OPTSOLVER_VERDICT_FAIL: i32 = 1;
/// Not evaluated.
pub const OPTSOLVER_VERDICT_NOT_EVALUATED: i32 = 2;

/// Result arrays.
pub const OPTSOLVER_ARRAY_X: i32 = 0;
/// Row duals.
pub const OPTSOLVER_ARRAY_Y: i32 = 1;
/// Reduced costs.
pub const OPTSOLVER_ARRAY_Z: i32 = 2;
/// Lower-bound multipliers.
pub const OPTSOLVER_ARRAY_Z_L: i32 = 3;
/// Upper-bound multipliers.
pub const OPTSOLVER_ARRAY_Z_U: i32 = 4;
/// Conic duals.
pub const OPTSOLVER_ARRAY_S: i32 = 5;
/// Row activity.
pub const OPTSOLVER_ARRAY_ROW_ACTIVITY: i32 = 6;

/// Certificate kinds.
pub const OPTSOLVER_CERTIFICATE_NONE: i32 = 0;
/// Farkas certificate of infeasibility.
pub const OPTSOLVER_CERTIFICATE_FARKAS: i32 = 1;
/// Unbounded ray.
pub const OPTSOLVER_CERTIFICATE_UNBOUNDED_RAY: i32 = 2;

/// Certificate arrays: Farkas row multipliers.
pub const OPTSOLVER_CERT_Y: i32 = 0;
/// Farkas bound multipliers.
pub const OPTSOLVER_CERT_Z: i32 = 1;
/// Farkas conic multipliers.
pub const OPTSOLVER_CERT_S: i32 = 2;
/// The unbounded ray.
pub const OPTSOLVER_CERT_RAY: i32 = 3;

/// Parameter value types.
pub const OPTSOLVER_PARAM_INT: i32 = 1;
/// Double.
pub const OPTSOLVER_PARAM_DOUBLE: i32 = 2;
/// String.
pub const OPTSOLVER_PARAM_STRING: i32 = 3;

// ---------------------------------------------------------------------------------------------
// Errors, panic containment and the silent panic hook.

thread_local! {
    /// This thread's last failure: message and vendor code (0 unless a solver error).
    static LAST_ERROR: RefCell<Option<(String, i32)>> = const { RefCell::new(None) };
    /// Whether this thread is inside a C entry point (the panic hook stays silent there).
    static IN_ENTRY: Cell<bool> = const { Cell::new(false) };
    /// The message of a panic caught inside an entry point.
    static PANIC_MESSAGE: RefCell<Option<String>> = const { RefCell::new(None) };
}

static HOOK: Once = Once::new();

/// Installs, once per process, a panic hook that is silent for panics inside an entry point
/// (recording the message) and passes every other panic to the previously installed hook.
fn install_hook() {
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let inside = IN_ENTRY.try_with(Cell::get).unwrap_or(false);
            if inside {
                let _ = PANIC_MESSAGE.try_with(|m| *m.borrow_mut() = Some(info.to_string()));
            } else {
                previous(info);
            }
        }));
    });
}

/// The code of an error.
pub fn code_of(e: &Error) -> i32 {
    match e {
        Error::Invalid { .. } => OPTSOLVER_ERR_INVALID,
        Error::TooLarge { .. } => OPTSOLVER_ERR_TOO_LARGE,
        Error::Unsupported { .. } => OPTSOLVER_ERR_UNSUPPORTED,
        Error::ParameterConflict { .. } => OPTSOLVER_ERR_PARAMETER_CONFLICT,
        Error::Solver { .. } => OPTSOLVER_ERR_SOLVER,
    }
}

fn set_error(message: String, vendor: i32) {
    LAST_ERROR.with(|e| *e.borrow_mut() = Some((message, vendor)));
}

/// Records `e` as this thread's last failure and returns its code.
pub(crate) fn fail(e: &Error) -> i32 {
    let vendor = match e {
        Error::Solver { code, .. } => *code,
        _ => 0,
    };
    set_error(e.to_string(), vendor);
    code_of(e)
}

/// Runs `f` with panics contained: on a panic the thread's error becomes the internal error
/// (message from the silent hook) and `default` is returned.
pub(crate) fn contain<T>(default: T, f: impl FnOnce() -> T) -> T {
    install_hook();
    let was = IN_ENTRY.with(|c| c.replace(true));
    let r = catch_unwind(AssertUnwindSafe(f));
    IN_ENTRY.with(|c| c.set(was));
    match r {
        Ok(v) => v,
        Err(payload) => {
            let message = PANIC_MESSAGE
                .with(|m| m.borrow_mut().take())
                .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_string()))
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "a panic with no message".to_string());
            set_error(format!("internal error: {message}"), 0);
            default
        }
    }
}

/// Runs a fallible entry-point body: `Ok` → [`OPTSOLVER_OK`]; `Err` → its code with the message
/// recorded; a panic → [`OPTSOLVER_ERR_INTERNAL`].
pub(crate) fn guard(f: impl FnOnce() -> Result<(), Error>) -> i32 {
    contain(OPTSOLVER_ERR_INTERNAL, || match f() {
        Ok(()) => OPTSOLVER_OK,
        Err(e) => fail(&e),
    })
}

/// [`Error::Invalid`] shorthand.
pub(crate) fn invalid(what: &'static str, detail: impl Into<String>) -> Error {
    Error::Invalid {
        what,
        detail: detail.into(),
    }
}

// ---------------------------------------------------------------------------------------------
// Version, errors, strings, test support.

/// The interface version, `(major << 16) | minor`.
#[unsafe(no_mangle)]
pub extern "C" fn optsolver_abi_version() -> u32 {
    contain(0, || {
        (OPTSOLVER_ABI_VERSION_MAJOR << 16) | OPTSOLVER_ABI_VERSION_MINOR
    })
}

/// A caller-owned copy of this thread's last failure message, or NULL if there was none. Free
/// it with [`optsolver_string_free`].
#[unsafe(no_mangle)]
pub extern "C" fn optsolver_last_error_message() -> *mut c_char {
    contain(core::ptr::null_mut(), || {
        LAST_ERROR.with(|e| match &*e.borrow() {
            None => core::ptr::null_mut(),
            Some((m, _)) => CString::new(m.replace('\0', " "))
                .map(CString::into_raw)
                .unwrap_or(core::ptr::null_mut()),
        })
    })
}

/// The vendor code of this thread's last failure if it was a solver error, else 0.
#[unsafe(no_mangle)]
pub extern "C" fn optsolver_last_error_solver_code() -> i32 {
    contain(0, || {
        LAST_ERROR.with(|e| e.borrow().as_ref().map_or(0, |(_, c)| *c))
    })
}

/// Frees a string returned by the library. NULL is a no-op.
///
/// # Safety
/// `s` must be NULL or a pointer returned by [`optsolver_last_error_message`], not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_string_free(s: *mut c_char) {
    contain((), || {
        if !s.is_null() {
            // SAFETY: by the contract `s` came from `CString::into_raw` and is freed once.
            drop(unsafe { CString::from_raw(s) });
        }
    })
}

/// Test support: panics inside the library; returns [`OPTSOLVER_ERR_INTERNAL`].
#[unsafe(no_mangle)]
pub extern "C" fn optsolver_test_panic() -> i32 {
    guard(|| panic!("optsolver_test_panic: deliberate panic for the containment test"))
}

static ENVIRONMENTS_CREATED: AtomicU64 = AtomicU64::new(0);

pub(crate) fn count_environment() {
    ENVIRONMENTS_CREATED.fetch_add(1, Ordering::Relaxed);
}

/// Test support: the number of backend environments the C interface has created in this
/// process (handles, one-shot solves and capability queries).
#[unsafe(no_mangle)]
pub extern "C" fn optsolver_environments_created() -> u64 {
    contain(0, || ENVIRONMENTS_CREATED.load(Ordering::Relaxed))
}

// ---------------------------------------------------------------------------------------------
// Code translations (one function per enum, each total over the Rust enum).

/// Status → code.
pub fn status_code(s: Status) -> i32 {
    match s {
        Status::Optimal => OPTSOLVER_STATUS_OPTIMAL,
        Status::Infeasible => OPTSOLVER_STATUS_INFEASIBLE,
        Status::Unbounded => OPTSOLVER_STATUS_UNBOUNDED,
        Status::InfeasibleOrUnbounded => OPTSOLVER_STATUS_INFEASIBLE_OR_UNBOUNDED,
        Status::StoppedAtLimit => OPTSOLVER_STATUS_STOPPED_AT_LIMIT,
        Status::NumericalDifficulty => OPTSOLVER_STATUS_NUMERICAL_DIFFICULTY,
        Status::Interrupted => OPTSOLVER_STATUS_INTERRUPTED,
        Status::LocallyOptimal => OPTSOLVER_STATUS_LOCALLY_OPTIMAL,
        Status::LocallyInfeasible => OPTSOLVER_STATUS_LOCALLY_INFEASIBLE,
        Status::NotSolved => OPTSOLVER_STATUS_NOT_SOLVED,
        Status::Unknown => OPTSOLVER_STATUS_UNKNOWN,
    }
}

/// Every status, in code order.
pub const STATUSES: [Status; 11] = [
    Status::Optimal,
    Status::Infeasible,
    Status::Unbounded,
    Status::InfeasibleOrUnbounded,
    Status::StoppedAtLimit,
    Status::NumericalDifficulty,
    Status::Interrupted,
    Status::LocallyOptimal,
    Status::LocallyInfeasible,
    Status::NotSolved,
    Status::Unknown,
];

/// Limit reason → code.
pub fn limit_code(l: Option<LimitReason>) -> i32 {
    match l {
        None => OPTSOLVER_LIMIT_NONE,
        Some(LimitReason::Time) => OPTSOLVER_LIMIT_TIME,
        Some(LimitReason::Iterations) => OPTSOLVER_LIMIT_ITERATIONS,
        Some(LimitReason::Nodes) => OPTSOLVER_LIMIT_NODES,
        Some(LimitReason::Solutions) => OPTSOLVER_LIMIT_SOLUTIONS,
        Some(LimitReason::ObjectiveCutoff) => OPTSOLVER_LIMIT_OBJECTIVE_CUTOFF,
        Some(LimitReason::ObjectiveTarget) => OPTSOLVER_LIMIT_OBJECTIVE_TARGET,
        Some(LimitReason::Work) => OPTSOLVER_LIMIT_WORK,
        Some(LimitReason::Memory) => OPTSOLVER_LIMIT_MEMORY,
    }
}

/// Every limit reason, in code order (code 1 first).
pub const LIMITS: [LimitReason; 8] = [
    LimitReason::Time,
    LimitReason::Iterations,
    LimitReason::Nodes,
    LimitReason::Solutions,
    LimitReason::ObjectiveCutoff,
    LimitReason::ObjectiveTarget,
    LimitReason::Work,
    LimitReason::Memory,
];

/// Solution returned → code.
pub fn returned_code(r: SolutionReturned) -> i32 {
    match r {
        SolutionReturned::None => OPTSOLVER_RETURNED_NONE,
        SolutionReturned::Feasible => OPTSOLVER_RETURNED_FEASIBLE,
        SolutionReturned::Optimal => OPTSOLVER_RETURNED_OPTIMAL,
    }
}

/// Every solution-returned value, in code order.
pub const RETURNED: [SolutionReturned; 3] = [
    SolutionReturned::None,
    SolutionReturned::Feasible,
    SolutionReturned::Optimal,
];

/// Basis status → code.
pub fn basis_code(b: BasisStatus) -> i32 {
    match b {
        BasisStatus::Basic => OPTSOLVER_BASIS_BASIC,
        BasisStatus::AtLower => OPTSOLVER_BASIS_AT_LOWER,
        BasisStatus::AtUpper => OPTSOLVER_BASIS_AT_UPPER,
        BasisStatus::Superbasic => OPTSOLVER_BASIS_SUPERBASIC,
        BasisStatus::Fixed => OPTSOLVER_BASIS_FIXED,
    }
}

/// Overall verification verdict → code (`None`: verification switched off).
pub fn overall_code(o: Option<Overall>) -> i32 {
    match o {
        None => OPTSOLVER_VERIFICATION_NOT_RUN,
        Some(Overall::Verified) => OPTSOLVER_VERIFICATION_VERIFIED,
        Some(Overall::Failed) => OPTSOLVER_VERIFICATION_FAILED,
        Some(Overall::PartiallyVerified) => OPTSOLVER_VERIFICATION_PARTIALLY_VERIFIED,
    }
}

/// Every overall verdict, in code order (code 1 first).
pub const OVERALLS: [Overall; 3] = [
    Overall::Verified,
    Overall::Failed,
    Overall::PartiallyVerified,
];

/// Measure verdict → code.
pub fn verdict_code(v: Verdict) -> i32 {
    match v {
        Verdict::Pass => OPTSOLVER_VERDICT_PASS,
        Verdict::Fail => OPTSOLVER_VERDICT_FAIL,
        Verdict::NotEvaluated => OPTSOLVER_VERDICT_NOT_EVALUATED,
    }
}

/// Cone kind → code.
pub fn cone_code(k: ConeKind) -> i32 {
    match k {
        ConeKind::Quadratic => OPTSOLVER_CONE_QUADRATIC,
        ConeKind::RotatedQuadratic => OPTSOLVER_CONE_ROTATED_QUADRATIC,
        ConeKind::PrimalExponential => OPTSOLVER_CONE_PRIMAL_EXPONENTIAL,
        ConeKind::DualExponential => OPTSOLVER_CONE_DUAL_EXPONENTIAL,
        ConeKind::PrimalPower => OPTSOLVER_CONE_PRIMAL_POWER,
        ConeKind::DualPower => OPTSOLVER_CONE_DUAL_POWER,
        ConeKind::Nonnegative => OPTSOLVER_CONE_NONNEGATIVE,
        ConeKind::Zero => OPTSOLVER_CONE_ZERO,
        ConeKind::PositiveSemidefinite => OPTSOLVER_CONE_POSITIVE_SEMIDEFINITE,
    }
}

/// Code → cone kind.
pub fn cone_kind(code: i32) -> Option<ConeKind> {
    ConeKind::ALL.into_iter().find(|&k| cone_code(k) == code)
}

/// Problem class → bit.
pub fn class_bit(c: ProblemClass) -> u32 {
    match c {
        ProblemClass::Lp => OPTSOLVER_CLASS_LP,
        ProblemClass::Milp => OPTSOLVER_CLASS_MILP,
        ProblemClass::Qp => OPTSOLVER_CLASS_QP,
        ProblemClass::Conic => OPTSOLVER_CLASS_CONIC,
    }
}

/// Code → method.
pub fn method_of(code: i32) -> Option<Method> {
    match code {
        OPTSOLVER_METHOD_AUTO => Some(Method::Auto),
        OPTSOLVER_METHOD_PRIMAL_SIMPLEX => Some(Method::PrimalSimplex),
        OPTSOLVER_METHOD_DUAL_SIMPLEX => Some(Method::DualSimplex),
        OPTSOLVER_METHOD_BARRIER => Some(Method::Barrier),
        OPTSOLVER_METHOD_CONCURRENT => Some(Method::Concurrent),
        _ => None,
    }
}

/// Code → presolve level.
pub fn presolve_of(code: i32) -> Option<Presolve> {
    match code {
        OPTSOLVER_PRESOLVE_AUTO => Some(Presolve::Auto),
        OPTSOLVER_PRESOLVE_OFF => Some(Presolve::Off),
        OPTSOLVER_PRESOLVE_CONSERVATIVE => Some(Presolve::Conservative),
        OPTSOLVER_PRESOLVE_AGGRESSIVE => Some(Presolve::Aggressive),
        _ => None,
    }
}

/// Parameter value → type code.
pub fn param_type(v: &ParamValue) -> i32 {
    match v {
        ParamValue::Int(_) => OPTSOLVER_PARAM_INT,
        ParamValue::Double(_) => OPTSOLVER_PARAM_DOUBLE,
        ParamValue::Str(_) => OPTSOLVER_PARAM_STRING,
    }
}

// ---------------------------------------------------------------------------------------------
// Names, derived from the core's own names and cached for the process lifetime.

static EMPTY: &CStr = c"";

/// A process-wide cache of NUL-terminated copies of the core's `&'static str` names. Each
/// distinct name is converted once and kept for the process lifetime (a bounded set).
pub(crate) fn static_name(name: &'static str) -> *const c_char {
    static CACHE: OnceLock<Mutex<HashMap<&'static str, &'static CStr>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let c = map.entry(name).or_insert_with(|| {
        let owned = CString::new(name.replace('\0', " ")).unwrap_or_default();
        Box::leak(owned.into_boxed_c_str())
    });
    c.as_ptr()
}

/// A name built from a `Debug` rendering, cached by the code it describes.
fn debug_name(
    table: &'static OnceLock<Vec<CString>>,
    names: impl FnOnce() -> Vec<String>,
    index: Option<usize>,
) -> *const c_char {
    let v = table.get_or_init(|| {
        names()
            .into_iter()
            .map(|s| CString::new(s).unwrap_or_default())
            .collect()
    });
    match index.and_then(|i| v.get(i)) {
        Some(c) => c.as_ptr(),
        None => EMPTY.as_ptr(),
    }
}

/// Name of a status code (the Rust variant name, for example `"Optimal"`), or `""`.
#[unsafe(no_mangle)]
pub extern "C" fn optsolver_status_name(code: i32) -> *const c_char {
    contain(EMPTY.as_ptr(), || {
        static T: OnceLock<Vec<CString>> = OnceLock::new();
        let i = usize::try_from(code - 1).ok();
        debug_name(
            &T,
            || STATUSES.iter().map(|s| format!("{s:?}")).collect(),
            i,
        )
    })
}

/// Name of a limit-reason code (for example `"Memory"`), `""` for none or unknown.
#[unsafe(no_mangle)]
pub extern "C" fn optsolver_limit_reason_name(code: i32) -> *const c_char {
    contain(EMPTY.as_ptr(), || {
        static T: OnceLock<Vec<CString>> = OnceLock::new();
        let i = usize::try_from(code - 1).ok();
        debug_name(&T, || LIMITS.iter().map(|s| format!("{s:?}")).collect(), i)
    })
}

/// Name of a solution-returned code (`"None"`, `"Feasible"`, `"Optimal"`), or `""`.
#[unsafe(no_mangle)]
pub extern "C" fn optsolver_returned_name(code: i32) -> *const c_char {
    contain(EMPTY.as_ptr(), || {
        static T: OnceLock<Vec<CString>> = OnceLock::new();
        let i = usize::try_from(code).ok();
        debug_name(
            &T,
            || RETURNED.iter().map(|s| format!("{s:?}")).collect(),
            i,
        )
    })
}

/// Name of a verification code (`"NotRun"`, `"Verified"`, `"Failed"`, `"PartiallyVerified"`).
#[unsafe(no_mangle)]
pub extern "C" fn optsolver_verification_name(code: i32) -> *const c_char {
    contain(EMPTY.as_ptr(), || {
        static T: OnceLock<Vec<CString>> = OnceLock::new();
        let i = usize::try_from(code).ok();
        debug_name(
            &T,
            || {
                std::iter::once("NotRun".to_string())
                    .chain(OVERALLS.iter().map(|o| format!("{o:?}")))
                    .collect()
            },
            i,
        )
    })
}

/// Name of a cone-kind code (`ConeKind::name`, for example `"rotated quadratic"`), or `""`.
#[unsafe(no_mangle)]
pub extern "C" fn optsolver_cone_name(code: i32) -> *const c_char {
    contain(EMPTY.as_ptr(), || match cone_kind(code) {
        Some(k) => static_name(k.name()),
        None => EMPTY.as_ptr(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(p: *const c_char) -> String {
        // SAFETY: the name functions return NUL-terminated strings valid for the process.
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }

    #[test]
    fn version_is_major_minor() {
        assert_eq!(optsolver_abi_version(), 1 << 16);
    }

    #[test]
    fn errors_map_to_codes_and_keep_vendor_code() {
        let cases = [
            (invalid("A.values", "x"), OPTSOLVER_ERR_INVALID, 0),
            (
                Error::TooLarge {
                    what: "m",
                    value: 3,
                },
                OPTSOLVER_ERR_TOO_LARGE,
                0,
            ),
            (
                Error::Unsupported {
                    field: "Q",
                    backend: "b",
                },
                OPTSOLVER_ERR_UNSUPPORTED,
                0,
            ),
            (
                Error::ParameterConflict {
                    name: "TimeLimit".into(),
                },
                OPTSOLVER_ERR_PARAMETER_CONFLICT,
                0,
            ),
            (
                Error::Solver {
                    code: 10007,
                    message: "m".into(),
                },
                OPTSOLVER_ERR_SOLVER,
                10007,
            ),
        ];
        for (e, code, vendor) in cases {
            let want = e.to_string();
            assert_eq!(guard(|| Err(e)), code);
            assert_eq!(optsolver_last_error_solver_code(), vendor);
            let m = optsolver_last_error_message();
            assert_eq!(s(m), want);
            // SAFETY: `m` came from optsolver_last_error_message and is freed once.
            unsafe { optsolver_string_free(m) };
        }
        // SAFETY: NULL is a no-op.
        unsafe { optsolver_string_free(core::ptr::null_mut()) };
    }

    #[test]
    fn messages_are_per_thread() {
        let a = std::thread::spawn(|| {
            guard(|| {
                Err(Error::Solver {
                    code: 7,
                    message: "first".into(),
                })
            });
            std::thread::sleep(std::time::Duration::from_millis(50));
            let m = optsolver_last_error_message();
            let r = (s(m), optsolver_last_error_solver_code());
            // SAFETY: freed once.
            unsafe { optsolver_string_free(m) };
            r
        });
        let b = std::thread::spawn(|| {
            guard(|| Err(invalid("c", "second")));
            let m = optsolver_last_error_message();
            let r = (s(m), optsolver_last_error_solver_code());
            // SAFETY: freed once.
            unsafe { optsolver_string_free(m) };
            r
        });
        let (ma, ca) = a.join().unwrap();
        let (mb, cb) = b.join().unwrap();
        assert_eq!((ma.as_str(), ca), ("solver error 7: first", 7));
        assert_eq!((mb.as_str(), cb), ("invalid c: second", 0));
    }

    #[test]
    fn contained_panic_is_internal_and_silent() {
        assert_eq!(optsolver_test_panic(), OPTSOLVER_ERR_INTERNAL);
        let m = optsolver_last_error_message();
        assert!(s(m).contains("deliberate panic"));
        // SAFETY: freed once.
        unsafe { optsolver_string_free(m) };
    }

    #[test]
    fn panic_outside_an_entry_point_reaches_the_previous_hook() {
        // Installs the wrapper (if not yet), then checks that a panic outside an entry point is
        // passed on: the wrapper calls whatever hook was installed before it, which for a test
        // binary is the default hook; we observe it indirectly by the panic still unwinding with
        // its payload and the in-entry flag being false.
        let _ = optsolver_abi_version();
        assert!(!IN_ENTRY.with(Cell::get));
        let r = std::panic::catch_unwind(|| panic!("outside"));
        assert!(r.is_err());
        assert!(PANIC_MESSAGE.with(|m| m.borrow().is_none()));
    }

    #[test]
    fn codes_translate_one_to_one() {
        for (i, s) in STATUSES.iter().enumerate() {
            assert_eq!(status_code(*s), i32::try_from(i).unwrap() + 1);
        }
        assert_eq!(limit_code(None), OPTSOLVER_LIMIT_NONE);
        for (i, l) in LIMITS.iter().enumerate() {
            assert_eq!(limit_code(Some(*l)), i32::try_from(i).unwrap() + 1);
        }
        for (i, r) in RETURNED.iter().enumerate() {
            assert_eq!(returned_code(*r), i32::try_from(i).unwrap());
        }
        assert_eq!(overall_code(None), OPTSOLVER_VERIFICATION_NOT_RUN);
        for (i, o) in OVERALLS.iter().enumerate() {
            assert_eq!(overall_code(Some(*o)), i32::try_from(i).unwrap() + 1);
        }
        for k in ConeKind::ALL {
            assert_eq!(cone_kind(cone_code(k)), Some(k));
        }
        assert_eq!(cone_kind(0), None);
        assert_eq!(cone_kind(10), None);
        let bases = [
            BasisStatus::Basic,
            BasisStatus::AtLower,
            BasisStatus::AtUpper,
            BasisStatus::Superbasic,
            BasisStatus::Fixed,
        ];
        for (i, b) in bases.iter().enumerate() {
            assert_eq!(basis_code(*b), i32::try_from(i).unwrap());
        }
        for (v, c) in [
            (Verdict::Pass, 0),
            (Verdict::Fail, 1),
            (Verdict::NotEvaluated, 2),
        ] {
            assert_eq!(verdict_code(v), c);
        }
        for (c, bit) in [
            (ProblemClass::Lp, 1),
            (ProblemClass::Milp, 2),
            (ProblemClass::Qp, 4),
            (ProblemClass::Conic, 8),
        ] {
            assert_eq!(class_bit(c), bit);
        }
        for code in 0..5 {
            assert!(method_of(code).is_some());
        }
        assert!(method_of(5).is_none() && method_of(-1).is_none());
        for code in 0..4 {
            assert!(presolve_of(code).is_some());
        }
        assert!(presolve_of(4).is_none());
        assert_eq!(param_type(&ParamValue::Int(1)), OPTSOLVER_PARAM_INT);
        assert_eq!(param_type(&ParamValue::Double(1.0)), OPTSOLVER_PARAM_DOUBLE);
        assert_eq!(
            param_type(&ParamValue::Str("a".into())),
            OPTSOLVER_PARAM_STRING
        );
    }

    #[test]
    fn names_come_from_the_core() {
        for s_ in STATUSES {
            assert_eq!(s(optsolver_status_name(status_code(s_))), format!("{s_:?}"));
        }
        assert_eq!(s(optsolver_status_name(0)), "");
        assert_eq!(s(optsolver_status_name(12)), "");
        for l in LIMITS {
            assert_eq!(
                s(optsolver_limit_reason_name(limit_code(Some(l)))),
                format!("{l:?}")
            );
        }
        assert_eq!(s(optsolver_limit_reason_name(0)), "");
        for r in RETURNED {
            assert_eq!(
                s(optsolver_returned_name(returned_code(r))),
                format!("{r:?}")
            );
        }
        assert_eq!(s(optsolver_verification_name(0)), "NotRun");
        for o in OVERALLS {
            assert_eq!(
                s(optsolver_verification_name(overall_code(Some(o)))),
                format!("{o:?}")
            );
        }
        for k in ConeKind::ALL {
            assert_eq!(s(optsolver_cone_name(cone_code(k))), k.name());
        }
        assert_eq!(s(optsolver_cone_name(99)), "");
    }
}
