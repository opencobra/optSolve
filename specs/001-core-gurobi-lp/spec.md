# Feature Specification: Solver-Independent Core And Gurobi LP Backend

**Feature Branch**: `001-core-gurobi-lp`

**Created**: 2026-11-14

**Status**: Draft

**Input**: User description: "Solver-independent core and Gurobi LP backend: solver-independent
problem input, options structure, generic result with solver-independent status, and independent
verification of feasibility and optimality; the Gurobi LP backend with bulk loading and
environment reuse; first user story: extract the frozen iDopaNeuroC corpus from the variational
kinetics repository into MPS fixtures with provenance and seeded random objectives, with tests
written once and run against every backend from one shared test module; benchmarks of load time
and peak memory on the iDopaNeuroC instances. The iDopaNeuro data licence is not to be verified
and does not block this feature."

**Exploration**: `specs/000-solver-interface-exploration/` (founding exploration, predates the
exploration template; see `design-notes.md` §5–§9 and the decision tables in
`open-questions.md`, "Decisions taken by the maintainer, 2026-11-14" and "Test corpus
decisions")

## Clarifications

### Session 2026-11-14

- Q: Which performance targets should the spec use? → A: Zero-copy memory (no per-nonzero
  term: at most 32 bytes per row and column plus 1 MiB), and at most 5% of solve time.
- Q: What factor applies when judging a verification measure against the effective solver
  tolerance? → A: 10× the effective tolerance.
- Q: How are the seeded random objectives stored, given an MPS file holds one objective? → A:
  One MPS file per instance with the stored zero objective; seeded objectives are regenerated
  from their recorded seeds by the shared test module.

## User Scenarios & Testing *(mandatory)*

The users are developers of programs that solve large linear programmes already in matrix form
(for example a RunMat adapter serving the COBRA Toolbox) and the maintainers of optSolver, who
need every solver backend checked the same way.

### User Story 1 - A frozen test corpus and one reusable test harness (Priority: P1)

A maintainer has a set of real genome-scale linear programmes, the iDopaNeuroC instances, frozen
as test fixtures with full provenance, and a test harness in which every solver test is written
once and runs unchanged against every available backend.

**Why this priority**: every later story is accepted by tests that run on this corpus through
this harness. Without it, each backend would grow its own tests and its own fixtures, which the
constitution forbids (Principle II).

**Independent Test**: on a machine with no solver installed, the fixtures load, their provenance
records are complete and match the source files, the seeded objectives regenerate identically
from their seeds, and the harness reports each backend-dependent test as skipped rather than
passed.

**Acceptance Scenarios**:

1. **Given** the variational kinetics repository's frozen iDopaNeuroC instances, **When** the
   extraction is run, **Then** one fixture per instance (problems 1 to 5 and the two full-rank
   variants, seven in all) exists in MPS format, holding that instance's constraint matrix, row
   right-hand sides and senses, variable bounds, objective sense and its stored (zero)
   objective; the seeds of its random objectives are recorded in its provenance record.
2. **Given** a fixture, **When** its provenance record is read, **Then** it names the source
   file and its SHA-256 hash, the source repository commit, the extracting code and its commit,
   the seed of each objective, the dimensions and nonzero count, and the citation recorded in
   the source model.
3. **Given** a recorded seed, **When** the shared test module generates the objective twice, on
   any supported platform, **Then** the two vectors are identical bit for bit.
4. **Given** a test written against the solver-independent contract, **When** the test suite
   runs, **Then** the test runs once per available backend, and once per declared backend that
   is unavailable it is reported as skipped with the reason.

---

### User Story 2 - The solver-independent contract and independent verification (Priority: P2)

A developer describes a linear programme once, in a solver-independent form, requests any
non-default behaviour through one options structure, and receives a solver-independent result
whose solution has been checked against the problem's feasibility and optimality criteria by
code that does not depend on any solver.

**Why this priority**: it is the interface every backend implements and every test uses; it can
be built and tested completely without a solver licence.

**Independent Test**: with no solver installed, invalid problems are rejected with errors that
name the defect, valid problems are accepted without copying their arrays, and the verifier
reports correct measures for hand-built solutions, including deliberately infeasible and
non-optimal ones.

**Acceptance Scenarios**:

1. **Given** a problem whose arrays have inconsistent lengths, decreasing column pointers, a row
   index out of range, unsorted or duplicate row indices within a column, or dimensions beyond
   what supported solvers accept, **When** it is constructed, **Then** construction fails with an
   error naming the defect, before any solver is involved.
2. **Given** a valid problem with 32-bit row indices, **When** it is passed to a backend, **Then**
   no copy of the row indices is made.
3. **Given** a candidate solution that violates a row bound by a known amount, **When** it is
   verified, **Then** the reported row-bound violation equals that amount in absolute form and
   its relative form is scaled by 1 plus the infinity norm of the relevant data.
4. **Given** a primal-dual pair that is feasible but has a nonzero duality gap, **When** it is
   verified, **Then** the gap and the complementary-slackness measure are reported and judged
   against the tolerances supplied.
5. **Given** a verification that fails, **When** the result is returned, **Then** the
   solver-independent status is unchanged and the failure is reported beside it.
6. **Given** an options structure with a field the chosen backend cannot honour, **When** a
   solve is requested, **Then** it fails before solving with an error naming the field and the
   backend.

---

### User Story 3 - Solve a linear programme with Gurobi (Priority: P3)

A developer solves a linear programme through Gurobi by passing the solver-independent problem
and options, and receives the solver-independent result: primal and dual solutions, reduced
costs, slacks, basis status, objective value, the solver-independent status, every raw Gurobi
status code, extra solver attributes, and the verification measures.

**Why this priority**: it is the first backend and proves the contract end to end.

**Independent Test**: on a machine with Gurobi and a licence, every corpus test from User Story 1
passes through the Gurobi backend; on a machine without, those tests are reported as skipped.

**Acceptance Scenarios**:

1. **Given** each corpus instance with each seeded objective, **When** it is solved, **Then** the
   status is "optimal" and the returned solution passes verification at the effective solver
   tolerances.
2. **Given** a small infeasible programme and a small unbounded programme, **When** they are
   solved with certificates requested, **Then** the statuses are "infeasible" and "unbounded"
   respectively and the returned certificates pass verification.
3. **Given** a time limit too short to finish, **When** the programme is solved, **Then** the
   status is "stopped at a limit" with the reason "time", and the feasible-solution field says
   whether a solution was returned.
4. **Given** every status code documented for the supported Gurobi version, and one code not in
   that list, **When** each is translated, **Then** each documented code maps to a named
   solver-independent status and the unknown code maps to "unknown" with the raw code kept; this
   check needs no licence.
5. **Given** a raw Gurobi parameter supplied in the options structure, **When** the programme is
   solved, **Then** the reported effective parameter set shows that value unchanged.
6. **Given** one environment, **When** several programmes are solved with it in sequence,
   **Then** the licence check runs once and each solve reports its own results.
7. **Given** a solve in progress with a cancellation flag and a progress callback supplied,
   **When** the callback has been called at least once and the flag is then set, **Then** the
   solve stops with the status "interrupted", and the callback's progress data (elapsed time,
   objective, bound) is solver-independent.

