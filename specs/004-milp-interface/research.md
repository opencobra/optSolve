# Research: Mixed-Integer Programmes Through The Existing Interface

Phase 0 of the plan. **G:n** is a line of `$GUROBI_HOME/include/gurobi_c.h` (Gurobi 13.0.3);
**H:n** a line of MOSEK 11.2 `mosek.h`. Gurobi documentation:
https://docs.gurobi.com/projects/optimizer/en/current/ ; MOSEK: [MIO]
https://docs.mosek.com/latest/capi/mip-optimizer.html, [SOL] …/accessing-solution.html. Facts
marked *probe* were confirmed 2026-11-15 by C probes against both libraries (two read-only
research agents; sources in the session scratchpad). Closes routed items CHK002–CHK005, CHK007,
CHK008, CHK010, CHK013, CHK016, CHK018. Reuse is the rule (spec SC-003): every decision names
the existing item it extends.

## R1. Which mixed-integer classes each solver accepts (FR-009; CHK004)

- **Gurobi**: MILP, convex MIQP, mixed-integer conic problems with its four cone kinds
  (quadratic, rotated quadratic, nonnegative, zero; second-order cones as convex quadratic
  constraints, feature 003). *Probe*: MIQP and a mixed-integer second-order cone solve Optimal.
- **MOSEK**: MILP, MIQP and mixed-integer conic problems with every cone kind except the
  semidefinite cone ([MIO] §13.4: "mixed-integer linear (MILO), quadratic (MIQO) … and conic
  (MICO) … except for mixed-integer semidefinite problems"). *Probe*: MIQP, and each of the six
  non-semidefinite cone kinds with integer variables, solve to INTEGER_OPTIMAL; an integer
  problem with a semidefinite cone fails with `MSK_RES_ERR_GLOBAL_INV_CONIC_PROBLEM` (1503, H:1447).
- **Correction to the spec draft**: MOSEK does solve MIQP (spec corrected; within the
  maintainer's "all MI classes the solvers support").
- **Non-convex Q with integers**: Gurobi solved min −x², x integer in [0, 5], without error even
  with `NonConvex` = 0 (presolve linearises it). optSolver does not check convexity (feature 002
  decision); the interface document states that a non-convex mixed-integer objective may be
  solved by Gurobi and rejected by MOSEK.

## R2. Declaring mixed-integer support: reuse `ProblemClass` and `ConeKind` (FR-005, FR-009; CHK003)

- **Decision**: `Capabilities` gains two fields of existing types: `integer_classes:
  Vec<ProblemClass>` (the base classes that may carry integer variables: `Lp`, `Qp`, `Conic`) and
  `integer_cones: Vec<ConeKind>` (the cone kinds allowed together with integer variables). The
  existing `ProblemClass::Milp` stays in `problem_classes` as the summary flag "accepts integer
  variables" (as features 001–003 already used it). `check_problem` (existing, shared): a problem
  whose variable types include an integer or binary column needs its base class in
  `integer_classes` and every cone kind in `integer_cones`; otherwise `Error::Unsupported` naming
  `"var_types"` (class) or the cone kind, before any solver call. Variable types that are all
  continuous need nothing (solved as the continuous class).
- Gurobi: `integer_classes` = Lp, Qp, Conic; `integer_cones` = its four kinds. MOSEK: Lp, Qp,
  Conic; every kind except `PositiveSemidefinite`.
- **Alternatives**: new `ProblemClass` variants `Miqp`, `Miconic` (rejected: multiplies variants
  by cone kinds and duplicates the base-class list); a new capability type (rejected: reuse).

## R3. Loading variable types in bulk (FR-007; CHK010)

- **Gurobi**: the caller's `var_types` bytes (`b'C'`, `b'I'`, `b'B'`) are Gurobi's vtype
  characters (G:107–109): passed straight to `GRBXloadmodel`'s `vtype` (i32 path) or
  `GRBXaddvars`' `vtype` per block (usize path; the block's slice of the caller's array), NULL when
  no variable types are set — zero copy. Gurobi copies the array (*probe*: overwriting it after
  the load leaves VType unchanged), the same evidence as feature 001's VII-A exception (manual +
  probe, kept as a licensed test). Cone auxiliary columns are continuous (`'C'`).
- **MOSEK**: `MSK_putvartypelist(task, num, subj, vartype)` (H:5737) with the integer and binary
  columns only (O(number of integer columns); `MSK_VAR_TYPE_INT` = 1, H:1834). MOSEK has no binary
  type: a binary column is an integer column whose bounds are intersected with [0, 1] by
  optSolver (bound values are copied O(n) only when a binary column's bounds lie outside [0, 1]).
- **Bounds the solvers adjust** (*probe*): Gurobi intersects binary bounds with [0, 1] and rounds
  fractional integer bounds inward ([0.5, 2.7] → [1, 2]); MOSEK accepts fractional integer bounds
  and in effect rounds inward; an empty integer range is reported infeasible by both. optSolver
  does the same for both (binary ∩ [0, 1]; integer bounds as given, rounding by the solver) and
  the document states it; verification judges binary columns against l, u ∩ [0, 1].

## R4. Options fields and their vendor parameters (FR-002, FR-008)

New fields of the existing `Options` (each mapped in each backend's existing `params::apply`):

| Field | Gurobi | MOSEK | Defaults (solver) |
|---|---|---|---|
| `mip_relative_gap` | `MIPGap` (G:1211) | `MSK_DPAR_MIO_TOL_REL_GAP` (H:519) | 1e-4 / 1e-4 |
| `mip_absolute_gap` | `MIPGapAbs` (G:1212) | `MSK_DPAR_MIO_TOL_ABS_GAP` (H:515) | 1e-10 / 0 |
| `node_limit` | `NodeLimit` (double, G:1196) | `MSK_IPAR_MIO_MAX_NUM_BRANCHES` (H:804) | ∞ / −1 |
| `solution_limit` | `SolutionLimit` (int ≥ 1, G:1197) | `MSK_IPAR_MIO_MAX_NUM_SOLUTIONS` (H:808) | 2e9 / −1 |
| `integrality_tolerance` | `IntFeasTol` (G:1209) | `MSK_DPAR_MIO_TOL_ABS_RELAX_INT` (H:516) | 1e-5 / 1e-5 |
| `fixed_model_duals` | (no parameter; R8) | (no parameter; R8) | off |

Existing fields reused: `time_limit` (stops the search on both), `threads`, `presolve`,
`solver_log`, `raw`, `cancel`, `progress`; `random_seed` gains a MOSEK mapping
(`MSK_IPAR_MIO_SEED`, H:824) for mixed-integer problems only (still `Unsupported` for continuous
problems, where MOSEK has no seed). `certificates`: for mixed-integer problems nothing is set
(*probe*: Gurobi's `DualReductions` = 0 turns an unbounded MILP into status 2 with ObjVal
−2e100), and no certificate is returned.

## R5. Statuses, solutions and bounds (FR-003, FR-007; CHK005)

- **Gurobi**: the existing `status::translate` is total and already maps NODE_LIMIT, SOLUTION_LIMIT,
  TIME_LIMIT, INTERRUPTED; *probes*: optimal MILP 2; infeasible MILP with feasible relaxation 3;
  unbounded MILP 5 (x = 0 returned, not a ray); node limit 8; solution limit 10. Solution
  returned when `SolCount` > 0 (existing rule). Bound: `ObjBound`; treat |ObjBound| ≥ 1e100 as
  none. Duals and basis: not available (Pi errors 10005).
- **MOSEK**: read solution type `MSK_SOL_ITG` (H:1749) for mixed-integer problems. The existing
  `status::translate` gains `MSK_SOL_STA_INTEGER_OPTIMAL` (9) → Optimal; `solution_returned`
  gains INTEGER_OPTIMAL → Optimal. *Probes*: infeasible MILP → prosta PRIM_INFEAS (4, solsta
  UNKNOWN) → Infeasible; unbounded MILP → prosta PRIM_INFEAS_OR_UNBOUNDED (8) →
  InfeasibleOrUnbounded (MOSEK does not separate them for MILP); node limit →
  `MSK_RES_TRM_MIO_NUM_BRANCHES` (100009) with PRIM_FEAS incumbent → StoppedAtLimit(Nodes);
  solution limit → 100015 → StoppedAtLimit(Solutions) (already mapped); time 100001. Bound:
  `MSK_DINF_MIO_OBJ_BOUND` (36), reported only when the root relaxation was solved
  (`MSK_IINF_MIO_NUM_RELAX` > 0) and finite.

## R6. The gap (FR-003, FR-004; CHK002)

- **Definition (one, solver-independent)**: with incumbent objective p and best bound b,
  absolute gap |p − b|; relative gap |p − b| / max(|p|, 1e-10) (MOSEK's δ₁ = 1e-10 floor; Gurobi
  divides by |p| and calls p = b = 0 a zero gap — identical except at p = 0, where both give
  0 when b = 0). No incumbent or no bound: no gap.
- **When a solver calls a solution optimal** (both, quoted docs): absolute gap ≤ absolute tolerance
  **or** relative gap ≤ relative tolerance (*probe*: Gurobi Optimal with `MIPGapAbs` 10 and
  relative gap 1).
- **Reuse**: the bound goes in the existing `Solution::bound`; the gap is computed from
  `objective` and `bound` by a new method `Solution::mip_gap() -> Option<(f64, f64)>` (absolute,
  relative), not stored separately.
- **Verification measure** `MIP_GAP`: passes when absolute ≤ 10 × absolute tolerance or relative
  ≤ 10 × relative tolerance (the solver's own rule with the verification factor); `Tolerances`
  gains `mip_relative_gap` and `mip_absolute_gap` (no existing field holds them), read back from
  each solver's effective parameters.

## R7. Integrality measure (FR-004)

`INTEGRALITY`: max over integer and binary columns of |x_j − round(x_j)|, absolute (no relative
scaling: integrality is unitless), judged against `Tolerances::integrality` (existing field,
now read from `IntFeasTol` / `MIO_TOL_ABS_RELAX_INT`) × 10. Required whenever x is returned for a
problem with integer columns. Dual measures are not listed for mixed-integer problems unless
fixed-model duals are returned (R8).

## R8. Fixed-model duals (FR-003a; CHK007, CHK008)

- **Gurobi**: `GRBfixmodel(model, &fixed)` (G:866; `GRBfixedmodel` G:893 is undocumented, not
  used) returns a continuous copy with integers fixed to their solution values **unrounded**
  (*probe*: 2.9999995 stays). optSolver sets the integer columns' bounds on the fixed model to the
  rounded values (`GRBsetdblattrlist`, one call each for LB and UB), solves it (parameters are
  inherited, including `QCPDual` for cones), and extracts y, z, z_l, z_u (and s, basis for LP) with
  the **existing** extraction code applied to the fixed model; frees it. *Probe*: MILP, MIQP and
  mixed-integer second-order cone fixed models give Pi and RC.
- **MOSEK**: no fix function: `MSK_clonetask` (H:3003), on the clone set integer columns
  continuous (`MSK_putvartypelist`) with bounds fixed at the rounded values
  (`MSK_putvarboundlist`), optimise, read BAS (LP) or ITR (QP, conic) with the existing
  extraction; the original task keeps its ITG solution (never read ITG from the clone). *Probe*:
  knapsack and mixed-integer second-order cone fixed problems solve; pobj = dobj.
- **Rounding (CHK007)**: integer and binary columns are fixed at round(x_j) (they are within the
  integrality tolerance); continuous columns are not fixed.
- **Marking (CHK008)**: the existing `Solution::attributes` gets `("fixed_model_duals", Int(1))`
  when they are returned, and `("fixed_model_status", Str(<status>))` always when requested; if
  the fixed problem is not optimal (rounding made it infeasible), no duals are returned and the
  status is recorded there — not an error.
- **Verification**: dual measures are judged against the fixed problem: the problem with each
  integer column's bounds replaced by round(x_j) (constructed inside `verify` as owned copies of l
  and u, O(n)).

## R9. Progress callbacks during branch-and-bound

Neither solver's existing progress mapping fires during the search (*probes*: Gurobi
SIMPLEX/BARRIER 0 calls, MIP 4059; MOSEK no INTPNT/simplex codes). Map Gurobi `GRB_CB_MIP`
(MIP_OBJBST 3000, MIP_OBJBND 3001, MIP_NODCNT 3002, RUNTIME) and MOSEK `MSK_CALLBACK_IM_MIO`,
`NEW_INT_MIO` (`MIO_OBJ_INT`, `MIO_OBJ_BOUND`, `MIO_NUM_BRANCH`) onto the existing `Progress`
(primal = incumbent, dual = bound, iterations = nodes; ±1e100 or "no relaxation yet" → None), with
a new `ProgressPhase::BranchAndBound` variant (no existing variant fits; `ProgressPhase` becomes
`#[non_exhaustive]`).

## R10. MPS integer markers and bound types (FR-010; CHK018)

- Gurobi writes `'MARKER' 'INTORG'`/`'INTEND'` around integer and binary columns and bound types
  `BV`, `LI`, `UI`, plus `UP`/`LO`; MOSEK writes no markers and only `BV`, `LI`, `UI`, and reads
  Gurobi's markers correctly (*probe*).
- **Trap**: an integer column inside the markers with no BOUNDS entry reads as [0, 1] in Gurobi
  (the old MPS convention, quoted in Gurobi's format page). The test MPS reader follows that
  convention; toy files (written by Gurobi, R11) always carry explicit bounds (`LI 0`, `PL`), and the
  generator checks that both tools read every file identically.
- JTASK (mixed-integer conic toys): `"var": {"type": ["cont", "int", …]}` (*probe*); the JTASK
  reader maps `"int"` to `b'I'` (binary is an integer column with [0, 1] bounds).

## R11. Toy instances and comparison (FR-010, FR-011; CHK016)

- New toy instances (existing generator `tools/corpus/make_toy_instances.m`, existing
  provenance format, existing toy and comparison trials): `toy_milp_knapsack` (binary),
  `toy_milp_general` (general integers, negative bounds), `toy_milp_mixed` (integer + continuous),
  `toy_milp_infeasible` (feasible relaxation), `toy_milp_unbounded`, `toy_miqp` (convex Q,
  integers), `toy_misocp` (quadratic cone, integers; both solvers), `toy_mi_pexp` (exponential
  cone with integers; MOSEK only). The PSD-with-integers rejection is an in-code test.
- **Statuses that differ by design**: MOSEK reports an unbounded MILP as InfeasibleOrUnbounded.
  The toy record lists the accepted statuses (`expected_status` plus `accepted_statuses`), and the
  comparison rule treats InfeasibleOrUnbounded as **compatible** with Infeasible and with
  Unbounded (reported as "agree (compatible statuses)"), never as passed silently.
- **Objective comparison (CHK016)**: two solutions each optimal within its solver's gap tolerances
  agree when |p_a − p_b| ≤ 10·[(abs_a + abs_b) + (rel_a + rel_b)·max(|p_a|, |p_b|, 1e-10)]; x is
  never compared.

## R12. Benchmark instance (FR-012; CHK013)

A derived corpus MILP, `iDopaNeuroC_problem_LP_int`, provenance in `tests/fixtures/milp/`
(as the derived QP): the LP fixture with its seed-1 objective and every column whose bounds are
both finite integers declared integer. The benchmark measures load memory and optSolver's
overhead (as features 001–003) with a time limit of 60 s per solve, recording the status reached;
if the instance proves trivial or intractable, the derivation rule is revisited in implementation
and recorded here.
