# Feature Specification: Mixed-Integer Programmes Through The Existing Interface

**Feature Branch**: `004-milp-interface`

**Created**: 2026-11-15

**Status**: Draft

**Input**: User description: "create an interface that admits MILP problems. reuse the existing
interface wherever possible rather than writing completely separate code" — expanded: "Mixed-
integer linear programmes (MILP) through the existing solver-independent interface, reusing it
wherever possible rather than writing separate code: the same Problem (its existing variable-
type field: continuous, integer, binary), Options, Solution, Status, verification and Backend
trait, extended only where integrality requires (integrality verification measure, best bound and
MIP gap, MILP limits and tolerances such as relative/absolute gap, node and solution limits, and
the statuses and limit reasons the classes already define). Both backends (Gurobi and MOSEK)
solve MILP; the existing per-backend test harness, toy instances (MPS with integer markers),
comparison trials, interface document and benchmark are extended to MILP rather than duplicated."

**Exploration**: none — direct request (maintainer, 2026-11-15). Background: the interface
already reserves variable types (`with_var_types`: continuous, integer, binary), the MILP
problem class, node and solution limit reasons and an integrality tolerance (features 001–003);
no backend accepts them yet.

## Clarifications

### Session 2026-11-15

- Q: Which mixed-integer classes are in scope? → A: every mixed-integer class the solvers
  support: MILP on both; mixed-integer QP on both (corrected in planning: MOSEK 11.2 supports
  MIQP, research R1); mixed-integer conic
  problems on Gurobi with its cone kinds (quadratic, rotated quadratic, nonnegative, zero) and on
  MOSEK with the cone kinds MOSEK allows with integer variables (planning confirms which; the
  semidefinite cone is expected to be excluded).
- Q (Gate 1): Is "no new public types" (SC-003) intended? → A: no — "there should be reuse where
  possible"; new types are allowed where reuse is not possible, each justified in the plan.
- Q: Should optSolver offer duals for mixed-integer problems? → A: optionally, as fixed-model
  duals: when requested through an options field, the backend re-solves the continuous problem
  with the integer and binary variables fixed at their values in the returned solution, and
  returns that problem's y, z, z_l, z_u (and s for conic problems), verified as duals of the
  fixed problem.

## User Scenarios & Testing *(mandatory)*

The users are developers (and the LLMs writing code for them, feature 003) who already solve LP,
QP and conic problems with optSolver and now need some variables to take integer or binary
values, and the maintainers.

### User Story 1 - Solve a MILP with the same code shape as an LP (Priority: P1)

A developer marks some columns of an existing LP as integer or binary and solves it with the same
problem, options and backend calls. The result has the same shape: a status, an integral x, the
objective, the best bound and the gap between them, and a verification report that also checks
integrality.

**Why this priority**: it is the request; it is the smallest change that makes MILP usable, and
it proves the interface was designed to admit it.

**Independent Test**: take a toy LP, mark columns integer, solve on each backend, and check the
known integral optimum, the bound, the gap and a Verified report; no new problem or result type
is used.

**Acceptance Scenarios**:

1. **Given** a problem with variable types set, **When** solved on a backend declaring MILP,
   **Then** the status is Optimal (optimal within the gap tolerance), x is integral within the
   integrality tolerance on integer and binary columns, and the objective, best bound and gap are
   reported.
2. **Given** the same problem, **When** verification runs, **Then** it checks row and variable
   bounds, integrality, and the gap between objective and bound; it does not judge dual measures
   (a MILP has no duals) and says so.
3. **Given** a node, solution or time limit that stops the search, **When** the result returns,
   **Then** the status is StoppedAtLimit with the limit reason, the best feasible x when one was
   found, and the bound and gap at that point.
4. **Given** an infeasible or unbounded MILP, **When** solved, **Then** the status says so (no
   certificate is promised for MILP).

---

### User Story 2 - The same tests, toy instances and comparison cover MILP (Priority: P2)

A maintainer runs the existing test suite and finds MILP covered by the same harness: toy MILP
instances (files with integer markers) with known optima, per-backend trials, and MOSEK–Gurobi
comparison of status and objective.

**Why this priority**: reuse of the test machinery is part of the request; MILP must not get a
parallel test system.

**Independent Test**: the toy-instance and comparison trials list the new MILP instances and pass
on both backends without new test infrastructure beyond the reader's integer markers.