---

### User Story 4 - Know the cost: load time and peak memory (Priority: P4)

A maintainer measures, per corpus instance, the time and extra memory optSolver adds on top of
Gurobi's own work, with environment creation, loading, solving, extraction and verification
measured separately.

**Why this priority**: speed and memory are the reason the library exists (Principle I); the
numbers become the baseline every later change is compared against.

**Independent Test**: the benchmark run produces, per instance, a record of each phase's time
over N repetitions, the peak memory measured in a dedicated process, the effective parameter set,
solver version, commit, machine and build profile.

**Acceptance Scenarios**:

1. **Given** each corpus instance, **When** the benchmark runs, **Then** a run record stores each
   phase's median and spread over N = 5 repetitions, cold and warm runs labelled.
2. **Given** a loading path that converts 64-bit row indices, **When** the benchmark runs,
   **Then** a positive control first shows which loading path was taken.

---

### Edge Cases

- A problem with zero rows, zero columns, or zero nonzeros.
- A column with no nonzeros.
- Infinite lower or upper bounds on variables and rows.
- Row bounds given as ranges where lower equals upper, and where both are infinite.
- Dimensions larger than the supported solver's integer width.
- Gurobi not installed, or installed without a valid licence: creating an environment returns an
  error carrying Gurobi's error code and message; backend tests are skipped with the reason,
  never passed.
