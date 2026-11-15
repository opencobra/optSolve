# Feature Specification: A C Interface Over The Solver-Independent API

**Feature Branch**: `005-c-abi`

**Created**: 2026-11-15

**Status**: Clarified (Bundle 1 complete under constitution 3.0.0; Gate 1 by compliance review)

**Input**: User description (request REQ-001 from the COBRA Toolbox fork, brought by a subagent
the maintainer authorised): "optSolver needs a C ABI: a C header plus a library (staticlib
and/or cdylib) exposing the existing Problem/Options/Solution interface. The COBRA Toolbox will
call optSolver from RunMat through a C MEX gateway `optsolver_solve` that lives in the COBRA
repository and is compiled with `runmat mex` (which accepts -I/-L/-l). RunMat must not embed
optSolver; the C ABI is the only bridge. The same C ABI should be usable by any other C caller.
Scope LP, QP and conic through Gurobi and MOSEK, mirroring the Rust API; zero-copy input;
environment reuse through an opaque handle; a memory limit so an over-size problem fails with a
status instead of the process being killed; no panics across the boundary, error codes and
messages, explicit ownership; a header and library the COBRA side links with -L/-l; C-level tests
against the Rust API results on the toy instances; a versioned ABI."

**Exploration**: none — direct request. The founding exploration
(`specs/000-solver-interface-exploration/design-notes.md`, sections 6 and 7) placed the RunMat
adapter outside this repository as Rust code registering RunMat builtins. The COBRA side has since
established (its feature `20261115-173718-runmat-testall-optsolver`, research R1) that RunMat
implements only MATLAB built-ins and that MATLAB-language code reaches compiled code only through
MEX or `loadlibrary`, both of which cross a C boundary; Rust has no stable ABI between separately
built binaries. The adapter therefore becomes a C MEX gateway in the COBRA repository, and this
repository supplies the C interface it is written against. This updates, and does not contradict,
the founding decision that the adapter lives outside this repository (constitution VIII).

## Clarifications

### Session 2026-11-15

Decided by the feature agent under constitution 3.0.0 (VI-A: clarification answers are agent
decisions checked by the Gate 1 compliance review), with the requesting side's (COBRA's)
preferences as input. Each vendor fact below was checked on 2026-11-15 against the source named
(constitution X); the plan's research record repeats them with the design consequences.

- Q: On MOSEK, how is the memory limit handled? → A: **Rejected as unsupported before any solver
  call** (option A). Rationale: COBRA preferred polling `MSK_getmemusagetask` from the progress
  callback (option B) only if allowed there. Evidence: MOSEK 11.2 C API manual (installed
  `doc/capi.pdf`), entry `MSKcallbackfunc`: "The user must not call any MOSEK function directly or
  indirectly from the callback function. The only exception is the possibility to retrieve a
  current best integer solution from the mixed-integer optimizer". `mosek.h` 11.2 declares no
  memory-limit parameter. Polling is therefore prohibited, and no probe can license what the
  vendor prohibits. Rejection uses the existing rule of constitution XVI-C (the field is absent
  from MOSEK's option mapping, so the shared option check returns "unsupported" naming the field
  and the backend).
