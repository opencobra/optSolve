# Research: Solver-Independent Core And Gurobi LP Backend

Phase 0 of the plan. Facts marked **header** were read from the installed Gurobi 13.0.3
`gurobi_c.h` on 2026-11-14; facts marked **verify** rest on vendor documentation not re-read in
this session and are checked by a task before they are relied on. Closes the routed checklist
items CHK011, CHK015, CHK022, CHK023, CHK024, CHK027, CHK028 (and the Gurobi half of CHK006).

## R1. Bindings: own declarations, not an existing crate (decision 25b)

- **Decision**: optSolver declares the Gurobi C functions it uses itself (about 25), in one FFI
  module, linked at build time against `libgurobi130`.
- **Rationale**: constitution IV allows an existing crate only if it exposes the bulk calls on
  borrowed slices. Assessed on 2026-11-14 from the crates' published sources:
  - `gurobi-rs` 1.1.0 (MIT): builds one `Var` object per variable and adds variables through
    iterators (`Model::add_vars`, `add_constrs`); no bulk CSC load. Fails IV.
  - `gurobi-sys` 0.3.0 (MIT): `GRBXloadmodel` is commented out; `GRBaddvars` declared with
    32-bit `vbeg`. Fails IV (no 64-bit bulk load).
  - `grb` 3.0.1 / `grb-sys` 0.2.1 (MIT): no `GRBX*` bulk call found in their sources. Fails IV.
- **Alternatives**: patching `gurobi-sys` (an external fork to maintain); `bindgen` at build time
  (libclang on every build machine; generated text derived from vendor headers, constitution
  VII-E). A header-checked test (R12) gives the safety of generation without either cost.

## R2. Loading: `GRBXloadmodel` in one call, indices through a fixed buffer

- **Header**: `GRBXloadmodel(env, &model, name, int numvars, int numconstrs, int objsense,
  double objcon, double *obj, char *sense, double *rhs, size_t *vbeg, int *vlen, int *vind,
  double *vval, double *lb, double *ub, char *vtype, char **varnames, char **constrnames)`;
  `GRBXaddvars(model, int numvars, size_t numnz, size_t *vbeg, int *vind, double *vval, double
  *obj, double *lb, double *ub, char *vtype, char **varnames)`.
- **Decision**: with `i32` row indices from the caller, load everything in one `GRBXloadmodel`
  call with zero copies of A. With `usize` indices, create the model with `GRBXloadmodel` and
  zero columns, then add columns in blocks with `GRBXaddvars`, converting each block's indices
  into one reused `i32` buffer of fixed capacity (default 2^18 entries = 1 MiB), with
  per-block rebased `vbeg` in a buffer of at most block-columns + 1 entries. This is what SC-004
  requires (no term proportional to nonzeros). `vlen` is not needed with `GRBXaddvars`.
- **Verify**: whether incremental `GRBXaddvars` costs Gurobi more time than one
  `GRBXloadmodel` (lazy update); measured by the benchmark positive control (US4-2), which also
  records which path ran.
- **Header**: `GRBXloadmodel` declares inputs non-`const`. **Verify** in the Gurobi reference
  manual that every array argument is input-only before passing pointers derived from shared
  slices (constitution VII-A); task T-verify-const.

## R3. Ranged rows through Gurobi (CHK027)

- **Header**: `GRBXloadmodel` accepts only sense + right-hand side; ranged rows exist as
  `GRBaddrangeconstr(s)`, which add an auxiliary variable per ranged row (**verify** in manual).
- **Decision**: rows given as ranges map as follows: lower = upper → `=`; lower = −∞ → `<` upper;
  upper = +∞ → `>` lower; both infinite → `<` with right-hand side `GRB_INFINITY` (a free row);
  both finite, lower < upper → loaded as `=` with right-hand side 0 after appending one
  auxiliary column per such row (coefficient −1 in that row, bounds [lower, upper]). Auxiliary
  columns are appended after the caller's n columns and removed from every result array.
  Memory O(number of ranged rows). The iDopaNeuroC corpus has none (all rows equalities); a
  small hand-built fixture covers them.
- **Alternative**: rejecting ranges in the Gurobi backend (contradicts FR-005 and decision 3).

## R4. Status translation (FR-017, CHK004 Gurobi half)

**Header** codes 1–19 and their translation (data-model.md defines the vocabulary):

