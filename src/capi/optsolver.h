/*
 * optsolver.h — the C interface of optSolver, interface version 1.0 (feature 005, ADR-0004).
 *
 * A thin translation onto optSolver's solver-independent Rust API: LP, convex QP and conic
 * problems on Gurobi and MOSEK, with arrays borrowed from the caller in place.
 *
 * Building and linking: see docs/llm-interface.md, section "C interface: building and linking".
 * The library is liboptsolver.a / liboptsolver.so, built with
 *   cargo build --release --features capi,gurobi,mosek
 * The vendor libraries (Gurobi, MOSEK) are needed at load time and are never shipped with
 * optSolver.
 *
 * Versioning. OPTSOLVER_ABI_VERSION_MAJOR.MINOR below; optsolver_abi_version() returns the
 * library's as (major << 16) | minor. An incompatible change (a removed or changed function,
 * type, field, constant meaning or ownership rule) increments the major part; a compatible
 * addition (a new function or constant, a field appended to an input structure) increments the
 * minor part. A caller compiled against a given major and minor version works with any library of
 * the same major version and an equal or higher minor version. Input structures begin with
 * struct_size (set it to sizeof the structure); the library reads only the fields that size
 * covers and treats absent trailing fields as unset. Minimum sizes in 1.0: optsolver_problem and
 * optsolver_buffers, the full structure; optsolver_options, struct_size alone.
 *
 * Errors. Fallible functions return an OPTSOLVER_OK / OPTSOLVER_ERR_* code. The message of the
 * calling thread's last failure is available as a caller-owned copy from
 * optsolver_last_error_message() (free it with optsolver_string_free()); one thread's failure
 * never overwrites another thread's message. Non-optimal outcomes (infeasible, unbounded, limits)
 * are statuses in the result, not errors.
 *
 * Ownership. Inputs are borrowed for the duration of the call and never retained. Every object
 * the library allocates has exactly one free function (optsolver_env_free, optsolver_result_free,
 * optsolver_caps_free, optsolver_string_free); freeing NULL is a no-op. Pointers returned by
 * accessors are owned by their object and valid until it is freed; name functions return strings
 * valid for the process lifetime. Accessors given a NULL object return 0, NULL or "".
 *
 * Threads. An environment handle may be used from any thread, including one other than the
 * thread that created it, but by one thread at a time; different handles may be used
 * concurrently. Creating and freeing Gurobi environments is serialised by the library. MOSEK
 * documents no guarantee for concurrent environment creation or freeing: create and free MOSEK
 * environments one at a time (unverified either way).
 *
 * Aliasing. Output arrays in optsolver_buffers must not overlap each other or any input array;
 * overlap is undefined behaviour.
 *
 * Panics. Every entry point contains internal panics and returns OPTSOLVER_ERR_INTERNAL with a
 * message; a panic hook installed on first use keeps such panics silent and passes every other
 * panic to the previously installed hook. A host that later replaces the process's Rust panic hook
 * ends that silence. A library built with panic = "abort" aborts instead of returning the code.
 * The library writes nothing to standard output or standard error; the solver log, when
 * requested, goes where the Rust API sends it.
 */
#ifndef OPTSOLVER_H
#define OPTSOLVER_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define OPTSOLVER_ABI_VERSION_MAJOR 1
#define OPTSOLVER_ABI_VERSION_MINOR 0

/* Codes returned by fallible functions. */
#define OPTSOLVER_OK 0
#define OPTSOLVER_ERR_INVALID 1            /* invalid input, before any solver call */
#define OPTSOLVER_ERR_TOO_LARGE 2          /* a dimension above the solvers' limit, or an array too large */
#define OPTSOLVER_ERR_UNSUPPORTED 3        /* field, problem class, cone kind or backend not supported */
#define OPTSOLVER_ERR_PARAMETER_CONFLICT 4 /* the same vendor parameter set by two routes */
#define OPTSOLVER_ERR_SOLVER 5             /* the solver reported an error (vendor code available) */
#define OPTSOLVER_ERR_INTERNAL 6           /* a contained internal panic */

