# optSolver Constitution

Project-specific values (names, paths, branch roles, supported solver versions) are declared
once in `.specify/memory/project.yml` and are referenced, never restated, here (Governance,
"Project Parameters Live In One Declaration"). Principle numbers of the imported governance
principles (V, VI, IX, XI, XIII, XV) match the variational kinetics repository from which they
were adopted, so shared machinery cites the same numbers in both repositories.

## Core Principles

### I. Performance And Memory First (NON-NEGOTIABLE)

optSolver exists to load large matrix-form optimisation problems into native solvers with the
least possible wall time and memory beyond what the solver itself needs.

* Every design choice on a load, solve or result path MUST be justified by its effect on wall
  time and peak resident memory; convenience never overrides this on those paths.
* Extra peak memory owned by optSolver MUST be O(rows + cols) plus at most one bounded
  temporary index buffer. Any other allocation proportional to the number of nonzeros MUST be
  justified in the plan's Complexity Tracking table.
* Solver environments (including the licence check) MUST be reusable across solves. A loaded
  model MUST be modifiable (objective, bounds, right-hand sides) and re-solvable without
  reloading, so that sequences of related solves keep the solver's warm start.
* Names and logging MUST cost nothing unless requested. Every output the solver makes
  available (duals, reduced costs, slacks, basis, bounds, counts, timings) is returned by
  default; the cost of extracting them MUST be measured and reported per backend, and an
  options-structure field (XVI-C) MAY switch array outputs off for callers making many small
  solves. Outputs that require changing solver behaviour (for example infeasibility
  certificates) are returned only on request.
* Performance is subordinate to correctness. A performance change MUST NOT alter the returned
  solution, status, objective, duals or error behaviour. Input validation MUST NOT be removed
  for speed; a faster unchecked path MAY exist only as an explicit opt-in (VII-D).
* **Solver configuration audit.** For each solver a feature invokes, the plan or its Phase 0
  research MUST enumerate the relevant configuration surface (parameters and their defaults)
  and cross-check each default against the structural profile of the problem data actually
  passed (dimensions, sparsity, integrality, scale). Defaults mismatched to that profile MUST be
  overridden in the single parameter location (VII-G), with the rationale recorded there. The
  audit MUST be grounded in the installed solver's documentation and at least one
  representative instance, not in reputation.

Rationale: the solver dominates run time and memory. The only levers this library has are not
adding copies, conversions or per-element work, and not leaving a solver default that is wrong
for the data it is given.

### II. Testing, Benchmarks, And Behavioural Evidence

* Every behavioural change MUST include the narrowest practical check before it is considered
  complete, and the report MUST state files changed, checks run, tests passed, tests failed,
  and behaviour not yet verified.
* **Two test tiers.** Tests that need no vendor software or licence (validation, conversion,
  chunking, error mapping) MUST run everywhere. Tests that need a licensed solver (known
  optimal values, duals and reduced costs, infeasible, unbounded and empty problems, columns
  with no nonzeros, infinite bounds) MUST be gated so they are skipped, visibly, where the
  solver is absent.
* **Tests are written once and reused.** Every test of solver behaviour MUST be written against
  the solver-independent contract (XVI) and run unchanged against every backend that supports
  its problem class, so adding a backend reuses every existing test. Shared test code (fixture
  loading, objective generation, verification checks, comparison and assertion helpers) MUST
  live in one shared test module that every test reuses; no test re-implements it. A
  backend-specific test exists only for behaviour outside the contract (for example status-code
  translation) and states why.
* **A frozen test corpus.** Test problems are frozen fixtures, each with a provenance record
  (source file and its hash, generating code and commit, seed, citation and licence, X). Any
  data synthesised for a test, such as an objective vector, comes from a recorded seed. The
  corpus is declared in `project.yml`.
* Every invariant checked before an FFI call MUST have a failing-input test, and invalid input
  MUST be shown to fail before any C function is called.
* Where the same problem class is supported by more than one backend, cross-solver agreement
  tests MUST exist.
* Numerical comparisons MUST use tolerances justified by the solver's own tolerances; exact
  floating-point equality is required only for quantities that are copied, not computed.
* **Benchmarks are acceptance criteria.** Every feature touching a load, solve or result path
  MUST state measurable wall-time and peak-memory targets in its Success Criteria, and MUST
  include benchmarks that check them under Principle XIV. A regression against recorded results
  blocks the feature unless the plan justifies it.
* **Behavioural integration evidence.** Where a change affects dispatch, a configuration
  control (VII-G) or a shared service, verification MUST show that a real caller reaches the
  intended behaviour. A test of a configuration object or of source text MAY supplement that
  evidence and MUST NOT be offered as proof the execution path runs. Where execution is
  unavailable (for example no licence), the limitation MUST be recorded and the behaviour left
  explicitly unverified.

Rationale: a single fast run is not evidence. The project needs reproducible evidence that the
intended path was taken, at the intended cost, with the intended result.

### III. Bounded Diagnostics And Preserved Statuses

* Library code MUST NOT write to standard output or standard error directly. Diagnostics are
  returned as data or emitted through the logging mechanism chosen in the plan, gated by a
  verbosity control.
* Solver logs MUST be off by default and enabled only through the single parameter location
  (VII-G).
* Every diagnostic block MUST be gated at its source; volume is fixed where it is emitted, not
  by changing how the code is invoked.
* Results MUST carry both the solver-independent status (XVI-B) and the solver's raw status
  code, unchanged. Errors MUST carry the vendor's error code and message, read immediately after the failing call.
* Suppression applies to trace volume only. Warnings and errors MUST NOT be suppressed.

### IV. Zero-Copy Borrowed Inputs And Bulk Native Calls

* Public input types MUST borrow caller-owned slices (CSC column pointers, row indices,
  values, objective, bounds, row data). Inputs MUST NOT be copied unless a solver's native
  interface requires a different type or encoding, and then only that array is converted, once.
* The library MUST accept each supported solver's native input forms (for example sense plus
  right-hand side as well as row ranges, byte variable-type codes, 32-bit row indices), so that
  callers already holding those forms pay no conversion.