- Q: On Gurobi, which memory parameter does the limit map to? → A: **`SoftMemLimit` only**; the
  hard limit `MemLimit` stays reachable through the raw parameter passthrough. Evidence: Gurobi
  Optimizer Reference Manual 13.0, Parameter Reference (docs.gurobi.com/projects/optimizer/en/13.0/
  reference/parameters.html, read 2026-11-15): `SoftMemLimit`, type double, default Infinity,
  minimum 0, "Limits the total amount of memory (in GB, i.e., 10^9 bytes) available to Gurobi. If
  more is needed, Gurobi will terminate with a MEM_LIMIT status code", "leads to a graceful exit of
  the optimization, such that it is possible to retrieve solution information afterwards", "only
  checked at places where optimization can be terminated gracefully, so memory use may exceed the
  limit between these checks", "allocated memory is tracked across all models within a Gurobi
  environment"; `MemLimit`: same units, "If more is needed, Gurobi will fail with an OUT_OF_MEMORY
  error", "it is not possible to retrieve solution information after an error termination".
  Installed `gurobi_c.h` 13.0.3 declares both names and `GRB_MEM_LIMIT` 17, which the Gurobi status
  translation already maps to "stopped at a limit" with limit reason "memory". Rationale: the soft
  limit gives a status (constitution VII-C), which the COBRA contract needs. Setting the
  solver-independent limit and `SoftMemLimit` by the raw route in one call is a parameter conflict
  (constitution VII-G); `MemLimit` by the raw route alongside it is not. Unverified until the
  licensed test of the memory-limit success criterion runs: that `SoftMemLimit` set on a model's
  environment (where optSolver sets parameters) is honoured.
- Q: Units of the memory limit? → A: gigabytes of 10^9 bytes, as `SoftMemLimit` (evidence in the
  previous answer); the field is named `memory_limit_gb` so the unit is in the name.
- Q: May an environment handle be used from a thread other than the one that created it? →
  A: **Yes, from any thread, one thread at a time** (option A); different handles concurrently.
  Evidence: MOSEK 11.2 C API manual §10.3: "Sharing a task between threads is safe, as long as it is
  not accessed from more than one thread at a time. Multiple tasks can be created and used in
  parallel without any problems", and §10.6: "Calling MSK_optimize from different threads using the
  same MOSEK environment only consumes one license token". Gurobi 13.0 C reference, environment
  functions (quoted in feature 001 research R17, item 7, checked 2026-11-14): "Multiple threads
  cannot work simultaneously within the same environment", "environment initialization is itself
  not thread-safe"; Gurobi support article 360043111231 (support.gurobi.com, read 2026-11-15):
  "Gurobi's environment creation and deletion are not thread-safe", "Pass each additional thread an
  initialized environment for its exclusive use". **This supersedes** the earlier rulings that the
  environments are neither `Send` nor `Sync` pending this vendor check — feature 001 research R15
  ("Making them `Send` waits for the vendor documentation check"), feature 002 research R2 (MOSEK
  `Env` "neither `Send` nor `Sync`, as for Gurobi") — and the Threading section of the interface
  document: the Rust types `gurobi::Env` and `mosek::Env` become `Send` (not `Sync`), each
  implementation citing the evidence above (constitution VII-B), and Gurobi environment creation
  and freeing are serialised by one process-wide lock in the Gurobi backend, because the vendor
  says they are not thread-safe. The rulings waited for exactly this check; the grounds are
  recorded here (constitution XV-E).
- Q: What happens to a caller-supplied output array of the wrong length, or for a quantity the
  solver did not return? → A: The caller passes each output array with its length; a wrong length
  is the "invalid input" error naming the array, before any solver call, as in the Rust API (the C
  layer adds no rule). Before solving, the C layer fills every supplied output array with NaN; an
  entry the backend does not write stays NaN, and the result's solution-returned value says whether
  a primal solution exists. Undefined content is never presented as data.
- Q: How are vectors compared between the C interface and the Rust API? → A: Both solves use the
  same backend, instance and options. Status, limit reason, solution-returned value, verification
  verdict and the names of failed measures must be identical; the objective, the bound and every
  entry of every returned vector must satisfy |a − b| ≤ 10 τ max(1, |b|), where b is the Rust API's
  value and τ is the effective tolerance the Rust API's verification report records — the
  row-bound measure's tolerance for x, row activity and the objective, the reduced-cost
  consistency measure's tolerance for y, z, z_l, z_u, s and the bound (10⁻⁶ when the report lacks
  that measure). Whether the two runs were bit-identical is reported, not required (Gurobi's
  default LP method may run concurrent algorithms, which the vendor documents as possibly
  non-deterministic). Missing arrays must be missing in both.

## User Scenarios & Testing *(mandatory)*

