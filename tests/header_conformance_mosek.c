/*
 * Header conformance check for src/mosek/ffi.rs (feature 002, task T018; constitution VII-E).
 *
 * Compiled (never linked or run) against the installed mosek.h by
 * tests/header_conformance_mosek.rs. Every function declared in ffi.rs is assigned to a
 * pointer whose type is spelled here with the header's own parameter types; a function that
 * does not exist, or whose signature differs from what is spelled here, is a compile error
 * (-Werror=incompatible-pointer-types). The Rust test then checks, parameter by parameter,
 * that the Rust declaration in ffi.rs has the width and pointer shape of the type spelled
 * here, and the _Static_asserts below check that every MOSEK enumeration type used has the
 * size of a 32-bit int (ffi.rs declares them as i32) and every constant ffi.rs defines.
 *
 * KEEP IN SYNC: adding or changing a declaration or constant in src/mosek/ffi.rs requires
 * adding or changing the matching line here.
 */
#include <stdint.h>
#include "mosek.h"

/* Enumeration types ffi.rs declares as i32. */
_Static_assert(sizeof(MSKrescodee) == 4, "MSKrescodee");
_Static_assert(sizeof(MSKrescodetypee) == 4, "MSKrescodetypee");
_Static_assert(sizeof(MSKfeaturee) == 4, "MSKfeaturee");
_Static_assert(sizeof(MSKstreamtypee) == 4, "MSKstreamtypee");
_Static_assert(sizeof(MSKcallbackcodee) == 4, "MSKcallbackcodee");
_Static_assert(sizeof(MSKobjsensee) == 4, "MSKobjsensee");
_Static_assert(sizeof(MSKboundkeye) == 4, "MSKboundkeye");
_Static_assert(sizeof(MSKiparame) == 4, "MSKiparame");
_Static_assert(sizeof(MSKdparame) == 4, "MSKdparame");
_Static_assert(sizeof(MSKparametertypee) == 4, "MSKparametertypee");
_Static_assert(sizeof(MSKsoltypee) == 4, "MSKsoltypee");
_Static_assert(sizeof(MSKsolstae) == 4, "MSKsolstae");
_Static_assert(sizeof(MSKprostae) == 4, "MSKprostae");
_Static_assert(sizeof(MSKstakeye) == 4, "MSKstakeye");
_Static_assert(sizeof(MSKdinfiteme) == 4, "MSKdinfiteme");
_Static_assert(sizeof(MSKiinfiteme) == 4, "MSKiinfiteme");
_Static_assert(sizeof(MSKbooleant) == 4, "MSKbooleant");
_Static_assert(sizeof(MSKint32t) == 4 && sizeof(MSKint64t) == 8, "integer widths");
_Static_assert(sizeof(MSKrealt) == 8, "MSKrealt");

/* Callback and stream types as declared in ffi.rs. */
typedef MSKint32t (MSKAPI *ffi_callbackfunc)(MSKtask_t, MSKuserhandle_t, MSKcallbackcodee,
                                             const MSKrealt *, const MSKint32t *,
                                             const MSKint64t *);
typedef void (MSKAPI *ffi_streamfunc)(MSKuserhandle_t, const char *);
_Static_assert(_Generic((MSKcallbackfunc)0, ffi_callbackfunc: 1, default: 0), "MSKcallbackfunc");
_Static_assert(_Generic((MSKstreamfunc)0, ffi_streamfunc: 1, default: 0), "MSKstreamfunc");

