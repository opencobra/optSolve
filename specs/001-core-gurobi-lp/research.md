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
- **Header**: `GRBXloadmodel` declares inputs non-`const`. Checked 2026-11-14 (T038, R17.1):
  the Gurobi 13.0 reference manual (C API, Model Creation and Modification,
  https://docs.gurobi.com/projects/optimizer/en/13.0/reference/c/model.html) describes every array argument of `GRBXloadmodel` and
  `GRBXaddvars` only as data supplied "to initialize the model data" (objective coefficients,
  senses, bounds, CSC arrays for "the new variables"); no argument is described as an output,
  but the manual does **not** state explicitly that the arrays are input-only or `const`. A probe
  against 13.0.3 confirmed the arrays are unchanged after the call and are copied at call time
  (R17.1). **Resolved (maintainer, 2026-11-14)**: the descriptive wording plus the probe are
  accepted as the basis (constitution 2.4.1, VII-A); pointers are passed directly, and the probe
  is kept as a licensed test in the loader (task T042).

## R3. Ranged rows through Gurobi (CHK027)

- **Header**: `GRBXloadmodel` accepts only sense + right-hand side; ranged rows exist as
  `GRBaddrangeconstr(s)`, which add an auxiliary variable per ranged row. Confirmed 2026-11-14
  (Gurobi 13.0 reference manual, https://docs.gurobi.com/projects/optimizer/en/13.0/reference/c/model.html, `GRBaddrangeconstr`): "adding a
  range constraint to the model adds both a new constraint and a new variable"; ranges are
  "stored internally as equality constraints" whose Sense is always `GRB_EQUAL`, the extra
  variable capturing the range (R17.2).
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
| primal feasibility tolerance | `FeasibilityTol` | absolute (manual 13.0, Tolerances and User-Scaling, checked 2026-11-14); whether it is judged on the scaled model is not stated (R17.4) |
| dual feasibility tolerance | `OptimalityTol` | absolute reduced-cost tolerance (manual 13.0, checked 2026-11-14; R17.4) |
| method | `Method` | Auto −1, PrimalSimplex 0, DualSimplex 1, Barrier 2, Concurrent 3 |
| presolve | `Presolve` | Auto −1, Off 0, Conservative 1, Aggressive 2 |
| random seed | `Seed` | |
| solver log | `OutputFlag` | optSolver default override: 0 (constitution III), recorded in the options module |
| certificates requested | `InfUnbdInfo` = 1 | also sets `DualReductions` = 0 so infeasible and unbounded are distinguished (manual 13.0 checked 2026-11-14: `DualReductions` = 0 is the documented way to resolve INF_OR_UNBD; `InfUnbdInfo` does not itself do so; R17.3) |

Verification tolerances (CHK011): row-bound and variable-bound violations are judged against
the effective `FeasibilityTol`; dual sign feasibility, complementary slackness and the relative
primal-dual gap against the effective `OptimalityTol`; integrality (later, MILP) against
`IntFeasTol`; each times the factor 10 (spec clarification 2). Effective values are read back
from the model's environment after parameters are applied.

## R6. Certificates

- **Header** attributes `FarkasDual`, `FarkasProof` (infeasible), `UnbdRay` (unbounded),
  available when `InfUnbdInfo` = 1. Checked 2026-11-14 (manual 13.0, parameter `InfUnbdInfo`;
  attributes `FarkasDual`, `FarkasProof`, `UnbdRay`): computed only when infeasibility or
  unboundedness is determined "and a simplex basis is available (from simplex or crossover)";
  not available if barrier detects it before crossover; LP models only (R17.3).
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
  parameters, threads and machine, and Gurobi version; checked 2026-11-14 (Gurobi knowledge
  base "Is Gurobi deterministic?", https://support.gurobi.com/hc/en-us/articles/360031636051):
  deterministic including multiple threads, but Method = −1 (the default) may run
  non-deterministic concurrent LP, so Method ∈ {0, 1, 2, 4} is required; TimeLimit is not
  deterministic (WorkLimit is) (R17.5). One additional corpus test runs with an empty
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
  `COLUMNS`, `RHS`, `RANGES` and `BOUNDS` sections; full double precision is required and
  checked by the round trip. Observed on 2026-11-14 in `iDopaNeuroC_problem_1.mps` (task T016):
  a leading `* Signature:` comment line; `NAME matlab`; `OBJSENSE MAX` on the header line; the
  sections `ROWS`, `COLUMNS`, `RHS` (header only, since b = 0), `BOUNDS`, `ENDATA`; no
  `RANGES` (all rows equality) and no objective entries (c = 0); default names `R0…`, `C0…`,
  objective row `OBJ`, sets `RHS1` and `BND1`; bounds only as `LO` (written whenever l ≠ 0,
  before `UP`) and `UP` (every finite u); numbers in shortest exact form or 17 significant
  digits. All seven fixtures round-tripped bit for bit through `gurobi_read`, and the Rust
  reader reproduces the source arrays' SHA-256 hashes. Problems 4 and 5 yield identical MPS
  files (same LP), so the corpus holds six distinct LPs.
- **Rust side**: a small MPS reader in the shared test module (test-only code), covering exactly
  the sections above; no new dependency.
- **Alternatives**: COBRA `writeLPProblem` (calls `BuildMPS`, `external/base/io/BuildMPS/`):
  inspected 2026-11-14 at the maintainer's prompt. It writes fixed-width MPS with every number
  cut to 12 characters (about 11 significant digits) and reorders rows as L, negated G, then E.
  Rejected by the maintainer on 2026-11-14 in favour of `gurobi_write`, which keeps full
  precision and row order; reading the
  `.mat` files directly from Rust (HDF5 dependency for tests; rejected in decision "Format").

## R12. Header conformance test

- **Decision**: a test, compiled only with the `gurobi` feature and run only where
  `GUROBI_HOME` points at an installed header, compiles a small C file against `gurobi_c.h` that
  takes the address of every declared function into a typed pointer and checks every constant
  optSolver uses (status codes, `GRB_INFINITY`, callback codes); mismatch fails the build of
  that test. Implemented with the `cc` crate as a dev-dependency: confirmed 2026-11-14 (task
  T037) — `cc::Build::try_get_compiler` finds the compiler and the test runs it with
  `-fsyntax-only -Werror=incompatible-pointer-types -Werror`; a deliberately wrong pointer type
  and a wrong constant were both rejected. `ctest2` not needed.

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

## R16. Dependency licences (task T003, 2026-11-14)

Resolved by `cargo metadata` after adding `log` (run time) and `libtest-mimic`, `cc`
(dev-dependencies). All permit use in an MIT OR Apache-2.0 crate; only `log` is a run-time
dependency.

| Crate | Version | Licence |
|---|---|---|
| anstream | 1.0.0 | MIT OR Apache-2.0 |
| anstyle | 1.0.14 | MIT OR Apache-2.0 |
| anstyle-parse | 1.0.0 | MIT OR Apache-2.0 |
| anstyle-query | 1.1.5 | MIT OR Apache-2.0 |
| anstyle-wincon | 3.0.11 | MIT OR Apache-2.0 |
| cc | 1.11.2 | MIT OR Apache-2.0 |
| clap | 4.6.7 | MIT OR Apache-2.0 |
| clap_builder | 4.6.7 | MIT OR Apache-2.0 |
| clap_derive | 4.6.7 | MIT OR Apache-2.0 |
| clap_lex | 1.1.1 | MIT OR Apache-2.0 |
| colorchoice | 1.0.5 | MIT OR Apache-2.0 |
| escape8259 | 0.5.4 | MIT |
| find-msvc-tools | 0.1.15 | MIT OR Apache-2.0 |
| heck | 0.5.0 | MIT OR Apache-2.0 |
| is_terminal_polyfill | 1.70.2 | MIT OR Apache-2.0 |
| libtest-mimic | 0.8.2 | MIT/Apache-2.0 |
| log | 0.4.35 | MIT OR Apache-2.0 |
| once_cell_polyfill | 1.70.2 | MIT OR Apache-2.0 |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| shlex | 2.0.1 | MIT OR Apache-2.0 |
| strsim | 0.11.1 | MIT |
| syn | 3.0.10 | MIT OR Apache-2.0 |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| utf8parse | 0.2.2 | Apache-2.0 OR MIT |
| windows-link | 0.2.1 | MIT OR Apache-2.0 |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 |

## R17. Vendor documentation checks (task T038)

Checked on 2026-11-14 against the Gurobi Optimizer Reference Manual 13.0
(`https://docs.gurobi.com/projects/optimizer/en/13.0/`, pages "last updated on Nov 10, 2026"),
the installed 13.0.3 header, and, where the manual is silent, small C probes against the
installed 13.0.3 library (probe sources kept outside the repository; results only recorded here).

1. **Array arguments of `GRBXloadmodel` and `GRBXaddvars`** (reference/c/model.html). The manual
   describes `GRBXloadmodel` as creating a model "using the provided arguments to initialize the
   model data" and each array as values "for the new variables" or "for the new constraints";
   `GRBXaddvars` likewise. No argument is described as written by Gurobi, but no sentence says
   the arrays are input-only or `const`. The manual's signatures show `const char **varnames` /
   `constrnames`, the installed header `char **`; optSolver follows the header (VII-E) and passes
   null names. Probe (13.0.3): every array passed to `GRBXloadmodel` was bit-identical after the
   call; overwriting the arrays passed to `GRBXaddvars` before `GRBupdatemodel` did not change
   the model (coefficients, objective and bounds kept the values at call time), so the arrays are
   copied at call time and the fixed conversion buffer of R2 may be reused between calls.
   Status: documented implicitly, confirmed empirically; explicit input-only wording **open
   (maintainer)** under VII-A.
2. **Ranged rows** (`GRBaddrangeconstr(s)`, reference/c/model.html): a range constraint "adds
   both a new constraint and a new variable", is "stored internally as equality constraints",
   its Sense is always `GRB_EQUAL`, and it is equivalent to aᵀx + s = U with 0 ≤ s ≤ U − L. R3's
   own auxiliary-column mapping (coefficient −1, bounds [L, U], right-hand side 0) is the same
   construction under optSolver's control.
3. **`InfUnbdInfo`, `DualReductions`, certificates** (reference/parameters.html;
   reference/attributes/constraintlinear.html, variable.html, model.html). `InfUnbdInfo` (default
   0, LP only) makes `UnbdRay`, `FarkasDual` and `FarkasProof` available; the extra information
   is computed only when the model "is determined to be infeasible or unbounded, and a simplex
   basis is available (from simplex or crossover)", and not when barrier decides before
   crossover. `DualReductions` (default 1): disable it "if you received an optimization status of
   INF_OR_UNBD and would like a more definitive conclusion". So the R5 pairing
   (`InfUnbdInfo` = 1 with `DualReductions` = 0) is consistent with the manual; `InfUnbdInfo`
   alone does not prevent INF_OR_UNBD. Farkas sign convention: λᵢ ≥ 0 for ≤ rows, λᵢ ≤ 0 for ≥
   rows, ā = λᵀA, the certificate inequality λᵀAx ≤ λᵀb with β = FarkasProof > 0 the minimum
   violation over the bounds. `UnbdRay` is a ray that, added to any feasible solution, stays
   feasible and improves the objective.
4. **Tolerances** (concepts/numericguide/tolerances_scaling.html; reference/parameters.html).
   `FeasibilityTol`: aᵀx ≤ b holds if aᵀx − b ≤ FeasibilityTol; `OptimalityTol`: dual
   constraint aᵀy ≤ c holds if aᵀy − c ≤ OptimalityTol (for simplex and crossover, reduced costs
   "must all be smaller than OptimalityTol in the improving direction"). "These tolerances are
   absolute; they do not depend on the scale of the quantities involved." Scaled vs unscaled:
   not stated for the tolerances; `ScaleFlag` says rows and columns are scaled by default, "the
   scaling is removed before the final solution is returned", and scaling "may lead to larger
   constraint violations in the original, unscaled model". So "on the solver's scaled model" in
   R5 is an inference, not documented; the independent verifier's factor 10 (R5) covers it.
5. **Determinism** (Gurobi knowledge base, "Is Gurobi deterministic?",
   https://support.gurobi.com/hc/en-us/articles/360031636051; manual concepts/parameters/
   guidelines.html; attribute `Work`). Same model and parameters, same computer, same Gurobi
   version give the same solution and path, also with several threads. Exceptions: Method = −1
   can run non-deterministic concurrent LP ("use Method=0, 1, 2, or 4"); `TimeLimit` is not
   deterministic (`WorkLimit` is); variable or constraint order and any parameter change alter
   the path. R10's Method = DualSimplex, Threads = 1, Seed = 0 satisfies this.
6. **Sign conventions of `Pi` and `RC`** (reference/attributes/constraintlinear.html `Pi`;
   variable.html `RC`). `Pi`: for minimisation, duals of ≥ rows are ≥ 0 and of ≤ rows ≤ 0;
   "for models with a maximization sense, the senses of the dual values are reversed". `RC`: "the
   reduced cost in the current solution", no formula given. Probe (13.0.3, a 2 × 2 LP with an
   active upper bound, solved as both min and max): `RC = c − Aᵀ Pi` held exactly for both
   senses (for example max: Pi = (1, 0), RC = (0, 1)). So the identity holds for both senses
   with Gurobi's sign conventions; documented for `Pi`, empirical for `RC`.
7. **Thread safety** (reference/c/env.html, `GRBloadenv`/`GRBstartenv`; reference/c/model.html,
   `GRBcopymodeltoenv`; reference/c/callback.html, `GRBterminate`). "Environments are not thread
   safe. In this case, you will need a separate environment for each of your threads." "Note
   that environment initialization is itself not thread-safe" (create environments on one
   thread, or under a lock). "Multiple threads cannot work simultaneously within the same
   environment." `GRBterminate` "can be called at any time during an optimization (from a
   callback, from another thread, from an interrupt handler, etc.)". This supports R15 (`Env`
   and models neither `Send` nor `Sync`); a later `Send` (not `Sync`) `Env` would also need
   creation serialised by a lock.
