# Data Model: Solver-Independent Core And Gurobi LP Backend

Phase 1 of the plan. Defines the solver-independent problem, options, result and verification
(constitution XVI). Closes the routed checklist items CHK002, CHK003, CHK004, CHK005, CHK006,
CHK010, CHK012 and CHK013. Symbols follow the constitution's Notation Convention; symbols marked
**new** are proposed for the inventory (constitution XII-C) and are added through
`/speckit-constitution` before tasks are generated.

## Symbols

| Symbol | Object | Status |
|---|---|---|
| A, c, b, l, u, x, m, n | as in the Notation Convention | existing |
| γ (`gamma`) | objective constant | **new** |
| b_l, b_u (`b_l`, `b_u`) | row lower and upper bounds when rows are given as ranges | **new** |
| y | row dual vector, length m | **new** |
| z | reduced cost vector, length n | **new** |
| σ (`sigma`) | objective sense: +1 minimise, −1 maximise | **new** |

## Problem

The linear programme: optimise σ-sense of cᵀx + γ subject to row bounds on Ax and l ≤ x ≤ u.

| Field | Type (conceptual) | Rule |
|---|---|---|
| m, n | non-negative integers | each ≤ 2^31 − 1 (Gurobi `int`) |
| A | borrowed CSC: column pointers (length n + 1, start 0, non-decreasing, last = nnz), row indices (length nnz, each < m, strictly increasing within a column), values (length nnz, finite) | row indices either 64-bit or 32-bit |
| c | borrowed, length n, finite | |
| γ | finite scalar, default 0 | |
| σ | minimise or maximise | |
| rows | either **sense + b**: sense per row in {`=`, `<`, `>`} and b of length m; or **ranges**: b_l and b_u of length m | |
| l, u | borrowed, length n, no NaN | ±∞ allowed |
| variable types | optional, reserved (MILP) | rejected by the LP backend's capabilities |
| Q | optional, reserved (QP) | rejected by the LP backend's capabilities |

**Infinite values in row bounds (CHK002).** In the sense form: b_i = +∞ with `<`, or b_i = −∞
with `>`, makes row i free; b_i infinite with `=`, b_i = −∞ with `<`, or b_i = +∞ with `>` is an
input error. In the range form: b_l,i = −∞ and b_u,i = +∞ makes row i free; b_l,i = +∞ or
b_u,i = −∞ is an input error; b_l,i > b_u,i (both finite) is passed to the solver and reported
through the status. Variable bounds: l_j = +∞ or u_j = −∞ is an input error; l_j > u_j is passed
to the solver. Values with magnitude ≥ 10^100 are treated as infinite by Gurobi; optSolver maps
±∞ to ±`GRB_INFINITY` (1e100) at the Gurobi boundary.

**Canonical row bounds.** For verification every row is converted (virtually, no copy) to
b_l,i ≤ (Ax)_i ≤ b_u,i: `=` gives b_l = b_u = b; `<` gives b_l = −∞, b_u = b; `>` gives
b_l = b, b_u = +∞.

**Objective constant (CHK003).** The reported primal objective is p = cᵀx + γ. γ is passed to
Gurobi as `objcon`. The dual objective (below) also includes γ, so the duality gap is
independent of γ.

## Options structure

Every field is optional; unset means default behaviour (constitution XVI-C). Shared fields
(CHK006; Gurobi mapping in research.md R5):

| Field | Meaning (solver-independent) |
|---|---|
| time limit | wall-clock seconds the solver may spend optimising |
| iteration limit | maximum simplex iterations |
| threads | maximum solver threads |
| primal feasibility tolerance | maximum violation of row and variable bounds the solver may accept |
| dual feasibility tolerance | maximum violation of reduced-cost sign conditions the solver may accept |
| method | Auto, PrimalSimplex, DualSimplex, Barrier, Concurrent |
| presolve | Auto, Off, Conservative, Aggressive |
| random seed | solver's random seed |
| solver log | whether the solver's own log is produced (default off) |
| certificates | return infeasibility and unboundedness certificates |
| array outputs | switch off duals, reduced costs, slacks and basis arrays (default on) |
| verification | switch off independent verification (default on) |
| raw parameters | list of (vendor parameter name, integer or double or string value) |
| cancellation flag | shared atomic boolean; setting it interrupts the solve |
| progress callback | caller closure receiving progress records |

Rules: a field the backend cannot honour is an error naming field and backend; the same vendor
parameter set by a shared field and a raw parameter is an error; defaults optSolver imposes are
declared once, beside the options type, each with its rationale (constitution VII-G).

**Progress record**: elapsed seconds, primal objective, dual objective or bound (when the
algorithm provides it), iteration count, phase (simplex or barrier).

## Result

| Field | Content |
|---|---|
| status | solver-independent status (below) |
| limit reason | present only when status is StoppedAtLimit |
| solution returned | None, Feasible, Optimal |
| x | primal solution, length n (auxiliary range columns removed) |
| y | row duals, length m |
| z | reduced costs, length n |
| row activity | Ax as reported by the solver, length m |
| basis | per variable and per row: Basic, AtLower, AtUpper, Superbasic, Fixed (when a basis exists) |
| objective value p, bound | when defined |
| counts | simplex iterations, barrier iterations |
| timings | load, solve, extraction, verification (seconds) |
| certificates | Farkas ray (infeasible) or unbounded ray, when requested and available |
| raw status | every status code the solver reported (Gurobi: `Status`) |
| solver | name and version (Gurobi: from `GRBversion`) |
| solver attributes | list of (name, value) extra attributes (for example Gurobi `Runtime`, `Work`, `MaxMemUsed`) |
| effective parameters | every parameter the applying function set, with its value |
| verification report | below; absent only when verification is switched off |