The users are authors of C (or C-compatible) callers: first the author of the COBRA Toolbox MEX
gateway `optsolver_solve`, then any other C program; and the maintainers.

### User Story 1 - Solve an LP or QP from C with the same meaning as the Rust API (Priority: P1)

A C programmer holding a sparse matrix in compressed sparse column form, an objective, variable
bounds and row bounds (either a sense and right-hand side per row, or lower and upper row bounds)
calls optSolver once and receives the same result the Rust API would give for the same problem
and options on the same backend: the status (with the limit reason when a limit stopped the
solver), what kind of solution is returned, the primal solution, the row duals, the reduced costs,
the lower- and upper-bound multipliers, the row activity, the objective and bound, the
verification verdict with the names of any failed measures, the raw solver statuses, the solver
name and version, and the solve time. A quadratic objective is given by its lower triangle, as in
the Rust API.

**Why this priority**: it is the request's core; without it the COBRA gateway cannot solve
anything. LP and QP are what COBRA's `solveCobraLP` and `solveCobraQP` route to optSolver.

**Independent Test**: a small C program, compiled against the header and linked to the library,
solves each LP and QP toy instance on each available backend; every returned quantity agrees with
the Rust API's result for the same instance and options by the comparison rule in the
Clarifications.

**Acceptance Scenarios**:

1. **Given** a feasible LP given with a sense and right-hand side per row, **When** solved through
   the C interface on Gurobi and on MOSEK, **Then** the status is Optimal, a solution is returned
   and every result field matches the Rust API's result on the same backend.
2. **Given** the same LP given with lower and upper row bounds, **When** solved, **Then** the
   result is the same as with the sense-and-right-hand-side form.
3. **Given** a convex QP whose quadratic term is given by its lower triangle, **When** solved on
   each backend, **Then** the result matches the Rust API's; an entry above the diagonal is
   rejected with an "invalid input" error naming the quadratic matrix, as in the Rust API.
4. **Given** an infeasible or an unbounded toy instance, **When** solved, **Then** the status is
   the one the Rust API returns (a status, not an error), with no solution returned.
5. **Given** options for the primal and dual feasibility tolerances, a time limit, the solver log
   and raw vendor parameters by name and value, **When** solved, **Then** each reaches the solver
   exactly as through the Rust API (the effective parameter list of the result shows them), and
   setting the same vendor parameter by two routes is a "parameter conflict" error, as in the Rust
   API.

---

### User Story 2 - Pass large problems without copies and reuse the licensed environment (Priority: P1)

The gateway author passes pointers into the caller's own arrays. optSolver reads them in place and
copies nothing before handing them to the backend (the backends copy into their own models, as
they do from Rust). The gateway creates a backend environment once — the licence is checked
once — and solves many problems with it, then frees it.

**Why this priority**: performance and memory come first in this project (constitution I, IV).
COBRA's `testAll` solves thousands of problems; creating an environment per solve would dominate
its run time, and a copy of a genome-scale matrix doubles the caller's footprint.

**Independent Test**: (a) a test shows the problem optSolver solves refers to exactly the caller's
arrays (same addresses, no allocation proportional to the number of rows, columns or nonzeros by
the C layer); (b) a test solves a sequence of problems through one environment handle and checks
the results equal those of separate one-shot solves, and that the licence is acquired once.

**Acceptance Scenarios**:

1. **Given** column pointers as unsigned machine-size integers and row indices as either unsigned
   machine-size integers or 32-bit signed integers, **When** a problem is built through the C
   interface, **Then** the problem refers to the caller's arrays in place, for every array (matrix,
   objective, bounds, row data, quadratic matrix, conic data).
2. **Given** an environment handle, **When** twenty problems are solved through it in sequence,
   **Then** each result equals the one-shot result for that problem, and the environment is
   created once.
3. **Given** a caller that does not want to manage an environment, **When** it calls the one-shot
   solve, **Then** the solve works, creating and freeing an environment internally.
