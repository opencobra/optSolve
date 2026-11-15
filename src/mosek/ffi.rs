//! Raw declarations of the MOSEK C interface used by optSolver (task T022; feature 002
//! research R1–R9).
//!
//! This is the only MOSEK module with an `extern "C"` block (constitution VII-A). Every
//! declaration is copied from the installed MOSEK 11.2 header `mosek.h` (read 2026-11-15) and
//! checked against it by `tests/header_conformance_mosek.rs` / `.c` (constitution VII-E).
//! **Adding a declaration here requires adding the matching check to
//! `tests/header_conformance_mosek.c`.**
//!
//! Type mapping: `MSKint32t` → `i32`, `MSKint64t` → `i64`, `MSKrealt` → `f64`,
//! `MSKbooleant` → `i32` (`typedef int`), `char` → [`c_char`], `MSKuserhandle_t` →
//! `*mut c_void`. MOSEK's enumeration types (`MSKrescodee`, `MSKsoltypee`, …) are C enums or
//! `typedef int`; they are declared here as `i32`, and the C file asserts that each has the
//! size of `int`. `MSKAPI` is empty on Linux, so the calling convention is the C ABI.
//!
//! Unlike Gurobi's, MOSEK's input arrays are declared `const` throughout, so no input pointer
//! derived from a shared slice needs the constitution VII-A exception.
#![allow(dead_code)] // Complete header-checked table; not every entry is used.
#![allow(non_camel_case_types)] // Vendor type names.

use core::ffi::{c_char, c_void};
use core::marker::{PhantomData, PhantomPinned};

