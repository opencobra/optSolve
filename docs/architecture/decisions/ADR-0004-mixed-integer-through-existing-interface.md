# ADR-0004: Mixed-Integer Problems Through The Existing Interface

- **Status**: Accepted
- **Date**: 2026-11-15
- **Decided by**: the maintainer, at Gate 2A of feature 004 (`specs/004-milp-interface/`)
- **Constitution**: IV, V, VII-G, XVI; extends ADR-0001, ADR-0002, ADR-0003

## Context

The interface reserved variable types, the MILP class, node and solution limit reasons and an
integrality tolerance from feature 001, but no backend accepted integer variables. The maintainer
asked for mixed-integer problems with reuse of the existing interface wherever possible, for
every mixed-integer class the solvers support, with optional fixed-model duals.

## Decision

1. Mixed-integer problems use the existing `Problem` (`with_var_types`), `Options`, `Solution`,
   `Status`, verification and `Backend` trait. No new type, module or test framework.
2. Backends declare mixed-integer support with two fields of existing types:
   `Capabilities::integer_classes` (base classes `Lp`, `Qp`, `Conic` that may carry integer
   columns) and `Capabilities::integer_cones` (cone kinds allowed with integer columns); the
   shared `check_problem` enforces them before any solver call. Gurobi: LP, QP, conic with its
   four cone kinds; MOSEK: LP, QP, conic with every kind except the semidefinite cone.
3. The best bound is the existing `Solution::bound`; the gap is computed from objective and bound
   (`Solution::mip_gap`), with one definition (absolute |p − b|, relative |p − b| / max(|p|,
   1e-10)); a solution is optimal when either gap test passes, as both solvers decide.
4. Mixed-integer problems have no duals and no certificates. Fixed-model duals, on request, are
   the duals of the continuous problem with integer columns fixed at their rounded values
   (Gurobi `GRBfixmodel`; MOSEK a cloned task), extracted by each backend's existing code, marked
   in `Solution::attributes`, and verified against that fixed problem.

## Consequences

- A statuses-differ-by-design rule enters the comparison: MOSEK's InfeasibleOrUnbounded for an
  unbounded MILP is compatible with Gurobi's Unbounded (reported as such, never silently passed).
- Gurobi may solve a non-convex mixed-integer QP (presolve linearisation) that MOSEK rejects;
  documented, not checked by optSolver.