4. **Given** caller-supplied output arrays of the right lengths, **When** a solve writes into them,
   **Then** the primal solution, row duals, reduced costs, bound multipliers, conic duals and row
   activity are written there by the backend, with no library-owned copy (so a MEX gateway can
   pass the data arrays of the output matrices it has already created).

---

### User Story 3 - Fail with a status, never a killed process, when a problem is too big (Priority: P2)

The caller sets a memory limit. A problem that would exceed it ends the solve with a status (or,
where the solver can only stop with an error, an error code and message) instead of the operating
system killing the caller's whole process — which, for COBRA under RunMat, is the whole test run.

**Why this priority**: COBRA's `testAll` contains genome-scale models; one killed process loses a
multi-hour run. The COBRA contract (`optsolver-mex.md`, `memLimitGB`) depends on it.

**Independent Test**: on Gurobi, a solve with a memory limit far below what a toy-sized problem
needs returns the memory-limit outcome; without the limit the same problem solves.

**Acceptance Scenarios**:

1. **Given** a memory limit the solve exceeds on Gurobi, **When** solved, **Then** the result
   reports "stopped at a limit" with the limit reason "memory", and the process continues.
2. **Given** a memory limit on MOSEK, **When** solved, **Then** the call fails with the
   "unsupported" error naming the memory-limit field and the MOSEK backend, before any solver
   call (memory-limit requirement, FR-011) — never silently ignored (constitution XVI-C).
3. **Given** the memory limit set through the Rust API instead, **When** solved, **Then** the
   outcome is the same: the limit is a solver-independent option of the Rust API, not only of the
   C interface.

---

### User Story 4 - Conic problems from C (Priority: P2)

A C programmer adds affine conic constraints Fx + g in K (the cone kinds the Rust API accepts:
quadratic, rotated quadratic, primal and dual exponential, primal and dual power, nonnegative,
zero, positive semidefinite) and solves on a backend that supports the cone kinds used; a backend
that does not support a kind says so before solving, as in the Rust API.

**Why this priority**: the Rust API supports cones and the COBRA side registers MOSEK for conic
problems (its request REQ-003 routes `solveCobraCLP` to `optSolver_mosek`); it reuses everything
User Stories 1 and 2 build.

**Independent Test**: the conic toy instances solve through the C interface on each supporting
backend with results equal to the Rust API's; an unsupported cone kind gives "unsupported" naming
it.

**Acceptance Scenarios**:

1. **Given** a conic toy instance with cones described by kind codes, dimensions and (for power
   cones) exponents, **When** solved on MOSEK, **Then** the result, including the conic duals,
   matches the Rust API's.
2. **Given** a cone kind Gurobi does not accept, **When** solved on Gurobi, **Then** the error is
   "unsupported" and names the cone kind, before any solver call.

---

### User Story 5 - Errors, ownership and versioning a C caller can rely on (Priority: P1)

The gateway author needs to know, for every call, whether it succeeded, why not, and who frees
what. A Rust panic never crosses into C. The gateway checks at start-up that the library it loaded
implements the interface version it was compiled against, and asks which backends are compiled in
and usable.

**Why this priority**: a crash or a leak in a MEX gateway takes down the host runtime; a version
mismatch between a gateway and a rebuilt library must be detected, not discovered as memory
corruption.

**Independent Test**: C tests pass null pointers, wrong lengths and invalid data and check the
returned error code and message; a test forces an internal panic and checks it becomes an error
code; the version function returns the version the header declares; tests run under a leak
checker free every library-owned object.

**Acceptance Scenarios**:

1. **Given** any invalid input (null pointer where an array is required, wrong length, unsorted
   row indices, non-finite value), **When** a call is made, **Then** it returns the error code for
   "invalid input" and a message naming the input, before any solver call.
2. **Given** an internal panic, **When** it occurs inside a call, **Then** the call returns the
   "internal error" code with a message, and the process continues.