#define OPTSOLVER_BACKEND_GUROBI 1
#define OPTSOLVER_BACKEND_MOSEK 2

#define OPTSOLVER_SENSE_MINIMISE 1
#define OPTSOLVER_SENSE_MAXIMISE (-1)

/* An integer option left unset. */
#define OPTSOLVER_UNSET (-1)

#define OPTSOLVER_STATUS_OPTIMAL 1
#define OPTSOLVER_STATUS_INFEASIBLE 2
#define OPTSOLVER_STATUS_UNBOUNDED 3
#define OPTSOLVER_STATUS_INFEASIBLE_OR_UNBOUNDED 4
#define OPTSOLVER_STATUS_STOPPED_AT_LIMIT 5
#define OPTSOLVER_STATUS_NUMERICAL_DIFFICULTY 6
#define OPTSOLVER_STATUS_INTERRUPTED 7
#define OPTSOLVER_STATUS_LOCALLY_OPTIMAL 8
#define OPTSOLVER_STATUS_LOCALLY_INFEASIBLE 9
#define OPTSOLVER_STATUS_NOT_SOLVED 10
#define OPTSOLVER_STATUS_UNKNOWN 11

#define OPTSOLVER_LIMIT_NONE 0
#define OPTSOLVER_LIMIT_TIME 1
#define OPTSOLVER_LIMIT_ITERATIONS 2
#define OPTSOLVER_LIMIT_NODES 3
#define OPTSOLVER_LIMIT_SOLUTIONS 4
#define OPTSOLVER_LIMIT_OBJECTIVE_CUTOFF 5
#define OPTSOLVER_LIMIT_OBJECTIVE_TARGET 6
#define OPTSOLVER_LIMIT_WORK 7
#define OPTSOLVER_LIMIT_MEMORY 8

#define OPTSOLVER_RETURNED_NONE 0
#define OPTSOLVER_RETURNED_FEASIBLE 1
#define OPTSOLVER_RETURNED_OPTIMAL 2

/* Cone kinds (the COBRA coneType codes). */
#define OPTSOLVER_CONE_QUADRATIC 1
#define OPTSOLVER_CONE_ROTATED_QUADRATIC 2
#define OPTSOLVER_CONE_PRIMAL_EXPONENTIAL 3
#define OPTSOLVER_CONE_DUAL_EXPONENTIAL 4
#define OPTSOLVER_CONE_PRIMAL_POWER 5
#define OPTSOLVER_CONE_DUAL_POWER 6
#define OPTSOLVER_CONE_NONNEGATIVE 7
#define OPTSOLVER_CONE_ZERO 8
#define OPTSOLVER_CONE_POSITIVE_SEMIDEFINITE 9

/* Problem-class bits. They describe the backend; interface version 1.0 cannot pass variable
 * types, so a mixed-integer problem cannot be posed through it whatever the backend reports. */
#define OPTSOLVER_CLASS_LP 1u
#define OPTSOLVER_CLASS_MILP 2u
#define OPTSOLVER_CLASS_QP 4u
#define OPTSOLVER_CLASS_CONIC 8u

#define OPTSOLVER_METHOD_AUTO 0
#define OPTSOLVER_METHOD_PRIMAL_SIMPLEX 1
#define OPTSOLVER_METHOD_DUAL_SIMPLEX 2
#define OPTSOLVER_METHOD_BARRIER 3
#define OPTSOLVER_METHOD_CONCURRENT 4

#define OPTSOLVER_PRESOLVE_AUTO 0
#define OPTSOLVER_PRESOLVE_OFF 1
#define OPTSOLVER_PRESOLVE_CONSERVATIVE 2
#define OPTSOLVER_PRESOLVE_AGGRESSIVE 3

#define OPTSOLVER_BASIS_BASIC 0
#define OPTSOLVER_BASIS_AT_LOWER 1
#define OPTSOLVER_BASIS_AT_UPPER 2
#define OPTSOLVER_BASIS_SUPERBASIC 3
#define OPTSOLVER_BASIS_FIXED 4

