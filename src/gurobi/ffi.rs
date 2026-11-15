//! Raw declarations of the Gurobi C interface used by optSolver (task T039; research R1-R7).
//!
//! This is the only module in the crate with an `extern "C"` block (constitution VII-A). Every
//! declaration is copied from the installed Gurobi 13.0.3 header
//! `$GUROBI_HOME/include/gurobi_c.h` (read 2026-11-14) and checked against it by
//! `tests/header_conformance.rs` / `tests/header_conformance.c` (research R12, constitution
//! VII-E). **Adding a declaration here requires adding the matching check to
//! `tests/header_conformance.c`.**
//!
//! Type mapping: `int` → [`c_int`], `double` → [`c_double`], `char` → [`c_char`], `void *` →
//! `*mut c_void`, `size_t` → `usize`. Gurobi declares its functions `__stdcall`; the header
//! defines `__stdcall` as empty on Linux, and on the supported 64-bit targets the calling
//! convention is the C ABI, hence `extern "C"`.
//!
//! The header declares most input arrays without `const`. The Gurobi 13.0 reference manual
//! describes them only as data used "to initialize the model data"; see research R2 and R17
//! for what is and is not documented before passing pointers derived from shared slices.
#![allow(dead_code)] // Complete header-checked table (task T037); not every entry is used yet.

use core::ffi::{c_char, c_double, c_int, c_void};
use core::marker::{PhantomData, PhantomPinned};