* Each solver is reached through its **native interface**: its Rust API where the solver is
  written in Rust, otherwise its C API through a binding crate (optSolver's own, or an existing
  crate). Problems MUST be passed through the solver's bulk load and slice calls. Per-element or
  per-row calls in loops, intermediate modelling layers, files, subprocesses and IPC are
  prohibited on load and result paths.
* An existing Rust wrapper crate MAY be used for a backend only if it exposes those bulk calls
  taking borrowed slices, with no per-element copy or per-variable object; otherwise the bulk
  calls are bound directly. The plan records which route each backend takes and why.
* Where conversion is unavoidable (row indices to 32 bits), it MUST be fused with validation in
  one pass, MUST be skippable when the caller supplies the native type, and MUST be boundable by
  processing column blocks through one reused buffer.
* Results MUST be written by the solver directly into caller-supplied buffers or into `Vec`s
  the caller can adopt without a copy.

### V. Spec-Driven Scope Control And Reuse

Behavioural changes MUST be driven by a feature specification, implementation plan, and task
list. Each change MUST stay within the smallest coherent dependency set needed for the active
feature.

New dependencies, helper files, abstractions, or repository layout changes MUST be justified by
a feature need and documented in the plan. Retired paths (IX-H) MUST NOT be edited unless the
feature is about them. For substantial changes the relevant files MUST be read and mapped
before editing; search-only edits are insufficient.

**Reuse assessment before new or materially expanded functionality.** A plan proposing new or
materially expanded functionality MUST record, proportionate to the size of the change: the
existing component that owns the responsibility; the functions and contracts actually
inspected, with source references; the disposition chosen (reuse, extend, extract, or new) and
why it beats the smallest conforming alternative; and any duplicated algorithm, policy or
default left in place, and why. "No reusable implementation exists" is a finding that MUST name
the scope inspected and the evidence. No exhaustive census is required.

**Limits.** Correctness takes precedence over shorter code. Small local repetition MAY be
preferable to an incoherent abstraction or an added coupling. Generalisation requires a present
feature need and an identifiable consumer; in particular a cross-backend abstraction MUST NOT be
extracted before at least two backends implement the behaviour concretely. This principle sets
no function-length limit and no obligation to refactor code the feature did not touch.

Rationale: controlled scope keeps unrelated movement from masking the behaviour under review,
and answering the reuse question, rather than assuming it, stops a second implementation of an
owned responsibility entering unexamined.

### VI. Strict Spec-Driven Development Gate

Implementation work MUST NOT begin from an ordinary natural-language request. All code
development MUST follow the Spec Kit workflow unless the user gives an explicit
`SPECKIT OVERRIDE`. This includes library code, tests, benchmarks, build scripts, bindings and
any other scripts, regardless of language, size, location, persistence or version-control
status, including temporary scripts, inline interpreter code and delegated work. Calling work
research, verification, a one-off or a small fix does not exempt it. There is no standing
exception.

Reading files, editing Spec Kit governance documents and exploration records, downloading
literature into its declared home (IX-J), and invoking existing tools without developing code
are not code development.

Ordinary requests such as "fix this", "implement this", "apply the above advice" or "now edit
the code" MUST NOT by themselves authorise implementation edits; they are requests to enter or
continue the workflow unless the required artifacts exist and the user explicitly invokes the
implementation phase.

The required phase order is:

```text
constitution
specify
clarify, if requirements are ambiguous
plan
architecture review
tasks
analyze, if available
implement
```

Tasks MUST NOT be generated before the feature's architecture delta is approved (XIII-C).
Implementation MUST NOT begin until the feature has a current `spec.md`, `plan.md` and
`tasks.md`, any required clarification or analysis artifacts, and no unresolved blocker.

Implementation files may be modified only when the user explicitly invokes the implementation
phase with the agent's implementation command (`/speckit-implement` or its equivalent), or with
the direct instruction `Run the Spec Kit implementation phase for the active feature`.
Planning-only prompts (a plan, design, explanation, debugging advice, research) MUST stop before
code edits. Brownfield and bug-fix work MUST attach to, or create, Spec Kit artifacts first. If
the user asks for code changes before the prerequisites exist, the agent MUST decline briefly and
name the next required command.

`SPECKIT OVERRIDE` is the single override mechanism. Only the user's use of that exact token
authorises it; an agent MUST NOT infer it from urgency or a broad request. It applies only to the
work the user identifies; the agent MUST state its scope and reason, the gates bypassed, and the
validation done. It never justifies a false verification claim.

These requirements are agent-neutral and apply identically to every agent or model.

Every implementation run MUST write the implementation receipt defined in Development Workflow
And Quality Gates. Every implementation decision MUST be traceable to this constitution, the
spec, the plan, a task, and the receipt. When in doubt, stop before editing and ask which phase
runs next.

#### VI-A. Human-Loop Gates Are Binding Where The Extension Is Installed

Where `.specify/extensions/human-loop/workflow.md` is present, the gates it defines — **Gate E,
Gate 1, Gate 2A, Gate 2 and Gate 3** — are MANDATORY for every agent. A gate MUST NOT be skipped
silently: a deferred or unrun gate is recorded in the feature's `human-loop.md` with its reason
and surfaced at the next gate. Where an agent has no interactive multiple-choice mechanism, the
gate is run as an explicit question and the answer recorded. Substituting an agent's own
equivalent mechanism is expected; omitting the gate is not.

Rationale: without binding gates an agent could run every phase and present finished work with
no human decision recorded anywhere. The gates are the points where a person, not a machine,
decides scope, architecture and approval.

#### VI-B. A Checklist Reaching A Gate Is Resolved Or Explicitly Routed

Every item of every checklist under `specs/<feature>/checklists/` MUST, when presented at a gate,
be either **resolved** (marked complete, with the closing artifact identifiable; marking asserts
verification) or **explicitly routed** (left open with its destination named: a later gate, a
task ID, or a recorded human decision to accept it open, with rationale). An unmarked item is
neither, and a checklist carrying one has not passed. The routing record lives in the feature's
`human-loop.md`. Accepting an item open is a human decision; an agent MUST NOT close an item by
declaring it unimportant.