/* Functions: one typed pointer per declaration in ffi.rs. */
MSKrescodee (MSKAPI *const p_MSK_makeenv)(MSKenv_t *, const char *) = MSK_makeenv;
MSKrescodee (MSKAPI *const p_MSK_deleteenv)(MSKenv_t *) = MSK_deleteenv;
MSKrescodee (MSKAPI *const p_MSK_checkoutlicense)(MSKenv_t, MSKfeaturee) = MSK_checkoutlicense;
MSKrescodee (MSKAPI *const p_MSK_getversion)(MSKint32t *, MSKint32t *, MSKint32t *) = MSK_getversion;
MSKrescodee (MSKAPI *const p_MSK_getcodedesc)(MSKrescodee, char *, char *) = MSK_getcodedesc;
MSKrescodee (MSKAPI *const p_MSK_getresponseclass)(MSKrescodee, MSKrescodetypee *) = MSK_getresponseclass;
MSKrescodee (MSKAPI *const p_MSK_makeemptytask)(MSKenv_t, MSKtask_t *) = MSK_makeemptytask;
MSKrescodee (MSKAPI *const p_MSK_deletetask)(MSKtask_t *) = MSK_deletetask;
MSKrescodee (MSKAPI *const p_MSK_getlasterror64)(MSKtask_t, MSKrescodee *, MSKint64t, MSKint64t *, char *) = MSK_getlasterror64;
MSKrescodee (MSKAPI *const p_MSK_linkfunctotaskstream)(MSKtask_t, MSKstreamtypee, MSKuserhandle_t, MSKstreamfunc) = MSK_linkfunctotaskstream;
MSKrescodee (MSKAPI *const p_MSK_putcallbackfunc)(MSKtask_t, MSKcallbackfunc, MSKuserhandle_t) = MSK_putcallbackfunc;
MSKrescodee (MSKAPI *const p_MSK_putmaxnumvar)(MSKtask_t, MSKint32t) = MSK_putmaxnumvar;
MSKrescodee (MSKAPI *const p_MSK_putmaxnumcon)(MSKtask_t, MSKint32t) = MSK_putmaxnumcon;
MSKrescodee (MSKAPI *const p_MSK_putmaxnumanz)(MSKtask_t, MSKint64t) = MSK_putmaxnumanz;
MSKrescodee (MSKAPI *const p_MSK_putmaxnumafe)(MSKtask_t, MSKint64t) = MSK_putmaxnumafe;
MSKrescodee (MSKAPI *const p_MSK_appendvars)(MSKtask_t, MSKint32t) = MSK_appendvars;
MSKrescodee (MSKAPI *const p_MSK_appendcons)(MSKtask_t, MSKint32t) = MSK_appendcons;
MSKrescodee (MSKAPI *const p_MSK_putacolslice64)(MSKtask_t, MSKint32t, MSKint32t, const MSKint64t *, const MSKint64t *, const MSKint32t *, const MSKrealt *) = MSK_putacolslice64;
MSKrescodee (MSKAPI *const p_MSK_putcslice)(MSKtask_t, MSKint32t, MSKint32t, const MSKrealt *) = MSK_putcslice;
MSKrescodee (MSKAPI *const p_MSK_putcfix)(MSKtask_t, MSKrealt) = MSK_putcfix;
MSKrescodee (MSKAPI *const p_MSK_putobjsense)(MSKtask_t, MSKobjsensee) = MSK_putobjsense;
MSKrescodee (MSKAPI *const p_MSK_putvarboundslice)(MSKtask_t, MSKint32t, MSKint32t, const MSKboundkeye *, const MSKrealt *, const MSKrealt *) = MSK_putvarboundslice;
MSKrescodee (MSKAPI *const p_MSK_putconboundslice)(MSKtask_t, MSKint32t, MSKint32t, const MSKboundkeye *, const MSKrealt *, const MSKrealt *) = MSK_putconboundslice;
MSKrescodee (MSKAPI *const p_MSK_putqobj)(MSKtask_t, MSKint32t, const MSKint32t *, const MSKint32t *, const MSKrealt *) = MSK_putqobj;
MSKrescodee (MSKAPI *const p_MSK_appendafes)(MSKtask_t, MSKint64t) = MSK_appendafes;
MSKrescodee (MSKAPI *const p_MSK_putafefentrylist)(MSKtask_t, MSKint64t, const MSKint64t *, const MSKint32t *, const MSKrealt *) = MSK_putafefentrylist;
MSKrescodee (MSKAPI *const p_MSK_putafegslice)(MSKtask_t, MSKint64t, MSKint64t, const MSKrealt *) = MSK_putafegslice;
MSKrescodee (MSKAPI *const p_MSK_appendquadraticconedomain)(MSKtask_t, MSKint64t, MSKint64t *) = MSK_appendquadraticconedomain;
MSKrescodee (MSKAPI *const p_MSK_appendrquadraticconedomain)(MSKtask_t, MSKint64t, MSKint64t *) = MSK_appendrquadraticconedomain;
MSKrescodee (MSKAPI *const p_MSK_appendprimalexpconedomain)(MSKtask_t, MSKint64t *) = MSK_appendprimalexpconedomain;
MSKrescodee (MSKAPI *const p_MSK_appenddualexpconedomain)(MSKtask_t, MSKint64t *) = MSK_appenddualexpconedomain;
MSKrescodee (MSKAPI *const p_MSK_appendprimalpowerconedomain)(MSKtask_t, MSKint64t, MSKint64t, const MSKrealt *, MSKint64t *) = MSK_appendprimalpowerconedomain;
MSKrescodee (MSKAPI *const p_MSK_appenddualpowerconedomain)(MSKtask_t, MSKint64t, MSKint64t, const MSKrealt *, MSKint64t *) = MSK_appenddualpowerconedomain;
MSKrescodee (MSKAPI *const p_MSK_appendrplusdomain)(MSKtask_t, MSKint64t, MSKint64t *) = MSK_appendrplusdomain;
MSKrescodee (MSKAPI *const p_MSK_appendrzerodomain)(MSKtask_t, MSKint64t, MSKint64t *) = MSK_appendrzerodomain;
MSKrescodee (MSKAPI *const p_MSK_appendsvecpsdconedomain)(MSKtask_t, MSKint64t, MSKint64t *) = MSK_appendsvecpsdconedomain;
MSKrescodee (MSKAPI *const p_MSK_appendaccsseq)(MSKtask_t, MSKint64t, const MSKint64t *, MSKint64t, MSKint64t, const MSKrealt *) = MSK_appendaccsseq;
MSKrescodee (MSKAPI *const p_MSK_putintparam)(MSKtask_t, MSKiparame, MSKint32t) = MSK_putintparam;
MSKrescodee (MSKAPI *const p_MSK_putdouparam)(MSKtask_t, MSKdparame, MSKrealt) = MSK_putdouparam;
MSKrescodee (MSKAPI *const p_MSK_putnaintparam)(MSKtask_t, const char *, MSKint32t) = MSK_putnaintparam;
MSKrescodee (MSKAPI *const p_MSK_putnadouparam)(MSKtask_t, const char *, MSKrealt) = MSK_putnadouparam;
MSKrescodee (MSKAPI *const p_MSK_putnastrparam)(MSKtask_t, const char *, const char *) = MSK_putnastrparam;
MSKrescodee (MSKAPI *const p_MSK_getintparam)(MSKtask_t, MSKiparame, MSKint32t *) = MSK_getintparam;
MSKrescodee (MSKAPI *const p_MSK_getdouparam)(MSKtask_t, MSKdparame, MSKrealt *) = MSK_getdouparam;
MSKrescodee (MSKAPI *const p_MSK_whichparam)(MSKtask_t, const char *, MSKparametertypee *, MSKint32t *) = MSK_whichparam;
MSKrescodee (MSKAPI *const p_MSK_optimizetrm)(MSKtask_t, MSKrescodee *) = MSK_optimizetrm;
MSKrescodee (MSKAPI *const p_MSK_solutiondef)(MSKtask_t, MSKsoltypee, MSKbooleant *) = MSK_solutiondef;
MSKrescodee (MSKAPI *const p_MSK_getsolsta)(MSKtask_t, MSKsoltypee, MSKsolstae *) = MSK_getsolsta;
MSKrescodee (MSKAPI *const p_MSK_getprosta)(MSKtask_t, MSKsoltypee, MSKprostae *) = MSK_getprosta;
MSKrescodee (MSKAPI *const p_MSK_getxx)(MSKtask_t, MSKsoltypee, MSKrealt *) = MSK_getxx;
MSKrescodee (MSKAPI *const p_MSK_getxc)(MSKtask_t, MSKsoltypee, MSKrealt *) = MSK_getxc;
MSKrescodee (MSKAPI *const p_MSK_gety)(MSKtask_t, MSKsoltypee, MSKrealt *) = MSK_gety;
MSKrescodee (MSKAPI *const p_MSK_getslx)(MSKtask_t, MSKsoltypee, MSKrealt *) = MSK_getslx;
MSKrescodee (MSKAPI *const p_MSK_getsux)(MSKtask_t, MSKsoltypee, MSKrealt *) = MSK_getsux;
MSKrescodee (MSKAPI *const p_MSK_getaccdotys)(MSKtask_t, MSKsoltypee, MSKrealt *) = MSK_getaccdotys;
MSKrescodee (MSKAPI *const p_MSK_getaccntot)(MSKtask_t, MSKint64t *) = MSK_getaccntot;
MSKrescodee (MSKAPI *const p_MSK_getskx)(MSKtask_t, MSKsoltypee, MSKstakeye *) = MSK_getskx;
MSKrescodee (MSKAPI *const p_MSK_getskc)(MSKtask_t, MSKsoltypee, MSKstakeye *) = MSK_getskc;
MSKrescodee (MSKAPI *const p_MSK_getprimalobj)(MSKtask_t, MSKsoltypee, MSKrealt *) = MSK_getprimalobj;
MSKrescodee (MSKAPI *const p_MSK_getdualobj)(MSKtask_t, MSKsoltypee, MSKrealt *) = MSK_getdualobj;
MSKrescodee (MSKAPI *const p_MSK_getdouinf)(MSKtask_t, MSKdinfiteme, MSKrealt *) = MSK_getdouinf;
MSKrescodee (MSKAPI *const p_MSK_getintinf)(MSKtask_t, MSKiinfiteme, MSKint32t *) = MSK_getintinf;

