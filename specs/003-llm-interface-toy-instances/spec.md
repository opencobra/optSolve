# Feature Specification: LLM-Usable Interface Documentation And Self-Contained Cross-Solver Test Instances

**Feature Branch**: `003-llm-interface-toy-instances`

**Created**: 2026-11-15

**Status**: Draft

**Input**: User description: "LLM-usable interface documentation and self-contained
cross-solver test instances. (a) Document the optSolver public interface so that a large
language model, given only that document, can correctly add the crate as a dependency
(features, build environment variables, licences) and write code that builds problems over
borrowed caller arrays (zero-copy data input: CSC matrix, bounds, objective, quadratic term,
conic data), chooses a backend, sets options, solves, and reads status, solution, certificates
and verification. (b) Self-contained toy test instances, committed as small MPS files (LP, QP)
and JSON task files (MOSEK JTASK, conic problems), covering every problem class optSolver
supports — LP, QP, and conic programmes with each supported cone type, plus infeasible and
unbounded cases — each with known optimal values and provenance, so instance-based tests run
without the variational kinetics corpus. (c) Tests that, when both MOSEK and Gurobi are
installed and licensed, solve the same instances on both and compare statuses, objectives and
verification outcomes, reporting each pair; when only one is available the comparison is
reported as skipped with the reason."

**Exploration**: none — direct request (maintainer, 2026-11-15, at the close of feature 002).
Background: `specs/002-mosek-lp-qp-clp/` (contract, research R10 agreement rule, R19
verification decisions, JTASK format decision) and `docs/test-corpus.md`.

## Clarifications

### Session 2026-11-15

- Q: Which classes does the MOSEK–Gurobi comparison cover? → A: extend the Gurobi backend to
  QP and to conic problems with quadratic and rotated quadratic cones (which Gurobi solves
  natively); those instances are compared on both solvers; exponential, power and positive
  semidefinite cones remain MOSEK-only and are reported as not comparable.
- Q: Where does the interface document live? → A: a stand-alone Markdown document for LLMs
  (`docs/llm-interface.md`) with an `llms.txt` entry point at the repository root pointing to
  it, and the same content as the crate-level Rust documentation, its examples compiled as
  documentation tests.
- Q: Are the MPS and JTASK readers part of optSolver's public interface? → A: no, test-only;
  the toy instances make the crate's own tests self-contained, and the interface document
  shows building problems from the caller's arrays only.
- Q: How is SC-002 (an LLM writes correct client code from the document) checked? → A: at
  verification (Gate 3), independent agents given only the interface document and three task
  statements (LP, QP, conic); their code is compiled and run against toy instances, three
  attempts per task; model, prompts, code and results recorded in the run bundle; a failure is
  a documentation defect.

## User Scenarios & Testing *(mandatory)*

The users are (1) developers, and the large language models (LLMs) that write code for them,
who add optSolver to another program to solve problems already held in matrix form, and (2)
the maintainers of optSolver, who need every problem class tested on instances small enough to
read, commit and run anywhere.

### User Story 1 - An LLM writes correct optSolver client code from one document (Priority: P1)

A developer asks an LLM to "solve this problem with optSolver". The LLM is given a single
interface document. From it alone it writes a dependency declaration with the right features
and build environment, builds the problem over the program's own arrays without copying them,
picks a backend, sets options, solves, and handles every status, the solution arrays, the
certificates and the verification report correctly.

**Why this priority**: the crate's purpose is to be depended on by other programs; an
interface that cannot be used correctly from its documentation is not usable, and LLMs now
write much of that client code.

**Independent Test**: every code example in the document is compiled and run by the test
suite (so the document cannot drift from the code), and the document is checked to mention
every public item of the solver-independent interface.

**Acceptance Scenarios**:

1. **Given** only the interface document, **When** a client program is written for an LP, a
   QP and a conic problem following its examples, **Then** it builds and solves each problem
   with no array of the caller's problem data copied by the client code.
2. **Given** a problem the chosen backend cannot solve (for example a QP on Gurobi), **When**
   the client follows the document, **Then** it handles the documented "unsupported" error
   rather than receiving a wrong answer.
3. **Given** a solve that ends in each documented status, **When** the client follows the
   document, **Then** it reads the solution, certificate or limit reason that status promises,
   and treats the verification verdict as the document says.
4. **Given** a public item added or renamed in the code, **When** the tests run, **Then** they
   fail until the document is updated.

---

### User Story 2 - Every problem class is tested on small committed instances (Priority: P2)

A maintainer runs the test suite on a machine without the variational kinetics repository and
every supported problem class — LP, QP and conic programmes with each supported cone type, and
infeasible and unbounded cases — is exercised on small instances read from committed files,
each with a known optimum or outcome and a provenance record.