Rationale: in the variational kinetics repository, sixteen features reached gates with their
domain checklists entirely unmarked; one, verified late, had only 13 of 36 items genuinely
satisfied and four defects that blocked coding.

### VII. Rust And FFI Coding Standards

These rules bind every Rust file in the repository, every agent and every contributor.
VII-A to VII-F govern code that crosses into C. They apply in full to binding code optSolver
owns. Where a backend uses an existing binding crate instead, the plan MUST state how that crate
meets VII-A to VII-E for the calls used, or record the gap as accepted risk; a backend over a
pure-Rust solver has no FFI and those sub-parts do not apply to it.

#### VII-A. Unsafe Code Is Confined And Argued

* `unsafe` code MUST be confined to FFI modules. Every crate or module with no FFI MUST carry
  `#![forbid(unsafe_code)]`.
* Every `unsafe` block MUST carry a `// SAFETY:` comment naming the invariants it relies on.
  Every public `unsafe fn` MUST document a `# Safety` contract.
* A pointer derived from a shared slice MAY be passed to a C parameter declared non-`const` only
  when the vendor documents that parameter as input-only, or documents it only as data used to
  initialise or describe the call's input and a recorded probe on the supported version shows the
  array unchanged after the call. The `SAFETY` comment MUST cite that documentation (and the
  probe, where used), and a probe relied on MUST be kept as a licensed test so that a change in a
  new vendor version is caught.

#### VII-B. Every Native Handle Is Owned

* Every C handle (environment, model, task) MUST be owned by a Rust type that frees it in
  `Drop`, held as a non-null pointer, with each vendor handle a distinct opaque type.
* Lifetimes MUST prevent a model or task outliving the environment it was created in.
* `Send` and `Sync` MUST NOT be implemented for a handle type unless the vendor's documentation
  supports it; the implementation MUST cite that documentation.

#### VII-C. Errors, Statuses And Panics

* A non-optimal outcome (infeasible, unbounded, limit reached) is a status, returned in `Ok`.
  Errors mean the solver could not be run or the input was invalid.
* Library code MUST NOT panic on user input. Panics are reserved for broken internal invariants.
* A panic MUST NOT unwind across an FFI boundary. Callback bodies MUST catch unwinding, stop the
  solve through the vendor's mechanism, and resume the panic after the solve returns.
* Public enums and result structs MUST be `#[non_exhaustive]` where future additions are
  plausible.

#### VII-D. Inputs Are Validated Before The First FFI Call