3. **Given** the header's declared interface version, **When** the library's version function is
   called, **Then** it returns the same version; the version changes by the rule in the versioning requirement (FR-015) whenever
   the interface changes.
4. **Given** a library built with only one backend, **When** the caller asks for capabilities,
   **Then** it learns which backends are compiled in, whether each is usable (library loaded,
   licence obtained) with a reason when not, the problem classes and cone kinds each supports, and
   its solver version.
5. **Given** every object the library allocates (environment handle, result, error message),
   **When** the caller frees it with the matching free function, **Then** no memory is leaked; and
   freeing a null pointer is a no-op.

### Edge Cases

- Empty problem (zero rows, or zero columns): handled as the Rust API handles it.
- A column with no nonzeros, infinite bounds, ranged rows: as the Rust API (the C layer adds no
  rule of its own).
- A dimension above what the solvers accept (row or column count beyond the 32-bit limit): the
  "too large" error, as in the Rust API.
- Row indices given as 32-bit integers that are negative: "invalid input", before any solver call.
- An output array supplied with the wrong length: the "invalid input" error naming the array,
  before any solver call, as in the Rust API. An output array for a quantity the solver did not
  return (for example duals of an infeasible problem): its entries are NaN (the C layer fills
  supplied arrays with NaN before solving) and the solution-returned value says whether a primal
  solution exists; never undefined content presented as data.
- An environment handle used after it has been freed, or used from two threads at once: documented
  as undefined behaviour the caller must avoid (the thread rule is stated in the thread-rule requirement, FR-008); the library
  does not attempt to detect use after free.
- A backend not compiled into the library is requested: the "unsupported" error naming the backend.
- A vendor shared library or licence absent at run time: environment creation fails with the
  solver error code and the vendor's message; capabilities report the backend as not usable with
  that reason.
- A string output (status names, messages, raw status names) with non-ASCII content: returned as
  UTF-8, null-terminated.
- Mixed-integer problems (variable types): out of scope while feature 004 is unmerged; a caller
  cannot pass variable types through this version of the interface (see Assumptions).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: optSolver MUST provide a C interface — a C header and a library — through which a C
  caller can do everything this specification lists without writing Rust. Every operation is a
  thin translation onto the existing solver-independent Rust API (problem, options, result,
  backend environments); the C layer MUST NOT re-implement validation, status translation,
  parameter application or verification.
- **FR-002**: Problem input through C MUST mirror the Rust API exactly, except variable types
  (out of scope; mixed-integer requirement, FR-020): an m-by-n matrix A in
  compressed sparse column form (column pointers as unsigned machine-size integers; row indices as
  unsigned machine-size integers or 32-bit signed integers; values as doubles); objective c;
  objective sense (minimise, maximise); objective constant; variable bounds l and u; row bounds
  either as one sense character and right-hand side per row, or as lower and upper row bounds; an
  optional quadratic matrix Q (lower triangle, compressed sparse column form, unsigned machine-size
  row indices); optional affine conic data F, g and the list of cones (kind, dimension or order,
  and exponents for power cones).
- **FR-003**: The C layer MUST NOT copy any caller input array before it reaches the Rust API; the
  problem the backend receives MUST refer to the caller's arrays in place. Any conversion a backend
  performs (for example of row indices to 32 bits) is the backend's existing, already-audited
  behaviour and is unchanged. Small per-problem descriptors (the list of cones, at most one entry
  per cone) MAY be built by the C layer, and the plan MUST state their size.
- **FR-004**: Options through C MUST mirror every shared field of the Rust options structure (time
  limit, iteration limit, threads, primal and dual feasibility tolerances, method, presolve, random
  seed, solver log, certificates, array outputs, verification) plus the memory limit of the memory-limit requirement (FR-011),
  each with an explicit "unset" state meaning "the solver's default", and the raw vendor parameter
  passthrough by name with an integer, double or string value. Precedence and conflict rules are
  those of the Rust API.