**Why this priority**: today most QP and conic tests build their problems in code or depend on
large extracted fixtures; instance files that any solver tool can read make the tests
self-contained, reviewable and reusable outside the crate.

**Independent Test**: delete or hide the variational kinetics fixtures; the toy-instance tests
still run and pass on each available backend.

**Acceptance Scenarios**:

1. **Given** the committed toy instances, **When** the tests run on a backend supporting the
   instance's class, **Then** each reaches its recorded status and, when optimal, its recorded
   objective within the agreement tolerance, with a Verified report.
2. **Given** an instance of a class the backend does not support, **When** the tests run,
   **Then** the backend reports "unsupported" and the test passes for that reason.
3. **Given** a toy instance file, **When** it is read by an independent solver tool (MOSEK or
   Gurobi from the command line), **Then** it describes the same problem (documented check per
   instance).
4. **Given** the provenance record of an instance, **When** a reader checks it, **Then** it
   states how the instance was made, its known solution and why that solution is correct.

---

### User Story 3 - MOSEK and Gurobi results are compared when both are present (Priority: P3)

A maintainer with both solvers licensed runs the tests; every toy instance (and corpus
instance) that both backends support is solved by both, and a per-instance comparison of
status, objective and verification outcome is reported. With only one solver, each comparison
is reported as skipped with the reason.

**Why this priority**: agreement between two independent solvers is the strongest evidence
that both backends and the verifier are right; feature 002 compared the LP corpus only.

**Independent Test**: with both solvers present, the comparison tests run and report each
pair; with one removed from the build, they report "skipped" naming the missing solver. Gurobi
gains QP and second-order-cone support (quadratic and rotated quadratic cones) so that those
instances are compared too.

**Acceptance Scenarios**:

1. **Given** both solvers available, **When** the comparison tests run, **Then** for each
   instance both backends support, statuses are equal, objectives agree within the agreement
   tolerance (feature 002 research R10), and both reports are Verified, or the disagreement is
   reported with both values.
2. **Given** an instance only one backend supports, **When** the comparison runs, **Then** it
   is reported as not comparable with the reason, never as passed.
3. **Given** only one solver available, **When** the comparison tests run, **Then** each is
   reported as skipped (ignored) with the missing solver named.
4. **Given** a comparison run, **When** it finishes, **Then** a comparison report (instance,
   both statuses, both objectives, difference, tolerance, verdict) is written to the run
   bundle.

### Edge Cases

- An instance whose optimum is not unique: the comparison is on objective value and status
  only, never on x.
- Infeasible and unbounded instances: compared on status; certificates are verified per
  backend, not compared (they are not unique).
- A backend that reports "infeasible or unbounded" where the other is definite: reported as a
  disagreement with both statuses (certificates requested to make statuses definite).
- An instance file a solver tool reads differently from optSolver's reader (for example a
  free-row or bound convention): the documented per-instance check catches it.
- The interface document's examples need a licence to run: examples that solve are compiled in
  every tier and executed where the backend is available, reported as ignored otherwise.

## Requirements *(mandatory)*

### Functional Requirements

**Interface documentation (US1)**

- **FR-001**: One interface document MUST describe, for a reader with no access to the source:
  adding the dependency (crate name, features `gurobi`/`mosek`, build environment variables,
  run-time library path, licence requirements, minimum Rust version); the problem model and
  notation (σ-sense, ½xᵀQx + cᵀx + γ, row bounds in both forms, l ≤ x ≤ u, Fx + g ∈ K, every
  cone with its dimension rules and the svec layout); the CSC input rules and validation errors;
  the zero-copy guarantees and the `i32` versus `usize` row-index choice; backend creation and
  capabilities; every options field; every status, limit reason and what each promises;
  solution fields, sign conventions of y, z, z_l, z_u and s; certificates; the verification
  report and its verdicts; caller-supplied output buffers; errors; threading rules.
- **FR-002**: The document MUST contain complete, runnable examples for an LP, a QP and a conic
  problem, each building the problem over borrowed arrays, plus examples of handling an
  unsupported class, a non-optimal status and caller buffers.
- **FR-003**: Every example in the document MUST be compiled by the test suite and executed
  where its backend is available, so the document cannot drift from the code.
- **FR-004**: A test MUST fail when a public item of the solver-independent interface is not
  mentioned in the document.
- **FR-005**: The document MUST be a stand-alone Markdown file `docs/llm-interface.md`, with an
  `llms.txt` entry point at the repository root pointing to it, and MUST also be the crate-level
  Rust documentation (one source, not two copies), its examples compiled and run as
  documentation tests.

**Self-contained toy instances (US2)**

- **FR-006**: Small toy instances MUST be committed as files: LP and QP in MPS (QP with its
  quadratic objective section), conic problems in MOSEK JTASK; each MUST have a provenance record
  stating its construction, its known status and optimal objective (and optimal x where unique),
  and the derivation of that optimum.