**Acceptance Scenarios**:

1. **Given** toy mixed-integer files (MILP: binary knapsack, general-integer, mixed, infeasible,
   unbounded; mixed-integer QP; mixed-integer conic), **When**
   the toy trials run, **Then** each reaches its recorded outcome on each backend.
2. **Given** both solvers, **When** the comparison runs, **Then** MILP instances are compared on
   status and objective (within the gap tolerances), never on x (optima may not be unique).
3. **Given** a backend without MILP, **When** given a MILP, **Then** it reports "unsupported"
   before any solver call (unchanged rule).

---

### User Story 3 - Mixed-integer QP and conic problems (Priority: P3)

A developer adds integer variables to a QP or a conic problem and solves it on a backend that
supports that combination, with the same calls and result shape as MILP; a backend that does not
support the combination says so before solving.

**Why this priority**: the maintainer chose every mixed-integer class the solvers support; it
reuses everything US1 builds.

**Independent Test**: toy mixed-integer QP and conic instances solve to their known optima on
each supporting backend; unsupported combinations report "unsupported"; comparable ones are
compared across solvers.

**Acceptance Scenarios**:

1. **Given** a mixed-integer QP, **When** solved on Gurobi or MOSEK, **Then** Optimal with the
   known optimum and a Verified report on both, and the two agree (MOSEK's MIQP support
   corrected in planning, research R1).
2. **Given** a mixed-integer second-order-cone problem, **When** solved on both, **Then** both
   reach the known optimum and agree.
3. **Given** a mixed-integer problem with a cone kind a backend does not allow with integers,
   **When** solved there, **Then** "unsupported" names the combination.

---

### User Story 4 - The interface document describes mixed-integer problems (Priority: P4)

An LLM given only the interface document writes correct MILP client code: setting variable
types, the MILP options, and reading bound, gap and the integrality verdict.

**Why this priority**: feature 003 made the document the contract for client code; a capability
it does not describe is not usable.

**Independent Test**: a MILP example in the document compiles and runs as a documentation test;
the drift test covers every new public item.

**Acceptance Scenarios**:

1. **Given** the document, **When** its MILP example runs, **Then** it solves to the stated
   optimum on any available backend.
2. **Given** any new public item, **When** the drift test runs, **Then** it fails until the item
   is documented.

### Edge Cases

- All columns continuous with variable types set: solved as an LP (same result as without types).
- Binary columns with bounds outside [0, 1]: the bounds are intersected with [0, 1] by the
  solver convention; optSolver states which (documented, tested).
- Integer columns with fractional bounds: rounded inward by the solvers; documented.
- A MILP whose LP relaxation is unbounded but the MILP is infeasible, or the reverse: statuses as
  reported by the solver, compared on both backends.
- Zero gap tolerance: allowed; the search may hit a limit instead.
- A solution reported with integrality violations within the solver's integrality tolerance:
  judged against that tolerance (×10, as other measures).

## Requirements *(mandatory)*

### Functional Requirements

**Interface (reuse first)**

- **FR-001**: MILP MUST use the existing problem type and its existing variable-type field
  (continuous, integer, binary); no new problem type.
- **FR-002**: MILP MUST use the existing options structure, extended only with the fields MILP
  needs that do not exist yet: relative gap tolerance, absolute gap tolerance, node limit,
  solution limit, and integrality tolerance; existing fields (time limit, threads, random seed,
  presolve, solver log, raw parameters, cancellation, progress) apply unchanged.
- **FR-003**: MILP MUST use the existing result type and status vocabulary: Optimal means optimal
  within the gap tolerances; StoppedAtLimit carries the existing limit reasons (time, nodes,
  solutions, …); the best bound uses the existing bound field; the gap MUST be reported (relative
  and absolute) without a separate result type. Dual arrays and the basis are empty for MILP
  unless fixed-model duals are requested (FR-003a).
- **FR-003a**: An options field MUST request fixed-model duals: after a solve that returns a
  solution, the backend re-solves the continuous problem with integer and binary variables fixed
  at the returned values and fills the existing dual arrays (y, z, z_l, z_u; s for conic
  problems) and, where available, the basis; the result MUST state that the duals belong to the
  fixed problem, and verification MUST judge them against the fixed problem. Not requested: no
  extra solve, no dual arrays.
