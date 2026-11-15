# Feature Specification: MOSEK Backend For LP, QP And Conic Linear Programmes

**Feature Branch**: `002-mosek-lp-qp-clp`

**Created**: 2026-11-15

**Status**: Draft

**Input**: User description: "MOSEK backend for linear programmes (LP), convex quadratic
programmes (QP) and conic linear programmes (CLP: linear constraints plus affine conic
constraints F x + g in K, with K a product of exponential, dual exponential and rotated
quadratic cones, as in the variational kinetics solveCobraCLP), behind the same
solver-independent contract as Gurobi; the core problem, result and verification extended to
QP and CLP; variational kinetics test problems for CLP, frozen with provenance; every existing
contract and corpus test run unchanged on MOSEK, with cross-solver agreement on LP. Also:
rename the LP corpus fixture iDopaNeuroC_problem_1 to iDopaNeuroC_problem_LP and document the
test corpus; verify as code behaviour, with contract-level tests run on every backend, the
infeasibility-certificate sign convention and that progress callbacks are invoked one at a time
on the solving thread; make the crate build on its declared minimum Rust 1.85 and add that
build to the in-gate checks."

**Exploration**: none — direct request. The design background is
`specs/000-solver-interface-exploration/` (MOSEK slice loading, design-notes §5, §9) and the
decisions and closeout record of `specs/001-core-gurobi-lp/` (corpus, research R18).

## Clarifications

### Session 2026-11-15

- Q: Which variational kinetics conic problems form the CLP test problems? → A: the first inner
  conic problem of each corpus instance, for the small iCoreED model and for iDopaNeuroC.
- Q (Gate 2A): What exactly is "the first inner conic problem"? → A: the problem
  `optimizeVKmodel` passes to `solveCobraCLP` (its placeholder objective included), solved once
  by the single direct route for the expected status; iDopaNeuroC_problem_4 is excluded (it
  stalls in MOSEK).
- Q: Which cone types does the solver-independent conic problem support? → A: a broader set:
  quadratic, rotated quadratic, primal and dual exponential, primal and dual power,
  nonnegative and zero domains, and the positive semidefinite cone (in vectorised form).
- Q: How do the large iDopaNeuroC conic problems run? → A: in a separately selected heavy tier
  (run at the in-gate checks and on request); iCoreED conic problems run in the normal licensed
  tier.

## User Scenarios & Testing *(mandatory)*

The users are developers of programs that solve large optimisation problems in matrix form
(for example the variational kinetics solver and COBRA through a RunMat adapter) and the
maintainers of optSolver.

### User Story 1 - A clearly named, documented test corpus that builds on the declared Rust (Priority: P1)

A maintainer finds the LP test problem under a name that says what it is
(iDopaNeuroC_problem_LP), reads one document describing every corpus problem (what it is, where
it came from, which problem class it tests, and why the other variational kinetics instances
are not LP test problems), and can build and test the crate on the minimum Rust version it
declares.

**Why this priority**: every later story adds problems to the corpus; the name and the
document must be right before more problems join it. A false minimum-version claim misleads
every user.

**Independent Test**: the corpus tests pass with the renamed fixture; the corpus document lists
every fixture declared in the project parameters; the crate and all its tests build and pass
with the declared minimum Rust version.

**Acceptance Scenarios**:

1. **Given** the renamed fixture, **When** the corpus tests run, **Then** they find
   iDopaNeuroC_problem_LP through the project parameters, its provenance records the source
   instance (iDopaNeuroC_problem_1 of the variational kinetics corpus) and the rename, and no
   reference to the old name remains outside historical records.
2. **Given** the corpus document, **When** a reader looks up any fixture, **Then** it states the
   problem class, dimensions, source, extraction method, objective rule (for example seeded
   random) and expected outcome.
3. **Given** the declared minimum Rust version, **When** the in-gate checks run, **Then** the
   library, every test and every benchmark build and every test passes with that version.

---

### User Story 2 - Contract behaviour verified on every backend (Priority: P2)

A maintainer relies on two behaviours of the contract because tests show them, not because a
comment says so: an infeasibility certificate always follows the solver-independent sign
convention, and the progress callback is called one call at a time on the thread that called
`solve`.

**Why this priority**: both were recorded at feature 001 closeout as resting on observation;
MOSEK must meet the same contract.

**Independent Test**: the two tests are written once against the contract and pass on every
available backend.

**Acceptance Scenarios**:

1. **Given** an infeasible problem whose infeasibility proof is known in closed form, **When** it
   is solved with certificates requested, **Then** the returned certificate's row multipliers
   have the signs the solver-independent convention prescribes for each violated row, and the
   certificate passes verification.
2. **Given** a solve with a progress callback, multiple solver threads, and an algorithm that
   reports progress, **When** the callback runs, **Then** every call happens on the thread that
   called `solve` and no two calls overlap in time.

---

### User Story 3 - Solve linear programmes with MOSEK (Priority: P3)