* All invariants the C libraries assume (array lengths, column pointer monotonicity, index
  ranges, dimensions within the vendor's integer width, 64-bit target) MUST be validated before
  the first FFI call, in a single pass fused with any conversion the backend needs.
* A validated input type is the proof of validation: its fields are private and it is
  constructed only by the validating function. An unchecked constructor MAY exist only as an
  `unsafe fn` with a `# Safety` contract.
* Every vendor return code MUST be checked immediately after the call.

#### VII-E. Vendor C Interfaces Are Bound As Verified

* Every vendor declaration optSolver itself makes MUST be checked against the installed vendor
  header for the supported version declared in `project.yml`, and where the header is present a test MUST check
  the Rust declarations against it.
* A vendor entry point that is a C macro is not a linkable symbol. Its underlying function MUST
  be called with arguments (such as version numbers) taken from the vendor's header or library,
  never hard-coded.
* Bindings generated from vendor headers are derived from vendor files and MUST NOT be committed
  unless the vendor's licence permits it (X).

#### VII-F. Crate Hygiene

* Edition and minimum supported Rust version are declared once for the workspace and tested.
* Lints are declared once for the workspace and include, at minimum: `unsafe_op_in_unsafe_fn`
  denied, `clippy::undocumented_unsafe_blocks` denied, `clippy::cast_possible_truncation`
  warned. `non_snake_case` MAY be allowed only on the public problem and result types, so their
  fields follow the Notation Convention (XII-C).
* Where optSolver owns the binding to a vendor library, exactly one of its crates declares
  Cargo's `links` key for that library, and none does where an existing binding crate is used.
  A binding crate's build script MUST
  declare every environment variable it reads, and MUST NOT embed licence data or absolute
  machine paths in any committed file.
* A library crate MUST NOT set a global allocator and MUST NOT rely on its own build profile.

#### VII-G. Solver Parameters Have One Location

Every solver parameter is set in exactly one location. A solver parameter is any vendor
optimiser parameter (for example Gurobi parameters, MOSEK integer, double and string
parameters) and any optSolver option that maps to one (time limit, threads, tolerances, method,
log output).

* **One declaration of options and defaults.** The options structure (XVI-C) and every
  default optSolver imposes are declared in one module of the solver-neutral core. A field the caller
  leaves unset means "the solver's own default" unless that module overrides it, and every such
  override records its rationale beside it (I, solver configuration audit).
* **One writer per backend.** In each backend, exactly one function applies parameters to the
  solver. It is the only code that calls the vendor's parameter-setting functions, whether on an
  environment or a model. Everywhere else, code MAY read options and MUST NOT set a solver
  parameter. This is checkable: the vendor's parameter-setting symbols appear in that one module
  only.
* **Caller passthrough.** A value the caller supplies MUST reach the solver unchanged. Every
  vendor parameter MUST be reachable through a raw name/value passthrough, so that callers (such
  as a COBRA adapter) can set any parameter without a code change here. Precedence is declared
  once in the options module; setting the same vendor parameter by two routes in one call is an
  error, not a silent choice.
* **Validation co-located.** Parameter validation happens in the applying function, before or
  as the parameter is set, and errors name the parameter.
* **Changes between solves go through the same function.** Re-solving with different
  parameters passes a new options value through the applying function; no ad hoc setter call
  exists anywhere else.
* **The effective parameters are reportable.** The applying function MUST be able to report the
  complete set of parameters it set, so a result can be reproduced (XIV-B).
* Adapters (for example the RunMat adapter outside this repository) translate their callers'
  parameter structures into the options type; they MUST NOT call vendor parameter functions.

Rationale: parameters set in several places silently override caller values and hide the
effective configuration. In the variational kinetics repository, distributed unconditional
assignments clobbered caller-supplied settings and masked an invalid verbosity value until all
defaulting was concentrated in one guarded, caller-respecting function. One location makes the
effective configuration readable from one place and lets any parameter be changed for an
experiment without editing solver code.

### VIII. Independence And Additive Backends

* optSolver MUST NOT depend on RunMat or on any particular sparse-matrix crate. Adapters for
  sparse-matrix crates MUST be optional and borrow-only. The RunMat adapter lives outside this
  repository.
* Each solver backend MUST sit behind its own Cargo feature. Features MUST be additive: no
  feature may change or remove behaviour of another, and no features may be mutually exclusive.
  Vendor version selection MUST NOT use mutually exclusive features.
* The solver-neutral core MUST build and pass its tests on a machine with no vendor software or
  licence.

### IX. Repository File Organisation And Artifact Placement

Every file MUST be placed by its role at the destination that
`docs/architecture/repo_folder_organisation.md` states for that role. **That document is the
single source of what goes where.** This constitution states no destination; any other document
that restates one is a defect, corrected by replacing the restatement with a pointer.

#### IX-A. Source And Generated Output Never Mix

A source tree MUST contain source only. Generated output (benchmark results, reports, profiles,
build artefacts) MUST NOT be placed in a source tree. The repository root MUST hold only
project-level metadata and configuration. Test fixtures consumed by tests are inputs, not
generated output.

#### IX-B. Dependency Manifests Travel With Their Module

A dependency manifest MUST exist only at its own crate's or workspace's root.

#### IX-C. Generated Output Has One Home Per Piece Of Work

All generated output of a feature, exploration or session MUST live in the one home the
placement document assigns to that piece of work, keyed by one name. Whether any of it is
tracked is stated in `.gitignore` only (Governance, "Tracking Is Declared Once").

#### IX-D. File-Role Categories Are Defined Once

The file-role categories and their destinations are defined in the placement document and MUST
be used. A retired destination MUST NOT be written to.

#### IX-E. Run Output Stays In-Repo And Is Bundled Per Run

Benchmark and experiment run output MUST be written into the repository, bundled per run with
its logs, in the layout the placement document defines. Code MUST NOT hard-code output paths or
write outside the repository.

#### IX-F. Placement Decision Procedure

When creating or relocating a file, decide its role, find that role in the placement document,
and use the destination stated there. If no role fits, stop and report the gap; the destination
is decided by amending that document first. Program code MUST NOT hard-code a destination; it
resolves one through `.specify/memory/project.yml`.

#### IX-G. Binary And Large-File Versioning

Large binary files (for example large test models) MUST NOT enter ordinary Git object history.
If they are tracked, `.gitattributes` routes them through Git LFS.

#### IX-H. Retired And Historical Content

Retired content is placed where the placement document states. It is read-only: it MUST NOT be
edited as part of feature work, and new work MUST NOT start inside it.

#### IX-I. Paths Are Resolved, Never Counted Or Stored

Code that locates the repository root or a file relative to it MUST resolve it from a stable
anchor (in Rust, the crate's manifest directory provided by Cargo; in scripts, an upward search
for `.specify` or `.git`) and MUST NOT count parent levels or store an absolute path. Python and
shell resolve declared locations through `.specify/scripts/python/project_params.py`.

#### IX-J. External Literature Is Downloaded Source With Recorded Provenance

The literature home holds external material downloaded from the internet (books, guides,
papers), each with its upstream licence and an entry in that home's sources index giving origin,
pinned version or URL, access date and licence. It MUST NOT hold notes, guides or summaries
written for this project; those are placed by their role (IX-F). Material whose licence does not
permit redistribution is kept local, excluded through `.gitignore`.

### X. Provenance, Licensing And Verified Facts

* Vendor headers, libraries, licence files and licence keys MUST NEVER be committed.
* Every external source used (crate, document, downloaded literature, test model) MUST have its
  origin and licence recorded. Third-party material MUST NOT be committed unless its licence
  permits redistribution.
* Every vendor function, signature, integer width, constant and behavioural claim relied on in
  code MUST be checked against the installed vendor header or the vendor's documentation for the
  supported version, and the check recorded with its date; unverified claims MUST be marked as
  such.

### XI. Problem-Space Exploration Before Specification

Problem-space research (the rationale that decides WHETHER a feature should be specified) MUST
be recorded when it occurs, in a durable artifact any agent can resume.

1. **The exploration artifact** is `specs/<name>/exploration.md`, instantiated from
   `.specify/templates/exploration-template.md`, the first artifact in a feature directory.
   Supporting material lives in an `exploration/` subdirectory beside it.
2. **Provisional naming, mature at go.** At exploration start the directory is
   `specs/<YYYYMMDD-HHMMSS>-<slug>/` with a full UTC timestamp, which `create-new-feature.sh`
   excludes from its sequential-number scan. On a "go" decision it is `git mv`'d to the next
   sequential `specs/<###-slug>/` and the specify phase reuses it
   (`create-new-feature.sh --number <###> --short-name <slug> --allow-existing-branch`). On
   no-go or park the timestamped directory remains as the recorded dead end.
3. **State externalisation and multi-agent discipline.** The exploration file, not any
   conversation, is the working memory. The template's rules are normative: append-only
   ledgers, agent identity and UTC timestamp on every entry, unique sequential entry IDs, refuted
   entries marked and never deleted, and a Synthesis section regenerated whole by whichever agent
   writes last.
4. **Human-only decision.** Only the human sets the exploration Status and signs the Decision.
   Before ending a substantive discussion of an incomplete feature, the agent MUST refresh one
   concise `Current conversation summary` below the title of that feature's `exploration.md`;
   it grants no implementation authority.
5. **Exploration grants no implementation authority.** Explore mode MUST NOT edit code, tests or
   build files.
6. **Spec provenance.** Every new `spec.md` MUST carry an **Exploration** field naming its
   exploration artifact, or stating "none — direct request" with the reason. Requirements
   motivated by exploration evidence MUST cite its entry IDs.
7. **When exploration is expected.** An exploration SHOULD be opened when the next feature is
   not yet decided, in particular at close-out of a feature that surfaced follow-up questions.

`specs/000-solver-interface-exploration/` predates this principle and is the exploration record
for the repository's founding design; it is not renamed or reformatted, and it MUST be read before
any feature work.

### XII. Names, Notation And History

#### XII-A. Descriptive Names, Not Codes

Every concept MUST be referred to by a human-readable descriptive name in specifications,
plans, tasks, reports and every report to the maintainer. Meaningless alphanumeric symbols
(single letters, letter-digit codes, register numbers) MUST NOT replace a name. Where a format
needs a stable key (a task identifier, a ledger entry), the key MAY appear beside the name as a
locator, and the sentence MUST read correctly with the key deleted. Identifiers with intrinsic
meaning (function, type, field and file names), mathematical symbols, feature directory numbers
and units are not affected. The constitution's principle numerals are locators; where cited, the
principle's name accompanies them.

#### XII-B. Historical Prose In Code Sits At The End Of The File

A comment recording history (what code used to do, which feature changed it, why an earlier
approach failed) MUST appear at the end of the file under a heading marking it as history, not
in a header, a documentation comment, or among the statements it describes. This binds a file
when it is edited and authorises no sweep of existing files.

#### XII-C. One Symbol Per Object, In Householder Notation

One mathematical object has one symbol, the same in source code, documentation, specifications
and discussion, as recorded in the Notation Convention section. Before introducing a symbol, the
inventory MUST be consulted; an object that already has a symbol keeps it, and a symbol MUST NOT
be reused for a second object. Where a solver imposes its own names (for example Gurobi's
`vbeg`, `vind`, `vval`), they MAY appear locally in binding code, with the mapping to the
canonical symbol stated beside them. A new object's symbol is added to the inventory by the
feature that introduces it, through `/speckit-constitution`.

### XIII. Architecture Conformance And Component Ownership

The repository's name for documentation, architecture artifacts and discourse is
`identity.repository_name` in `project.yml`.

#### XIII-A. The Architecture Model Is An Authoritative Subordinate Companion

The architecture model lives in `docs/architecture/`: `components.yaml` and `boundaries.yaml`
hold the component model and design boundaries, and `repo_folder_organisation.md` holds the
placement rules (IX). The model is subordinate to this constitution: it MUST NOT restate a rule
the constitution binds, it cites the principle instead, and it MUST NOT assert a component that
does not exist without marking it as specified but not yet created.

#### XIII-B. Ownership Is Declared Only In The Architecture Model

Component boundaries and file, module or function ownership are declared in
`docs/architecture/` and nowhere else. Agent definition and instruction files MUST NOT assert
ownership of any file, directory or component; they MAY carry only a derived scope statement
citing the component it derives from.

#### XIII-C. Every Feature Declares An Architecture Delta, Approved Before Tasks

Every feature MUST carry `specs/<feature>/architecture-delta.yaml` declaring the components and
dependency edges it adds, modifies or removes, the public interfaces and persistent data
structures it changes, and the edges it declares forbidden.

NO TASKS ARE GENERATED UNTIL THE ARCHITECTURE DELTA IS APPROVED. The review terminates in one of
four verdicts: `APPROVE`, `APPROVE_WITH_ADR`, `REVISE_ARCHITECTURE` or `REJECT`. Only the first
two permit task generation; `APPROVE_WITH_ADR` requires an ADR under `docs/architecture/decisions/`
before implementation. An ADR is REQUIRED when a feature introduces a crate or layer, changes an
allowed dependency direction, introduces a shared data representation, changes an encapsulation
boundary, couples to a solver or framework, or deliberately accepts architectural debt. The
review also examines semantic duplication left in place, reachability of anything proposed as
shared, and whether a declared adoption scope names real consumers; these are grounds for
`REVISE_ARCHITECTURE`.

#### XIII-D. Observed Architecture Is Reconciled Against The Approved Delta

At closeout the observed change MUST be compared with the approved delta and reported as:
approved and implemented; approved but not implemented; implemented but not approved; existing
violations worsened. Implemented-but-not-approved components or edges are drift and MUST be
reported. Closeout also reconciles consumer adoption, remaining duplication, the behavioural
integration evidence of Principle II, documentation and contracts the change invalidated, and
remaining work routed to a named owner. A model being valid, generated artifacts being fresh,
and source conforming are three separate facts and MUST NOT be conflated.

#### XIII-E. Granularity, Extraction, And Enforcement

The component model is resolved at module and function granularity; one crate may realise
several components. Building the model is read-only with respect to source. A proposal to share
or extract a function MUST state the one operation and contract being shared, where it belongs
in the ownership model, which callers must reach it and the Rust visibility and crate-dependency
constraints on that, whether extraction changes a dependency direction or creates a cycle, and
the distinctions the shared interface must preserve. Reuse MUST NOT make a lower layer depend on
a higher one and MUST NOT introduce a framework for a single caller. Conformance is checked at
the architecture review and again at closeout.

#### XIII-F. Naming A New Layer Is A Human Decision, Taken At The Architecture Delta

A name MUST mean what the thing does. Where a change would alter what an existing name denotes
(inserting a layer, splitting or merging a component, promoting an internal helper to a public
element), the agent MUST stop before implementing and ask the maintainer for the names. The
answer is recorded in the delta and its Gate 2A record. A standing instruction to proceed covers
execution, not naming.

#### XIII-G. A Shared Capability Is Not Delivered Until A Real Consumer Adopts It

When a feature introduces or materially changes a reusable capability, the plan MUST name the
real consumers and a bounded adoption scope, which consumers migrate now and which deliberately
remain, at least one representative consumer test, and the disposition of each replaced
implementation. Isolated tests of the capability are not evidence of adoption. Callers outside
the approved scope MUST NOT be migrated to satisfy this.

#### XIII-H. Contracts Across Component Boundaries Are Declared

Where one component consumes another's configuration, results or metadata, it SHOULD do so
through a declared interface or data contract. A new dependency on another component's private
implementation details MUST be justified in the plan and recorded in an ADR if durable.

### XIV. Evidentiary Preconditions For Performance Claims

A claim that optSolver is fast, memory-efficient, or faster than an alternative is a measurement
and MUST meet these preconditions before it is reported.

#### XIV-A. Benchmark Instances Are Declared

Every benchmark names its instances (with dimensions, nonzeros, problem class and source, whose
provenance is recorded under X) and states each criterion per instance. Results MUST NOT be
aggregated across unlike instances.

#### XIV-B. The Effective Configuration Is Part Of The Measurement

Every recorded result MUST carry the complete effective solver parameter set (VII-G), the solver
versions, the optSolver commit, the machine, and the build profile. Comparisons with an
alternative (a vendor MATLAB interface, another crate) MUST use identical instances and
equivalent parameters, or state the difference.

#### XIV-C. Phases Are Measured Separately

Environment creation, load, solve, and result extraction MUST be timed separately, and cold and
warm runs distinguished. Peak memory MUST be measured in a process dedicated to one scenario,
since a process's high-water mark is shared by everything it ran, and reported relative to the
solver's own model copy where that can be established.

#### XIV-D. A Null Result Requires A Positive Control

Where a finding could be "no difference" (for example that chunked loading costs nothing), the
feature MUST first show that the mechanism under test is actually engaged (that the chunked
path, the zero-copy path or the warm start was taken). An unwired mechanism and an inert one
produce identical output.

### XV. Agent Working Discipline And Result Reporting

These rules are agent-neutral. The governing habit: **before running anything expensive, ask what
would have to be true for a clean-looking result to mean what it will be made to mean, and check
that first.**

#### XV-A. Every Reported Number Carries Its Replicate Count And Its Scope

* Every timing or memory figure MUST state the number of repetitions behind it and its spread; a
  single run is labelled `N = 1`.
* A claim MUST name what it was measured on (instance, solver, machine) in the sentence making
  it. A claim stated more broadly than it was measured is a defect.
* Where the evidence supports one of two readings, the agent MUST say which and why.
* A claim later falsified MUST be marked withdrawn with its reason in the feature's
  `human-loop.md`, never silently deleted.
* An evidence standard stated by the maintainer MUST be written into the spec, the harness or
  this constitution in the same session.

#### XV-B. The Premise Is Established By Reading The Path, Not By Assuming It

Before a value is described or built upon, the code path that produces it MUST be read from its
first line, guard clauses first. Check the type and length of any solver output before using it.
A quantity's name is not its contract.

#### XV-C. A Guard Is Tested Against The Failure It Guards

Any check, gate or comparison harness MUST be exercised against the failure it guards, not only
against success; a comparison harness MUST include an "everything failed" case reported as
uninterpretable, never as agreement. An agent's own threshold is the first suspect when it
rejects a case; ground thresholds in quantities the solver defines. A builder that can detect at
construction that an artifact would be uninterpretable SHOULD refuse to emit it.

#### XV-D. Work In Progress Is A Claim Requiring Evidence

"Still running" MUST be evidenced by the time of the last write to the job's output, not by the
existence of the process. Large output MUST NOT be allowed into an agent's context: redirect it to
a log and search it. Before concluding from a log, establish that it is complete.

#### XV-E. A Prior Ruling Is Not Reversed As A Side Effect

Before adding or widening a repository-wide rule (an ignore pattern, a default, a naming
convention), search for the rule being contradicted; a task description is not authority to
reverse a deliberate earlier decision, and such a task MUST be raised rather than executed.
Overriding a control is sometimes right and never silent: record the grounds.

### XVI. A Solver-Independent Contract

Callers program against one solver-independent contract. Solver-specific behaviour lives behind
it, in each backend, and is reached only through the escape hatches this principle names.

#### XVI-A. Problems And Results Have Generic Formats

* Every backend MUST accept the same solver-independent problem type and options structure, and
  return the same solver-independent result type. No solver-specific type appears in a solve
  entry point's signature other than the backend's own environment or handle.
* Solver-specific information in a result is confined to clearly separated fields: the raw
  status code, the solver's identity and version, and any optional solver-specific extension.
  Nothing a caller needs in order to interpret a solve requires reading those fields.
* The representation of the generic types (borrowed views, traits, ownership of results,
  caller-supplied buffers) is NOT fixed by this constitution. It is decided by a feature, through
  exploration and plan, and recorded in the architecture model. Whatever is chosen MUST satisfy
  Principles I and IV.

#### XVI-B. A Solver-Independent Status Says How The Solve Went

* Every result MUST carry a status from one solver-independent, named (non-numeric) vocabulary
  declared once in the solver-neutral core. The vocabulary MUST distinguish at least: optimality proven;
  infeasibility proven; unboundedness proven; infeasible-or-unbounded undetermined; stopped at a
  limit; stopped by numerical difficulty; interrupted; and, separately from the status, whether a
  feasible solution is returned.
* Each backend translates its solver's codes into that vocabulary in exactly one translation
  function. The translation MUST be total over every code the supported solver version
  documents; a code not recognised maps to an explicit unknown status that keeps the raw code,
  never to a guess and never to a panic.
* Each translation MUST be tested code by code; the test needs no licence.
* Translation to any caller's own convention (for example COBRA's numeric status values) is the
  caller's adapter's job and lives outside this repository.