| Gurobi | Code | Solver-independent status | Limit reason |
|---|---|---|---|
| LOADED | 1 | NotSolved | — |
| OPTIMAL | 2 | Optimal | — |
| INFEASIBLE | 3 | Infeasible | — |
| INF_OR_UNBD | 4 | InfeasibleOrUnbounded | — |
| UNBOUNDED | 5 | Unbounded | — |
| CUTOFF | 6 | StoppedAtLimit | ObjectiveCutoff |
| ITERATION_LIMIT | 7 | StoppedAtLimit | Iterations |
| NODE_LIMIT | 8 | StoppedAtLimit | Nodes |
| TIME_LIMIT | 9 | StoppedAtLimit | Time |
| SOLUTION_LIMIT | 10 | StoppedAtLimit | Solutions |
| INTERRUPTED | 11 | Interrupted | — |
| NUMERIC | 12 | NumericalDifficulty | — |
| SUBOPTIMAL | 13 | NumericalDifficulty | — (solution field says Feasible when one exists) |
| INPROGRESS | 14 | NotSolved | — |
| USER_OBJ_LIMIT | 15 | StoppedAtLimit | ObjectiveTarget |
| WORK_LIMIT | 16 | StoppedAtLimit | Work |
| MEM_LIMIT | 17 | StoppedAtLimit | Memory |
| LOCALLY_OPTIMAL | 18 | LocallyOptimal | — |
| LOCALLY_INFEASIBLE | 19 | LocallyInfeasible | — |
| any other | — | Unknown | — (raw code kept) |

The "solution returned" field comes from the `SolCount` attribute and the status: Optimal when
the status is Optimal, Feasible when `SolCount` > 0 otherwise, None when `SolCount` = 0.

## R5. Shared options and their Gurobi mapping (CHK006 Gurobi half, CHK011)

**Header** parameter names. Mapping applied only by the backend's parameter-applying function
(constitution VII-G):

| Shared field | Gurobi parameter | Notes |
|---|---|---|
| time limit (seconds) | `TimeLimit` | |
| iteration limit | `IterationLimit` | simplex iterations |
| threads | `Threads` | |
| primal feasibility tolerance | `FeasibilityTol` | absolute, on the solver's scaled model (**verify**) |
| dual feasibility tolerance | `OptimalityTol` | absolute reduced-cost tolerance (**verify**) |
| method | `Method` | Auto −1, PrimalSimplex 0, DualSimplex 1, Barrier 2, Concurrent 3 |
| presolve | `Presolve` | Auto −1, Off 0, Conservative 1, Aggressive 2 |
| random seed | `Seed` | |
| solver log | `OutputFlag` | optSolver default override: 0 (constitution III), recorded in the options module |
| certificates requested | `InfUnbdInfo` = 1 | also sets `DualReductions` = 0 so infeasible and unbounded are distinguished (**verify**) |

Verification tolerances (CHK011): row-bound and variable-bound violations are judged against
the effective `FeasibilityTol`; dual sign feasibility, complementary slackness and the relative
primal-dual gap against the effective `OptimalityTol`; integrality (later, MILP) against
`IntFeasTol`; each times the factor 10 (spec clarification 2). Effective values are read back
from the model's environment after parameters are applied.

## R6. Certificates

- **Header** attributes `FarkasDual`, `FarkasProof` (infeasible), `UnbdRay` (unbounded),
  available when `InfUnbdInfo` = 1 (**verify** availability conditions per algorithm).
- **Decision**: returned in the result when certificates are requested; verified by the
  solver-independent verifier (data-model.md, Verification).

## R7. Callbacks: cancellation and progress (FR-019)

- **Header**: `GRBsetcallbackfunc(model, cb, usrdata)`, `GRBcbget(cbdata, where, what, result)`,
  `GRBterminate(model)`; where-codes `GRB_CB_POLLING` 0, `GRB_CB_SIMPLEX` 2, `GRB_CB_BARRIER` 7;
  what-codes `GRB_CB_RUNTIME` 6002, `GRB_CB_SPX_OBJVAL` 2001, `GRB_CB_SPX_ITRCNT` 2000,
  `GRB_CB_BARRIER_PRIMOBJ` 7002, `GRB_CB_BARRIER_DUALOBJ` 7003.
- **Decision**: one internal `extern "C"` callback, installed only when a cancellation flag or
  progress closure is supplied; it reads the flag (relaxed atomic load), calls `GRBterminate`
  when set, and passes solver-independent progress (elapsed seconds, primal objective, dual
  objective or bound, iteration count) to the closure inside `catch_unwind` (constitution VII-C).
  A caught panic terminates the solve and is resumed after `GRBoptimize` returns.

## R8. Memory baseline for SC-004 (CHK022)

- **Header**: model attributes `MemUsed` and `MaxMemUsed` (GB) report Gurobi's own allocation in
  the master environment.
- **Decision**: "memory optSolver adds" is measured two ways in a dedicated benchmark process:
  (1) primary, exact: the peak of optSolver's Rust heap allocations during load, counted by a
  counting global allocator installed in the benchmark binary only; (2) cross-check: rise in
  process peak resident memory (`VmHWM`) across the load minus Gurobi's `MaxMemUsed`. SC-004 is
  judged on (1); (2) is reported.