A developer solves an LP with MOSEK through exactly the same problem, options and result types
used for Gurobi, and gets the same answer.

**Why this priority**: it proves the contract with a second backend and delivers the MOSEK path
the RunMat adapter needs for COBRA's `mosekopt` users.

**Independent Test**: every existing contract and corpus test runs unchanged on MOSEK and
passes; the LP corpus problem gives the same objective with both solvers.

**Acceptance Scenarios**:

1. **Given** every per-backend test of feature 001, **When** MOSEK is available, **Then** each runs
   unchanged against MOSEK and passes, or is reported as not applicable only where MOSEK's
   declared capabilities lack a feature the test needs.
2. **Given** iDopaNeuroC_problem_LP with each seeded objective, **When** solved by Gurobi and by
   MOSEK, **Then** both return Optimal, both solutions pass verification, and the objective values
   agree within the solvers' tolerances.
3. **Given** every response and status code documented for the supported MOSEK version, **When**
   translated, **Then** each maps to a named solver-independent status and an unknown code to
   "unknown"; this check needs no licence.

---

### User Story 4 - Solve convex quadratic programmes with MOSEK (Priority: P4)

A developer adds a positive semidefinite quadratic objective term to an LP and solves it with
MOSEK, receiving a verified solution.

**Why this priority**: the variational kinetics driver's second problem is a quadratically
penalised flux balance analysis; COBRA's QP users need it.

**Independent Test**: a QP built from the LP corpus problem as the driver builds it, and small
hand-built QPs with known solutions, solve to Optimal and pass the extended verification.

**Acceptance Scenarios**:

1. **Given** a QP with a known optimum, **When** solved by MOSEK, **Then** the status is Optimal,
   the objective matches the known value, and verification (including the quadratic term in
   dual feasibility and the gap) passes.
2. **Given** a quadratic matrix that is not symmetric or not of the declared size, **When** the
   problem is built, **Then** it is rejected with an error naming the defect.

---

### User Story 5 - Solve conic linear programmes with MOSEK (Priority: P5)

A developer solves the variational kinetics conic problems (linear constraints plus affine
conic constraints in exponential, dual exponential and rotated quadratic cones) through the
solver-independent contract and receives a solution verified for conic feasibility and
optimality.

**Why this priority**: it is the problem class the variational kinetics solver depends on.

**Independent Test**: frozen variational kinetics conic test problems, with provenance, solve
with MOSEK to the status recorded for them, and their solutions pass conic verification.

**Acceptance Scenarios**:

1. **Given** each frozen variational kinetics conic test problem, **When** solved by MOSEK,
   **Then** the status matches the expected status recorded in its provenance and, when optimal,
   verification passes: linear and bound feasibility, membership of Fx + g in each cone, dual
   cone membership of the conic duals, and the duality gap.
2. **Given** a cone list whose dimensions do not sum to the number of rows of F, or an unknown
   cone type, **When** the problem is built, **Then** it is rejected with an error naming the
   defect.
3. **Given** a conic problem and a backend whose capabilities lack conic support (Gurobi),
   **When** a solve is requested, **Then** it is rejected before any solver call.

---

### Edge Cases

- A QP with Q = 0 (an LP stated as a QP), and a QP that is not convex (Q not positive
  semidefinite): the solver's status is reported; no crash.
- A conic problem with no conic rows (k = 0), and one with only conic rows (m = 0).
- Infinite bounds in every position for QP and CLP as for LP.
- MOSEK not installed or no licence: environment creation returns MOSEK's error code and
  message; MOSEK tests are reported as ignored with the reason.
- The same problem passed to a backend that supports a subset of classes.

## Requirements *(mandatory)*

### Functional Requirements

**Corpus and toolchain**

- **FR-001**: The LP corpus fixture MUST be named iDopaNeuroC_problem_LP everywhere it is
  referenced (project parameters, fixtures, provenance, tests, documents), with its provenance
  naming the source instance and the rename.
- **FR-002**: The project MUST have one document describing every corpus fixture: problem class,
  dimensions, source, extraction method, objective rule and expected outcome, including why
  variational kinetics problems 2–5 are not LP test problems.
- **FR-003**: The crate, its tests and its benchmarks MUST build, and its tests pass, with the
  declared minimum Rust version, and that build MUST be one of the in-gate checks.

**Contract behaviour**

- **FR-004**: A contract-level test, written once and run on every backend, MUST show that a
  returned infeasibility certificate has the row-multiplier signs the solver-independent
  convention prescribes, on a problem whose proof is known in closed form.
- **FR-005**: A contract-level test, written once and run on every backend, MUST show that the
  progress callback is only ever invoked on the thread that called `solve` and never
  concurrently, with more than one solver thread configured.

**Problem classes in the core**

- **FR-006**: The solver-independent problem MUST accept an optional positive semidefinite
  quadratic objective term (Q, n × n, symmetric, given by its lower triangle) and validate it.