- **FR-005**: Every field of the Rust result MUST be retrievable through C: the status, the limit
  reason, what solution is returned, the primal solution, row duals, reduced costs, lower- and
  upper-bound multipliers, conic duals, row activity, basis, objective, bound (each "absent" state
  distinguishable from a value), iteration counts, the four phase timings, the certificate when
  requested, every raw status code by name, the solver name and version, extra solver attributes,
  the effective parameters, and the verification report (overall verdict, or "not run" when
  verification is switched off, and each measure with its name, absolute and relative value,
  tolerance, verdict and whether it is required).
- **FR-006**: Results MUST be obtainable in either of two ways: written by the backend directly
  into caller-supplied arrays, each passed with its length (the Rust API's caller-buffer path,
  with the NaN rule of the Clarifications), or held in a library-owned
  result object from which the caller reads them in place and which the caller frees with its
  free function. Neither path makes a copy of a result array that the Rust API does not make.
- **FR-007**: A caller MUST be able to create a backend environment handle (acquiring the licence
  once), solve any number of problems with it, and free it; and MUST also be able to solve once
  without managing a handle. Results through either path are identical for the same problem and
  options.
- **FR-008**: The header MUST document the thread rules, grounded in the vendors' documentation for
  the supported versions (Clarifications): an environment handle may be used from any thread,
  including one other than the thread that created it, but by one thread at a time; one handle
  MUST NOT be used by two threads at the same time; different handles may be used concurrently;
  creating and freeing Gurobi environment handles is serialised by the library.
- **FR-009**: No Rust panic MUST unwind across the C boundary. Every entry point MUST contain a
  panic and return an "internal error" code with a message.
