# optSolver: interface for client authors and LLMs

`optsolver` solves optimisation problems that a program already holds in matrix form, through
the native interfaces of Gurobi and MOSEK, **without copying the caller's arrays** where the
solver allows it. One solver-independent problem, options and result type is shared by every
backend; every solution is independently verified against the problem.

This document is complete: it is written so that a reader (or an LLM) with no access to the
source can add the dependency and write correct client code. It is also the crate-level Rust
documentation, and every Rust example below is compiled and run by the test suite.

Contents: [Add the dependency](#add-the-dependency) ·
[Problem model and notation](#problem-model-and-notation) ·
[Building a problem without copying](#building-a-problem-without-copying) ·
[Quadratic objective](#quadratic-objective) · [Conic constraints](#conic-constraints) ·
[Backends and capabilities](#backends-and-capabilities) · [Options](#options) ·
[Solving](#solving) · [Statuses](#statuses-and-limit-reasons) ·
[Reading the solution](#reading-the-solution) · [Certificates](#certificates) ·
[Verification](#verification) · [Caller buffers](#caller-buffers) · [Errors](#errors) ·
[Threading](#threading) · [Examples](#complete-examples) ·
[Low-level items](#low-level-public-items)

## Add the dependency

The crate is `optsolver` (licence MIT OR Apache-2.0, Rust edition 2024, minimum Rust 1.85,
64-bit targets only). It is not on crates.io; depend on it by path or git:

```toml
[dependencies]
optsolver = { path = "../optSolver", features = ["mosek"] }   # or features = ["gurobi"], or both
```

Features (additive; with none, the crate builds and validates problems but cannot solve):

| Feature | Solver | Build-time variable | Run time |
|---|---|---|---|
| `gurobi` | Gurobi 13.0 | `GUROBI_HOME` = the `linux64` directory (library in `$GUROBI_HOME/lib`) | library directory on `LD_LIBRARY_PATH`; licence via `GRB_LICENSE_FILE` |
| `mosek` | MOSEK 11.2 | `MOSEK_BINDIR` = `…/mosek/11.2/tools/platform/linux64x86/bin` | that directory on `LD_LIBRARY_PATH`; licence via `MOSEKLM_LICENSE_FILE` |

Backend modules: `optsolver::gurobi` and `optsolver::mosek`, each exporting `Env`. A missing
licence is reported when the `Env` is created (`Error::Solver`), not later.

## Problem model and notation

A problem is

> optimise (σ-sense) ½ xᵀQx + cᵀx + γ
> subject to row bounds on Ax, l ≤ x ≤ u, and Fx + g ∈ K,

with A (m × n), c, x, l, u (length n), Q (n × n, optional), F (k × n) and g (length k, optional),
K a product of cones. σ = +1 for `Sense::Minimise` and −1 for `Sense::Maximise`
(`Sense::sigma`). Infinite bounds are `f64::INFINITY` / `f64::NEG_INFINITY`.

Duals follow one convention for every backend (stationarity):
**z = c + Qx − Aᵀy − Fᵀs**, with y the row duals, z the reduced costs, s the conic duals, and
sign conditions σy ≥ 0 on rows bounded below, σz ≥ 0 at lower bounds, σs ∈ K* (the dual cone).
z is also given split: z = z_l − z_u with σz_l ≥ 0 (lower-bound multipliers) and σz_u ≥ 0
(upper-bound multipliers).

## Building a problem without copying

Every array is **borrowed** from your program; a `Problem` never owns data. Build it in three
steps; each validates once (single pass) and returns `Error::Invalid` with the defective input
named:

1. The constraint matrix A as a `CscRef`: compressed sparse column form,
   `CscRef::new(m, n, col_ptrs, row_indices, values)`. Rules: `col_ptrs` has length n + 1,
   starts at 0, is non-decreasing and ends at nnz; `row_indices` and `values` have length nnz;
   row indices are in range and strictly increasing within each column (sorted, no duplicates);
   values are finite; m, n ≤ `MAX_DIMENSION` (i32::MAX). Accessors: `m`, `n`, `nnz`,
   `col_ptrs`, `row_indices`, `values`, `column`.
2. Row bounds as a `RowBounds`, either `RowBounds::SenseRhs { sense, b }` (bytes `b'='`, `b'<'`,
   `b'>'` with right-hand side b) or `RowBounds::Ranges { b_l, b_u }` (b_l ≤ Ax ≤ b_u).
   `RowBounds::bounds(i)` gives the canonical pair of row i; `len`, `is_empty`.
3. `Problem::new(A, c, sense, rows, l, u)`, then optionally `with_gamma(γ)`, `with_Q(Q)`,
   `with_conic(conic)`, `with_var_types(types)` (reserved for MILP, no backend supports it yet).
   Accessors: `A`, `c`, `gamma`, `sense`, `rows`, `l`, `u`, `var_types`, `Q`, `conic`, `m`,
   `n`, `k`.

**Row-index type** (`RowIndex`, sealed — implemented only for `usize` and `i32`, trait
`Sealed`): with `i32` row indices the arrays are passed to the solver as they are (zero copy);
with `usize` they are converted to 32-bit block by block through a fixed buffer (at most
`BLOCK_CAPACITY` entries). `RowIndex::index` and `RowIndex::as_i32_slice` expose this. Prefer
`i32` indices for the largest problems.

Copies the solver forces (and nothing else): MOSEK bound keys (4 B per row and column) and, only
when infinite bounds are present, bound values; Q as triplets (both solvers); on Gurobi, conic
rows through a fixed-size block buffer.

## Quadratic objective

`with_Q(Q)` takes Q as a `CscRef<usize>` holding **only the lower triangle** (entries with row ≥
column); the objective uses ½ xᵀQx with the symmetric matrix that triangle defines. An entry
above the diagonal is `Error::Invalid`. Convexity is not checked by optSolver: a non-convex Q
(minimise) is an `Error::Solver` from the solver (Gurobi code 10020, MOSEK 1295).

## Conic constraints

`ConicRef::new(F, g, cones)` with F a `CscRef<usize>` (k × n), g of length k and a slice of
`Cone`; the cone dimensions must sum to k, and cone i applies to the next block of rows of
Fx + g in order. Then `problem.with_conic(conic)` (F must have n columns). Accessors: `F`, `g`,
`cones`, `k`, `blocks` (each cone with its first row).

| `Cone` | Dimension | Set (u is the block of Fx + g) | `ConeKind` |
|---|---|---|---|
| `Cone::Quadratic(d)` | d ≥ 1 | u₁ ≥ ‖u₂:d‖₂ | `Quadratic` |
| `Cone::RotatedQuadratic(d)` | d ≥ 2 | 2u₁u₂ ≥ ‖u₃:d‖₂², u₁, u₂ ≥ 0 | `RotatedQuadratic` |
| `Cone::PrimalExponential` | 3 | u₁ ≥ u₂ exp(u₃/u₂), u₂ > 0 (closure) | `PrimalExponential` |
| `Cone::DualExponential` | 3 | u₁ ≥ −u₃ exp(u₂/u₃ − 1), u₃ < 0 (closure) | `DualExponential` |
| `Cone::PrimalPower { dim, alpha }` | dim > alpha.len() | ∏ u_i^{α_i} ≥ ‖tail‖₂, α normalised to sum 1 | `PrimalPower` |
| `Cone::DualPower { dim, alpha }` | as above | ∏ (u_i/α_i)^{α_i} ≥ ‖tail‖₂ | `DualPower` |
| `Cone::Nonnegative(d)` | d ≥ 1 | u ≥ 0 | `Nonnegative` |
| `Cone::Zero(d)` | d ≥ 1 | u = 0 | `Zero` |
| `Cone::PositiveSemidefinite { order }` | r(r+1)/2 | svec⁻¹(u) ⪰ 0 | `PositiveSemidefinite` |

The semidefinite layout (svec): the lower triangle of the r × r matrix X column by column
(X₁₁, X₂₁, …, X_r1, X₂₂, …), every off-diagonal entry multiplied by √2 (put the √2 in F).
`Cone::dim`, `Cone::kind`, `Cone::violation(u)` and `Cone::dual_violation(u)` (0 inside, > 0
outside); `ConeKind::name` gives a stable name, `ConeKind::ALL` lists every kind.

## Backends and capabilities

`optsolver::gurobi::Env::new()` or `optsolver::mosek::Env::new()` creates an environment
(licence checked out); `Env::version()` gives the library version. Both implement the trait
`Backend`, so client code can be written once against `&dyn Backend`. Reuse one `Env` for many
solves.

`backend.capabilities()` returns `Capabilities`: `backend` (name), `problem_classes`
(`ProblemClass::Lp`, `Qp`, `Conic`, `Milp`), `cones` (accepted `ConeKind`s), `options` (the
shared options fields it honours, each an `OptionMapping` with `field` and `vendor_parameters`),
`certificates`, `basis`, `solver` (a `SolverIdentity` with `name` and `version`).

| Backend | Classes | Cone kinds | Notes |
|---|---|---|---|
| `mosek` | LP, QP, conic | all | interior point for QP and conic (no basis); certificates for LP, QP, conic |
| `gurobi` | LP, QP, conic | quadratic, rotated quadratic, nonnegative, zero | QP and conic by barrier (no basis); **no certificates for QP and conic** |

A problem outside the backend's capabilities is rejected **before any solver call** with
`Error::Unsupported { field, backend }`, where `field` is `"Q"`, `"conic"`, `"var_types"`,
`"LP"` or a cone kind's name (for example `"primal exponential"`). The shared checks are public:
`optsolver::options::check::check_problem` and `optsolver::options::check::check`.

## Options

`Options` is `#[non_exhaustive]`: create it with `Options::default()` and assign fields. Every
field left unset means "solver default".

| Field | Meaning |
|---|---|
| `time_limit` | seconds |
| `iteration_limit` | simplex iterations |
| `threads` | solver threads |
| `primal_feasibility_tolerance`, `dual_feasibility_tolerance` | solver tolerances |
| `method` | `Method::Auto`, `PrimalSimplex`, `DualSimplex`, `Barrier`, `Concurrent` (Gurobi only) |
| `presolve` | `Presolve::Auto`, `Off`, `Conservative`, `Aggressive` (the last two Gurobi only) |
| `random_seed` | Gurobi only (MOSEK has none for continuous problems) |
| `solver_log` | forward the solver's log to the `log` crate (default off) |
| `certificates` | return infeasibility/unboundedness certificates |
| `array_outputs` | return y, z, s, row activity, basis (default on) |
| `verification` | run independent verification (default on) |
| `memory_limit_gb` | memory limit in gigabytes (10⁹ bytes). Gurobi: `SoftMemLimit` — the solve stops with `Status::StoppedAtLimit` and `LimitReason::Memory` (checked only where Gurobi can stop gracefully, so use may exceed the limit between checks; `MemLimit` stays available through `raw`). MOSEK: `Error::Unsupported` (MOSEK 11.2 has no memory limit) |
| `raw` | vendor parameters by name: `Vec<(String, ParamValue)>` with `ParamValue::Int`, `Double`, `Str` |
| `cancel` | `Option<&AtomicBool>`: set it to interrupt the solve |
| `progress` | `Option<&mut dyn FnMut(&Progress)>`: called during the solve |

A field the backend cannot honour is `Error::Unsupported` naming the field; the same vendor
parameter set by a field and by `raw` is `Error::ParameterConflict`. `set_shared_fields` lists the
fields set; `effective_solver_log`, `effective_array_outputs`, `effective_verification` give the
values in force. The defaults optSolver imposes are in `optsolver::options::defaults`:
`SOLVER_LOG`, `ARRAY_OUTPUTS`, `VERIFICATION`, `VERIFICATION_FACTOR` (10).

`Progress` has `elapsed` (seconds), `primal_objective`, `dual_objective`, `iterations` and
`phase` (`ProgressPhase::Simplex`, `Barrier`, `Other`). The callback runs on the thread that
called `solve`, one call at a time; a panic in it propagates out of `solve`.

## Solving

`backend.solve(problem_ref, &mut options)` returns `Result<Solution, Error>`. Pass the problem
as `(&problem).into()`: a `ProblemRef` (`ProblemRef::Usize` or `ProblemRef::I32`).
`solve_into` writes the arrays into your buffers instead ([Caller buffers](#caller-buffers)).
`Error` means the solve could not run (invalid input, unsupported feature, solver error); a
solve that ran but did not find an optimum is a `Status`, not an error.

## Statuses and limit reasons

`Solution::status` is a `Status`:

| `Status` | Meaning | What you get |
|---|---|---|
| `Optimal` | optimum within tolerances | x, y, z, z_l, z_u, s, objective |
| `Infeasible` | no feasible point | a `Farkas` certificate if requested and available |
| `Unbounded` | objective unbounded | an `UnboundedRay` if requested and available |
| `InfeasibleOrUnbounded` | one of the two | request `certificates` to decide |
| `StoppedAtLimit` | a limit stopped it | `limit_reason`: `LimitReason::Time`, `Iterations`, `Nodes`, `Solutions`, `ObjectiveCutoff`, `ObjectiveTarget`, `Work`, `Memory` |
| `NumericalDifficulty` | numerical trouble | possibly a feasible x |
| `Interrupted` | `cancel` set or callback stopped it | possibly a feasible x |
| `LocallyOptimal`, `LocallyInfeasible` | non-convex classes only | — |
| `NotSolved` | no optimisation completed | — |
| `Unknown` | an unrecognised solver code | see `raw_status` |

`Solution::returned` (`SolutionReturned::Optimal`, `Feasible` or `None`) says which point is in
x. `raw_status` keeps every solver code by name.

## Reading the solution

`Solution` fields: `status`, `limit_reason`, `returned`, `x` (length n), `y` (m), `z` (n), `z_l`
(n), `z_u` (n), `s` (k), `row_activity` (Ax as the solver reports it, m), `basis` (a `Basis` with
`variables` and `rows`, each a `BasisStatus`: `Basic`, `AtLower`, `AtUpper`, `Superbasic`,
`Fixed`; only for basic solutions), `objective` (½xᵀQx + cᵀx + γ), `bound` (the dual objective),
`counts` (`Counts`: `simplex_iterations`, `barrier_iterations`), `timings` (`Timings`: `load`,
`solve`, `extraction`, `verification` seconds), `certificate`, `raw_status`, `solver`,
`attributes` (extra solver values), `effective_parameters` (every parameter set, with its value),
`verification`. An array is empty when not available (or switched off by `array_outputs`).
`Solution::empty` builds an empty result; `Solution::view` gives the `SolutionView` used by
`verify` (fields `status`, `returned`, `x`, `y`, `z`, `z_l`, `z_u`, `s`, `basic`, `certificate`).

## Certificates

Request with `options.certificates = true`. `Certificate::Farkas { y, z, s }` proves
infeasibility with the sense-independent convention Aᵀy + z + Fᵀs = 0, s ∈ K*, and a positive
dual objective (row and bound terms selected by sign, minus gᵀs).
`Certificate::UnboundedRay { r }` is a direction with Ar within the row bounds' recession cone,
Fr ∈ K, Qr = 0 and σcᵀr < 0. Gurobi provides none for QP and conic problems: the status is still
reported, and verification is then `PartiallyVerified`.

## Verification

`Solution::verification` is a `VerificationReport` (`measures`, `overall`), computed by
`optsolver::verify` from your problem and the returned arrays alone — nothing the solver says
about residuals is trusted. Each `Measure` has `name`, `absolute`, `relative`, `tolerance` (the
solver's effective tolerance), `verdict` (`Verdict::Pass`, `Fail`, `NotEvaluated`) and `required`;
a relative value passes when ≤ `VERIFICATION_FACTOR` × tolerance. `Overall::Verified` (every
required measure passes), `Overall::Failed` (a deciding measure fails), `Overall::PartiallyVerified`
(nothing fails but something required could not be evaluated). Measure names (module
`optsolver::verify::names`): `ROW_BOUND`, `VARIABLE_BOUND`, `REDUCED_COST`, `DUAL_SIGN`,
`COMPLEMENTARY_SLACKNESS` (required for basic solutions only), `GAP` (|p − d| relative to
max(1, min(|p|, |d|))), `FARKAS`, `RAY`, `PRIMAL_CONE`, `DUAL_CONE`, `CONIC_COMPLEMENTARITY` (per
cone). You can call `verify(problem, &view, &tolerances)` yourself with `Tolerances { primal, dual,
integrality }`. Treat `Failed` as "do not trust this solution" even if the status is `Optimal`.

## Caller buffers

`solve_into(problem, &mut options, buffers)` takes `SolutionBuffers { x, y, z, row_activity, s,
z_l, z_u }`: `x` is required (`&mut [f64]` of length n), the others are `Option<&mut [f64]>` of
lengths m, n, m, k, n, n. Wrong lengths are `Error::Invalid` before solving. The returned
`Solution` then has those arrays empty.

## Errors

`Error` (also `optsolver::error::Result`): `Error::Invalid { what, detail }` (input failed
validation), `Error::TooLarge { what, value }` (dimension above `MAX_DIMENSION`),
`Error::Unsupported { field, backend }`, `Error::ParameterConflict { name }`,
`Error::Solver { code, message }` (the vendor's code and message, read at the failing call).

## Threading

`gurobi::Env` and `mosek::Env` are `Send` but not `Sync`: an environment may be created on one
thread and used or dropped on another, but used by one thread at a time; different environments
may be used concurrently. Creating and dropping Gurobi environments is serialised by optSolver
(Gurobi documents it as not thread-safe); MOSEK documents no guarantee for concurrent creation or
dropping, so create MOSEK environments one at a time. Problems borrow your data and are `Copy`;
each solve builds and frees its own solver model.

## Complete examples

Every example chooses whichever backend is compiled in and licensed, and returns quietly when
none is (so it also builds without solver features).

### LP

```rust
use optsolver::{Backend, CscRef, Options, Overall, Problem, RowBounds, Sense, Status};

fn backend() -> Option<Box<dyn Backend>> {
    #[cfg(feature = "mosek")]
    if let Ok(env) = optsolver::mosek::Env::new() {
        return Some(Box::new(env));
    }
    #[cfg(feature = "gurobi")]
    if let Ok(env) = optsolver::gurobi::Env::new() {
        return Some(Box::new(env));
    }
    None
}

// maximise x1 + 2 x2  subject to  x1 + x2 <= 4,  x1 + 3 x2 <= 6,  x >= 0.
// A in CSC form with i32 row indices: passed to the solver without copying.
let col_ptrs = [0usize, 2, 4];
let row_indices = [0i32, 1, 0, 1];
let values = [1.0, 1.0, 1.0, 3.0];
let a = CscRef::new(2, 2, &col_ptrs, &row_indices, &values)?;
let c = [1.0, 2.0];
let rows = RowBounds::SenseRhs { sense: b"<<", b: &[4.0, 6.0] };
let (l, u) = ([0.0, 0.0], [f64::INFINITY, f64::INFINITY]);
let problem = Problem::new(a, &c, Sense::Maximise, rows, &l, &u)?;

let Some(backend) = backend() else { return Ok(()) };
let solution = backend.solve((&problem).into(), &mut Options::default())?;
assert_eq!(solution.status, Status::Optimal);
assert_eq!(solution.verification.as_ref().map(|r| r.overall), Some(Overall::Verified));
assert!((solution.objective.unwrap() - 5.0).abs() < 1e-6);   // x = (3, 1)
assert_eq!(solution.x.len(), 2);
# Ok::<(), optsolver::Error>(())
```

### QP

```rust
use optsolver::{Backend, CscRef, Options, Problem, ProblemClass, RowBounds, Sense, Status};

fn backend() -> Option<Box<dyn Backend>> {
    #[cfg(feature = "mosek")]
    if let Ok(env) = optsolver::mosek::Env::new() {
        return Some(Box::new(env));
    }
    #[cfg(feature = "gurobi")]
    if let Ok(env) = optsolver::gurobi::Env::new() {
        return Some(Box::new(env));
    }
    None
}

// minimise ½ xᵀQx with Q = [[2, 1], [1, 2]]  subject to  x1 + x2 = 2,  x >= 0.
let a = CscRef::new(1, 2, &[0usize, 1, 2], &[0usize, 0], &[1.0, 1.0])?;
// Q's lower triangle only: (0,0) = 2, (1,0) = 1, (1,1) = 2.
let q = CscRef::new(2, 2, &[0usize, 2, 3], &[0usize, 1, 1], &[2.0, 1.0, 2.0])?;
let rows = RowBounds::SenseRhs { sense: b"=", b: &[2.0] };
let (c, l, u) = ([0.0, 0.0], [0.0, 0.0], [f64::INFINITY, f64::INFINITY]);
let problem = Problem::new(a, &c, Sense::Minimise, rows, &l, &u)?.with_Q(q)?;

let Some(backend) = backend() else { return Ok(()) };
if !backend.capabilities().problem_classes.contains(&ProblemClass::Qp) {
    return Ok(());
}
let solution = backend.solve((&problem).into(), &mut Options::default())?;
assert_eq!(solution.status, Status::Optimal);
assert!((solution.objective.unwrap() - 3.0).abs() < 1e-6);   // x = (1, 1)
# Ok::<(), optsolver::Error>(())
```

### Conic (second-order cone)

```rust
use optsolver::{Backend, Cone, ConicRef, CscRef, Options, Problem, RowBounds, Sense, Status};

fn backend() -> Option<Box<dyn Backend>> {
    #[cfg(feature = "mosek")]
    if let Ok(env) = optsolver::mosek::Env::new() {
        return Some(Box::new(env));
    }
    #[cfg(feature = "gurobi")]
    if let Ok(env) = optsolver::gurobi::Env::new() {
        return Some(Box::new(env));
    }
    None
}

// minimise x0  subject to  x1 = 3,  x2 = 4,  (x0, x1, x2) in the quadratic cone (x0 >= ‖(x1, x2)‖).
let a = CscRef::new(2, 3, &[0usize, 0, 1, 2], &[0usize, 1], &[1.0, 1.0])?;
let rows = RowBounds::SenseRhs { sense: b"==", b: &[3.0, 4.0] };
let free = [f64::NEG_INFINITY; 3];
let inf = [f64::INFINITY; 3];
let c = [1.0, 0.0, 0.0];
// Fx + g ∈ K with F = I (3 × 3), g = 0, K = one quadratic cone of dimension 3.
let f = CscRef::new(3, 3, &[0usize, 1, 2, 3], &[0usize, 1, 2], &[1.0, 1.0, 1.0])?;
let g = [0.0; 3];
let cones = [Cone::Quadratic(3)];
let problem = Problem::new(a, &c, Sense::Minimise, rows, &free, &inf)?
    .with_conic(ConicRef::new(f, &g, &cones)?)?;

let Some(backend) = backend() else { return Ok(()) };
if !backend.capabilities().cones.contains(&cones[0].kind()) {
    return Ok(());
}
let solution = backend.solve((&problem).into(), &mut Options::default())?;
assert_eq!(solution.status, Status::Optimal);
assert!((solution.objective.unwrap() - 5.0).abs() < 1e-6);   // x = (5, 3, 4)
assert_eq!(solution.s.len(), 3);   // conic duals, here (1, -0.6, -0.8)
# Ok::<(), optsolver::Error>(())
```

### A problem the backend cannot solve

```rust
use optsolver::{Backend, Cone, ConicRef, CscRef, Error, Options, Problem, RowBounds, Sense};

// minimise x0 subject to (x0, 1, 1) in the primal exponential cone: needs that cone kind.
let a = CscRef::new(2, 3, &[0usize, 0, 1, 2], &[0usize, 1], &[1.0, 1.0])?;
let rows = RowBounds::SenseRhs { sense: b"==", b: &[1.0, 1.0] };
let (c, l, u) = ([1.0, 0.0, 0.0], [f64::NEG_INFINITY; 3], [f64::INFINITY; 3]);
let f = CscRef::new(3, 3, &[0usize, 1, 2, 3], &[0usize, 1, 2], &[1.0, 1.0, 1.0])?;
let cones = [Cone::PrimalExponential];
let problem = Problem::new(a, &c, Sense::Minimise, rows, &l, &u)?
    .with_conic(ConicRef::new(f, &[0.0; 3], &cones)?)?;

#[cfg(feature = "gurobi")]
if let Ok(env) = optsolver::gurobi::Env::new() {
    // Gurobi has no exponential cone: rejected before any solver call.
    match env.solve((&problem).into(), &mut Options::default()) {
        // `field` names what is missing, here the cone kind "primal exponential".
        Err(Error::Unsupported { field, backend }) => {
            assert_eq!(backend, "gurobi");
            println!("unsupported: {field}");
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}
# Ok::<(), optsolver::Error>(())
```

### A non-optimal status

```rust
use optsolver::{Backend, Certificate, CscRef, Options, Problem, RowBounds, Sense, Status};

fn backend() -> Option<Box<dyn Backend>> {
    #[cfg(feature = "mosek")]
    if let Ok(env) = optsolver::mosek::Env::new() {
        return Some(Box::new(env));
    }
    #[cfg(feature = "gurobi")]
    if let Ok(env) = optsolver::gurobi::Env::new() {
        return Some(Box::new(env));
    }
    None
}

// x1 + x2 >= 3 with 0 <= x <= 1 is infeasible.
let a = CscRef::new(1, 2, &[0usize, 1, 2], &[0usize, 0], &[1.0, 1.0])?;
let rows = RowBounds::SenseRhs { sense: b">", b: &[3.0] };
let problem = Problem::new(a, &[1.0, 1.0], Sense::Minimise, rows, &[0.0, 0.0], &[1.0, 1.0])?;

let Some(backend) = backend() else { return Ok(()) };
let mut options = Options::default();
options.certificates = true;
let solution = backend.solve((&problem).into(), &mut options)?;
match solution.status {
    Status::Optimal => unreachable!("the problem is infeasible"),
    Status::Infeasible => {
        // y_F > 0 on the violated row, z_F < 0 at the upper bounds (Aᵀy + z = 0).
        if let Some(Certificate::Farkas { y, z, .. }) = &solution.certificate {
            assert!(y[0] > 0.0 && z.iter().all(|&v| v < 0.0));
        }
    }
    Status::StoppedAtLimit => println!("limit: {:?}", solution.limit_reason),
    other => println!("status {other:?}, raw codes {:?}", solution.raw_status),
}
# Ok::<(), optsolver::Error>(())
```

### Caller buffers, options, progress and cancellation

```rust
use optsolver::{Backend, CscRef, Method, Options, Problem, Progress, RowBounds, Sense,
                SolutionBuffers, Status};
use std::sync::atomic::AtomicBool;

fn backend() -> Option<Box<dyn Backend>> {
    #[cfg(feature = "mosek")]
    if let Ok(env) = optsolver::mosek::Env::new() {
        return Some(Box::new(env));
    }
    #[cfg(feature = "gurobi")]
    if let Ok(env) = optsolver::gurobi::Env::new() {
        return Some(Box::new(env));
    }
    None
}

let a = CscRef::new(2, 2, &[0usize, 2, 4], &[0usize, 1, 0, 1], &[1.0, 1.0, 1.0, 3.0])?;
let rows = RowBounds::Ranges { b_l: &[f64::NEG_INFINITY; 2], b_u: &[4.0, 6.0] };
let problem = Problem::new(a, &[1.0, 2.0], Sense::Maximise, rows, &[0.0; 2], &[f64::INFINITY; 2])?;

let Some(backend) = backend() else { return Ok(()) };
let cancel = AtomicBool::new(false);       // set from another thread to interrupt
let mut calls = 0;
let mut on_progress = |_: &Progress| calls += 1;
let mut options = Options::default();
options.threads = Some(1);
options.method = Some(Method::DualSimplex);
options.time_limit = Some(10.0);
options.cancel = Some(&cancel);
options.progress = Some(&mut on_progress);

// Your own arrays receive x and y; the returned Solution then has them empty.
let (mut x, mut y) = (vec![0.0; 2], vec![0.0; 2]);
let buffers = SolutionBuffers {
    x: &mut x,
    y: Some(&mut y),
    z: None,
    row_activity: None,
    s: None,
    z_l: None,
    z_u: None,
};
let solution = backend.solve_into((&problem).into(), &mut options, buffers)?;
drop(options);
assert_eq!(solution.status, Status::Optimal);
assert!(solution.x.is_empty() && (x[0] - 3.0).abs() < 1e-6 && (x[1] - 1.0).abs() < 1e-6);
let _ = calls;
# Ok::<(), optsolver::Error>(())
```

## C interface

optSolver has a C interface over this API (feature 005, ADR-0004): header
`src/capi/optsolver.h`, interface version 1.0 (`OPTSOLVER_ABI_VERSION_MAJOR`,
`OPTSOLVER_ABI_VERSION_MINOR`, `optsolver_abi_version()` returning `(major << 16) | minor`). It is a
translation only: the same validation, statuses, parameters and verification as the Rust API. The
header documents every rule in full; this section names every item.

### C interface: building and linking

Build the libraries (with the backends you need; the build needs `GUROBI_HOME` and `MOSEK_BINDIR`
as for the Rust crate):

```text
cargo build --release --features capi,gurobi,mosek
```

This produces `target/release/liboptsolver.a` and `target/release/liboptsolver.so`. Compile with
`-I <optSolver>/src/capi`. Link one of them:

- Static: `<optSolver>/target/release/liboptsolver.a` (or `-L<optSolver>/target/release
  -l:liboptsolver.a`), then the vendor libraries `-L$GUROBI_HOME/lib -lgurobi<major><minor>` (the
  Gurobi version declared in `.specify/memory/project.yml`, `domain.solvers.gurobi`; the build
  script derives the same name) and `-L$MOSEK_BINDIR -lmosek64` (for the backends compiled in), then the Rust runtime's system
  libraries `-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc` (x86_64 Linux; `cargo rustc --lib
  --crate-type staticlib -- --print native-static-libs` prints them for other targets).
- Shared: `-L<optSolver>/target/release -loptsolver`; the shared library records the vendor
  libraries itself.

At run time the vendor shared libraries (Gurobi's `libgurobi<major><minor>.so`; MOSEK's
`libmosek64.so` and the `libtbb.so` beside it, of the versions declared in `project.yml`) and, for the shared form, `liboptsolver.so` must be found by the dynamic
loader: put their directories on `LD_LIBRARY_PATH`, or pass `-Wl,-rpath,<dir>` when linking. The
vendor libraries and licences are never shipped with optSolver. A library built with a backend
needs that vendor library at load time even if only the other backend is used. With `runmat mex`
the same `-I`, `-L` and `-l` flags are passed (`--linker-argument` for `-Wl,...`).

### C interface: items

Codes: `OPTSOLVER_OK`, `OPTSOLVER_ERR_INVALID`, `OPTSOLVER_ERR_TOO_LARGE`,
`OPTSOLVER_ERR_UNSUPPORTED`, `OPTSOLVER_ERR_PARAMETER_CONFLICT`, `OPTSOLVER_ERR_SOLVER` (vendor code
from `optsolver_last_error_solver_code()`), `OPTSOLVER_ERR_INTERNAL` (a contained panic). The
message of the calling thread's last failure: `optsolver_last_error_message()` returns a
caller-owned copy, freed with `optsolver_string_free()`.

Constants: backends `OPTSOLVER_BACKEND_GUROBI`, `OPTSOLVER_BACKEND_MOSEK`; senses
`OPTSOLVER_SENSE_MINIMISE`, `OPTSOLVER_SENSE_MAXIMISE`; `OPTSOLVER_UNSET`; statuses
`OPTSOLVER_STATUS_OPTIMAL`, `OPTSOLVER_STATUS_INFEASIBLE`, `OPTSOLVER_STATUS_UNBOUNDED`,
`OPTSOLVER_STATUS_INFEASIBLE_OR_UNBOUNDED`, `OPTSOLVER_STATUS_STOPPED_AT_LIMIT`,
`OPTSOLVER_STATUS_NUMERICAL_DIFFICULTY`, `OPTSOLVER_STATUS_INTERRUPTED`,
`OPTSOLVER_STATUS_LOCALLY_OPTIMAL`, `OPTSOLVER_STATUS_LOCALLY_INFEASIBLE`,
`OPTSOLVER_STATUS_NOT_SOLVED`, `OPTSOLVER_STATUS_UNKNOWN`; limit reasons `OPTSOLVER_LIMIT_NONE`,
`OPTSOLVER_LIMIT_TIME`, `OPTSOLVER_LIMIT_ITERATIONS`, `OPTSOLVER_LIMIT_NODES`,
`OPTSOLVER_LIMIT_SOLUTIONS`, `OPTSOLVER_LIMIT_OBJECTIVE_CUTOFF`, `OPTSOLVER_LIMIT_OBJECTIVE_TARGET`,
`OPTSOLVER_LIMIT_WORK`, `OPTSOLVER_LIMIT_MEMORY`; solution returned `OPTSOLVER_RETURNED_NONE`,
`OPTSOLVER_RETURNED_FEASIBLE`, `OPTSOLVER_RETURNED_OPTIMAL`; cone kinds (COBRA's `coneType` codes)
`OPTSOLVER_CONE_QUADRATIC`, `OPTSOLVER_CONE_ROTATED_QUADRATIC`, `OPTSOLVER_CONE_PRIMAL_EXPONENTIAL`,
`OPTSOLVER_CONE_DUAL_EXPONENTIAL`, `OPTSOLVER_CONE_PRIMAL_POWER`, `OPTSOLVER_CONE_DUAL_POWER`,
`OPTSOLVER_CONE_NONNEGATIVE`, `OPTSOLVER_CONE_ZERO`, `OPTSOLVER_CONE_POSITIVE_SEMIDEFINITE`;
problem-class bits `OPTSOLVER_CLASS_LP`, `OPTSOLVER_CLASS_MILP`, `OPTSOLVER_CLASS_QP`,
`OPTSOLVER_CLASS_CONIC`; methods `OPTSOLVER_METHOD_AUTO`, `OPTSOLVER_METHOD_PRIMAL_SIMPLEX`,
`OPTSOLVER_METHOD_DUAL_SIMPLEX`, `OPTSOLVER_METHOD_BARRIER`, `OPTSOLVER_METHOD_CONCURRENT`; presolve
`OPTSOLVER_PRESOLVE_AUTO`, `OPTSOLVER_PRESOLVE_OFF`, `OPTSOLVER_PRESOLVE_CONSERVATIVE`,
`OPTSOLVER_PRESOLVE_AGGRESSIVE`; basis `OPTSOLVER_BASIS_BASIC`, `OPTSOLVER_BASIS_AT_LOWER`,
`OPTSOLVER_BASIS_AT_UPPER`, `OPTSOLVER_BASIS_SUPERBASIC`, `OPTSOLVER_BASIS_FIXED`; verification
`OPTSOLVER_VERIFICATION_NOT_RUN`, `OPTSOLVER_VERIFICATION_VERIFIED`, `OPTSOLVER_VERIFICATION_FAILED`,
`OPTSOLVER_VERIFICATION_PARTIALLY_VERIFIED`; measure verdicts `OPTSOLVER_VERDICT_PASS`,
`OPTSOLVER_VERDICT_FAIL`, `OPTSOLVER_VERDICT_NOT_EVALUATED`; result arrays `OPTSOLVER_ARRAY_X`,
`OPTSOLVER_ARRAY_Y`, `OPTSOLVER_ARRAY_Z`, `OPTSOLVER_ARRAY_Z_L`, `OPTSOLVER_ARRAY_Z_U`,
`OPTSOLVER_ARRAY_S`, `OPTSOLVER_ARRAY_ROW_ACTIVITY`; certificates `OPTSOLVER_CERTIFICATE_NONE`,
`OPTSOLVER_CERTIFICATE_FARKAS`, `OPTSOLVER_CERTIFICATE_UNBOUNDED_RAY`, arrays `OPTSOLVER_CERT_Y`,
`OPTSOLVER_CERT_Z`, `OPTSOLVER_CERT_S`, `OPTSOLVER_CERT_RAY`; parameter types `OPTSOLVER_PARAM_INT`,
`OPTSOLVER_PARAM_DOUBLE`, `OPTSOLVER_PARAM_STRING`. Names: `optsolver_status_name`,
`optsolver_limit_reason_name`, `optsolver_returned_name`, `optsolver_verification_name`,
`optsolver_cone_name`.

Problem: `optsolver_problem` (fields `struct_size`, `A`, `A_row_indices_i32`, `c`, `sense`,
`gamma`, `l`, `u`, `row_sense`, `b`, `b_l`, `b_u`, `Q`, `F`, `g`, `cones`, `num_cones`), matrices
`optsolver_csc` (`rows`, `cols`, `col_ptrs`, `row_indices`, `values`), cones `optsolver_cone`
(`kind`, `dim`, `exponents`, `exponents_len`). Every array is borrowed in place; `size_t` row
indices, or `int32_t` through `A_row_indices_i32`. Options: `optsolver_options` (the Rust fields of
the same names, NaN or `OPTSOLVER_UNSET` meaning unset, `memory_limit_gb`, and `raw` /
`num_raw` of `optsolver_param`), filled unset by `optsolver_options_init`. Every array pointer
(problem arrays and output buffers) must be aligned for its element type and may be NULL only with
length 0; anything else is `OPTSOLVER_ERR_INVALID` naming the field, before any solver call.
Structure pointers (`problem`, `options`, `buffers`, `Q`, `F`, handles) are not checked for
alignment: pass properly aligned structures. A NULL required top-level argument (`env`,
`problem`, `buffers`, `out`) is `OPTSOLVER_ERR_INVALID` naming it; `options`, `Q` and `F` may be
NULL (unset, absent).

Environments: `optsolver_env`, `optsolver_env_create`, `optsolver_env_free`,
`optsolver_backend_compiled`. Solving: `optsolver_solve` (with a handle), `optsolver_solve_once`
(creates and frees an environment), `optsolver_solve_into` (into caller arrays:
`optsolver_buffers` of `optsolver_array`, NaN-filled first). Results: `optsolver_result`, freed
with `optsolver_result_free`; `optsolver_result_status`, `optsolver_result_limit_reason`,
`optsolver_result_returned`, `optsolver_result_array`, `optsolver_result_objective`,
`optsolver_result_bound`, `optsolver_result_simplex_iterations`,
`optsolver_result_barrier_iterations`, `optsolver_result_timings` (`optsolver_timings`),
`optsolver_result_has_basis`, `optsolver_result_basis_variable`, `optsolver_result_basis_row`,
`optsolver_result_certificate`, `optsolver_result_certificate_array`,
`optsolver_result_raw_status_count`, `optsolver_result_raw_status`,
`optsolver_result_solver_name`, `optsolver_result_solver_version`,
`optsolver_result_attribute_count`, `optsolver_result_attribute`,
`optsolver_result_parameter_count`, `optsolver_result_parameter` (`optsolver_value`),
`optsolver_result_verification`, `optsolver_result_measure_count`, `optsolver_result_measure`
(`optsolver_measure`). Capabilities: `optsolver_capabilities` (a temporary environment; a
compiled-in backend that cannot be used reports not usable with the reason),
`optsolver_env_capabilities`, `optsolver_caps`, `optsolver_caps_usable`, `optsolver_caps_reason`,
`optsolver_caps_problem_classes`, `optsolver_caps_cones` (bit `1u << kind`),
`optsolver_caps_certificates`, `optsolver_caps_basis`, `optsolver_caps_option_count`,
`optsolver_caps_option`, `optsolver_caps_solver_name`, `optsolver_caps_solver_version`,
`optsolver_caps_free`. Test support: `optsolver_test_panic`, `optsolver_environments_created`.

Threads, ownership and panics: as the Threading section above for handles; every library-allocated
object has one free function, freeing NULL is a no-op; every entry point contains panics silently
(a hook installed on first use passes other panics to the previous hook).

## Low-level public items

These are public for backends and benchmarks; client code rarely needs them.

- Module `optsolver::problem::convert`: `i32_row_indices(A)` (the row indices as `&[i32]` when
  they already are), `IndexBuffer` (`IndexBuffer::new`, `with_capacity`, `capacity`, `blocks`),
  `Blocks` (`next_block`) and `Block` (fields `cols`, `starts`, `row_indices`, `values`), with
  `BLOCK_CAPACITY`.
- Modules: `optsolver::backend`, `optsolver::conic`, `optsolver::error`, `optsolver::options`,
  `optsolver::problem`, `optsolver::result`, `optsolver::verify`; the `Backend` trait methods
  `capabilities`, `solve`, `solve_into`.
- Module `optsolver::verify` also exports `verify` and the report types above; the
  `Tolerances` fields are `primal`, `dual`, `integrality`.
- `Solution::view` returns a `SolutionView`; `SolverIdentity` has `name` and `version`.
- Each backend module also has a public `status` module with `translate` (raw codes to `Status`)
  and `solution_returned`.