#### XVI-C. Non-Default Solver Behaviour Is Requested Through A Structure

* A solve MUST take an options structure whose fields set non-default solver behaviour. A field
  left unset requests default behaviour; an empty structure requests default behaviour
  throughout, meaning the solver's defaults modified only by the audited overrides declared in
  the options module (I, VII-G).
* Fields for concepts shared across solvers (time limit, threads, tolerances, method, logging)
  have solver-independent names. Where a shared field's meaning differs between solvers, its
  documentation states the mapping for each backend.
* Solver-specific parameters are set through the structure's raw name/value passthrough
  (VII-G), never through another route.
* A field a backend cannot honour MUST NOT be silently ignored: it is honoured, rejected as an
  error, or reported in the result, by a rule declared once in the options module.

#### XVI-D. Every Solution Is Verified Independently Of The Solver

* The solver-neutral core MUST provide solver-independent verification that measures, from the generic
  problem and the generic result alone, how well the returned solution satisfies the
  feasibility and optimality criteria of its problem class. For LP and MILP this covers at least:
  violation of row bounds, variable bounds and integrality; and, where duals are returned, dual
  sign feasibility, complementary slackness, and the gap between primal and dual objectives
  (for MILP, the gap to the reported bound). Where a solver returns a certificate of
  infeasibility or unboundedness, verification checks it.