Arrays are returned as owned vectors the caller can keep, or written into caller-supplied
buffers of the stated lengths.

## Status vocabulary (CHK004)

| Status | Meaning |
|---|---|
| Optimal | the solver proved optimality within its tolerances |
| Infeasible | the solver proved the problem infeasible |
| Unbounded | the solver proved the objective unbounded |
| InfeasibleOrUnbounded | the solver established one of the two but not which |
| StoppedAtLimit | the solver stopped at a limit; see limit reason |
| NumericalDifficulty | the solver stopped because of numerical trouble, or could not meet its tolerances |
| Interrupted | stopped by the cancellation flag or the progress callback |
| LocallyOptimal | local optimum (non-convex problem classes only) |
| LocallyInfeasible | locally infeasible (non-convex problem classes only) |
| NotSolved | no optimisation has completed |
| Unknown | the solver reported a code the translation does not recognise; raw code kept |

**Limit reasons (CHK005)**: Time, Iterations, Nodes, Solutions, ObjectiveCutoff,
ObjectiveTarget, Work, Memory.

## Verification (FR-012)

Computed from the problem and the returned x, y, z only (never from the solver's reported
residuals or row activity). With σ = +1 for minimise and −1 for maximise:

| Measure | Absolute value | Relative scaling (CHK010) | Judged against |
|---|---|---|---|
| row-bound violation | max_i max(b_l,i − (Ax)_i, (Ax)_i − b_u,i, 0) | ÷ (1 + max finite \|b_l\|, \|b_u\|) | primal feasibility tolerance |
| variable-bound violation | max_j max(l_j − x_j, x_j − u_j, 0) | ÷ (1 + max finite \|l\|, \|u\|) | primal feasibility tolerance |
| reduced-cost consistency | ‖z − (c − Aᵀy)‖∞ | per column ÷ (1 + \|c_j\| + Σ_i \|A_ij y_i\|), then the maximum (amended 2026-11-14, maintainer: with duals up to 1.4e10 on iDopaNeuroC problems 4 and 5, cancellation alone exceeds 1 + ‖c‖∞) | dual feasibility tolerance |
| dual sign violation | max over rows and columns of the amount by which σy_i or σz_j has the wrong sign: σy_i must be ≥ 0 where b_u,i = +∞, ≤ 0 where b_l,i = −∞, 0 where both infinite; σz_j likewise with l_j, u_j | ÷ (1 + ‖c‖∞) | dual feasibility tolerance |
| complementary slackness | max of \|y_i\| times the distance of (Ax)_i from the bound its sign selects, and \|z_j\| times the distance of x_j from the bound its sign selects | ÷ (1 + \|p\|) | dual feasibility tolerance |
| primal-dual gap | \|p − d\|, with d = γ + Σ_i y_i·(b_l,i if σy_i > 0 else b_u,i) + Σ_j z_j·(l_j if σz_j > 0 else u_j), zero-dual terms omitted | ÷ (1 + \|p\| + \|d\|) | dual feasibility tolerance |
| infeasibility certificate (when returned) | ‖Aᵀy_F + z_F‖∞ and sign violations as above with c = 0; certificate value d_F (as d with γ = 0) must be strictly positive | ÷ (1 + ‖y_F‖∞) | primal feasibility tolerance |
| unbounded ray (when returned) | violation of the recession conditions on A r and r by the infinite bounds, and σcᵀr < 0 required | ÷ (1 + ‖r‖∞) | primal feasibility tolerance |

**Verdict structure (CHK012)**: each measure carries absolute value, relative value, the
effective tolerance, and a verdict: Pass (relative value ≤ 10 × tolerance), Fail, or
NotEvaluated (the inputs it needs were not returned). The report's overall verdict is Verified
(all evaluated measures pass and none of the measures the status requires is NotEvaluated),
Failed (any measure fails), or PartiallyVerified (no failure, but some required measure was not
evaluated). Optimal requires all primal and dual measures; Infeasible requires the
infeasibility certificate; Unbounded requires the ray; other statuses require the primal
measures when a solution is returned.

**Dual outputs switched off (CHK013)**: dual measures are NotEvaluated, so an Optimal status
can be at most PartiallyVerified; this is reported, never treated as a pass.

**Verification never changes the status**; a Failed report sits beside it.

## Capabilities

Per backend: problem classes (LP for Gurobi in this feature), whether certificates, basis and
progress callbacks are supported, solver version. A problem using a reserved field not covered
by the capabilities is rejected before any solver call.

## Fixture provenance record (US1-2)

JSON beside each MPS file: instance name; source repository name and commit; source file name
and SHA-256; extracting script path and optSolver commit; dimensions m, n, nnz; objective sense;
seeds of the random objectives; citation from the source model's `PleaseCite` field; licence
"not verified (maintainer decision 2026-11-14)"; MPS file SHA-256; round-trip check result.