- **FR-007**: The solver-independent problem MUST accept optional affine conic constraints:
  F (k × n, sparse), g (length k) and a list of cones (type and dimension) whose dimensions sum to
  k, with cone types quadratic, rotated quadratic, primal and dual exponential, primal and dual
  power (with their exponents), nonnegative, zero, and positive semidefinite (vectorised lower
  triangle with the standard √2 scaling of off-diagonal entries); and validate them.
- **FR-008**: The solver-independent result MUST carry the conic duals s (length k) when the
  problem has conic constraints.
- **FR-009**: Independent verification MUST cover QP (the quadratic term in reduced costs and in
  the primal and dual objectives) and CLP (membership of Fx + g in K and of s in the dual cone
  K*, conic complementarity, and the gap), in absolute and relative form, judged as in feature 001.

**MOSEK backend**

- **FR-010**: The library MUST solve LP, QP and CLP through MOSEK by bulk slice loading, never
  element by element, behind its own additive feature, with the same contract as Gurobi.
- **FR-011**: A MOSEK environment MUST be reusable across solves; MOSEK parameters MUST be applied
  by one function only (constitution VII-G); MOSEK response, problem and solution statuses MUST
  be translated by one total translation; all raw codes MUST be kept.
- **FR-012**: MOSEK MUST support the cancellation flag, the progress callback and certificates on
  request, with the same behaviour the contract tests require of every backend.
- **FR-013**: Each backend's capabilities MUST state which problem classes it supports; a problem
  of an unsupported class MUST be rejected before any solver call.

**Test problems**

- **FR-014**: The corpus MUST gain frozen QP test problems (the QP the variational kinetics
  driver builds from the LP problem, and small hand-built QPs with known optima) and frozen
  variational kinetics conic test problems — for each variational kinetics corpus instance of
  the small iCoreED model and of iDopaNeuroC (except iDopaNeuroC_problem_4, which stalls), the
  conic problem the variational kinetics solver passes to its conic interface
  (`solveCobraCLP`), with the expected status obtained through that interface's single direct
  MOSEK route — each with provenance including its expected status.
- **FR-015**: Every test MUST be written once against the contract and run on every backend
  supporting its problem class; cross-solver agreement tests MUST run for every problem class two
  available backends support.
- **FR-017**: Tests on genome-scale conic problems (iDopaNeuroC) MUST form a separately selected
  heavy tier, run at the in-gate checks and on request; all other tests stay in the normal tiers.
- **FR-016**: Benchmarks MUST report, for MOSEK as for Gurobi, per-phase time and extra peak
  memory during load on the LP corpus problem, against SC-004 and SC-005 of feature 001.

### Key Entities

- **Quadratic term**: Q (n × n, lower triangle, positive semidefinite) in ½ xᵀQx + cᵀx + γ.
- **Affine conic constraints**: F, g and the cone list; the problem requires Fx + g ∈ K.
- **Cone**: type (quadratic, rotated quadratic, primal or dual exponential, primal or dual power
  with exponents, nonnegative, zero, positive semidefinite) and dimension.
- **Conic duals**: s, length k, in the dual cone K*.
- **Corpus document**: one description of every fixture.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every per-backend test of feature 001 passes on MOSEK unchanged (or is reported not
  applicable by a declared capability), and iDopaNeuroC_problem_LP with seeds 1, 2, 3 gives
  objective values agreeing between Gurobi and MOSEK to within 10 × the larger of the two
  solvers' optimality tolerances, relative (N = 1 per solve, deterministic settings recorded).
- **SC-002**: Every frozen CLP test problem solves with MOSEK to its recorded expected status,
  and every optimal one passes conic verification (N = 1 per problem).
- **SC-003**: The driver-built QP from iDopaNeuroC_problem_LP and each hand-built QP solve with
  MOSEK to Optimal with verification passing (N = 1 per problem).
- **SC-004**: On iDopaNeuroC_problem_LP, MOSEK loading adds at most 32 bytes per row and column
  plus 1 MiB of peak memory beyond MOSEK's own, and optSolver's added time is at most 5% of the
  MOSEK solve time, median of N = 5, for both row-index types.
- **SC-005**: The crate builds and all tests pass with the declared minimum Rust version (one
  build per gate).
- **SC-006**: The certificate-sign and callback-thread tests pass on every available backend.

## Assumptions

- MOSEK 11.2 is the supported version (to be declared in `project.yml`); its licence is
  available on the maintainer's machine through `MOSEKLM_LICENSE_FILE`.
- Gurobi gains no QP or conic support in this feature; its capabilities reject those classes.
- The QP test problem follows the variational kinetics driver's quadratically penalised flux
  balance analysis (`optimizeCbModel` with `minNorm`) on the LP corpus problem.
- Whether to use the official `mosek` Rust crate or optSolver's own declarations is decided in
  research against constitution IV, as for Gurobi.
- Persistent models, warm starts, MILP and the RunMat adapter are later features.
