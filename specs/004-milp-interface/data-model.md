# Data Model: Mixed-Integer Programmes Through The Existing Interface

Extends features 001–003 (unchanged parts not repeated). Every entity below is an existing one,
extended; new items are justified in the plan's reuse assessment. Closes CHK002, CHK008.

## Problem (unchanged type)

Variable types: the existing `with_var_types(&[u8])`, one byte per column: `b'C'` continuous,
`b'I'` integer, `b'B'` binary (integer in [l, u] ∩ [0, 1]). A problem is **mixed-integer** when
at least one column is `I` or `B`; its **base class** is Conic if it has conic data, else QP if it
has Q, else LP.

## Capabilities (extended)

- `integer_classes: Vec<ProblemClass>` — base classes that may carry integer columns.
- `integer_cones: Vec<ConeKind>` — cone kinds allowed together with integer columns.
- `problem_classes` keeps `ProblemClass::Milp` as "accepts integer columns".

| Backend | integer_classes | integer_cones |
|---|---|---|
| Gurobi | Lp, Qp, Conic | quadratic, rotated quadratic, nonnegative, zero |
| MOSEK | Lp, Qp, Conic | every kind except positive semidefinite |

Rule (`check_problem`): mixed-integer problem → base class in `integer_classes` (else
`Unsupported { field: "var_types" }`), every cone kind in `integer_cones` (else
`Unsupported { field: <kind name> }`), plus the existing continuous-class rules.

## Options (extended)

`mip_relative_gap: Option<f64>`, `mip_absolute_gap: Option<f64>`, `node_limit: Option<f64>`,
`solution_limit: Option<u32>`, `integrality_tolerance: Option<f64>`, `fixed_model_duals: bool`
(research R4). Fields are shared fields: `set_shared_fields` lists them; capabilities map them.

## Solution (extended use, one method)

- `status`: Optimal = optimal within the gap tolerances (either test, research R6);
  StoppedAtLimit with `LimitReason::Nodes` / `Solutions` / `Time` (existing reasons).
- `returned`: Feasible for an incumbent at a limit; `x` the incumbent.
- `objective`: incumbent objective; `bound`: best bound (existing field, now the MIP bound);
  `Solution::mip_gap() -> Option<(f64, f64)>`: (absolute, relative) from objective and bound
  (research R6), `None` without either.
- `y`, `z`, `z_l`, `z_u`, `s`, `basis`: empty, unless fixed-model duals were requested and the
  fixed problem solved; then those of the fixed problem, with `attributes` containing
  `("fixed_model_duals", Int(1))` and `("fixed_model_status", Str(..))`.
- `counts`: gains `nodes: Option<f64>` (no existing count holds branch-and-bound nodes).
- `certificate`: none for mixed-integer problems.

## Gap

p = objective of the incumbent, b = best bound. Absolute |p − b|; relative |p − b| /
max(|p|, 1e-10). Undefined (None) when there is no incumbent or no finite bound. Maximise: the
same formulas (both are absolute values).

## Tolerances (extended)

`mip_relative_gap: f64`, `mip_absolute_gap: f64` (effective values read back from the solver);
`integrality` (existing) now read from the solver for mixed-integer problems.

## Verification (extended)

For a mixed-integer problem with x returned: row and variable-bound violations (binary columns
judged against [l, u] ∩ [0, 1]), cone violation (existing), **integrality** (max |x_j − round(x_j)|
over integer and binary columns; required). With status Optimal: **MIP gap** (required; passes
when absolute ≤ 10 × mip_absolute_gap or relative ≤ 10 × mip_relative_gap; NotEvaluated without a
bound). Dual measures are listed only with fixed-model duals, and are then judged against the
fixed problem (integer columns' bounds set to round(x_j)).

## Progress (extended)

`ProgressPhase::BranchAndBound` (new variant; `ProgressPhase` becomes `#[non_exhaustive]`):
`primal_objective` = incumbent, `dual_objective` = bound, `iterations` = nodes.

## Toy instances (existing format)

Provenance gains optional `accepted_statuses` (list): statuses a backend may legitimately report
instead of `expected_status` (MOSEK: InfeasibleOrUnbounded for an unbounded MILP). New instances:
research R11. Benchmark: derived `iDopaNeuroC_problem_LP_int` (research R12).
