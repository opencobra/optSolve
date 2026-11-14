# Open questions and suggested first features

## To decide

1. **Crate and repository name.** The repository is `optSolver`; the crate name
   (for example `optsolver`) and whether it is one crate with features or a
   workspace (`optsolver-core`, `optsolver-gurobi`, `optsolver-mosek`) is open.
2. **Bindings.** Generate our own `-sys` bindings with `bindgen` from the
   installed headers, or depend on `gurobi-rs` (raw layer, `gurobi13`) and
   the official `mosek` 11.2 crate. Criterion: access to the bulk loaders
   (`GRBXloadmodel`, `GRBXaddvars`, `MSK_putacolslice64`) without per-element
   wrapper overhead.
3. **Licence** of optSolver (proposed MIT OR Apache-2.0, as RunMat and
   `oximo`).
4. **Problem classes in scope for version 1**: LP and MILP first; QP (convex)
   next; QCP/SOCP later?
5. **Result content**: primal, duals, reduced costs, slacks, objective, raw
   and normalised status, MIP gap and bound, timings; whether basis status
   (Gurobi `VBasis`/`CBasis`, MOSEK `skc`/`skx`) is needed (COBRA usage to be
   checked).
6. **Index chunk size** for the 32-bit index conversion, and whether to offer
   a no-chunk path when memory is plentiful.
7. **Threading and environments**: one environment per process or per caller;
   thread safety of reusing `GRBenv` / `MSKenv` across solves.
8. **Where the RunMat adapter lives**: in the RunMat repository behind a
   feature, or in a separate crate depending on both.

## To verify

- Every C function used, against the vendors' documentation for Gurobi 13.0.3
  and MOSEK 11.2 (signatures, index widths, infinity handling).
- RunMat numeric array constructors take ownership of a `Vec<f64>` without
  copying (index buffers do: `HostIndexBuffer::new(Vec)` →
  `HostData::from_vec`).
- The fields of the Gurobi `model`/`params`/result structs and the MOSEK
  `prob`/`res` structs that COBRA's `solveCobraLP`, `solveCobraQP` and
  `solveCobraMILP` read and write. This list is the specification for the
  RunMat adapter.
- COBRA finding X13 (free solvers fail validation under RunMat): diagnose in
  the RunMat repository before relying on end-to-end COBRA tests.

## Benchmarks wanted

- Load time and peak resident memory for large models (for example Recon3D
  and larger genome-scale models) through: optSolver; the vendor MEX in MATLAB
  (outside this repository); `oximo` via its MPS reader.
- Environment creation cost versus model load cost.

## Suggested first features (SpecKit)

1. `/speckit-constitution`: principles drawn from this exploration
   (performance and memory first, zero-copy borrowed inputs, no RunMat
   dependency, solver backends behind features, provenance and licence
   recording, benchmarks as acceptance criteria).
2. Feature 001: core types (`LpRef`, row-bound forms, `Solution`, status) and
   the Gurobi LP/MILP backend with bulk load and environment reuse.
3. Feature 002: MOSEK LP/MILP backend.
4. Feature 003: QP for both.
5. Later: HiGHS, SCIP, Clarabel backends; the RunMat adapter (RunMat side).

## Added 2026-11-14 with constitution 2.1.0

Raised by Principle XVI (solver-independent contract) and the review of the constitution.
Each is for a feature to decide.

### Generic problem input (XVI-A)

1. Representation: a struct of borrowed slices, a trait that lends slices (so `sprs`, `faer` or
   RunMat types need no adapter struct), or a builder.
2. Index type: `usize` only, or generic over `usize`/`i32` so 32-bit callers skip conversion.
3. Row bounds: sense + right-hand side, lower/upper ranges, or both (constitution IV requires
   both native forms to be accepted).
4. Policy for unsorted or duplicate row indices within a column: reject, sort, or sum.
5. Objective sense, objective constant, and how QP, MILP and later conic data extend the type
   without breaking LP callers.

### Generic result output (XVI-A)

6. Owned `Vec`s, caller-supplied buffers, or both.
7. Which outputs are optional and requested in the options structure: duals, reduced costs,
   slacks, basis status, MIP bound and gap, iteration and node counts, timings.
8. Infeasibility and unboundedness certificates (Farkas and unbounded rays): needed for
   XVI-D to verify those statuses; Gurobi and MOSEK both can supply them.
9. Shape of the solver-specific extension field.

### Status vocabulary (XVI-B)

10. The exact names, beyond the minimum set in XVI-B: separate limit reasons (time, iteration,
    node, solution, objective) or one "limit" status with a reason field.