8. **Environment creation and the licence banner** (reference/c/env.html). `GRBemptyenv` is for
   setting parameters "before actually starting the environment"; `GRBstartenv` "will apply all
   parameter changes specified by the user prior to this call"; if it fails "the environment
   will have the same state as it had before the call" (so it must still be freed).
   `GRBgeterrormsg` returns "the error message associated with the most recent error". Probe
   (13.0.3): `GRBstartenv` prints the licence banner ("Set parameter Username", licence ID and
   type) to standard output unless `OutputFlag` = 0 is set on the empty environment before
   starting; with that setting nothing is printed. `GRBemptyenv` succeeds without a licence.
9. **`GRBgetparamtype`**: declared in the header but not documented in the 13.0 C reference
   pages checked. Probe (13.0.3): returns 1 for an integer, 2 for a double, 3 for a string
   parameter and −1 for an unknown name; works on an unstarted environment. Recorded as
   observed behaviour in `src/gurobi/ffi.rs` (`PARAM_TYPE_*`), not as a header constant.

Still open: the explicit input-only wording for the `GRBX*` arrays (item 1, maintainer decision
under VII-A); whether tolerances are judged on the scaled model (item 4, not documented); the
R2 incremental-load cost and the R11 MPS section usage, which are measured or checked by later
tasks, not by documentation.