- **FR-004**: Verification MUST add an integrality measure (largest distance of an integer or
  binary column from the nearest integer, judged against the effective integrality tolerance) and
  an objective–bound gap measure (judged against the effective gap tolerances); for MILP the dual
  measures are not required and not listed, except for fixed-model duals (FR-003a), which are
  judged as LP (or QP, conic) duals of the fixed problem.
- **FR-005**: The backend contract (capabilities, the shared problem check, solve and solve into
  caller buffers) MUST be reused; backends declare the MILP class through the existing
  capabilities.
- **FR-006**: Every new public item MUST be documented in the interface document (drift test), with
  a MILP example run as a documentation test.

**Backends**

- **FR-007**: Gurobi and MOSEK MUST both solve MILP through their existing backends, loading the
  variable types in bulk (no per-column calls), with their statuses translated by the existing
  translation functions (extended only for MILP-specific codes).
- **FR-008**: Each new options field MUST map to a vendor parameter in each backend's single
  parameter-applying function, or be reported as unsupported by that backend.
- **FR-009**: Integer and binary variables MUST be accepted together with a quadratic objective
  or conic constraints wherever the backend supports that combination (Gurobi: mixed-integer QP
  and mixed-integer conic problems with its cone kinds; MOSEK: mixed-integer conic problems with
  the cone kinds it allows with integer variables, every kind except the semidefinite cone;
  MOSEK also solves mixed-integer QP, research R1). Capabilities
  MUST state which combinations each backend accepts, and the shared problem check MUST reject
  any other combination as unsupported before a solver call.
- **FR-009a**: For mixed-integer QP and conic problems the same rules as MILP apply: integrality
  and objective–bound gap measures, primal (row, bound, cone) measures, no dual measures, no
  certificates.

**Tests (reuse)**

- **FR-010**: The test MPS reader MUST accept integer markers (and binary bound types); toy MILP
  instances MUST be added to the existing toy set (generator, provenance, two-tool checks).
- **FR-011**: MILP MUST be covered by the existing per-backend toy trials and the existing
  comparison trials (status and objective; never x), with no parallel test framework.
- **FR-012**: The existing benchmark MUST gain a MILP scenario (load memory and optSolver's
  overhead), with a MILP instance with provenance.

### Key Entities

- **Variable types**: the existing per-column field (continuous, integer, binary).
- **MILP options**: the new fields of the existing options structure (gap tolerances, node and
  solution limits, integrality tolerance).
- **Mixed-integer capability**: per backend, which problem classes may carry integer variables
  (and, for conic problems, which cone kinds).
- **Gap**: the relative and absolute difference between objective and best bound, reported with
  the result and judged by verification.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every toy MILP instance reaches its recorded status and objective (within the gap
  tolerances) on both backends with a Verified report, in 100% of runs.
- **SC-002**: With both solvers, 100% of MILP instances agree in status and objective, or each
  disagreement is listed.
- **SC-003**: Reuse where possible: every mixed-integer capability is delivered by extending an
  existing type, function, test or tool, and each new public type, module, test framework or
  tool the plan introduces is justified in the plan by naming the existing item it could not
  reuse and why (checked at Gate 2A against the plan's reuse assessment).
- **SC-004**: Loading a MILP adds no more than the variable-type data each solver's bulk call
  requires (at most 1 byte per column plus the feature 001–003 limits), measured on the MILP
  benchmark instance with N = 5.
- **SC-005**: The document's MILP example compiles in every tier and runs where a backend is
  available; the drift test passes.
- **SC-006**: Every supported mixed-integer combination (MIQP on both; mixed-integer conic on
  each backend for its allowed cone kinds) has at least one toy instance that solves to its known
  optimum, and every unsupported combination is rejected before solving.
- **SC-007**: With fixed-model duals requested, every toy MILP with an optimal solution returns
  duals that verify against the fixed problem on both backends.

## Assumptions

- MIP starts (warm starts), callbacks for incumbent solutions, lazy constraints, semi-continuous
  and SOS constraints are out of scope.
- No certificates are promised for MILP (neither solver provides infeasibility certificates for
  integer problems).
- The verification factor (×10 the effective tolerance) applies to the new measures.
- Licences and solver versions are those of features 001–003.
- The MILP benchmark and corpus instance can be a toy-sized or derived instance if no frozen
  MILP corpus instance exists (decided in planning).