- An options field set both as a shared field and as a raw vendor parameter for the same
  setting: an error.
- MILP or QP fields supplied to the LP-only backend: rejected before solving through the
  backend's capabilities.
- A status code the translation does not recognise.
- A solver returning a solution that fails verification.

## Requirements *(mandatory)*

### Functional Requirements

**Test corpus and harness**

- **FR-001**: The project MUST hold the seven iDopaNeuroC instances as MPS fixtures, extracted
  from the variational kinetics repository's frozen problem corpus by existing code from that
  repository where it can be reused, each with a provenance record (Principle II, X).
- **FR-002**: Each fixture's provenance record MUST list the seeds of its random objectives; the
  shared test module MUST regenerate each objective from its seed, bit-for-bit reproducibly on
  every supported platform, rather than storing it.
- **FR-003**: Every solver test MUST be written once against the solver-independent contract and
  run unchanged against every backend supporting its problem class; fixture loading, objective
  generation, verification and assertions MUST come from one shared test module.
- **FR-004**: Tests needing a solver licence MUST be reported as skipped, with the reason, where
  the solver is unavailable.

**Solver-independent contract**

- **FR-005**: The library MUST accept a solver-independent problem made of borrowed arrays named
  per the constitution's Notation Convention (A, c, b, l, u), with row bounds given either as
  senses with right-hand sides or as lower and upper ranges, row indices as 64-bit or 32-bit
  integers, an objective sense and constant, and optional fields reserved for variable types and
  Q.
- **FR-006**: The library MUST validate a problem once, before any solver call, rejecting
  inconsistent lengths, decreasing column pointers, out-of-range, unsorted or duplicate row
  indices, non-finite entries in A or c, NaN in any bound, and dimensions beyond what supported
  solvers accept, with errors naming the defect. A lower bound above its upper bound is not an
  input error: it is passed to the solver, whose status reports the infeasibility.
- **FR-007**: The library MUST NOT copy the caller's arrays except where the solver requires a
  different type, and then only the affected array, in a pass fused with validation, through a
  buffer whose size is fixed and independent of the number of nonzeros.
- **FR-008**: The library MUST take an options structure whose fields set non-default behaviour:
  shared fields with solver-independent definitions and documented per-backend mappings, and
  typed raw vendor parameters; a field a backend cannot honour, or one setting set by two routes,
  MUST be an error.
- **FR-009**: The library MUST return a solver-independent result as owned arrays the caller can
  keep, or written into caller-supplied buffers, containing every output the solver makes
  available, with infeasibility and unboundedness certificates returned on request.
- **FR-010**: Every result MUST carry a named solver-independent status, a separate limit reason,
  a separate field saying what solution is returned (none, feasible, optimal), every raw status
  code the solver reports, the solver's name and version, and a name/value list of extra solver
  attributes.
- **FR-011**: Every result MUST carry the complete effective parameter set applied to the solver.
- **FR-012**: The library MUST verify every returned solution independently of the solver,
  reporting row-bound, variable-bound and (where present) integrality violations, dual sign
  feasibility, complementary slackness and the primal-dual objective gap, each in absolute and
  relative form, judged as passing when at most 10 times the corresponding effective solver
  tolerance; verification MUST be on by
  default, switchable off in the options structure, and MUST NOT change the status.
- **FR-013**: Each backend MUST report its capabilities, and a problem it cannot solve MUST be
  rejected before any solver call.

