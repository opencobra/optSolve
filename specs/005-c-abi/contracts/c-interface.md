# Contract: optSolver C interface, ABI version 1.0

Header `src/capi/optsolver.h` (C11, `<stddef.h>` and `<stdint.h>` only). Library `liboptsolver.a`
and `liboptsolver.so`, built with `cargo build --release --features capi,gurobi,mosek`.
Parameter names in prototypes are descriptive (`result`, `caps`, `env`); they are not part of the
ABI. Every name is prefixed `optsolver_` (functions, types) or `OPTSOLVER_` (constants).

## Version (FR-015)

```c
#define OPTSOLVER_ABI_VERSION_MAJOR 1
#define OPTSOLVER_ABI_VERSION_MINOR 0
uint32_t optsolver_abi_version(void);   /* (major << 16) | minor */
```

An incompatible change (a removed or changed function, type, field, constant meaning or ownership
rule) increments the major part; a compatible addition (a new function, a new constant, a field
appended to an input structure) increments the minor part. A caller compiled against a given major and minor
version works with any library of the same major version and an equal or higher minor version. Input structures begin with `struct_size`
(= `sizeof` the caller's structure); the library reads only the fields that size covers and
treats absent trailing fields as unset. Minimum sizes in version 1.0 (a smaller `struct_size` is
`OPTSOLVER_ERR_INVALID`): `optsolver_problem` and `optsolver_buffers` — the full 1.0 structure
(every field is needed); `optsolver_options` — up to and including `struct_size` only (every
option may be absent, meaning unset). A test passes an options structure truncated after
`time_limit` and checks the remaining fields are treated as unset (FR-020).

## Codes (FR-010)

| Constant | Value | Rust `Error` |
|---|---|---|
| `OPTSOLVER_OK` | 0 | — |
| `OPTSOLVER_ERR_INVALID` | 1 | `Invalid` (also a null required pointer, a bad code) |
| `OPTSOLVER_ERR_TOO_LARGE` | 2 | `TooLarge` |
| `OPTSOLVER_ERR_UNSUPPORTED` | 3 | `Unsupported` (field, class, cone kind, backend not compiled in) |
| `OPTSOLVER_ERR_PARAMETER_CONFLICT` | 4 | `ParameterConflict` |
| `OPTSOLVER_ERR_SOLVER` | 5 | `Solver` (vendor code via `optsolver_last_error_solver_code`) |
| `OPTSOLVER_ERR_INTERNAL` | 6 | a contained panic |

```c
char *optsolver_last_error_message(void);        /* copy of this thread's last failure message, or NULL */
void optsolver_string_free(char *s);             /* frees a string the library returned; NULL: no-op */
int32_t optsolver_last_error_solver_code(void);  /* vendor code of the last OPTSOLVER_ERR_SOLVER, else 0 */
```

The message is the Rust error's `Display` text (it names the input, field or parameter). The
library records it per thread (a failure on one thread never overwrites another thread's); each
call of `optsolver_last_error_message` returns a new caller-owned copy, freed with
`optsolver_string_free` (FR-016).

## Backends, statuses and other codes

`OPTSOLVER_BACKEND_GUROBI` 1, `OPTSOLVER_BACKEND_MOSEK` 2.
`OPTSOLVER_SENSE_MINIMISE` 1, `OPTSOLVER_SENSE_MAXIMISE` −1.
Status: `OPTSOLVER_STATUS_OPTIMAL` 1, `_INFEASIBLE` 2, `_UNBOUNDED` 3,
`_INFEASIBLE_OR_UNBOUNDED` 4, `_STOPPED_AT_LIMIT` 5, `_NUMERICAL_DIFFICULTY` 6, `_INTERRUPTED` 7,
`_LOCALLY_OPTIMAL` 8, `_LOCALLY_INFEASIBLE` 9, `_NOT_SOLVED` 10, `_UNKNOWN` 11.
Limit reason: `OPTSOLVER_LIMIT_NONE` 0, `_TIME` 1, `_ITERATIONS` 2, `_NODES` 3, `_SOLUTIONS` 4,
`_OBJECTIVE_CUTOFF` 5, `_OBJECTIVE_TARGET` 6, `_WORK` 7, `_MEMORY` 8.
Solution returned: `OPTSOLVER_RETURNED_NONE` 0, `_FEASIBLE` 1, `_OPTIMAL` 2.
Cone kinds (COBRA `coneType` codes): `OPTSOLVER_CONE_QUADRATIC` 1, `_ROTATED_QUADRATIC` 2,
`_PRIMAL_EXPONENTIAL` 3, `_DUAL_EXPONENTIAL` 4, `_PRIMAL_POWER` 5, `_DUAL_POWER` 6,
`_NONNEGATIVE` 7, `_ZERO` 8, `_POSITIVE_SEMIDEFINITE` 9.
Problem classes (bit flags): `OPTSOLVER_CLASS_LP` 1, `_MILP` 2, `_QP` 4, `_CONIC` 8. The bits describe
the backend; interface version 1.0 cannot pass variable types, so a mixed-integer problem cannot be
posed through it whatever the backend reports.
Method: `OPTSOLVER_METHOD_AUTO` 0, `_PRIMAL_SIMPLEX` 1, `_DUAL_SIMPLEX` 2, `_BARRIER` 3,
`_CONCURRENT` 4. Presolve: `OPTSOLVER_PRESOLVE_AUTO` 0, `_OFF` 1, `_CONSERVATIVE` 2,
`_AGGRESSIVE` 3. `OPTSOLVER_UNSET` −1 (integer option unset).
Basis: `OPTSOLVER_BASIS_BASIC` 0, `_AT_LOWER` 1, `_AT_UPPER` 2, `_SUPERBASIC` 3, `_FIXED` 4.
Verification overall: `OPTSOLVER_VERIFICATION_NOT_RUN` 0, `_VERIFIED` 1, `_FAILED` 2,
`_PARTIALLY_VERIFIED` 3; measure verdict `OPTSOLVER_VERDICT_PASS` 0, `_FAIL` 1,
`_NOT_EVALUATED` 2. Arrays: `OPTSOLVER_ARRAY_X` 0, `_Y` 1, `_Z` 2, `_Z_L` 3, `_Z_U` 4, `_S` 5,
`_ROW_ACTIVITY` 6. Certificates: `OPTSOLVER_CERTIFICATE_NONE` 0, `_FARKAS` 1,
`_UNBOUNDED_RAY` 2; certificate arrays `OPTSOLVER_CERT_Y` 0, `_Z` 1, `_S` 2 (Farkas row, bound and
conic multipliers), `OPTSOLVER_CERT_RAY` 3 (the unbounded ray).
Parameter value types: `OPTSOLVER_PARAM_INT` 1, `_DOUBLE` 2, `_STRING` 3.

`const char *optsolver_status_name(int32_t)`, `optsolver_limit_reason_name(int32_t)`,
`optsolver_returned_name(int32_t)`, `optsolver_verification_name(int32_t)`,
`optsolver_cone_name(int32_t)` return NUL-terminated strings owned by the library for the process lifetime (`"Optimal"`,
`"Memory"`, `"PartiallyVerified"`, `"rotated quadratic"` …), or `""` for an unknown code. They are
derived from the core's own names (the Rust enum variant names and `ConeKind::name`) on first use
into a process-wide cache, not a second list.

## Problem (FR-002, FR-003)

```c
typedef struct optsolver_csc {
    size_t rows, cols;          /* A: m x n; Q: n x n; F: k x n */
    const size_t *col_ptrs;     /* cols + 1 */
    const size_t *row_indices;  /* col_ptrs[cols] */
    const double *values;       /* col_ptrs[cols] */
} optsolver_csc;

typedef struct optsolver_cone {
    int32_t kind;               /* OPTSOLVER_CONE_* */
    size_t dim;                 /* dimension; for positive semidefinite, the order */
    const double *exponents;    /* power cones: exponents of the leading coordinates */
    size_t exponents_len;
} optsolver_cone;

typedef struct optsolver_problem {
    uint32_t struct_size;
    optsolver_csc A;
    const int32_t *A_row_indices_i32;  /* if non-null: A's row indices as int32 (A.row_indices NULL) */
    const double *c;                   /* n */
    int32_t sense;                     /* OPTSOLVER_SENSE_* */
    double gamma;
    const double *l, *u;               /* n */
    const char *row_sense;             /* m; '=', '<', '>' — with b */
    const double *b;                   /* m */
    const double *b_l, *b_u;           /* m — instead of row_sense and b */
    const optsolver_csc *Q;            /* NULL, or n x n lower triangle */
    const optsolver_csc *F;            /* NULL, or k x n */
    const double *g;                   /* k */
    const optsolver_cone *cones;       /* num_cones */
    size_t num_cones;
} optsolver_problem;
```

A pointer may be NULL only when the length it describes is 0. Exactly one row-bound form is
given (both or neither: `OPTSOLVER_ERR_INVALID`). All validation of the problem is the Rust API's;
the C layer adds only checks the Rust API cannot see, each returning `OPTSOLVER_ERR_INVALID`
naming the field: a NULL pointer with a nonzero length; both or neither row-bound form; both or
neither index width for A; `struct_size` below the minimum; the bound `nnz · 8 ≤ isize::MAX`
(else `OPTSOLVER_ERR_TOO_LARGE`); unknown sense, backend, cone-kind, method, presolve and
parameter-type codes; `threads` above `u32::MAX`; `random_seed` outside `int32_t`; an
exponential-cone `dim` other than 3 or 0; a parameter name or string value that is not UTF-8.
A NULL pointer with length 0 becomes an empty slice (a dangling, aligned pointer), never a NULL
passed to `slice::from_raw_parts`. Every slice built from a caller pointer is bounded by
`len · sizeof(element) ≤ PTRDIFF_MAX` and `n + 1` must not overflow (else `OPTSOLVER_ERR_TOO_LARGE`).
The problem is built and validated before any environment is created, so invalid input is
reported identically with or without a licence. Each check has a failing-input test.

Aliasing: output arrays in `optsolver_buffers` must not overlap each other or any input array;
overlap is undefined behaviour (stated in the header and in the `# Safety` contract of
`optsolver_solve_into`).

## Options (FR-004)

```c
typedef struct optsolver_param {
    const char *name;
    int32_t type;               /* OPTSOLVER_PARAM_* */
    int64_t int_value; double double_value; const char *string_value;
} optsolver_param;

typedef struct optsolver_options {
    uint32_t struct_size;
    double time_limit, iteration_limit;                         /* NaN = unset */
    int64_t threads;                                            /* OPTSOLVER_UNSET or ≥ 0 */
    double primal_feasibility_tolerance, dual_feasibility_tolerance;  /* NaN = unset */
    int32_t method, presolve;                                   /* OPTSOLVER_UNSET or code */
    int64_t random_seed; int32_t random_seed_set;               /* used when _set != 0 */
    int32_t solver_log, array_outputs, verification;            /* OPTSOLVER_UNSET, 0 or 1 */
    int32_t certificates;                                       /* 0 or 1 */
    double memory_limit_gb;                                     /* NaN = unset */
    const optsolver_param *raw; size_t num_raw;
} optsolver_options;

void optsolver_options_init(optsolver_options *options);  /* everything unset */
```

A NULL options pointer means every field unset.

## Environments (FR-007, FR-008)

```c
typedef struct optsolver_env optsolver_env;
int32_t optsolver_env_create(int32_t backend, optsolver_env **out);
void optsolver_env_free(optsolver_env *env);                 /* NULL: no-op */
uint64_t optsolver_environments_created(void);               /* test support: count since load */
```

Thread rule: a handle may be used from any thread, by one thread at a time; different handles may
be used concurrently; Gurobi creation and freeing are serialised by a lock in the Gurobi backend;
MOSEK documents no guarantee for concurrent environment creation, so MOSEK environments should be
created and freed one at a time (concurrent creation and freeing unverified either way) (the Rust
`gurobi::Env` and `mosek::Env` are `Send`, not `Sync`).

## Solving (FR-005 to FR-007)

```c
typedef struct optsolver_result optsolver_result;
typedef struct optsolver_array { double *data; size_t len; } optsolver_array;  /* data NULL = not wanted */
typedef struct optsolver_buffers {
    uint32_t struct_size;
    optsolver_array x;                          /* length n, required */
    optsolver_array y, z, row_activity, s, z_l, z_u;  /* lengths m, n, m, k, n, n */
} optsolver_buffers;

int32_t optsolver_solve(optsolver_env *env, const optsolver_problem *problem,
                        const optsolver_options *options, optsolver_result **out);
int32_t optsolver_solve_once(int32_t backend, const optsolver_problem *problem,
                             const optsolver_options *options, optsolver_result **out);
int32_t optsolver_solve_into(optsolver_env *env, const optsolver_problem *problem,
                             const optsolver_options *options, const optsolver_buffers *buffers,
                             optsolver_result **out);
void optsolver_result_free(optsolver_result *result);       /* NULL: no-op */
```

`solve_into` passes each supplied array with its length to the Rust API, which rejects a wrong
length as `OPTSOLVER_ERR_INVALID` naming the array before any solver call; it fills each supplied
array with NaN — before the call, so on any error return the supplied arrays hold NaN — then the
backend writes the quantities it returns; the result object then holds the scalar and descriptive fields and its arrays for the
buffered quantities are empty, as in the Rust API.

## Reading a result (FR-005)

```c
int32_t optsolver_result_status(const optsolver_result *result);
int32_t optsolver_result_limit_reason(const optsolver_result *result);
int32_t optsolver_result_returned(const optsolver_result *result);
const double *optsolver_result_array(const optsolver_result *result, int32_t which, size_t *len);
int32_t optsolver_result_objective(const optsolver_result *result, double *value);  /* 1 present, 0 absent */
int32_t optsolver_result_bound(const optsolver_result *result, double *value);
int32_t optsolver_result_simplex_iterations(const optsolver_result *result, double *value);
int32_t optsolver_result_barrier_iterations(const optsolver_result *result, int64_t *value);
typedef struct optsolver_timings { double load, solve, extraction, verification; } optsolver_timings;
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
typedef struct optsolver_value { int32_t type; int64_t int_value; double double_value; const char *string_value; } optsolver_value;
size_t optsolver_result_attribute_count(const optsolver_result *result);
const char *optsolver_result_attribute(const optsolver_result *result, size_t i, optsolver_value *value);
size_t optsolver_result_parameter_count(const optsolver_result *result);
const char *optsolver_result_parameter(const optsolver_result *result, size_t i, optsolver_value *value);
int32_t optsolver_result_verification(const optsolver_result *result);             /* OPTSOLVER_VERIFICATION_* */
typedef struct optsolver_measure { const char *name; double absolute, relative, tolerance; int32_t verdict, required; } optsolver_measure;
size_t optsolver_result_measure_count(const optsolver_result *result);
int32_t optsolver_result_measure(const optsolver_result *result, size_t i, optsolver_measure *out);
```

Every pointer returned is owned by the result and valid until `optsolver_result_free`. Names held
by the Rust result as Rust strings (solver name and version, raw status, attribute and parameter
names, string-valued attributes and parameters) are converted to NUL-terminated strings on first
read; verification measure names and cone-kind names are converted from the strings the core
owns (`Measure::name`, `ConeKind::name`) on first use into a process-wide cache — no second list
of names.

## Capabilities (FR-017)

```c
typedef struct optsolver_caps optsolver_caps;
int32_t optsolver_backend_compiled(int32_t backend);           /* 1 or 0 */
int32_t optsolver_capabilities(int32_t backend, optsolver_caps **out);   /* temporary environment */
int32_t optsolver_env_capabilities(const optsolver_env *env, optsolver_caps **out);
int32_t optsolver_caps_usable(const optsolver_caps *caps);
const char *optsolver_caps_reason(const optsolver_caps *caps);    /* "" when usable */
uint32_t optsolver_caps_problem_classes(const optsolver_caps *caps);  /* OPTSOLVER_CLASS_* bits */
uint32_t optsolver_caps_cones(const optsolver_caps *caps);        /* bit (1 << kind) per cone kind */
int32_t optsolver_caps_certificates(const optsolver_caps *caps);
int32_t optsolver_caps_basis(const optsolver_caps *caps);
size_t optsolver_caps_option_count(const optsolver_caps *caps);
const char *optsolver_caps_option(const optsolver_caps *caps, size_t i);  /* shared field name */
const char *optsolver_caps_solver_name(const optsolver_caps *caps);
const char *optsolver_caps_solver_version(const optsolver_caps *caps);
void optsolver_caps_free(optsolver_caps *caps);
```

`optsolver_capabilities` returns `OPTSOLVER_OK` with a not-usable object (reason = the vendor
message) when the backend is compiled in but its environment cannot be created, and
`OPTSOLVER_ERR_UNSUPPORTED` when the backend is not compiled in.

## Test support

```c
int32_t optsolver_test_panic(void);  /* panics inside the library; returns OPTSOLVER_ERR_INTERNAL */
```