/* Constants. */
_Static_assert(MSK_INFINITY == 1.0e30, "MSK_INFINITY");
_Static_assert(MSK_MAX_STR_LEN == 1024, "MSK_MAX_STR_LEN");
_Static_assert(MSK_RES_OK == 0, "MSK_RES_OK");
_Static_assert(MSK_RES_ERR_OBJ_Q_NOT_PSD == 1295, "MSK_RES_ERR_OBJ_Q_NOT_PSD");
_Static_assert(MSK_RES_TRM_MAX_ITERATIONS == 100000, "MSK_RES_TRM_MAX_ITERATIONS");
_Static_assert(MSK_RES_TRM_MAX_TIME == 100001, "MSK_RES_TRM_MAX_TIME");
_Static_assert(MSK_RES_TRM_OBJECTIVE_RANGE == 100002, "MSK_RES_TRM_OBJECTIVE_RANGE");
_Static_assert(MSK_RES_TRM_STALL == 100006, "MSK_RES_TRM_STALL");
_Static_assert(MSK_RES_TRM_USER_CALLBACK == 100007, "MSK_RES_TRM_USER_CALLBACK");
_Static_assert(MSK_RES_TRM_MIO_NUM_RELAXS == 100008, "MSK_RES_TRM_MIO_NUM_RELAXS");
_Static_assert(MSK_RES_TRM_MIO_NUM_BRANCHES == 100009, "MSK_RES_TRM_MIO_NUM_BRANCHES");
_Static_assert(MSK_RES_TRM_NUM_MAX_NUM_INT_SOLUTIONS == 100015, "MSK_RES_TRM_NUM_MAX_NUM_INT_SOLUTIONS");
_Static_assert(MSK_RES_TRM_MAX_NUM_SETBACKS == 100020, "MSK_RES_TRM_MAX_NUM_SETBACKS");
_Static_assert(MSK_RES_TRM_NUMERICAL_PROBLEM == 100025, "MSK_RES_TRM_NUMERICAL_PROBLEM");
_Static_assert(MSK_RES_TRM_LOST_RACE == 100027, "MSK_RES_TRM_LOST_RACE");
_Static_assert(MSK_RES_TRM_INTERNAL == 100030, "MSK_RES_TRM_INTERNAL");
_Static_assert(MSK_RES_TRM_INTERNAL_STOP == 100031, "MSK_RES_TRM_INTERNAL_STOP");
_Static_assert(MSK_RES_TRM_SERVER_MAX_TIME == 100032, "MSK_RES_TRM_SERVER_MAX_TIME");
_Static_assert(MSK_RES_TRM_SERVER_MAX_MEMORY == 100033, "MSK_RES_TRM_SERVER_MAX_MEMORY");
_Static_assert(MSK_RESPONSE_OK == 0, "MSK_RESPONSE_OK");
_Static_assert(MSK_RESPONSE_WRN == 1, "MSK_RESPONSE_WRN");
_Static_assert(MSK_RESPONSE_TRM == 2, "MSK_RESPONSE_TRM");
_Static_assert(MSK_RESPONSE_ERR == 3, "MSK_RESPONSE_ERR");
_Static_assert(MSK_RESPONSE_UNK == 4, "MSK_RESPONSE_UNK");
_Static_assert(MSK_SOL_ITR == 0, "MSK_SOL_ITR");
_Static_assert(MSK_SOL_BAS == 1, "MSK_SOL_BAS");
_Static_assert(MSK_SOL_ITG == 2, "MSK_SOL_ITG");
_Static_assert(MSK_SOL_STA_UNKNOWN == 0, "MSK_SOL_STA_UNKNOWN");
_Static_assert(MSK_SOL_STA_OPTIMAL == 1, "MSK_SOL_STA_OPTIMAL");
_Static_assert(MSK_SOL_STA_PRIM_FEAS == 2, "MSK_SOL_STA_PRIM_FEAS");
_Static_assert(MSK_SOL_STA_DUAL_FEAS == 3, "MSK_SOL_STA_DUAL_FEAS");
_Static_assert(MSK_SOL_STA_PRIM_AND_DUAL_FEAS == 4, "MSK_SOL_STA_PRIM_AND_DUAL_FEAS");
_Static_assert(MSK_SOL_STA_PRIM_INFEAS_CER == 5, "MSK_SOL_STA_PRIM_INFEAS_CER");
_Static_assert(MSK_SOL_STA_DUAL_INFEAS_CER == 6, "MSK_SOL_STA_DUAL_INFEAS_CER");
_Static_assert(MSK_SOL_STA_PRIM_ILLPOSED_CER == 7, "MSK_SOL_STA_PRIM_ILLPOSED_CER");
_Static_assert(MSK_SOL_STA_DUAL_ILLPOSED_CER == 8, "MSK_SOL_STA_DUAL_ILLPOSED_CER");
_Static_assert(MSK_SOL_STA_INTEGER_OPTIMAL == 9, "MSK_SOL_STA_INTEGER_OPTIMAL");
_Static_assert(MSK_PRO_STA_UNKNOWN == 0, "MSK_PRO_STA_UNKNOWN");
_Static_assert(MSK_PRO_STA_PRIM_AND_DUAL_FEAS == 1, "MSK_PRO_STA_PRIM_AND_DUAL_FEAS");
_Static_assert(MSK_PRO_STA_PRIM_FEAS == 2, "MSK_PRO_STA_PRIM_FEAS");
_Static_assert(MSK_PRO_STA_DUAL_FEAS == 3, "MSK_PRO_STA_DUAL_FEAS");
_Static_assert(MSK_PRO_STA_PRIM_INFEAS == 4, "MSK_PRO_STA_PRIM_INFEAS");
_Static_assert(MSK_PRO_STA_DUAL_INFEAS == 5, "MSK_PRO_STA_DUAL_INFEAS");
_Static_assert(MSK_PRO_STA_PRIM_AND_DUAL_INFEAS == 6, "MSK_PRO_STA_PRIM_AND_DUAL_INFEAS");
_Static_assert(MSK_PRO_STA_ILL_POSED == 7, "MSK_PRO_STA_ILL_POSED");
_Static_assert(MSK_PRO_STA_PRIM_INFEAS_OR_UNBOUNDED == 8, "MSK_PRO_STA_PRIM_INFEAS_OR_UNBOUNDED");
_Static_assert(MSK_BK_LO == 0, "MSK_BK_LO");
_Static_assert(MSK_BK_UP == 1, "MSK_BK_UP");
_Static_assert(MSK_BK_FX == 2, "MSK_BK_FX");
_Static_assert(MSK_BK_FR == 3, "MSK_BK_FR");
_Static_assert(MSK_BK_RA == 4, "MSK_BK_RA");
_Static_assert(MSK_SK_UNK == 0, "MSK_SK_UNK");
_Static_assert(MSK_SK_BAS == 1, "MSK_SK_BAS");
_Static_assert(MSK_SK_SUPBAS == 2, "MSK_SK_SUPBAS");
_Static_assert(MSK_SK_LOW == 3, "MSK_SK_LOW");
_Static_assert(MSK_SK_UPR == 4, "MSK_SK_UPR");
_Static_assert(MSK_SK_FIX == 5, "MSK_SK_FIX");
_Static_assert(MSK_SK_INF == 6, "MSK_SK_INF");
_Static_assert(MSK_OBJECTIVE_SENSE_MINIMIZE == 0, "MSK_OBJECTIVE_SENSE_MINIMIZE");
_Static_assert(MSK_OBJECTIVE_SENSE_MAXIMIZE == 1, "MSK_OBJECTIVE_SENSE_MAXIMIZE");
_Static_assert(MSK_STREAM_LOG == 0, "MSK_STREAM_LOG");
_Static_assert(MSK_FEATURE_PTS == 0, "MSK_FEATURE_PTS");
_Static_assert(MSK_PAR_INVALID_TYPE == 0, "MSK_PAR_INVALID_TYPE");
_Static_assert(MSK_PAR_DOU_TYPE == 1, "MSK_PAR_DOU_TYPE");
_Static_assert(MSK_PAR_INT_TYPE == 2, "MSK_PAR_INT_TYPE");
_Static_assert(MSK_PAR_STR_TYPE == 3, "MSK_PAR_STR_TYPE");
_Static_assert(MSK_OPTIMIZER_CONIC == 0, "MSK_OPTIMIZER_CONIC");
_Static_assert(MSK_OPTIMIZER_DUAL_SIMPLEX == 1, "MSK_OPTIMIZER_DUAL_SIMPLEX");
_Static_assert(MSK_OPTIMIZER_FREE == 2, "MSK_OPTIMIZER_FREE");
_Static_assert(MSK_OPTIMIZER_INTPNT == 4, "MSK_OPTIMIZER_INTPNT");
_Static_assert(MSK_OPTIMIZER_PRIMAL_SIMPLEX == 8, "MSK_OPTIMIZER_PRIMAL_SIMPLEX");
_Static_assert(MSK_PRESOLVE_MODE_OFF == 0, "MSK_PRESOLVE_MODE_OFF");
_Static_assert(MSK_PRESOLVE_MODE_FREE == 2, "MSK_PRESOLVE_MODE_FREE");
_Static_assert(MSK_IPAR_LOG == 34, "MSK_IPAR_LOG");
_Static_assert(MSK_IPAR_NUM_THREADS == 100, "MSK_IPAR_NUM_THREADS");
_Static_assert(MSK_IPAR_OPTIMIZER == 110, "MSK_IPAR_OPTIMIZER");
_Static_assert(MSK_IPAR_PRESOLVE_USE == 121, "MSK_IPAR_PRESOLVE_USE");
_Static_assert(MSK_IPAR_SIM_MAX_ITERATIONS == 147, "MSK_IPAR_SIM_MAX_ITERATIONS");
_Static_assert(MSK_IPAR_INTPNT_BASIS == 17, "MSK_IPAR_INTPNT_BASIS");
_Static_assert(MSK_DPAR_BASIS_TOL_S == 2, "MSK_DPAR_BASIS_TOL_S");
_Static_assert(MSK_DPAR_BASIS_TOL_X == 3, "MSK_DPAR_BASIS_TOL_X");
_Static_assert(MSK_DPAR_INTPNT_CO_TOL_DFEAS == 16, "MSK_DPAR_INTPNT_CO_TOL_DFEAS");
_Static_assert(MSK_DPAR_INTPNT_CO_TOL_PFEAS == 20, "MSK_DPAR_INTPNT_CO_TOL_PFEAS");
_Static_assert(MSK_DPAR_INTPNT_QO_TOL_DFEAS == 22, "MSK_DPAR_INTPNT_QO_TOL_DFEAS");
_Static_assert(MSK_DPAR_INTPNT_QO_TOL_PFEAS == 26, "MSK_DPAR_INTPNT_QO_TOL_PFEAS");
_Static_assert(MSK_DPAR_INTPNT_TOL_DFEAS == 28, "MSK_DPAR_INTPNT_TOL_DFEAS");
_Static_assert(MSK_DPAR_INTPNT_TOL_PFEAS == 33, "MSK_DPAR_INTPNT_TOL_PFEAS");
_Static_assert(MSK_DPAR_OPTIMIZER_MAX_TIME == 50, "MSK_DPAR_OPTIMIZER_MAX_TIME");
_Static_assert(MSK_CALLBACK_CONIC == 34, "MSK_CALLBACK_CONIC");
_Static_assert(MSK_CALLBACK_INTPNT == 90, "MSK_CALLBACK_INTPNT");
_Static_assert(MSK_CALLBACK_UPDATE_DUAL_SIMPLEX == 100, "MSK_CALLBACK_UPDATE_DUAL_SIMPLEX");
_Static_assert(MSK_CALLBACK_UPDATE_PRIMAL_SIMPLEX == 104, "MSK_CALLBACK_UPDATE_PRIMAL_SIMPLEX");
_Static_assert(MSK_CALLBACK_UPDATE_SIMPLEX == 106, "MSK_CALLBACK_UPDATE_SIMPLEX");
_Static_assert(MSK_DINF_INTPNT_DUAL_OBJ == 13, "MSK_DINF_INTPNT_DUAL_OBJ");
_Static_assert(MSK_DINF_INTPNT_PRIMAL_OBJ == 18, "MSK_DINF_INTPNT_PRIMAL_OBJ");
_Static_assert(MSK_DINF_OPTIMIZER_TIME == 50, "MSK_DINF_OPTIMIZER_TIME");
_Static_assert(MSK_DINF_SIM_OBJ == 64, "MSK_DINF_SIM_OBJ");
_Static_assert(MSK_IINF_INTPNT_ITER == 17, "MSK_IINF_INTPNT_ITER");
_Static_assert(MSK_IINF_SIM_DUAL_ITER == 121, "MSK_IINF_SIM_DUAL_ITER");
_Static_assert(MSK_IINF_SIM_PRIMAL_ITER == 128, "MSK_IINF_SIM_PRIMAL_ITER");