- **FR-007**: The toy set MUST cover: LP (optimal, infeasible, unbounded, ranged rows, free
  rows, empty column, maximise); QP (convex, with off-diagonal terms, maximise); conic: one
  instance per supported cone type (quadratic, rotated quadratic, primal and dual exponential,
  primal and dual power, nonnegative, zero, positive semidefinite) in minimise and maximise
  form where meaningful, plus one conic infeasible instance. Existing small LP fixtures are
  reused, not duplicated.
- **FR-008**: The tests MUST read the toy instances through the shared readers and run them on
  every backend supporting their class; a backend without the class MUST report "unsupported".
- **FR-009**: Each toy instance MUST be checked once against an independent solver tool reading
  the same file (MOSEK and/or Gurobi command-line or MATLAB interface), recorded in its
  provenance (tool, version, status, objective).
- **FR-010**: Toy instances and their tests MUST NOT depend on the variational kinetics
  repository or on the large corpus fixtures.
- **FR-010a**: The instance readers (MPS, JTASK) remain part of the shared test module, not of
  the crate's public interface (clarification 2026-11-15).

**Cross-solver comparison (US3)**

- **FR-011**: When two or more backends are available, every toy and corpus instance supported
  by at least two of them MUST be solved by each and compared: status equal; objectives within
  the agreement tolerance of feature 002 research R10; both verification reports Verified.
- **FR-012**: Comparisons MUST be registered once per backend pair through the shared harness and
  reported as ignored with the reason when a member is unavailable; an instance supported by only
  one member of the pair MUST be reported as not comparable, never as passed.
- **FR-013**: Each comparison run MUST write a report (instance, backend pair, statuses,
  objectives, absolute and relative difference, tolerance, verdict) to the run bundle.
- **FR-014**: The Gurobi backend MUST be extended to convex QP and to conic problems whose cones
  are all quadratic or rotated quadratic (and nonnegative or zero, which are linear), with the
  same solver-independent contract, verification, certificates where Gurobi provides them, and
  status translation; a conic problem with any other cone type MUST be reported by Gurobi as
  unsupported before any solver call (capability check per cone type).
- **FR-015**: The comparison MUST cover LP, QP and the conic instances both backends support;
  the remaining conic instances are reported as not comparable with the reason (cone type not
  supported by Gurobi).
- **FR-016**: The Gurobi extension MUST load Q and F without per-nonzero copies beyond what the
  vendor call shapes force (as feature 002 did for MOSEK), with any forced copy recorded in the
  plan's Complexity Tracking.

### Key Entities

- **Interface document**: the single description of the public interface for client authors
  and LLMs, with executable examples.
- **Toy instance**: a small problem file (MPS or JTASK) with a provenance record holding its
  construction, known outcome and derivation.
- **Comparison report**: per instance and backend pair, both outcomes and the verdict.
- **Gurobi capability per cone type**: which cones the Gurobi backend accepts (quadratic,
  rotated quadratic, nonnegative, zero); others are unsupported.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of the document's examples compile, and every example whose backend is
  available runs to the result the document states (counted per tier).
- **SC-002**: Independent agents given only the document produce client code for a stated LP,
  QP and conic task that compiles and solves the corresponding toy instance correctly in 3 of 3
  attempts per task, without copying caller arrays (model, prompts, code and results recorded
  in the run bundle at Gate 3).
- **SC-003**: Every supported problem class and every supported cone type is covered by at
  least one committed toy instance; the toy-instance tests pass on each backend supporting the
  class, with no access to the variational kinetics corpus.
- **SC-004**: With both solvers available, 100% of instances both support agree in status and
  objective (R10 tolerance) with both reports Verified, or each disagreement is listed with both
  values; with one solver, 100% of comparisons are reported as skipped with the reason.
- **SC-005**: Every toy instance file is read by an independent solver tool to the recorded
  status and objective (provenance), 100% of instances.
- **SC-006**: Gurobi solves every QP and quadratic/rotated-quadratic conic toy instance to its
  recorded status and objective with a Verified report, and rejects every other cone type
  before calling the solver.

## Assumptions

- "JSON input" means MOSEK's JTASK format, the conic fixture format chosen in feature 002
  (research R19); MPS covers LP and QP (quadratic objective section), which both solvers read.
- The positive semidefinite cone may not be expressible in JTASK through an affine conic
  constraint domain that MOSEK documents (feature 002's reader rejects an undocumented
  `svecpsd` domain); if MOSEK writes it, the instance is a file; otherwise the PSD case is
  recorded as an in-code instance with the reason.
- The agreement tolerance is feature 002's research R10 rule (with MOSEK's normalisation from
  R19).
- Licences and vendor installations are those of features 001 and 002; no new solver.
- SC-002's trial runs at verification (Gate 3) with independent agents, not as an automated
  test in the suite.
