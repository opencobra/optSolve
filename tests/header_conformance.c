/*
 * Header conformance check for src/gurobi/ffi.rs (task T037; research R12; constitution VII-E).
 *
 * Compiled (never linked or run) against the installed $GUROBI_HOME/include/gurobi_c.h by
 * tests/header_conformance.rs. Every function declared in ffi.rs is assigned to a pointer of
 * the exact type ffi.rs declares, spelled out here in C; a mismatch with the header is a
 * compile error (with -Werror=incompatible-pointer-types). Every constant ffi.rs defines is
 * checked with _Static_assert.
 *
 * KEEP IN SYNC: adding or changing a declaration or constant in src/gurobi/ffi.rs requires
 * adding or changing the matching check here. The Rust test also checks that every
 * `pub fn GRB...` and `pub const GRB_...` in ffi.rs is named in this file.
 *
 * Rust-to-C type mapping used below: c_int=int, c_double=double, c_char=char, usize=size_t,
 * *mut c_void=void *, *mut T=T *, *const T=const T *. Gurobi's `__stdcall` is empty on Linux.
 */
#include <stddef.h>
#include "gurobi_c.h"

/* Callback type as declared in ffi.rs (GRBcallback without the Option). */
typedef int (__stdcall *ffi_callback)(GRBmodel *model, void *cbdata, int where, void *usrdata);

/* Functions: one typed pointer per declaration in ffi.rs. */
int (__stdcall *const p_GRBloadenvinternal)(GRBenv **, const char *, int, int, int) = GRBloadenvinternal;
int (__stdcall *const p_GRBemptyenvinternal)(GRBenv **, int, int, int) = GRBemptyenvinternal;
int (__stdcall *const p_GRBstartenv)(GRBenv *) = GRBstartenv;
void (__stdcall *const p_GRBfreeenv)(GRBenv *) = GRBfreeenv;
const char *(__stdcall *const p_GRBgeterrormsg)(GRBenv *) = GRBgeterrormsg;
void (__stdcall *const p_GRBversion)(int *, int *, int *) = GRBversion;
GRBenv *(__stdcall *const p_GRBgetenv)(GRBmodel *) = GRBgetenv;
int (__stdcall *const p_GRBXloadmodel)(GRBenv *, GRBmodel **, const char *, int, int, int,
                                        double, double *, char *, double *, size_t *, int *,
                                        int *, double *, double *, double *, char *, char **,
                                        char **) = GRBXloadmodel;
int (__stdcall *const p_GRBXaddvars)(GRBmodel *, int, size_t, size_t *, int *, double *,
                                      double *, double *, double *, char *, char **) = GRBXaddvars;
int (__stdcall *const p_GRBaddqpterms)(GRBmodel *, int, int *, int *, double *) = GRBaddqpterms;
int (__stdcall *const p_GRBaddqconstr)(GRBmodel *, int, int *, double *, int, int *, int *,
                                        double *, char, double, const char *) = GRBaddqconstr;
int (__stdcall *const p_GRBXaddconstrs)(GRBmodel *, int, size_t, size_t *, int *, double *,
                                         char *, double *, char **) = GRBXaddconstrs;
int (__stdcall *const p_GRBXchgcoeffs)(GRBmodel *, size_t, int *, int *, double *) = GRBXchgcoeffs;
int (__stdcall *const p_GRBupdatemodel)(GRBmodel *) = GRBupdatemodel;
int (__stdcall *const p_GRBoptimize)(GRBmodel *) = GRBoptimize;
int (__stdcall *const p_GRBfreemodel)(GRBmodel *) = GRBfreemodel;
void (__stdcall *const p_GRBterminate)(GRBmodel *) = GRBterminate;
int (__stdcall *const p_GRBgetintattr)(GRBmodel *, const char *, int *) = GRBgetintattr;
int (__stdcall *const p_GRBgetdblattr)(GRBmodel *, const char *, double *) = GRBgetdblattr;
int (__stdcall *const p_GRBgetintattrarray)(GRBmodel *, const char *, int, int, int *) = GRBgetintattrarray;
int (__stdcall *const p_GRBgetdblattrarray)(GRBmodel *, const char *, int, int, double *) = GRBgetdblattrarray;
int (__stdcall *const p_GRBsetintparam)(GRBenv *, const char *, int) = GRBsetintparam;
int (__stdcall *const p_GRBsetdblparam)(GRBenv *, const char *, double) = GRBsetdblparam;
int (__stdcall *const p_GRBsetstrparam)(GRBenv *, const char *, const char *) = GRBsetstrparam;
int (__stdcall *const p_GRBgetparamtype)(GRBenv *, const char *) = GRBgetparamtype;
int (__stdcall *const p_GRBgetintparam)(GRBenv *, const char *, int *) = GRBgetintparam;
int (__stdcall *const p_GRBgetdblparam)(GRBenv *, const char *, double *) = GRBgetdblparam;
int (__stdcall *const p_GRBgetstrparam)(GRBenv *, const char *, char *) = GRBgetstrparam;
int (__stdcall *const p_GRBsetcallbackfunc)(GRBmodel *, ffi_callback, void *) = GRBsetcallbackfunc;
int (__stdcall *const p_GRBcbget)(void *, int, int, void *) = GRBcbget;