* Measures are reported individually, in absolute and scaled form, not collapsed into one
  number, and never recomputed from the solver's own reported residuals.
* Verification MUST NOT change the status: a disagreement between status and verification is
  reported alongside both.
* Every licensed-solver test MUST verify each solver's returned solution with it, and
  cross-solver agreement (II) compares verified measures. Verification MUST itself be tested
  against deliberately violated solutions (XV-C).
* Whether verification runs during ordinary solves, and the definition of its tolerances and
  scaling, are decided in a feature; if it runs at solve time it is controlled through the
  options structure (XVI-C) and its cost is reported.

Rationale: a caller comparing or switching solvers must not need to know any solver's status
codes, parameter names or data layout, and must not have to trust a solver's own claim of
optimality. A single contract with independent verification is what makes results from
different solvers comparable at all.

## Notation Convention

Householder notation: matrices are upper-case Roman letters, vectors lower-case Roman letters,
scalars Greek letters where practical. In code, a symbol is written as its letter (`A`, `Q`,
`c`), and a subscript or qualifier becomes a descriptive suffix.

| Symbol | Object |
|---|---|
| A | Constraint matrix, m rows by n columns, stored in CSC form |
| Q | Quadratic objective matrix, n by n |
| c | Linear objective coefficient vector, length n |
| b | Row right-hand side vector, length m, used with a row sense |
| l | Variable lower bound vector, length n |
| u | Variable upper bound vector, length n |
| x | Primal variable vector, length n |
| b_l, b_u | Lower and upper row bounds, length m, when rows are given as ranges |
| y | Row dual vector, length m |
| z | Reduced cost vector, length n |
| γ (`gamma`) | Objective constant |
| σ (`sigma`) | Objective sense: +1 minimise, −1 maximise |
| m | Number of rows of A |
| n | Number of columns of A |