#define OPTSOLVER_VERIFICATION_NOT_RUN 0
#define OPTSOLVER_VERIFICATION_VERIFIED 1
#define OPTSOLVER_VERIFICATION_FAILED 2
#define OPTSOLVER_VERIFICATION_PARTIALLY_VERIFIED 3

#define OPTSOLVER_VERDICT_PASS 0
#define OPTSOLVER_VERDICT_FAIL 1
#define OPTSOLVER_VERDICT_NOT_EVALUATED 2

#define OPTSOLVER_ARRAY_X 0
#define OPTSOLVER_ARRAY_Y 1
#define OPTSOLVER_ARRAY_Z 2
#define OPTSOLVER_ARRAY_Z_L 3
#define OPTSOLVER_ARRAY_Z_U 4
#define OPTSOLVER_ARRAY_S 5
#define OPTSOLVER_ARRAY_ROW_ACTIVITY 6

#define OPTSOLVER_CERTIFICATE_NONE 0
#define OPTSOLVER_CERTIFICATE_FARKAS 1
#define OPTSOLVER_CERTIFICATE_UNBOUNDED_RAY 2

/* Certificate arrays: Farkas row, bound and conic multipliers; the unbounded ray. */
#define OPTSOLVER_CERT_Y 0
#define OPTSOLVER_CERT_Z 1
#define OPTSOLVER_CERT_S 2
#define OPTSOLVER_CERT_RAY 3

#define OPTSOLVER_PARAM_INT 1
#define OPTSOLVER_PARAM_DOUBLE 2
#define OPTSOLVER_PARAM_STRING 3

/* ---- Version, errors, names ------------------------------------------------------------- */

uint32_t optsolver_abi_version(void);
/* Caller-owned copy of this thread's last failure message, or NULL; free with optsolver_string_free. */
char *optsolver_last_error_message(void);
void optsolver_string_free(char *s);
/* Vendor code of this thread's last OPTSOLVER_ERR_SOLVER, else 0. */
int32_t optsolver_last_error_solver_code(void);

const char *optsolver_status_name(int32_t code);       /* "Optimal", ... ; "" if unknown */
const char *optsolver_limit_reason_name(int32_t code); /* "Memory", ... */
const char *optsolver_returned_name(int32_t code);     /* "None", "Feasible", "Optimal" */
const char *optsolver_verification_name(int32_t code); /* "NotRun", "Verified", "Failed", "PartiallyVerified" */
const char *optsolver_cone_name(int32_t code);         /* "quadratic", "rotated quadratic", ... */

/* ---- Problem ------------------------------------------------------------------------------ */

/* A borrowed compressed-sparse-column matrix with size_t row indices. A: m x n; Q: n x n (lower
 * triangle); F: k x n. */
typedef struct optsolver_csc {
    size_t rows, cols;
    const size_t *col_ptrs;    /* cols + 1 */
    const size_t *row_indices; /* col_ptrs[cols] */
    const double *values;      /* col_ptrs[cols] */
} optsolver_csc;

typedef struct optsolver_cone {
    int32_t kind;              /* OPTSOLVER_CONE_* */
    size_t dim;                /* dimension; for positive semidefinite, the order */
    const double *exponents;   /* power cones: exponents of the leading coordinates */
    size_t exponents_len;
} optsolver_cone;

/* A pointer may be NULL only when the length it describes is 0. Give exactly one row-bound form
 * (row_sense and b, or b_l and b_u) when m > 0, and exactly one of A.row_indices and
 * A_row_indices_i32 when A has nonzeros. A's dimensions are validated before any other array is
 * read. The problem is validated before any environment is created. */
typedef struct optsolver_problem {
    uint32_t struct_size;
    optsolver_csc A;
    const int32_t *A_row_indices_i32; /* if non-NULL: A's row indices as int32 (A.row_indices NULL) */
    const double *c;                  /* n */
    int32_t sense;                    /* OPTSOLVER_SENSE_* */
    double gamma;                     /* objective constant */
    const double *l, *u;              /* n */
    const char *row_sense;            /* m: '=', '<', '>' (with b) */
    const double *b;                  /* m */
    const double *b_l, *b_u;          /* m (instead of row_sense and b) */
    const optsolver_csc *Q;           /* NULL, or the lower triangle of Q */
    const optsolver_csc *F;           /* NULL, or F */
    const double *g;                  /* k */
    const optsolver_cone *cones;      /* num_cones */
    size_t num_cones;
} optsolver_problem;