## R18. Which iDopaNeuroC instances are LPs (investigation, 2026-11-14)

- Read as COBRA flux-balance LPs (S v = b, lb ≤ v ≤ ub), problems 2, 3 and their full-rank
  variants are infeasible (Gurobi conflict set: row 34dhphe[e] requires v(34DHPHELAT1tc) =
  −0.0536 with lb 0); problems 1, 4, 5 are feasible.
- Cause: in `driver_optimizeVKmodel_VK1to3m.m` the single FBA LP (`optimizeCbModel`, line 862)
  runs before variational kinetics experiments 2 and 3 set b := N(v_f − v_r) from a generated
  kinetic steady state (lines ~1588, ~1755); the stored instances are the inputs to
  `optimizeVKmodel`, which uses `internalNetFluxBounds = 'none'`, so the stored lb/ub are not
  its constraints. COBRA `buildOptProblemFromModel` on each stored model gives exactly the
  fixture arrays; setting b = 0 on problems 2 and 3 is still infeasible.
- Decision (maintainer): the LP test corpus is iDopaNeuroC_problem_1 only, the flux-balance LP,
  with seeded random objectives. Problems 4 and 5 are variational kinetics problems too.
- Verification scaling: on problems 4 and 5 the duals reached ‖y‖∞ = 1.4e10 and the reduced-cost
  residual 4.3e-5 was cancellation (≈ 1e-12 of Σ|A_ij y_i| = 2.75e7); reduced-cost consistency
  is now scaled per column (data-model.md).
| dhat | 0.3.3 | MIT OR Apache-2.0 (dev-dependency added by task T048 for exact load-phase heap measurement without `unsafe` in optSolver) |