`m` and `n` are dimensions, written in lower case by convention.

## Technical Constraints

* Language: Rust. Edition and minimum supported version per VII-F. One crate, `optsolver`,
  with solver backends as modules behind additive features.
* Targets: 64-bit only, asserted at compile time.
* Supported solvers and versions: declared in `project.yml`. Further solvers (with a Rust API,
  or a C API reachable through a binding crate) are added only after checking for a bulk CSC or
  CSR load call.
* Problem classes, in order: LP and MILP; convex QP; later QCP and conic.
* The public API is synchronous; asynchronous callers run solves on blocking threads.
* Licence of optSolver's own code: MIT OR Apache-2.0.

## Development Workflow And Quality Gates

Before implementation, the plan MUST pass the Constitution Check in the plan template; any
violation MUST be listed in Complexity Tracking with a reason and a rejected simpler alternative.
Tasks MUST be ordered so tests are created before or with the behaviour they verify, and each
user story MUST be independently testable.

At the architecture review and at closeout, the following MUST be run and their results recorded
(there is no continuous integration; enforcement is in-gate): formatting check, linting with
warnings denied, the unlicensed test tier, feature-combination checks proving additivity (VIII),
the minimum-supported-version build, and, where a licence is present, the licensed tier.

Review MUST confirm: no new copies on hot paths (I, IV); solver parameters set only in their one
location (VII-G); the solver-independent contract kept, statuses translated totally and
solutions verified (XVI); `SAFETY` comments and validation before FFI (VII); feature additivity (VIII);
tests and benchmarks present (II, XIV); provenance recorded (X).

After implementation, the contributor MUST report files edited, checks run, what passed, what
failed, and what remains unverified.

### Implementation Receipt Ledger

Every implementation run MUST write a receipt at
`<FEATURE_DIR>/agent-runs/<UTC-timestamp>-<short-task-or-run-name>/implementation-receipt.md`.
Implementation is not complete until it exists. It records only: **Prompt**, **Final response**
(the actual final user-facing completion response, verbatim, not a paraphrase), **Diff
summary**, **Tests**, **Unresolved issues**, and optionally a brief **Other information**. It
MUST exclude interim progress messages, raw transcripts, credentials, personal data and large
logs; any redaction MUST be marked.

## Governance

This constitution supersedes conflicting habits, templates and preferences. The Strict
Spec-Driven Development Gate has precedence over convenience, speed and agent initiative.
Amendments require an update to this file, a Sync Impact Report in the changelog, review of
dependent templates and skills, and review of affected agent instruction files. Feature plans and
reviews MUST check compliance before implementation and again before completion.

Versioning follows semantic versioning: MAJOR for incompatible governance changes, principle
removals or redefinitions; MINOR for new principles, sections or materially expanded
requirements; PATCH for clarifications.

### Commits Carry In-Flight Manual Work

Commits, pushes and pull requests are made only on the maintainer's explicit instruction; a gate
answer MAY give that instruction. Whenever a commit is made, by any agent or by hand:

* it MUST include the in-flight manual changes to tracked files present in the working tree, and
  the message MUST list them, one line each, distinguished from the work being committed, with
  their verification status stated plainly;
* untracked files are NOT swept in: each is named deliberately and checked against IX and X
  before being added;
* gitignored artifacts, vendor files, credentials, secrets and personal data are never staged;
* an in-flight change that looks like a partial refactor, a debugging scaffold or an unreadable
  file is reported and asked about, not committed unexamined.