/* ---- Options ------------------------------------------------------------------------------ */

typedef struct optsolver_param {
    const char *name;                 /* vendor parameter name, UTF-8 */
    int32_t type;                     /* OPTSOLVER_PARAM_* */
    int64_t int_value;
    double double_value;
    const char *string_value;         /* UTF-8 */
} optsolver_param;

typedef struct optsolver_options {
    uint32_t struct_size;
    double time_limit, iteration_limit;                              /* NaN = unset */
    int64_t threads;                                                 /* OPTSOLVER_UNSET or >= 0 */
    double primal_feasibility_tolerance, dual_feasibility_tolerance; /* NaN = unset */
    int32_t method, presolve;                                        /* OPTSOLVER_UNSET or code */
    int64_t random_seed; int32_t random_seed_set;                    /* used when _set != 0 */
    int32_t solver_log, array_outputs, verification;                 /* OPTSOLVER_UNSET, 0 or 1 */
    int32_t certificates;                                            /* 0 or 1 */
    double memory_limit_gb;           /* NaN = unset; Gurobi SoftMemLimit; unsupported on MOSEK */
    const optsolver_param *raw; size_t num_raw;
} optsolver_options;

/* Fills every field unset (struct_size = sizeof(optsolver_options)). A NULL options pointer
 * passed to a solve means every field unset. */
void optsolver_options_init(optsolver_options *options);

/* ---- Environments ------------------------------------------------------------------------ */

typedef struct optsolver_env optsolver_env;
int32_t optsolver_env_create(int32_t backend, optsolver_env **out);
void optsolver_env_free(optsolver_env *env);
int32_t optsolver_backend_compiled(int32_t backend);     /* 1 or 0 */

/* ---- Solving ----------------------------------------------------------------------------- */

typedef struct optsolver_result optsolver_result;

typedef struct optsolver_array {
    double *data;                     /* NULL = not wanted */
    size_t len;
} optsolver_array;

typedef struct optsolver_buffers {
    uint32_t struct_size;
    optsolver_array x;                                 /* length n, required */
    optsolver_array y, z, row_activity, s, z_l, z_u;   /* lengths m, n, m, k, n, n */
} optsolver_buffers;

int32_t optsolver_solve(optsolver_env *env, const optsolver_problem *problem,
                        const optsolver_options *options, optsolver_result **out);
/* Validates, then creates and frees an environment of `backend` around one solve. */
int32_t optsolver_solve_once(int32_t backend, const optsolver_problem *problem,
                             const optsolver_options *options, optsolver_result **out);
/* Every supplied array is filled with NaN before the call (so also on an error return); the
 * backend then writes what it returns; a wrong length is OPTSOLVER_ERR_INVALID naming the array,
 * before any solver call. The result's arrays for buffered quantities are empty. */
int32_t optsolver_solve_into(optsolver_env *env, const optsolver_problem *problem,
                             const optsolver_options *options, const optsolver_buffers *buffers,
                             optsolver_result **out);
void optsolver_result_free(optsolver_result *result);

/* ---- Reading a result -------------------------------------------------------------------- */

typedef struct optsolver_timings { double load, solve, extraction, verification; } optsolver_timings;
typedef struct optsolver_value {
    int32_t type;                     /* OPTSOLVER_PARAM_* */
    int64_t int_value;
    double double_value;
    const char *string_value;         /* owned by the result */
} optsolver_value;
typedef struct optsolver_measure {
    const char *name;
    double absolute, relative, tolerance;
    int32_t verdict, required;        /* OPTSOLVER_VERDICT_*; 1 if the status requires it */
} optsolver_measure;