- **FR-010**: Every fallible entry point MUST return an error code from one documented set that
  distinguishes at least: success; invalid input; too large; unsupported (field, problem class,
  cone kind or backend); parameter conflict; solver error (carrying the vendor's code); and
  internal error (contained panic). A human-readable message for the failure MUST be retrievable,
  and a failure on one thread or handle MUST NOT overwrite the message another is reading.
  Non-optimal outcomes (infeasible, unbounded, limits) are statuses, not errors (constitution
  VII-C).
- **FR-011**: A memory limit MUST be available as a solver-independent option of the Rust API,
  `memory_limit_gb` (gigabytes of 10^9 bytes), and hence through C. On Gurobi it maps to
  `SoftMemLimit` only, so that an over-size solve stops with status "stopped at a limit" and limit
  reason "memory" rather than exhausting the machine's memory; `MemLimit` remains available through
  the raw passthrough. On MOSEK — whose 11.2 C interface has no memory-limit parameter (checked
  2026-11-15 against `mosek.h` 11.2 and the 11.2 C API manual) and forbids calling MOSEK functions
  from the callback — setting it MUST fail with the "unsupported" error naming the field and the
  backend, before any solver call (Clarifications).
- **FR-012**: The library MUST be built as a static library and as a shared library from the
  existing crate, behind an additive Cargo feature that changes no behaviour of the Rust API
  (constitution VIII). The documentation MUST state the exact compile and link flags a caller uses
  (`-I`, `-L`, `-l` and the vendor libraries each form needs), the run-time library dependencies
  (Gurobi and MOSEK shared libraries and how they are located), and that the vendor libraries and
  licences are never shipped with optSolver (constitution X).
- **FR-013**: The header MUST be kept in agreement with the library by a test: a header that
  disagrees with the exported functions or types (names, argument types, struct layout, constant
  values) MUST fail the test suite.
- **FR-014**: C-level tests MUST compile and run a C program against the header and the library
  inside the project's test harness, on the LP, QP and conic toy instances for each available
  backend, comparing every result field with the Rust API's result for the same instance and
  options, by the comparison rule in the Clarifications, implemented once in the shared test module
  and reused for every instance and backend (constitution II). Tests needing a licence are skipped visibly where the licence is absent;
  validation, error-code, panic-containment and version tests need no licence and run everywhere.
- **FR-015**: The interface MUST carry a version, declared once in the header and returned by a
  library function, with a documented rule: an incompatible change (a removed or changed function,
  type, field, constant meaning or ownership rule) increments the major part; a compatible
  addition increments the minor part. A caller compiled against a given major and minor version works with any library of the same major
  version and an equal or higher minor version.
- **FR-016**: Ownership MUST be explicit for every pointer that crosses the boundary: inputs are
  borrowed for the duration of the call only and never retained after it returns (unless a later
  feature introduces a persistent loaded model, which would state otherwise); every object or
  string the library allocates has exactly one documented free function; freeing null is a no-op.
- **FR-017**: A capabilities query MUST report, per backend compiled into the library: whether it
  is usable now (with the reason when not), the problem classes it supports, the cone kinds it
  accepts, the shared options it honours, and its solver name and version — the information of the
  Rust API's capabilities — so the COBRA gateway can fill its `capabilities` reply without
  solving.
- **FR-018**: The C interface MUST NOT write to standard output or standard error; the solver log,
  when requested, goes where the Rust API sends it (constitution III).
- **FR-019**: The interface document (`docs/llm-interface.md`, the single public-interface
  document) MUST describe the C interface, and its drift test MUST cover every exported C
  function, type and constant.
- **FR-020**: Mixed-integer problems are out of scope while feature 004 is unmerged. The interface
  MUST leave a compatible extension point (adding variable types and mixed-integer options later
  is a minor version change under the versioning requirement, FR-015). The extension mechanism
  is that input structures begin with their own size, so a later minor version appends fields; a
  test shows that a caller structure of an earlier, shorter size is accepted.
- **FR-021**: The system MUST preserve the documented Rust public interface and its status and
  error semantics. The Rust-API changes are: the memory-limit option of the memory-limit
  requirement (FR-011), added without breaking any existing caller (the options structure is
  already non-exhaustive); and `gurobi::Env` and `mosek::Env` becoming `Send` (not `Sync`), with
  Gurobi environment creation and freeing serialised (Clarifications), which only widens what
  callers may do. The interface document's Threading section is updated accordingly.
- **FR-022**: The narrowest checks that prove the behaviour are: the unlicensed C test (version,
  validation errors, panic containment, header agreement) and the licensed C toy-instance
  comparison on each backend.

### Key Entities

- **C problem description**: the caller's borrowed arrays and dimensions for A, c, the objective
  sense and constant, l, u, the row bounds in one of the two forms, the optional Q, and the
  optional conic data; maps one-to-one onto the Rust problem.
- **C options**: every shared options field with an "unset" state, the memory limit, and the raw
  vendor parameter list; maps onto the Rust options structure.
- **Environment handle**: an opaque owner of one backend environment (licence acquired once);
  created and freed by the caller.
- **Result object**: an opaque, library-owned holder of one Rust result, read through accessors,
  freed by the caller; or, on the caller-buffer path, the arrays the caller supplied plus the same
  scalar and descriptive fields.
- **Error**: a code from one set, plus a retrievable message.
- **Interface version**: major and minor numbers, in the header and returned by the library.
- **Capabilities**: per compiled-in backend, usability with reason, problem classes, cone kinds,
  honoured options, solver identity.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For every LP, QP and conic toy instance that a backend supports, the C interface's
  result on that backend agrees with the Rust API's by the comparison rule in the Clarifications
  (identical status, limit reason, solution returned, verification verdict and failed-measure
  names; objective, bound and every returned vector within 10 τ relative), on the owned-result and
  the caller-buffer paths. Checked on every run of the licensed tier (N = 1 per instance, path and
  backend); bit-identity reported.
- **SC-002**: The C layer adds no allocation proportional to m, n or the number of nonzeros on the
  input path: on `iDopaNeuroC_problem_LP` (LP, m = 1344, n = 2942, nnz(A) = 13748, Gurobi 13.0.3
  and MOSEK 11.2), the peak heap memory owned by optSolver (heap profiler) of a solve through the
  C interface exceeds that of the same solve through the Rust API by at most 4 KiB, measured in a
  process dedicated to each scenario; allocation is deterministic, so N = 3 runs with their spread
  are reported. The input-pointer identity test is the positive control showing the zero-copy path
  is the one taken (constitution XIV-D).
- **SC-003**: On the same instance and backends, the end-to-end wall time of a solve call through
  the C interface (owned-result path, including the result accessors) exceeds the Rust API's by
  at most 1 % or 1 ms, whichever is larger, median of N = 5 warm runs with spread; the per-phase
  timings (load, solve, extraction) the result reports are recorded for both paths.
- **SC-004**: Solving every toy instance a backend supports in sequence through one environment
  handle creates the environment once (checked by the library's environment-creation count) and
  gives, per instance, the same status, solution-returned value and verification verdict as a
  one-shot solve, with objective and vectors equal within the comparison tolerance; per backend,
  N = 1 sequence (deterministic solves). Wall time of the sequence and of the one-shot solves is
  recorded (informational, N = 1).
- **SC-005**: On Gurobi, a solve with a memory limit below the instance's requirement returns the
  memory-limit outcome (status "stopped at a limit", limit reason "memory") and the test process
  continues; on MOSEK the memory limit gives the "unsupported" error naming the field, shown by a
  test (N = 1, deterministic outcome).
- **SC-006**: Every invalid-input case of the existing Rust failing-input tests that a C caller can
  express returns the "invalid input" (or "too large") code with a message naming the input, before
  any solver call; a forced panic returns the "internal error" code; zero leaks are reported by a
  leak checker over the C test program.
- **SC-007**: A minimal C program built with the system C compiler using only the compile and link
  flags the documentation states solves the LP toy instance, for the static and for the shared
  library — demonstrated once on the reference machine (N = 1), recorded with the exact commands.
  The same flags passed to `runmat mex` are demonstrated by the consumer (the COBRA gateway build);
  on the reference machine the RunMat binary does not start (missing `libhdf5.so.103`, a known
  COBRA-side request), so that demonstration is outside this feature.

## Assumptions

- The first consumer is the COBRA Toolbox MEX gateway `optsolver_solve`
  (`src/base/solvers/optSolver/optsolver_solve.c` in the COBRA repository, contract
  `contracts/optsolver-mex.md` of COBRA feature `20261115-173718-runmat-testall-optsolver`); its
  fields (`abiVersion`, per-backend `available`, `reason`, problem classes, cones, `version`;
  problem fields; options `primalFeasTol`, `dualFeasTol`, `timeLimit`, `solverLog`, `memLimitGB`,
  raw parameters; result fields) are all covered by the functional requirements on problem input, options, results,
  environments, errors and capabilities (FR-002 to FR-017). MATLAB sparse indices
  (`mwIndex`) are unsigned machine-size integers on 64-bit platforms, the Rust API's native row
  index type, so the gateway needs no index conversion.
- Supported platforms are the 64-bit targets the crate already supports; the header uses
  fixed-width and `size_t` types only.
- Cancellation and progress callbacks are out of scope for this version; they are a compatible
  addition later (versioning requirement, FR-015).
- Persistent loaded models (modify and re-solve without reloading, constitution I) do not exist in
  the Rust API yet; when they are added, their C exposure is a later minor version.
- A library built with a backend feature links that vendor's shared library; a caller using such a
  library needs that vendor library present at load time even if it uses only the other backend.
  Loading vendor libraries lazily at run time is not in scope.
- Where the request (REQ-001) lives in this repository: the COBRA process asks for
  `requests/from-cobratoolbox/REQ-001-*.md`, a destination optSolver's placement document does not
  yet name; per constitution IX-F the plan amends the placement document first, and the request
  text is also quoted in this spec's Input.
- Environment-creation counting (environment-reuse criterion, SC-004) and forced-panic containment
  (error-handling criterion, SC-006) need two
  test-support entry points in the C interface; they are documented as such and change no
  behaviour of the solving functions.