Carrying a manual edit into a commit is not an agent implementation and needs no receipt.

### Feature Branches Begin And End At The Integration Branch

The integration branch is `branches.integration` in `project.yml`.

* A feature branch MUST be created from the integration branch, after updating it from its
  remotes if any exist. `create-new-feature.sh` branches from whatever is checked out; checking
  out the integration branch first is the enforcement.
* At Gate 3, and only on "accept and close", the integration branch is brought up to date with
  the feature branch (fast-forward where possible, a merge commit otherwise) and the working
  branch switched to it. Where remotes exist, the integration branch is published to each and
  the tip each host shows is recorded. The integration branch MUST NOT be force-pushed or have
  its history rewritten. Feature branches are kept.
* Two features MUST NOT share a working tree; a feature started while another is open gets its
  own `git worktree` from the integration branch.
* A feature's work is committed to its branch, when the maintainer authorises it, before each
  gate is presented.
* At the entry of every feature the agent confirms that closed features are contained in the
  integration branch, that local and remote integration branches agree where remotes exist, and
  that the working tree holds no other feature's uncommitted work.
* Before integrating, check that no file exceeds the hosting limit declared in `project.yml`.

### Agent Instruction Files Delegate To Spec Kit

Control over agent behaviour resides in Spec Kit (this constitution, the skills and the
templates), not in per-agent instruction files. `CLAUDE.md` and `AGENTS.md` are thin delegators
with identical content: a statement naming this constitution as the single source of truth and
`/speckit-constitution` as the way to change it, and stable pointers to the Spec Kit workflow and
skills. They MUST NOT hold active-feature names, generated context blocks, runtime instructions
or any behavioural rule; such content found there is relocated into Spec Kit and replaced with a
pointer. Tool-specific adapters translate command names or interaction mechanisms only and MUST
NOT create different permissions, gates or evidence standards.

### Project Knowledge Lives In The Repository

Project rules, standards, workflow, environment facts and findings MUST be recorded in this
repository, readable by any agent. An agent-private store (a per-agent memory directory, a
host-managed memory feature, a machine-local scratch file) MUST NOT be their home; an agent MAY
keep a private pointer to the repository copy. Placement by kind: a rule → this constitution; an
operational how-to for a recurring task → a skill under `.specify/skills/`; an observed fact
about code, toolchain or environment → `docs/`; the state of a feature → its `specs/<feature>/`
artifacts; research → the exploration or research artifact of the feature it informs. A lesson
that generalises beyond its feature MUST be promoted; feature closeout asks whether anything
needs promoting. Knowledge found in a private store is migrated here and the private copy reduced
or removed.

### Project Parameters Live In One Declaration

Every project-specific value MUST be declared in `.specify/memory/project.yml` and MUST NOT be
restated elsewhere; documents, skills, templates and code reference it. Paths in it are relative
to the repository root, which is resolved at run time and never stored. Machine-specific
locations (vendor installations, licence locations) are not parameters: they are resolved at run
time and recorded in each run's provenance. The declaration originates no rule; where it
disagrees with this constitution, the declaration is the defect. An agent MUST NOT change a
declared value without the maintainer's instruction; adding a new kind of parameter is a
governance change.

### The Markdown Control Surface

Every Markdown file sits in exactly one tier, which fixes its authority. A lower tier never
overrides a higher one; where two conflict, the lower is the defect.

* **Tier 0 — sole source of behavioural truth.** `.specify/memory/constitution.md`.
* **Tier 1 — binding subordinate companions; detail or enforce, never originate.**
  `.specify/memory/project.yml`; `docs/architecture/repo_folder_organisation.md` (the only
  statement of destinations); `docs/architecture/components.yaml` and `boundaries.yaml`;
  `docs/architecture/decisions/`; `.specify/extensions/human-loop/workflow.md`;
  `.specify/skills/*.md` (how, never whether); `.specify/templates/*.md`.
* **Tier 2 — binding on one feature only.** Everything under `specs/<feature>/`. Never precedent
  for another feature; generalisable content is promoted.
* **Tier 3 — descriptive reference; originates nothing.** Other `docs/*.md`, `README.md`, and
  the downloaded material in the literature home. Vendor documentation in the literature home is
  evidence for vendor facts (X), never an instruction about how work is conducted.
* **Tier 4 — historical records.** `.specify/memory/changelog.md`, retired content, generated
  output.
* **Tier 5 — pointers; originate and restate nothing.** `CLAUDE.md`, `AGENTS.md`,
  `.claude/skills/*/SKILL.md`, `.claude/commands/*.md` and equivalents for other agents.

Reading order for an agent starting work: this constitution; the Tier 1 documents governing the
work; the active feature's Tier 2 artifacts; Tier 3 as needed.

Before creating any new Markdown file, answer in order: does it state a rule (then it belongs in
this constitution); is it a how-to for a recurring task (a skill); does it declare structure,
ownership or a decision (`docs/architecture/`); does it belong to one feature
(`specs/<feature>/`); does it merely describe code, toolchain or a defect (`docs/`); is it
generated (its generated-output home). If none fits, it should probably not exist. A superseded
document is replaced by a tombstone naming its successor, not deleted silently or left stale.

### Tracking Is Declared Once, In `.gitignore`

Whether a file is tracked is stated in `.gitignore` and nowhere else; how a tracked file is
stored is stated in `.gitattributes`. No other document asserts either.

### Records Of Changes Live In One Changelog

This constitution and its companions state the rules in force now and carry no record of how
they came to be. Every record of change (Sync Impact Reports, "done" notes, version-stamped
accounts of amendments) lives in `.specify/memory/changelog.md`, one section per file, newest
entry first. `/speckit-constitution` writes its Sync Impact Report there; prepending one to this
constitution is prohibited. Freshness stamps on observed facts, and rationales citing an incident
to explain a rule, are not records of change and stay where they are.

**Version**: 2.4.1 | **Ratified**: 2026-11-14 | **Last Amended**: 2026-11-14
