# ADR-0003: Gurobi QP And Second-Order Cones, Per-Cone Capabilities, And The Interface Document

- **Status**: Accepted
- **Date**: 2026-11-15
- **Decided by**: the maintainer, at Gate 2A of feature 003 (`specs/003-llm-interface-toy-instances/`)
- **Constitution**: II, IV, VII-G, XII-C, XIII, XVI; extends ADR-0001 and ADR-0002

## Context

Feature 003 compares MOSEK and Gurobi on every instance both support, which needs Gurobi to
solve convex QP and conic problems, and publishes one interface document for client authors and
LLMs. Gurobi has no affine conic constraints: its second-order cones are quadratic constraints
it recognises, and its duals for them must be mapped onto the solver-independent conic dual s.

## Decision

1. The Gurobi backend solves convex QP (`GRBaddqpterms`; diagonal halved, off-diagonal once;
   `NonConvex` = 0 so a non-convex Q is an error) and conic problems whose cones are quadratic,
   rotated quadratic, nonnegative or zero (feature 003 research R1–R3).
2. Quadratic and rotated quadratic blocks are modelled with auxiliary columns u, linear rows
   Fx − u = −g and one convex quadratic constraint per cone; nonnegative and zero blocks are
   linear rows Fx ≥ −g, Fx = −g. With `QCPDual` = 1 the conic dual is s = `Pi` of those rows
   (derived and probed, research R3); quadratic-constraint duals (`QCPi`) are not used, since they
   miss the bound multiplier at the cone apex. Auxiliary columns never appear in results.
3. Gurobi returns no certificates for QP and conic problems (vendor documentation and probe,
   research R4); results report the status without one.
4. Backends declare the cone kinds they accept (`Capabilities::cones`, `ConeKind`); the shared
   `check_problem` rejects any other kind before a solver call. One rule, every backend.
5. The interface document `docs/llm-interface.md` is the crate-level documentation (one source,
   included with `include_str!`); its examples are documentation tests and a drift test ties it to
   the public items. Its LLM entry point is `docs/llms.txt` (not the repository root, whose
   file list in the architecture model stays unchanged).

## Consequences

- Gurobi results for QP and conic problems are interior-point solutions (no crossover): no basis,
  per-item complementary slackness informational (feature 002 research R19).
- The stacked [A; F] load on Gurobi uses a fixed-size buffer of indices and values; QP uses
  O(nnz(Q)) triplets (feature 003 plan, Complexity Tracking).
- Exponential, power and semidefinite cones remain MOSEK-only; comparisons report them as not
  comparable.
- A public-interface change must update the document in the same change, or the drift test fails.