/* The macros ffi.rs replaces must still expand to the functions it binds. */
_Static_assert(GRB_VERSION_MAJOR >= 13, "ffi.rs targets Gurobi 13 or later");

/* Status codes. */
_Static_assert(GRB_LOADED == 1, "GRB_LOADED");
_Static_assert(GRB_OPTIMAL == 2, "GRB_OPTIMAL");
_Static_assert(GRB_INFEASIBLE == 3, "GRB_INFEASIBLE");
_Static_assert(GRB_INF_OR_UNBD == 4, "GRB_INF_OR_UNBD");
_Static_assert(GRB_UNBOUNDED == 5, "GRB_UNBOUNDED");
_Static_assert(GRB_CUTOFF == 6, "GRB_CUTOFF");
_Static_assert(GRB_ITERATION_LIMIT == 7, "GRB_ITERATION_LIMIT");
_Static_assert(GRB_NODE_LIMIT == 8, "GRB_NODE_LIMIT");
_Static_assert(GRB_TIME_LIMIT == 9, "GRB_TIME_LIMIT");
_Static_assert(GRB_SOLUTION_LIMIT == 10, "GRB_SOLUTION_LIMIT");
_Static_assert(GRB_INTERRUPTED == 11, "GRB_INTERRUPTED");
_Static_assert(GRB_NUMERIC == 12, "GRB_NUMERIC");
_Static_assert(GRB_SUBOPTIMAL == 13, "GRB_SUBOPTIMAL");
_Static_assert(GRB_INPROGRESS == 14, "GRB_INPROGRESS");
_Static_assert(GRB_USER_OBJ_LIMIT == 15, "GRB_USER_OBJ_LIMIT");
_Static_assert(GRB_WORK_LIMIT == 16, "GRB_WORK_LIMIT");
_Static_assert(GRB_MEM_LIMIT == 17, "GRB_MEM_LIMIT");
_Static_assert(GRB_LOCALLY_OPTIMAL == 18, "GRB_LOCALLY_OPTIMAL");
_Static_assert(GRB_LOCALLY_INFEASIBLE == 19, "GRB_LOCALLY_INFEASIBLE");

/* Numbers, senses and sizes. */
_Static_assert(GRB_INFINITY == 1e100, "GRB_INFINITY");
_Static_assert(GRB_ERROR_DATA_NOT_AVAILABLE == 10005, "GRB_ERROR_DATA_NOT_AVAILABLE");
_Static_assert(GRB_ERROR_Q_NOT_PSD == 10020, "GRB_ERROR_Q_NOT_PSD");
_Static_assert(GRB_ERROR_QCP_EQUALITY_CONSTRAINT == 10021, "GRB_ERROR_QCP_EQUALITY_CONSTRAINT");
_Static_assert(GRB_ERROR_NOT_SUPPORTED == 10024, "GRB_ERROR_NOT_SUPPORTED");
_Static_assert(GRB_MINIMIZE == 1, "GRB_MINIMIZE");
_Static_assert(GRB_MAXIMIZE == -1, "GRB_MAXIMIZE");
_Static_assert(GRB_LESS_EQUAL == '<', "GRB_LESS_EQUAL");
_Static_assert(GRB_GREATER_EQUAL == '>', "GRB_GREATER_EQUAL");
_Static_assert(GRB_EQUAL == '=', "GRB_EQUAL");
_Static_assert(GRB_MAX_STRLEN == 512, "GRB_MAX_STRLEN");

/* Callback where/what codes. */
_Static_assert(GRB_CB_POLLING == 0, "GRB_CB_POLLING");
_Static_assert(GRB_CB_SIMPLEX == 2, "GRB_CB_SIMPLEX");
_Static_assert(GRB_CB_BARRIER == 7, "GRB_CB_BARRIER");
_Static_assert(GRB_CB_SPX_ITRCNT == 2000, "GRB_CB_SPX_ITRCNT");
_Static_assert(GRB_CB_SPX_OBJVAL == 2001, "GRB_CB_SPX_OBJVAL");
_Static_assert(GRB_CB_RUNTIME == 6002, "GRB_CB_RUNTIME");
_Static_assert(GRB_CB_BARRIER_ITRCNT == 7001, "GRB_CB_BARRIER_ITRCNT");
_Static_assert(GRB_CB_BARRIER_PRIMOBJ == 7002, "GRB_CB_BARRIER_PRIMOBJ");
_Static_assert(GRB_CB_BARRIER_DUALOBJ == 7003, "GRB_CB_BARRIER_DUALOBJ");

/* Type widths ffi.rs assumes (64-bit targets). */
_Static_assert(sizeof(int) == 4, "c_int is 32-bit");
_Static_assert(sizeof(size_t) == 8, "usize maps to size_t on 64-bit targets");
_Static_assert(sizeof(double) == 8, "c_double is 64-bit");