int32_t optsolver_result_status(const optsolver_result *result);
int32_t optsolver_result_limit_reason(const optsolver_result *result);
int32_t optsolver_result_returned(const optsolver_result *result);
/* NULL and *len = 0 when absent. */
const double *optsolver_result_array(const optsolver_result *result, int32_t which, size_t *len);
int32_t optsolver_result_objective(const optsolver_result *result, double *value); /* 1 present, 0 absent */
int32_t optsolver_result_bound(const optsolver_result *result, double *value);
int32_t optsolver_result_simplex_iterations(const optsolver_result *result, double *value);
int32_t optsolver_result_barrier_iterations(const optsolver_result *result, int64_t *value);
optsolver_timings optsolver_result_timings(const optsolver_result *result);
int32_t optsolver_result_has_basis(const optsolver_result *result);
int32_t optsolver_result_basis_variable(const optsolver_result *result, size_t j); /* OPTSOLVER_BASIS_* or -1 */
int32_t optsolver_result_basis_row(const optsolver_result *result, size_t i);
int32_t optsolver_result_certificate(const optsolver_result *result);              /* OPTSOLVER_CERTIFICATE_* */
const double *optsolver_result_certificate_array(const optsolver_result *result, int32_t which, size_t *len);
size_t optsolver_result_raw_status_count(const optsolver_result *result);
const char *optsolver_result_raw_status(const optsolver_result *result, size_t i, int64_t *code);
const char *optsolver_result_solver_name(const optsolver_result *result);
const char *optsolver_result_solver_version(const optsolver_result *result);
size_t optsolver_result_attribute_count(const optsolver_result *result);
const char *optsolver_result_attribute(const optsolver_result *result, size_t i, optsolver_value *value);
size_t optsolver_result_parameter_count(const optsolver_result *result);
const char *optsolver_result_parameter(const optsolver_result *result, size_t i, optsolver_value *value);
int32_t optsolver_result_verification(const optsolver_result *result);             /* OPTSOLVER_VERIFICATION_* */
size_t optsolver_result_measure_count(const optsolver_result *result);
int32_t optsolver_result_measure(const optsolver_result *result, size_t i, optsolver_measure *out);

/* ---- Capabilities ------------------------------------------------------------------------ */

typedef struct optsolver_caps optsolver_caps;
/* Creates and frees a temporary environment; a compiled-in backend that cannot be used gives
 * OPTSOLVER_OK and a not-usable object with the reason (its problem classes, cones, options and
 * version are then empty: they are read from a usable environment); not compiled in:
 * OPTSOLVER_ERR_UNSUPPORTED. */
int32_t optsolver_capabilities(int32_t backend, optsolver_caps **out);
int32_t optsolver_env_capabilities(const optsolver_env *env, optsolver_caps **out);
int32_t optsolver_caps_usable(const optsolver_caps *caps);
const char *optsolver_caps_reason(const optsolver_caps *caps);       /* "" when usable */
uint32_t optsolver_caps_problem_classes(const optsolver_caps *caps); /* OPTSOLVER_CLASS_* bits */
uint32_t optsolver_caps_cones(const optsolver_caps *caps);           /* bit (1u << kind) per cone kind */
int32_t optsolver_caps_certificates(const optsolver_caps *caps);
int32_t optsolver_caps_basis(const optsolver_caps *caps);
size_t optsolver_caps_option_count(const optsolver_caps *caps);
const char *optsolver_caps_option(const optsolver_caps *caps, size_t i); /* shared field name */
const char *optsolver_caps_solver_name(const optsolver_caps *caps);
const char *optsolver_caps_solver_version(const optsolver_caps *caps);
void optsolver_caps_free(optsolver_caps *caps);

/* ---- Test support ------------------------------------------------------------------------ */

/* Panics inside the library; returns OPTSOLVER_ERR_INTERNAL. */
int32_t optsolver_test_panic(void);
/* Number of backend environments the C interface has created in this process. */
uint64_t optsolver_environments_created(void);

#ifdef __cplusplus
}
#endif

#endif /* OPTSOLVER_H */