/// Opaque Gurobi environment (`typedef struct _GRBenv GRBenv;`). Only ever used behind a
/// pointer; the marker makes it `!Send`, `!Sync` and `!Unpin`.
#[repr(C)]
pub struct GRBenv {
    _private: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

/// Opaque Gurobi model (`typedef struct _GRBmodel GRBmodel;`). Only ever used behind a
/// pointer; the marker makes it `!Send`, `!Sync` and `!Unpin`.
#[repr(C)]
pub struct GRBmodel {
    _private: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

/// User callback type: `int (__stdcall *cb)(CB_ARGS)` with
/// `CB_ARGS = GRBmodel *model, void *cbdata, int where, void *usrdata`. `None` is `NULL`
/// (removes the callback).
pub type GRBcallback = Option<
    unsafe extern "C" fn(
        model: *mut GRBmodel,
        cbdata: *mut c_void,
        where_: c_int,
        usrdata: *mut c_void,
    ) -> c_int,
>;

// ---------------------------------------------------------------------------------------
// Constants (header values; each is `_Static_assert`ed in tests/header_conformance.c).
// ---------------------------------------------------------------------------------------

/// `GRB_LOADED`: model loaded, no optimisation run.
pub const GRB_LOADED: c_int = 1;
/// `GRB_OPTIMAL`: optimal solution found.
pub const GRB_OPTIMAL: c_int = 2;
/// `GRB_INFEASIBLE`: model proven infeasible.
pub const GRB_INFEASIBLE: c_int = 3;
/// `GRB_INF_OR_UNBD`: infeasible or unbounded, not determined which.
pub const GRB_INF_OR_UNBD: c_int = 4;
/// `GRB_UNBOUNDED`: model proven unbounded.
pub const GRB_UNBOUNDED: c_int = 5;
/// `GRB_CUTOFF`: objective worse than the `Cutoff` parameter.
pub const GRB_CUTOFF: c_int = 6;
/// `GRB_ITERATION_LIMIT`: simplex or barrier iteration limit reached.
pub const GRB_ITERATION_LIMIT: c_int = 7;
/// `GRB_NODE_LIMIT`: branch-and-bound node limit reached.
pub const GRB_NODE_LIMIT: c_int = 8;
/// `GRB_TIME_LIMIT`: time limit reached.
pub const GRB_TIME_LIMIT: c_int = 9;
/// `GRB_SOLUTION_LIMIT`: solution-count limit reached.
pub const GRB_SOLUTION_LIMIT: c_int = 10;
/// `GRB_INTERRUPTED`: optimisation terminated by the user (for example `GRBterminate`).
pub const GRB_INTERRUPTED: c_int = 11;
/// `GRB_NUMERIC`: terminated by unrecoverable numerical difficulties.
pub const GRB_NUMERIC: c_int = 12;
/// `GRB_SUBOPTIMAL`: tolerances not met; a sub-optimal solution may be available.
pub const GRB_SUBOPTIMAL: c_int = 13;
/// `GRB_INPROGRESS`: asynchronous optimisation still running.
pub const GRB_INPROGRESS: c_int = 14;
/// `GRB_USER_OBJ_LIMIT`: user objective limit (`BestObjStop`/`BestBdStop`) reached.
pub const GRB_USER_OBJ_LIMIT: c_int = 15;
/// `GRB_WORK_LIMIT`: work limit reached.
pub const GRB_WORK_LIMIT: c_int = 16;
/// `GRB_MEM_LIMIT`: memory limit reached.
pub const GRB_MEM_LIMIT: c_int = 17;
/// `GRB_LOCALLY_OPTIMAL`: local optimum (non-convex classes).
pub const GRB_LOCALLY_OPTIMAL: c_int = 18;
/// `GRB_LOCALLY_INFEASIBLE`: locally infeasible (non-convex classes).
pub const GRB_LOCALLY_INFEASIBLE: c_int = 19;

/// `GRB_INFINITY` (`1e100`): Gurobi's representation of an infinite bound.
pub const GRB_INFINITY: c_double = 1e100;
/// `GRB_ERROR_DATA_NOT_AVAILABLE`: the requested attribute is not available.
pub const GRB_ERROR_DATA_NOT_AVAILABLE: c_int = 10005;
/// `GRB_ERROR_Q_NOT_PSD`: the quadratic objective is not convex (with `NonConvex` = 0).
pub const GRB_ERROR_Q_NOT_PSD: c_int = 10020;
/// `GRB_ERROR_QCP_EQUALITY_CONSTRAINT`: a quadratic equality constraint.
pub const GRB_ERROR_QCP_EQUALITY_CONSTRAINT: c_int = 10021;
/// `GRB_ERROR_NOT_SUPPORTED`: the operation is not supported.
pub const GRB_ERROR_NOT_SUPPORTED: c_int = 10024;
/// `GRB_MINIMIZE`: objective sense argument of `GRBXloadmodel` for minimisation.
pub const GRB_MINIMIZE: c_int = 1;
/// `GRB_MAXIMIZE`: objective sense argument of `GRBXloadmodel` for maximisation.
pub const GRB_MAXIMIZE: c_int = -1;

/// `GRB_LESS_EQUAL` (`'<'`): row sense `≤`.
pub const GRB_LESS_EQUAL: c_char = b'<' as c_char;
/// `GRB_GREATER_EQUAL` (`'>'`): row sense `≥`.
pub const GRB_GREATER_EQUAL: c_char = b'>' as c_char;
/// `GRB_EQUAL` (`'='`): row sense `=`.
pub const GRB_EQUAL: c_char = b'=' as c_char;

/// `GRB_MAX_STRLEN` (512): size of the buffer `GRBgetstrparam` writes into, including the
/// terminating NUL.
pub const GRB_MAX_STRLEN: usize = 512;

/// Callback `where`: `GRB_CB_POLLING`, periodic polling.
pub const GRB_CB_POLLING: c_int = 0;
/// Callback `where`: `GRB_CB_SIMPLEX`, during simplex.
pub const GRB_CB_SIMPLEX: c_int = 2;
/// Callback `where`: `GRB_CB_BARRIER`, during barrier.
pub const GRB_CB_BARRIER: c_int = 7;
/// Callback `what`: `GRB_CB_SPX_ITRCNT` (double), simplex iteration count.
pub const GRB_CB_SPX_ITRCNT: c_int = 2000;
/// Callback `what`: `GRB_CB_SPX_OBJVAL` (double), current simplex objective value.
pub const GRB_CB_SPX_OBJVAL: c_int = 2001;
/// Callback `what`: `GRB_CB_RUNTIME` (double), elapsed seconds; valid for every `where`.
pub const GRB_CB_RUNTIME: c_int = 6002;
/// Callback `what`: `GRB_CB_BARRIER_ITRCNT` (int), barrier iteration count.
pub const GRB_CB_BARRIER_ITRCNT: c_int = 7001;
/// Callback `what`: `GRB_CB_BARRIER_PRIMOBJ` (double), barrier primal objective.
pub const GRB_CB_BARRIER_PRIMOBJ: c_int = 7002;
/// Callback `what`: `GRB_CB_BARRIER_DUALOBJ` (double), barrier dual objective.
pub const GRB_CB_BARRIER_DUALOBJ: c_int = 7003;

/// Return value of `GRBgetparamtype` for an integer parameter. Not a header constant: observed
/// on Gurobi 13.0.3 by probe on 2026-11-14 (research R17); the 13.0 manual does not document
/// `GRBgetparamtype`.
pub const PARAM_TYPE_INT: c_int = 1;
/// Return value of `GRBgetparamtype` for a double parameter (observed; see [`PARAM_TYPE_INT`]).
pub const PARAM_TYPE_DBL: c_int = 2;
/// Return value of `GRBgetparamtype` for a string parameter (observed; see [`PARAM_TYPE_INT`]).
pub const PARAM_TYPE_STR: c_int = 3;
/// Return value of `GRBgetparamtype` for an unknown parameter name (observed; see
/// [`PARAM_TYPE_INT`]).
pub const PARAM_TYPE_UNKNOWN: c_int = -1;

// ---------------------------------------------------------------------------------------
// Functions.
// ---------------------------------------------------------------------------------------

unsafe extern "C" {
    /// `int GRBloadenvinternal(GRBenv **envP, const char *logfilename, int major, int minor,
    /// int tech)`: the function behind the `GRBloadenv` macro; creates and starts an
    /// environment.
    pub fn GRBloadenvinternal(
        envP: *mut *mut GRBenv,
        logfilename: *const c_char,
        major: c_int,
        minor: c_int,
        tech: c_int,
    ) -> c_int;

    /// `int GRBemptyenvinternal(GRBenv **envP, int major, int minor, int tech)`: the function
    /// behind the `GRBemptyenv` macro; creates an unstarted environment.
    pub fn GRBemptyenvinternal(
        envP: *mut *mut GRBenv,
        major: c_int,
        minor: c_int,
        tech: c_int,
    ) -> c_int;

    /// `int GRBstartenv(GRBenv *env)`: starts an empty environment (acquires the licence).
    pub fn GRBstartenv(env: *mut GRBenv) -> c_int;

    /// `void GRBfreeenv(GRBenv *env)`: frees an environment.
    pub fn GRBfreeenv(env: *mut GRBenv);

    /// `const char *GRBgeterrormsg(GRBenv *env)`: message of the most recent error in `env`.
    pub fn GRBgeterrormsg(env: *mut GRBenv) -> *const c_char;

    /// `void GRBversion(int *majorP, int *minorP, int *technicalP)`: version of the loaded
    /// library.
    pub fn GRBversion(majorP: *mut c_int, minorP: *mut c_int, technicalP: *mut c_int);

    /// `GRBenv *GRBgetenv(GRBmodel *model)`: the model's own copy of its environment.
    pub fn GRBgetenv(model: *mut GRBmodel) -> *mut GRBenv;

    /// `int GRBXloadmodel(GRBenv *env, GRBmodel **modelP, const char *Pname, int numvars,
    /// int numconstrs, int objsense, double objcon, double *obj, char *sense, double *rhs,
    /// size_t *vbeg, int *vlen, int *vind, double *vval, double *lb, double *ub, char *vtype,
    /// char **varnames, char **constrnames)`: creates a model from CSC data.
    pub fn GRBXloadmodel(
        env: *mut GRBenv,
        modelP: *mut *mut GRBmodel,
        Pname: *const c_char,
        numvars: c_int,
        numconstrs: c_int,
        objsense: c_int,
        objcon: c_double,
        obj: *mut c_double,
        sense: *mut c_char,
        rhs: *mut c_double,
        vbeg: *mut usize,
        vlen: *mut c_int,
        vind: *mut c_int,
        vval: *mut c_double,
        lb: *mut c_double,
        ub: *mut c_double,
        vtype: *mut c_char,
        varnames: *mut *mut c_char,
        constrnames: *mut *mut c_char,
    ) -> c_int;

    /// `int GRBXaddvars(GRBmodel *model, int numvars, size_t numnz, size_t *vbeg, int *vind,
    /// double *vval, double *obj, double *lb, double *ub, char *vtype, char **varnames)`:
    /// appends columns (applied at the next update or optimise).
    pub fn GRBXaddvars(
        model: *mut GRBmodel,
        numvars: c_int,
        numnz: usize,
        vbeg: *mut usize,
        vind: *mut c_int,
        vval: *mut c_double,
        obj: *mut c_double,
        lb: *mut c_double,
        ub: *mut c_double,
        vtype: *mut c_char,
        varnames: *mut *mut c_char,
    ) -> c_int;

    /// `int GRBaddqpterms(GRBmodel *model, int numqnz, int *qrow, int *qcol, double *qval)`:
    /// adds quadratic objective terms qval·x_qrow·x_qcol (objective xᵀQx form, no ½; terms
    /// are summed and (i, j) is the same term as (j, i); feature 003 research R1).
    pub fn GRBaddqpterms(
        model: *mut GRBmodel,
        numqnz: c_int,
        qrow: *mut c_int,
        qcol: *mut c_int,
        qval: *mut c_double,
    ) -> c_int;

    /// `int GRBaddqconstr(GRBmodel *model, int numlnz, int *lind, double *lval, int numqnz,
    /// int *qrow, int *qcol, double *qval, char sense, double rhs, const char *QCname)`: adds
    /// one quadratic constraint (feature 003 research R2).
    pub fn GRBaddqconstr(
        model: *mut GRBmodel,
        numlnz: c_int,
        lind: *mut c_int,
        lval: *mut c_double,
        numqnz: c_int,
        qrow: *mut c_int,
        qcol: *mut c_int,
        qval: *mut c_double,
        sense: c_char,
        rhs: c_double,
        QCname: *const c_char,
    ) -> c_int;

    /// `int GRBXaddconstrs(GRBmodel *model, int numconstrs, size_t numnz, size_t *cbeg,
    /// int *cind, double *cval, char *sense, double *rhs, char **constrnames)`: appends linear
    /// rows.
    pub fn GRBXaddconstrs(
        model: *mut GRBmodel,
        numconstrs: c_int,
        numnz: usize,
        cbeg: *mut usize,
        cind: *mut c_int,
        cval: *mut c_double,
        sense: *mut c_char,
        rhs: *mut c_double,
        constrnames: *mut *mut c_char,
    ) -> c_int;

    /// `int GRBXchgcoeffs(GRBmodel *model, size_t cnt, int *cind, int *vind, double *val)`:
    /// sets matrix coefficients (row cind[t], column vind[t]) to val[t].
    pub fn GRBXchgcoeffs(
        model: *mut GRBmodel,
        cnt: usize,
        cind: *mut c_int,
        vind: *mut c_int,
        val: *mut c_double,
    ) -> c_int;

    /// `int GRBupdatemodel(GRBmodel *model)`: applies pending modifications.
    pub fn GRBupdatemodel(model: *mut GRBmodel) -> c_int;

    /// `int GRBoptimize(GRBmodel *model)`: optimises the model.
    pub fn GRBoptimize(model: *mut GRBmodel) -> c_int;

    /// `int GRBfreemodel(GRBmodel *model)`: frees a model.
    pub fn GRBfreemodel(model: *mut GRBmodel) -> c_int;

    /// `void GRBterminate(GRBmodel *model)`: requests termination of the running optimisation
    /// (callable from a callback or another thread, per the 13.0 manual).
    pub fn GRBterminate(model: *mut GRBmodel);

    /// `int GRBgetintattr(GRBmodel *model, const char *attrname, int *valueP)`.
    pub fn GRBgetintattr(
        model: *mut GRBmodel,
        attrname: *const c_char,
        valueP: *mut c_int,
    ) -> c_int;

    /// `int GRBgetdblattr(GRBmodel *model, const char *attrname, double *valueP)`.
    pub fn GRBgetdblattr(
        model: *mut GRBmodel,
        attrname: *const c_char,
        valueP: *mut c_double,
    ) -> c_int;

    /// `int GRBgetintattrarray(GRBmodel *model, const char *attrname, int first, int len,
    /// int *values)`.
    pub fn GRBgetintattrarray(
        model: *mut GRBmodel,
        attrname: *const c_char,
        first: c_int,
        len: c_int,
        values: *mut c_int,
    ) -> c_int;

    /// `int GRBgetdblattrarray(GRBmodel *model, const char *attrname, int first, int len,
    /// double *values)`.
    pub fn GRBgetdblattrarray(
        model: *mut GRBmodel,
        attrname: *const c_char,
        first: c_int,
        len: c_int,
        values: *mut c_double,
    ) -> c_int;

    /// `int GRBsetintparam(GRBenv *env, const char *paramname, int value)`. Called only from
    /// `gurobi::params` (constitution VII-G).
    pub fn GRBsetintparam(env: *mut GRBenv, paramname: *const c_char, value: c_int) -> c_int;

    /// `int GRBsetdblparam(GRBenv *env, const char *paramname, double value)`. Called only
    /// from `gurobi::params` (constitution VII-G).
    pub fn GRBsetdblparam(env: *mut GRBenv, paramname: *const c_char, value: c_double) -> c_int;

    /// `int GRBsetstrparam(GRBenv *env, const char *paramname, const char *value)`. Called
    /// only from `gurobi::params` (constitution VII-G).
    pub fn GRBsetstrparam(
        env: *mut GRBenv,
        paramname: *const c_char,
        value: *const c_char,
    ) -> c_int;

    /// `int GRBgetparamtype(GRBenv *env, const char *paramname)`: parameter type code (see
    /// [`PARAM_TYPE_INT`] and siblings).
    pub fn GRBgetparamtype(env: *mut GRBenv, paramname: *const c_char) -> c_int;

    /// `int GRBgetintparam(GRBenv *env, const char *paramname, int *valueP)`.
    pub fn GRBgetintparam(env: *mut GRBenv, paramname: *const c_char, valueP: *mut c_int) -> c_int;

    /// `int GRBgetdblparam(GRBenv *env, const char *paramname, double *valueP)`.
    pub fn GRBgetdblparam(
        env: *mut GRBenv,
        paramname: *const c_char,
        valueP: *mut c_double,
    ) -> c_int;

    /// `int GRBgetstrparam(GRBenv *env, const char *paramname, char *valueP)`: writes at most
    /// [`GRB_MAX_STRLEN`] bytes including the NUL into `valueP`.
    pub fn GRBgetstrparam(env: *mut GRBenv, paramname: *const c_char, valueP: *mut c_char)
    -> c_int;

    /// `int GRBsetcallbackfunc(GRBmodel *model, int (__stdcall *cb)(CB_ARGS), void
    /// *usrdata)`: installs (or, with `None`, removes) the user callback.
    pub fn GRBsetcallbackfunc(model: *mut GRBmodel, cb: GRBcallback, usrdata: *mut c_void)
    -> c_int;

    /// `int GRBcbget(void *cbdata, int where, int what, void *resultP)`: queries progress
    /// information inside a callback.
    pub fn GRBcbget(cbdata: *mut c_void, where_: c_int, what: c_int, resultP: *mut c_void)
    -> c_int;
}