**Gurobi LP backend**

- **FR-014**: The library MUST solve linear programmes through Gurobi by bulk loading the whole
  problem, never element by element.
- **FR-015**: A Gurobi environment MUST be reusable across solves, with the licence check made
  once per environment.
- **FR-016**: Gurobi parameters MUST be applied by one function only, and caller-supplied values
  MUST reach Gurobi unchanged.
- **FR-017**: Gurobi status codes MUST be translated by one total translation covering every code
  documented for the supported version, with unknown codes mapped to "unknown".
- **FR-018**: Solver logging MUST be off unless requested; the library MUST NOT print.
- **FR-019**: The library MUST support a caller-supplied cancellation flag and progress callback
  for Gurobi solves.

**Benchmarks**

- **FR-020**: The project MUST provide benchmarks reporting, per corpus instance, the time of
  environment creation, loading, solving, extraction and verification, and peak memory measured
  in a dedicated process, with the effective parameter set, solver version, commit, machine and
  build profile recorded.

### Key Entities

- **Problem**: one linear programme in matrix form: constraint matrix A (m by n, compressed
  sparse columns), objective c and constant, objective sense, row bounds, variable bounds l and
  u; optional variable types and Q reserved for later problem classes.
- **Options structure**: shared settings, typed raw vendor parameters, requested outputs,
  verification switch, cancellation flag and progress callback.
- **Result**: primal x, row duals, reduced costs, slacks, basis status, objective value, bound
  and gap where defined, counts and timings, certificates when requested, status, limit reason,
  solution-returned field, raw status codes, solver identity, extra attributes, effective
  parameter set, verification report.
- **Verification report**: each measure in absolute and relative form, with the tolerance it
  was judged against and the verdict.
- **Fixture**: one corpus instance in MPS form with its provenance record, which lists the seeds
  of its random objectives.
- **Capabilities**: the problem classes and outputs a backend supports.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All seven iDopaNeuroC fixtures exist with complete provenance records, and every
  provenance hash matches its source file.
- **SC-002**: With no solver installed, the full test suite passes, and every licence-dependent
  test is reported as skipped with its reason (none reported as passed).
- **SC-003**: On each of the seven instances, with each of three seeded objectives, the Gurobi
  backend returns "optimal" and the solution passes verification at the effective tolerances
  (7 × 3 = 21 solves; N = 1 per solve, deterministic solver settings recorded).
- **SC-004**: On each instance (largest: iDopaNeuroC_problem_1, 1344 rows, 2942 columns,
  13,748 nonzeros), the peak memory optSolver adds during loading, beyond Gurobi's own, is at
  most 32 bytes per row and column plus 1 MiB, with no term proportional to the number of
  nonzeros, median of N = 5 runs in dedicated processes.
- **SC-005**: On each instance, the time optSolver adds to loading (validation and conversion),
  plus extraction and verification, is at most 5% of the Gurobi solve time, median of N = 5 runs.
- **SC-006**: Every Gurobi status code documented for the supported version translates to a named
  status, and an undocumented code to "unknown", in a test that needs no licence.

## Assumptions

- The frozen iDopaNeuroC instances in the variational kinetics repository are the source of
  record; their stored objectives are all zero, so tests use seeded random objectives (three
  seeds per instance unless the plan justifies otherwise).
- The variational kinetics repository and MATLAB are available on the maintainer's machine for
  extraction; once extracted, fixtures need neither.
- Licence of the iDopaNeuro data: by maintainer decision on 2026-11-14 it is not verified and
  does not block this feature; the provenance record states "licence not verified" and carries
  the citation found in the source model.
- Gurobi 13.0.3, as declared in `.specify/memory/project.yml`, is the supported version; linking
  is at build time.
- One environment per thread; environments are not shared between threads.
- Persistent models (modify and re-solve), warm starts, MILP, QP and the MOSEK backend are later
  features; the problem, options and result types reserve room for them.
- Whether to use an existing Gurobi binding crate or optSolver's own declarations is decided in
  this feature's research against constitution IV.