/// Opaque MOSEK environment (`MSKenv`; `MSKenv_t` = `MSKenv *`). Behind a pointer only; the
/// marker makes it `!Send`, `!Sync` and `!Unpin`.
#[repr(C)]
pub struct MSKenv {
    _private: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

/// Opaque MOSEK task (`MSKtask`; `MSKtask_t` = `MSKtask *`).
#[repr(C)]
pub struct MSKtask {
    _private: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

/// `MSKenv_t`.
pub type MSKenv_t = *mut MSKenv;
/// `MSKtask_t`.
pub type MSKtask_t = *mut MSKtask;

/// `MSKcallbackfunc`: `MSKint32t (MSKAPI *)(MSKtask_t task, MSKuserhandle_t usrptr,
/// MSKcallbackcodee caller, const MSKrealt *douinf, const MSKint32t *intinf,
/// const MSKint64t *lintinf)`. `None` is `NULL`.
pub type MSKcallbackfunc = Option<
    unsafe extern "C" fn(
        task: MSKtask_t,
        usrptr: *mut c_void,
        caller: i32,
        douinf: *const f64,
        intinf: *const i32,
        lintinf: *const i64,
    ) -> i32,
>;

/// `MSKstreamfunc`: `void (MSKAPI *)(MSKuserhandle_t handle, const char *str)`.
pub type MSKstreamfunc = Option<unsafe extern "C" fn(handle: *mut c_void, str: *const c_char)>;

// ---------------------------------------------------------------------------------------
// Constants (header values; each is `_Static_assert`ed in tests/header_conformance_mosek.c).
// ---------------------------------------------------------------------------------------

/// `MSK_INFINITY` (1.0e30): the value MOSEK treats as infinite in bounds.
pub const MSK_INFINITY: f64 = 1.0e30;
/// `MSK_MAX_STR_LEN`: buffer length for `MSK_getcodedesc` strings.
pub const MSK_MAX_STR_LEN: usize = 1024;

/// `MSK_RES_OK`.
pub const MSK_RES_OK: i32 = 0;
/// `MSK_RES_ERR_OBJ_Q_NOT_PSD`: the quadratic objective is not convex.
pub const MSK_RES_ERR_OBJ_Q_NOT_PSD: i32 = 1295;
/// `MSK_RES_TRM_MAX_ITERATIONS`.
pub const MSK_RES_TRM_MAX_ITERATIONS: i32 = 100000;
/// `MSK_RES_TRM_MAX_TIME`.
pub const MSK_RES_TRM_MAX_TIME: i32 = 100001;
/// `MSK_RES_TRM_OBJECTIVE_RANGE`.
pub const MSK_RES_TRM_OBJECTIVE_RANGE: i32 = 100002;
/// `MSK_RES_TRM_STALL`.
pub const MSK_RES_TRM_STALL: i32 = 100006;
/// `MSK_RES_TRM_USER_CALLBACK`: a callback returned non-zero.
pub const MSK_RES_TRM_USER_CALLBACK: i32 = 100007;
/// `MSK_RES_TRM_MIO_NUM_RELAXS`.
pub const MSK_RES_TRM_MIO_NUM_RELAXS: i32 = 100008;
/// `MSK_RES_TRM_MIO_NUM_BRANCHES`.
pub const MSK_RES_TRM_MIO_NUM_BRANCHES: i32 = 100009;
/// `MSK_RES_TRM_NUM_MAX_NUM_INT_SOLUTIONS`.
pub const MSK_RES_TRM_NUM_MAX_NUM_INT_SOLUTIONS: i32 = 100015;
/// `MSK_RES_TRM_MAX_NUM_SETBACKS`.
pub const MSK_RES_TRM_MAX_NUM_SETBACKS: i32 = 100020;
/// `MSK_RES_TRM_NUMERICAL_PROBLEM`.
pub const MSK_RES_TRM_NUMERICAL_PROBLEM: i32 = 100025;
/// `MSK_RES_TRM_LOST_RACE`.
pub const MSK_RES_TRM_LOST_RACE: i32 = 100027;
/// `MSK_RES_TRM_INTERNAL`.
pub const MSK_RES_TRM_INTERNAL: i32 = 100030;
/// `MSK_RES_TRM_INTERNAL_STOP`.
pub const MSK_RES_TRM_INTERNAL_STOP: i32 = 100031;
/// `MSK_RES_TRM_SERVER_MAX_TIME`.
pub const MSK_RES_TRM_SERVER_MAX_TIME: i32 = 100032;
/// `MSK_RES_TRM_SERVER_MAX_MEMORY`.
pub const MSK_RES_TRM_SERVER_MAX_MEMORY: i32 = 100033;

/// `MSK_RESPONSE_OK` (response class).
pub const MSK_RESPONSE_OK: i32 = 0;
/// `MSK_RESPONSE_WRN`.
pub const MSK_RESPONSE_WRN: i32 = 1;
/// `MSK_RESPONSE_TRM`.
pub const MSK_RESPONSE_TRM: i32 = 2;
/// `MSK_RESPONSE_ERR`.
pub const MSK_RESPONSE_ERR: i32 = 3;
/// `MSK_RESPONSE_UNK`.
pub const MSK_RESPONSE_UNK: i32 = 4;

/// `MSK_SOL_ITR`: interior-point solution.
pub const MSK_SOL_ITR: i32 = 0;
/// `MSK_SOL_BAS`: basic solution.
pub const MSK_SOL_BAS: i32 = 1;
/// `MSK_SOL_ITG`: integer solution.
pub const MSK_SOL_ITG: i32 = 2;

/// `MSK_SOL_STA_UNKNOWN`.
pub const MSK_SOL_STA_UNKNOWN: i32 = 0;
/// `MSK_SOL_STA_OPTIMAL`.
pub const MSK_SOL_STA_OPTIMAL: i32 = 1;
/// `MSK_SOL_STA_PRIM_FEAS`.
pub const MSK_SOL_STA_PRIM_FEAS: i32 = 2;
/// `MSK_SOL_STA_DUAL_FEAS`.
pub const MSK_SOL_STA_DUAL_FEAS: i32 = 3;
/// `MSK_SOL_STA_PRIM_AND_DUAL_FEAS`.
pub const MSK_SOL_STA_PRIM_AND_DUAL_FEAS: i32 = 4;
/// `MSK_SOL_STA_PRIM_INFEAS_CER`.
pub const MSK_SOL_STA_PRIM_INFEAS_CER: i32 = 5;
/// `MSK_SOL_STA_DUAL_INFEAS_CER`.
pub const MSK_SOL_STA_DUAL_INFEAS_CER: i32 = 6;
/// `MSK_SOL_STA_PRIM_ILLPOSED_CER`.
pub const MSK_SOL_STA_PRIM_ILLPOSED_CER: i32 = 7;
/// `MSK_SOL_STA_DUAL_ILLPOSED_CER`.
pub const MSK_SOL_STA_DUAL_ILLPOSED_CER: i32 = 8;
/// `MSK_SOL_STA_INTEGER_OPTIMAL`.
pub const MSK_SOL_STA_INTEGER_OPTIMAL: i32 = 9;

/// `MSK_PRO_STA_UNKNOWN`.
pub const MSK_PRO_STA_UNKNOWN: i32 = 0;
/// `MSK_PRO_STA_PRIM_AND_DUAL_FEAS`.
pub const MSK_PRO_STA_PRIM_AND_DUAL_FEAS: i32 = 1;
/// `MSK_PRO_STA_PRIM_FEAS`.
pub const MSK_PRO_STA_PRIM_FEAS: i32 = 2;
/// `MSK_PRO_STA_DUAL_FEAS`.
pub const MSK_PRO_STA_DUAL_FEAS: i32 = 3;
/// `MSK_PRO_STA_PRIM_INFEAS`.
pub const MSK_PRO_STA_PRIM_INFEAS: i32 = 4;
/// `MSK_PRO_STA_DUAL_INFEAS`.
pub const MSK_PRO_STA_DUAL_INFEAS: i32 = 5;
/// `MSK_PRO_STA_PRIM_AND_DUAL_INFEAS`.
pub const MSK_PRO_STA_PRIM_AND_DUAL_INFEAS: i32 = 6;
/// `MSK_PRO_STA_ILL_POSED`.
pub const MSK_PRO_STA_ILL_POSED: i32 = 7;
/// `MSK_PRO_STA_PRIM_INFEAS_OR_UNBOUNDED`.
pub const MSK_PRO_STA_PRIM_INFEAS_OR_UNBOUNDED: i32 = 8;

/// `MSK_BK_LO`: lower bound only.
pub const MSK_BK_LO: i32 = 0;
/// `MSK_BK_UP`: upper bound only.
pub const MSK_BK_UP: i32 = 1;
/// `MSK_BK_FX`: fixed.
pub const MSK_BK_FX: i32 = 2;
/// `MSK_BK_FR`: free.
pub const MSK_BK_FR: i32 = 3;
/// `MSK_BK_RA`: ranged.
pub const MSK_BK_RA: i32 = 4;

/// `MSK_SK_UNK`.
pub const MSK_SK_UNK: i32 = 0;
/// `MSK_SK_BAS`.
pub const MSK_SK_BAS: i32 = 1;
/// `MSK_SK_SUPBAS`.
pub const MSK_SK_SUPBAS: i32 = 2;
/// `MSK_SK_LOW`.
pub const MSK_SK_LOW: i32 = 3;
/// `MSK_SK_UPR`.
pub const MSK_SK_UPR: i32 = 4;
/// `MSK_SK_FIX`.
pub const MSK_SK_FIX: i32 = 5;
/// `MSK_SK_INF`.
pub const MSK_SK_INF: i32 = 6;

/// `MSK_OBJECTIVE_SENSE_MINIMIZE`.
pub const MSK_OBJECTIVE_SENSE_MINIMIZE: i32 = 0;
/// `MSK_OBJECTIVE_SENSE_MAXIMIZE`.
pub const MSK_OBJECTIVE_SENSE_MAXIMIZE: i32 = 1;

/// `MSK_STREAM_LOG`.
pub const MSK_STREAM_LOG: i32 = 0;
/// `MSK_FEATURE_PTS`: the base optimiser licence feature.
pub const MSK_FEATURE_PTS: i32 = 0;

/// `MSK_PAR_INVALID_TYPE` (`MSK_whichparam`).
pub const MSK_PAR_INVALID_TYPE: i32 = 0;
/// `MSK_PAR_DOU_TYPE`.
pub const MSK_PAR_DOU_TYPE: i32 = 1;
/// `MSK_PAR_INT_TYPE`.
pub const MSK_PAR_INT_TYPE: i32 = 2;
/// `MSK_PAR_STR_TYPE`.
pub const MSK_PAR_STR_TYPE: i32 = 3;

/// `MSK_OPTIMIZER_CONIC`.
pub const MSK_OPTIMIZER_CONIC: i32 = 0;
/// `MSK_OPTIMIZER_DUAL_SIMPLEX`.
pub const MSK_OPTIMIZER_DUAL_SIMPLEX: i32 = 1;
/// `MSK_OPTIMIZER_FREE`.
pub const MSK_OPTIMIZER_FREE: i32 = 2;
/// `MSK_OPTIMIZER_INTPNT`.
pub const MSK_OPTIMIZER_INTPNT: i32 = 4;
/// `MSK_OPTIMIZER_PRIMAL_SIMPLEX`.
pub const MSK_OPTIMIZER_PRIMAL_SIMPLEX: i32 = 8;

/// `MSK_PRESOLVE_MODE_OFF`.
pub const MSK_PRESOLVE_MODE_OFF: i32 = 0;
/// `MSK_PRESOLVE_MODE_FREE`.
pub const MSK_PRESOLVE_MODE_FREE: i32 = 2;

/// `MSK_IPAR_LOG`.
pub const MSK_IPAR_LOG: i32 = 34;
/// `MSK_IPAR_NUM_THREADS`.
pub const MSK_IPAR_NUM_THREADS: i32 = 100;
/// `MSK_IPAR_OPTIMIZER`.
pub const MSK_IPAR_OPTIMIZER: i32 = 110;
/// `MSK_IPAR_PRESOLVE_USE`.
pub const MSK_IPAR_PRESOLVE_USE: i32 = 121;
/// `MSK_IPAR_SIM_MAX_ITERATIONS`.
pub const MSK_IPAR_SIM_MAX_ITERATIONS: i32 = 147;
/// `MSK_IPAR_INTPNT_BASIS`: basis identification after the interior-point optimiser.
pub const MSK_IPAR_INTPNT_BASIS: i32 = 17;

/// `MSK_DPAR_BASIS_TOL_S`.
pub const MSK_DPAR_BASIS_TOL_S: i32 = 2;
/// `MSK_DPAR_BASIS_TOL_X`.
pub const MSK_DPAR_BASIS_TOL_X: i32 = 3;
/// `MSK_DPAR_INTPNT_CO_TOL_DFEAS`.
pub const MSK_DPAR_INTPNT_CO_TOL_DFEAS: i32 = 16;
/// `MSK_DPAR_INTPNT_CO_TOL_PFEAS`.
pub const MSK_DPAR_INTPNT_CO_TOL_PFEAS: i32 = 20;
/// `MSK_DPAR_INTPNT_QO_TOL_DFEAS`.
pub const MSK_DPAR_INTPNT_QO_TOL_DFEAS: i32 = 22;
/// `MSK_DPAR_INTPNT_QO_TOL_PFEAS`.
pub const MSK_DPAR_INTPNT_QO_TOL_PFEAS: i32 = 26;
/// `MSK_DPAR_INTPNT_TOL_DFEAS`.
pub const MSK_DPAR_INTPNT_TOL_DFEAS: i32 = 28;
/// `MSK_DPAR_INTPNT_TOL_PFEAS`.
pub const MSK_DPAR_INTPNT_TOL_PFEAS: i32 = 33;
/// `MSK_DPAR_OPTIMIZER_MAX_TIME`.
pub const MSK_DPAR_OPTIMIZER_MAX_TIME: i32 = 50;

/// `MSK_CALLBACK_CONIC`.
pub const MSK_CALLBACK_CONIC: i32 = 34;
/// `MSK_CALLBACK_INTPNT`.
pub const MSK_CALLBACK_INTPNT: i32 = 90;
/// `MSK_CALLBACK_UPDATE_DUAL_SIMPLEX`.
pub const MSK_CALLBACK_UPDATE_DUAL_SIMPLEX: i32 = 100;
/// `MSK_CALLBACK_UPDATE_PRIMAL_SIMPLEX`.
pub const MSK_CALLBACK_UPDATE_PRIMAL_SIMPLEX: i32 = 104;
/// `MSK_CALLBACK_UPDATE_SIMPLEX`.
pub const MSK_CALLBACK_UPDATE_SIMPLEX: i32 = 106;

/// `MSK_DINF_INTPNT_DUAL_OBJ`.
pub const MSK_DINF_INTPNT_DUAL_OBJ: i32 = 13;
/// `MSK_DINF_INTPNT_PRIMAL_OBJ`.
pub const MSK_DINF_INTPNT_PRIMAL_OBJ: i32 = 18;
/// `MSK_DINF_OPTIMIZER_TIME`.
pub const MSK_DINF_OPTIMIZER_TIME: i32 = 50;
/// `MSK_DINF_SIM_OBJ`.
pub const MSK_DINF_SIM_OBJ: i32 = 64;
/// `MSK_IINF_INTPNT_ITER`.
pub const MSK_IINF_INTPNT_ITER: i32 = 17;
/// `MSK_IINF_SIM_DUAL_ITER`.
pub const MSK_IINF_SIM_DUAL_ITER: i32 = 121;
/// `MSK_IINF_SIM_PRIMAL_ITER`.
pub const MSK_IINF_SIM_PRIMAL_ITER: i32 = 128;

// ---------------------------------------------------------------------------------------
// Functions. Every one returns `MSKrescodee` (0 = MSK_RES_OK).
// ---------------------------------------------------------------------------------------

unsafe extern "C" {
    /// `MSK_makeenv(MSKenv_t *env, const char *dbgfile)`.
    pub fn MSK_makeenv(env: *mut MSKenv_t, dbgfile: *const c_char) -> i32;
    /// `MSK_deleteenv(MSKenv_t *env)`.
    pub fn MSK_deleteenv(env: *mut MSKenv_t) -> i32;
    /// `MSK_checkoutlicense(MSKenv_t env, MSKfeaturee feature)`.
    pub fn MSK_checkoutlicense(env: MSKenv_t, feature: i32) -> i32;
    /// `MSK_getversion(MSKint32t *major, MSKint32t *minor, MSKint32t *revision)`.
    pub fn MSK_getversion(major: *mut i32, minor: *mut i32, revision: *mut i32) -> i32;
    /// `MSK_getcodedesc(MSKrescodee code, char *symname, char *str)`: both buffers
    /// `MSK_MAX_STR_LEN` bytes.
    pub fn MSK_getcodedesc(code: i32, symname: *mut c_char, str: *mut c_char) -> i32;
    /// `MSK_getresponseclass(MSKrescodee r, MSKrescodetypee *rc)`.
    pub fn MSK_getresponseclass(r: i32, rc: *mut i32) -> i32;

    /// `MSK_makeemptytask(MSKenv_t env, MSKtask_t *task)`.
    pub fn MSK_makeemptytask(env: MSKenv_t, task: *mut MSKtask_t) -> i32;
    /// `MSK_deletetask(MSKtask_t *task)`.
    pub fn MSK_deletetask(task: *mut MSKtask_t) -> i32;
    /// `MSK_getlasterror64(MSKtask_t task, MSKrescodee *lastrescode, MSKint64t sizelastmsg,
    /// MSKint64t *lastmsglen, char *lastmsg)`.
    pub fn MSK_getlasterror64(
        task: MSKtask_t,
        lastrescode: *mut i32,
        sizelastmsg: i64,
        lastmsglen: *mut i64,
        lastmsg: *mut c_char,
    ) -> i32;
    /// `MSK_linkfunctotaskstream(MSKtask_t task, MSKstreamtypee whichstream,
    /// MSKuserhandle_t handle, MSKstreamfunc func)`.
    pub fn MSK_linkfunctotaskstream(
        task: MSKtask_t,
        whichstream: i32,
        handle: *mut c_void,
        func: MSKstreamfunc,
    ) -> i32;
    /// `MSK_putcallbackfunc(MSKtask_t task, MSKcallbackfunc func, MSKuserhandle_t handle)`.
    pub fn MSK_putcallbackfunc(task: MSKtask_t, func: MSKcallbackfunc, handle: *mut c_void) -> i32;

    // Loading.
    /// `MSK_putmaxnumvar(MSKtask_t task, MSKint32t maxnumvar)`.
    pub fn MSK_putmaxnumvar(task: MSKtask_t, maxnumvar: i32) -> i32;
    /// `MSK_putmaxnumcon(MSKtask_t task, MSKint32t maxnumcon)`.
    pub fn MSK_putmaxnumcon(task: MSKtask_t, maxnumcon: i32) -> i32;
    /// `MSK_putmaxnumanz(MSKtask_t task, MSKint64t maxnumanz)`.
    pub fn MSK_putmaxnumanz(task: MSKtask_t, maxnumanz: i64) -> i32;
    /// `MSK_putmaxnumafe(MSKtask_t task, MSKint64t maxnumafe)`.
    pub fn MSK_putmaxnumafe(task: MSKtask_t, maxnumafe: i64) -> i32;
    /// `MSK_appendvars(MSKtask_t task, MSKint32t num)`.
    pub fn MSK_appendvars(task: MSKtask_t, num: i32) -> i32;
    /// `MSK_appendcons(MSKtask_t task, MSKint32t num)`.
    pub fn MSK_appendcons(task: MSKtask_t, num: i32) -> i32;
    /// `MSK_putacolslice64(MSKtask_t task, MSKint32t first, MSKint32t last,
    /// const MSKint64t *ptrb, const MSKint64t *ptre, const MSKint32t *asub,
    /// const MSKrealt *aval)`.
    pub fn MSK_putacolslice64(
        task: MSKtask_t,
        first: i32,
        last: i32,
        ptrb: *const i64,
        ptre: *const i64,
        asub: *const i32,
        aval: *const f64,
    ) -> i32;
    /// `MSK_putcslice(MSKtask_t task, MSKint32t first, MSKint32t last,
    /// const MSKrealt *slice)`.
    pub fn MSK_putcslice(task: MSKtask_t, first: i32, last: i32, slice: *const f64) -> i32;
    /// `MSK_putcfix(MSKtask_t task, MSKrealt cfix)`.
    pub fn MSK_putcfix(task: MSKtask_t, cfix: f64) -> i32;
    /// `MSK_putobjsense(MSKtask_t task, MSKobjsensee sense)`.
    pub fn MSK_putobjsense(task: MSKtask_t, sense: i32) -> i32;
    /// `MSK_putvarboundslice(MSKtask_t task, MSKint32t first, MSKint32t last,
    /// const MSKboundkeye *bkx, const MSKrealt *blx, const MSKrealt *bux)`.
    pub fn MSK_putvarboundslice(
        task: MSKtask_t,
        first: i32,
        last: i32,
        bkx: *const i32,
        blx: *const f64,
        bux: *const f64,
    ) -> i32;
    /// `MSK_putconboundslice(MSKtask_t task, MSKint32t first, MSKint32t last,
    /// const MSKboundkeye *bkc, const MSKrealt *blc, const MSKrealt *buc)`.
    pub fn MSK_putconboundslice(
        task: MSKtask_t,
        first: i32,
        last: i32,
        bkc: *const i32,
        blc: *const f64,
        buc: *const f64,
    ) -> i32;
    /// `MSK_putqobj(MSKtask_t task, MSKint32t numqonz, const MSKint32t *qosubi,
    /// const MSKint32t *qosubj, const MSKrealt *qoval)`.
    pub fn MSK_putqobj(
        task: MSKtask_t,
        numqonz: i32,
        qosubi: *const i32,
        qosubj: *const i32,
        qoval: *const f64,
    ) -> i32;
    /// `MSK_appendafes(MSKtask_t task, MSKint64t num)`.
    pub fn MSK_appendafes(task: MSKtask_t, num: i64) -> i32;
    /// `MSK_putafefentrylist(MSKtask_t task, MSKint64t numentr, const MSKint64t *afeidx,
    /// const MSKint32t *varidx, const MSKrealt *val)`.
    pub fn MSK_putafefentrylist(
        task: MSKtask_t,
        numentr: i64,
        afeidx: *const i64,
        varidx: *const i32,
        val: *const f64,
    ) -> i32;
    /// `MSK_putafegslice(MSKtask_t task, MSKint64t first, MSKint64t last,
    /// const MSKrealt *slice)`.
    pub fn MSK_putafegslice(task: MSKtask_t, first: i64, last: i64, slice: *const f64) -> i32;
    /// `MSK_appendquadraticconedomain(MSKtask_t task, MSKint64t n, MSKint64t *domidx)`.
    pub fn MSK_appendquadraticconedomain(task: MSKtask_t, n: i64, domidx: *mut i64) -> i32;
    /// `MSK_appendrquadraticconedomain(MSKtask_t task, MSKint64t n, MSKint64t *domidx)`.
    pub fn MSK_appendrquadraticconedomain(task: MSKtask_t, n: i64, domidx: *mut i64) -> i32;
    /// `MSK_appendprimalexpconedomain(MSKtask_t task, MSKint64t *domidx)`.
    pub fn MSK_appendprimalexpconedomain(task: MSKtask_t, domidx: *mut i64) -> i32;
    /// `MSK_appenddualexpconedomain(MSKtask_t task, MSKint64t *domidx)`.
    pub fn MSK_appenddualexpconedomain(task: MSKtask_t, domidx: *mut i64) -> i32;
    /// `MSK_appendprimalpowerconedomain(MSKtask_t task, MSKint64t n, MSKint64t nleft,
    /// const MSKrealt *alpha, MSKint64t *domidx)`.
    pub fn MSK_appendprimalpowerconedomain(
        task: MSKtask_t,
        n: i64,
        nleft: i64,
        alpha: *const f64,
        domidx: *mut i64,
    ) -> i32;
    /// `MSK_appenddualpowerconedomain(MSKtask_t task, MSKint64t n, MSKint64t nleft,
    /// const MSKrealt *alpha, MSKint64t *domidx)`.
    pub fn MSK_appenddualpowerconedomain(
        task: MSKtask_t,
        n: i64,
        nleft: i64,
        alpha: *const f64,
        domidx: *mut i64,
    ) -> i32;
    /// `MSK_appendrplusdomain(MSKtask_t task, MSKint64t n, MSKint64t *domidx)`.
    pub fn MSK_appendrplusdomain(task: MSKtask_t, n: i64, domidx: *mut i64) -> i32;
    /// `MSK_appendrzerodomain(MSKtask_t task, MSKint64t n, MSKint64t *domidx)`.
    pub fn MSK_appendrzerodomain(task: MSKtask_t, n: i64, domidx: *mut i64) -> i32;
    /// `MSK_appendsvecpsdconedomain(MSKtask_t task, MSKint64t n, MSKint64t *domidx)`.
    pub fn MSK_appendsvecpsdconedomain(task: MSKtask_t, n: i64, domidx: *mut i64) -> i32;
    /// `MSK_appendaccsseq(MSKtask_t task, MSKint64t numaccs, const MSKint64t *domidxs,
    /// MSKint64t numafeidx, MSKint64t afeidxfirst, const MSKrealt *b)`.
    pub fn MSK_appendaccsseq(
        task: MSKtask_t,
        numaccs: i64,
        domidxs: *const i64,
        numafeidx: i64,
        afeidxfirst: i64,
        b: *const f64,
    ) -> i32;

    // Parameters (set only in `mosek::params`, constitution VII-G).
    /// `MSK_putintparam(MSKtask_t task, MSKiparame param, MSKint32t parvalue)`.
    pub fn MSK_putintparam(task: MSKtask_t, param: i32, parvalue: i32) -> i32;
    /// `MSK_putdouparam(MSKtask_t task, MSKdparame param, MSKrealt parvalue)`.
    pub fn MSK_putdouparam(task: MSKtask_t, param: i32, parvalue: f64) -> i32;
    /// `MSK_putnaintparam(MSKtask_t task, const char *paramname, MSKint32t parvalue)`.
    pub fn MSK_putnaintparam(task: MSKtask_t, paramname: *const c_char, parvalue: i32) -> i32;
    /// `MSK_putnadouparam(MSKtask_t task, const char *paramname, MSKrealt parvalue)`.
    pub fn MSK_putnadouparam(task: MSKtask_t, paramname: *const c_char, parvalue: f64) -> i32;
    /// `MSK_putnastrparam(MSKtask_t task, const char *paramname, const char *parvalue)`.
    pub fn MSK_putnastrparam(
        task: MSKtask_t,
        paramname: *const c_char,
        parvalue: *const c_char,
    ) -> i32;
    /// `MSK_getintparam(MSKtask_t task, MSKiparame param, MSKint32t *parvalue)`.
    pub fn MSK_getintparam(task: MSKtask_t, param: i32, parvalue: *mut i32) -> i32;
    /// `MSK_getdouparam(MSKtask_t task, MSKdparame param, MSKrealt *parvalue)`.
    pub fn MSK_getdouparam(task: MSKtask_t, param: i32, parvalue: *mut f64) -> i32;
    /// `MSK_whichparam(MSKtask_t task, const char *parname, MSKparametertypee *partype,
    /// MSKint32t *param)`.
    pub fn MSK_whichparam(
        task: MSKtask_t,
        parname: *const c_char,
        partype: *mut i32,
        param: *mut i32,
    ) -> i32;

    // Optimisation and solutions.
    /// `MSK_optimizetrm(MSKtask_t task, MSKrescodee *trmcode)`.
    pub fn MSK_optimizetrm(task: MSKtask_t, trmcode: *mut i32) -> i32;
    /// `MSK_solutiondef(MSKtask_t task, MSKsoltypee whichsol, MSKbooleant *isdef)`.
    pub fn MSK_solutiondef(task: MSKtask_t, whichsol: i32, isdef: *mut i32) -> i32;
    /// `MSK_getsolsta(MSKtask_t task, MSKsoltypee whichsol, MSKsolstae *solutionsta)`.
    pub fn MSK_getsolsta(task: MSKtask_t, whichsol: i32, solutionsta: *mut i32) -> i32;
    /// `MSK_getprosta(MSKtask_t task, MSKsoltypee whichsol, MSKprostae *problemsta)`.
    pub fn MSK_getprosta(task: MSKtask_t, whichsol: i32, problemsta: *mut i32) -> i32;
    /// `MSK_getxx(MSKtask_t task, MSKsoltypee whichsol, MSKrealt *xx)`: n values.
    pub fn MSK_getxx(task: MSKtask_t, whichsol: i32, xx: *mut f64) -> i32;
    /// `MSK_getxc(MSKtask_t task, MSKsoltypee whichsol, MSKrealt *xc)`: m values.
    pub fn MSK_getxc(task: MSKtask_t, whichsol: i32, xc: *mut f64) -> i32;
    /// `MSK_gety(MSKtask_t task, MSKsoltypee whichsol, MSKrealt *y)`: m values.
    pub fn MSK_gety(task: MSKtask_t, whichsol: i32, y: *mut f64) -> i32;
    /// `MSK_getslx(MSKtask_t task, MSKsoltypee whichsol, MSKrealt *slx)`: n values.
    pub fn MSK_getslx(task: MSKtask_t, whichsol: i32, slx: *mut f64) -> i32;
    /// `MSK_getsux(MSKtask_t task, MSKsoltypee whichsol, MSKrealt *sux)`: n values.
    pub fn MSK_getsux(task: MSKtask_t, whichsol: i32, sux: *mut f64) -> i32;
    /// `MSK_getaccdotys(MSKtask_t task, MSKsoltypee whichsol, MSKrealt *doty)`:
    /// `MSK_getaccntot` values.
    pub fn MSK_getaccdotys(task: MSKtask_t, whichsol: i32, doty: *mut f64) -> i32;
    /// `MSK_getaccntot(MSKtask_t task, MSKint64t *n)`.
    pub fn MSK_getaccntot(task: MSKtask_t, n: *mut i64) -> i32;
    /// `MSK_getskx(MSKtask_t task, MSKsoltypee whichsol, MSKstakeye *skx)`: n values.
    pub fn MSK_getskx(task: MSKtask_t, whichsol: i32, skx: *mut i32) -> i32;
    /// `MSK_getskc(MSKtask_t task, MSKsoltypee whichsol, MSKstakeye *skc)`: m values.
    pub fn MSK_getskc(task: MSKtask_t, whichsol: i32, skc: *mut i32) -> i32;
    /// `MSK_getprimalobj(MSKtask_t task, MSKsoltypee whichsol, MSKrealt *primalobj)`.
    pub fn MSK_getprimalobj(task: MSKtask_t, whichsol: i32, primalobj: *mut f64) -> i32;
    /// `MSK_getdualobj(MSKtask_t task, MSKsoltypee whichsol, MSKrealt *dualobj)`.
    pub fn MSK_getdualobj(task: MSKtask_t, whichsol: i32, dualobj: *mut f64) -> i32;
    /// `MSK_getdouinf(MSKtask_t task, MSKdinfiteme whichdinf, MSKrealt *dvalue)`.
    pub fn MSK_getdouinf(task: MSKtask_t, whichdinf: i32, dvalue: *mut f64) -> i32;
    /// `MSK_getintinf(MSKtask_t task, MSKiinfiteme whichiinf, MSKint32t *ivalue)`.
    pub fn MSK_getintinf(task: MSKtask_t, whichiinf: i32, ivalue: *mut i32) -> i32;
}