11. How "feasible solution returned" is represented (a field, or part of the status).
12. Whether MOSEK's separate problem status and solution status are both kept in the raw
    fields.

### Options structure (XVI-C)

13. The shared fields and their solver-independent definitions, especially tolerances, whose
    meanings differ between Gurobi and MOSEK.
14. The rule for a field a backend cannot honour: error, or warning recorded in the result.
15. Raw passthrough typing: string values only, or typed integer, double and string values.

### Verification (XVI-D)

16. Measures per problem class (LP, MILP, QP), and absolute versus scaled definitions (scaled by
    which norms).
17. Tolerances: independent of the solver, or derived from the effective solver tolerances.
18. Whether it runs by default during ordinary solves (it is O(nnz), small next to a solve).

### Lifecycle, concurrency and interface

19. One-shot `solve` only, or also a persistent model handle for modify-and-re-solve
    (constitution I requires modification without reload to be possible).
20. Warm-start inputs: initial point, basis.
21. Thread-safety model of environments and models per vendor (to be checked in vendor
    documentation).
22. Cancellation and progress callbacks.
23. Logging mechanism for diagnostics (constitution III).
24. Capability reporting: how a caller learns which problem classes and outputs a backend
    supports.
25. Bindings: link-time linking or run-time loading of vendor libraries; own bindings or
    existing crates (question 2 above).

## Decisions taken by the maintainer, 2026-11-14

Answers to the questions above (numbers refer to "Added 2026-11-14 with constitution 2.1.0"),
given interactively. Binding on feature 001 onwards as design inputs; the constitution
amendments they required are in constitution 2.2.0.

| Question | Decision |
|---|---|
| 1. Problem input representation | A plain struct of borrowed slices with one lifetime, validated once by a constructor; adapters for other crates are small optional functions that fill it |
| 2. Index type | Generic over `usize` and `i32` through a small sealed trait: `usize` converted (fused with validation, chunk-bounded), `i32` passed through with no copy |
| 3. Row bounds | Both forms: sense + right-hand side, or lower/upper ranges |
| 4. Unsorted or duplicate row indices | Rejected: row indices strictly increasing within each column |
| 5. Problem-class extension | One problem struct with optional fields (variable types for MILP, Q for QP, cones later); objective sense and constant always present. Householder notation for field names (A, Q, c, b, l, u, x) |
| 6. Result ownership | Both: `solve` returns adoptable `Vec`s; `solve_into` writes into caller buffers |
| 7. Optional outputs | All available outputs returned by default (constitution I amended accordingly) |
| 8. Certificates | Infeasibility and unboundedness certificates included on request; without them such statuses are reported unverified |
| 9. Solver-specific extension | A name/value list of extra solver attributes, alongside the raw status codes and solver identity |
| 10. Limits | One "stopped at a limit" status with a separate named reason field |
| 11. Feasible solution returned | A separate field (none, feasible, optimal), independent of the status |
| 12. Raw status | All raw codes the solver reports are kept (for MOSEK: problem status, solution status, response code, solution type used) |
| 13. Shared tolerances | Solver-independent definitions, with the per-backend mapping documented |
| 14. Unsupported option field | Error naming the field and backend, before solving |
| 15. Raw passthrough typing | Typed values: integer, double or string |
| 16. Verification scaling | Absolute, and relative to 1 + the infinity norm of the relevant data |
| 17. Verification tolerances | Judged against the effective solver tolerances (with a declared safety factor); raw numbers also reported |
| 18. Verification at solve time | On by default; an options field switches it off |
| 19. Lifecycle | One-shot `solve` in feature 001; persistent model with modify and re-solve in an early later feature, designed for from the start |
| 20. Warm start | Basis status and initial point (MIP start) inputs; requires a generic basis-status vocabulary |
| 21. Threads | One environment per thread; handles at most `Send`, never `Sync` |
| 22. Callbacks | Atomic cancel flag and a caller-supplied progress closure receiving generic progress data |
| 23. Logging | The `log` facade |
| 24. Capabilities | Each backend exposes a capabilities query; unsupported input is an error before any solver call |
| 25. Linking | Link-time linking first, binding layer shaped so run-time loading can replace it |
| 25. Bindings | Assessed per backend in feature 001 research against constitution IV; existing crate if it passes, otherwise own declarations |
| Licence (earlier question 3) | MIT OR Apache-2.0 |
| Crate layout (earlier question 1) | A single crate, `optsolver`, with backend modules behind features |

Still open: index-conversion chunk size (earlier question 6, to be set by benchmark); which
vendor documentation confirms the thread rules in decision 21.