## R9. Timing for SC-005 (CHK023)

- **Decision**: "Gurobi solve time" is the wall-clock time of the `GRBoptimize` call measured
  with a monotonic clock (Gurobi's `Runtime` attribute recorded beside it). optSolver's added
  time is the wall-clock sum of validation and index conversion, result extraction, and
  verification. Phases are timed separately, N = 5, median and spread reported (constitution
  XIV-C, XV-A).

## R10. Deterministic settings for corpus tests and benchmarks (CHK024)

- **Decision**: corpus tests and benchmarks set, through the options structure, Method =
  DualSimplex, Threads = 1, Seed = 0, Presolve = Auto. Gurobi is deterministic for identical
  parameters, threads and machine (**verify**). One additional corpus test runs with an empty
  options structure (vendor defaults plus the audited log override) to exercise defaults.
- **Solver configuration audit (constitution I)**: the corpus is small (≤ 2,942 columns, ≤ 13,748
  nonzeros), sparse, all equality rows, all bounds finite. No Gurobi default was found to be
  mismatched to this profile; the only override optSolver imposes by default is `OutputFlag` = 0.
  Revisit for larger instances.

## R11. Fixture extraction and MPS conventions (CHK014, CHK028)

- **Decision**: a MATLAB script in optSolver (destination proposed in the plan, Artifact
  placement) that reuses existing code: the variational kinetics corpus locator
  `vkProblemInstanceDir` (variational kinetics repository, `src/matlab/experiment/corpus/`, at
  commit `f164a3ac`) to find the instances, MATLAB `load` to read each `model`, and Gurobi's
  MATLAB `gurobi_write` to write the MPS file from a Gurobi model structure built field by field
  from the COBRA fields (A = S, rhs = b, sense = csense with E→'=', L→'<', G→'>', lb, ub,
  obj = c, modelsense from osenseStr). After writing, the script reads the file back with
  `gurobi_read` and requires every array to be bit-for-bit identical; it then writes the
  provenance record as JSON beside the MPS file.
- **MPS conventions**: the free-format MPS that Gurobi writes, with `OBJSENSE`, `ROWS`,
  `COLUMNS`, `RHS`, `RANGES` and `BOUNDS` sections (**verify** exact section usage against the
  first written file); full double precision is required and checked by the round trip.
- **Rust side**: a small MPS reader in the shared test module (test-only code), covering exactly
  the sections above; no new dependency.
- **Alternatives**: COBRA `writeLPProblem` / `convertCobraLP2mps` (considered; their numeric
  formatting precision not established, and the round trip would still be needed); reading the
  `.mat` files directly from Rust (HDF5 dependency for tests; rejected in decision "Format").

## R12. Header conformance test

- **Decision**: a test, compiled only with the `gurobi` feature and run only where
  `GUROBI_HOME` points at an installed header, compiles a small C file against `gurobi_c.h` that
  takes the address of every declared function into a typed pointer and checks every constant
  optSolver uses (status codes, `GRB_INFINITY`, callback codes); mismatch fails the build of
  that test. Implemented with the `cc` crate as a dev-dependency (**verify** the approach in the
  plan's first task; fall back to `ctest2` if needed).

## R13. Seeded objectives (CHK015)

- **Decision**: a SplitMix64 generator implemented in the shared test module (about ten lines,
  integer arithmetic only, so identical on every platform); each objective entry is
  `(next_u64() >> 11) as f64 * 2^-53 * 2 - 1`, uniform on [−1, 1). Seeds per instance: 1, 2 and
  3, recorded in each provenance record. Objective sense: maximise, as stored in the instances.
- **Rationale**: avoids depending on a random-number crate whose float conversion could change
  between versions. All corpus bounds are finite, so every seeded objective gives a bounded LP.

## R14. Test harness and skipping (CHK016)

- **Decision**: corpus and contract tests use a custom test harness (`harness = false`) built
  with the `libtest-mimic` dev-dependency. At start-up it lists the backends compiled in
  (Cargo features) and, for each, whether an environment can be created (licence present). A
  backend is **available** when both hold. Each test is registered once per declared backend;
  for an unavailable backend the trial is registered as ignored, with the reason in its name, so
  `cargo test` reports it as ignored, never as passed (SC-002).
- **Alternative**: `#[ignore]` attributes (static; cannot depend on a run-time licence check).

## R15. Thread safety (decision 21)

- **Decision**: `Env` and models are not `Send` or `Sync` in this feature. One environment per
  thread is achievable by creating it on that thread. Making them `Send` waits for the vendor
  documentation check (constitution VII-B).
