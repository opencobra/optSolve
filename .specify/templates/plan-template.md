# Implementation Plan: [FEATURE]

**Branch**: `[###-feature-name]` | **Date**: [DATE] | **Spec**: [link]

**Input**: Feature specification from `/specs/[###-feature-name]/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

[Extract from feature spec: primary requirement + technical approach from research]

## Technical Context

<!--
  ACTION REQUIRED: Replace the content in this section with the technical details
  for the project. The structure here is presented in advisory capacity to guide
  the iteration process.
-->

**Language/Version**: [e.g., Python 3.11, Swift 5.9, Rust 1.75 or NEEDS CLARIFICATION]

**Primary Dependencies**: [e.g., FastAPI, UIKit, LLVM or NEEDS CLARIFICATION]

**Storage**: [if applicable, e.g., PostgreSQL, CoreData, files or N/A]

**Testing**: [e.g., pytest, XCTest, cargo test or NEEDS CLARIFICATION]

**Target Platform**: [e.g., Linux server, iOS 15+, WASM or NEEDS CLARIFICATION]

**Project Type**: [e.g., library/cli/web-service/mobile-app/compiler/desktop-app or NEEDS CLARIFICATION]

**Performance Goals**: [domain-specific, e.g., 1000 req/s, 10k lines/sec, 60 fps or NEEDS CLARIFICATION]

**Constraints**: [domain-specific, e.g., <200ms p95, <100MB memory, offline-capable or NEEDS CLARIFICATION]

**Scale/Scope**: [domain-specific, e.g., 10k users, 1M LOC, 50 screens or NEEDS CLARIFICATION]

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design. Answer each bullet;
write N/A only where stated.*

- **Development scope**: [Principle VI — include temporary scripts, inline code, benchmark
  harnesses and bindings in the approved scope. Identify any explicitly scoped
  SPECKIT OVERRIDE; do not infer an exemption from file location, persistence or task size.]
- **Performance and memory**: [Principle I — state wall-time and peak-memory targets for each
  load, solve and result path touched; list every allocation proportional to nonzeros and
  justify it; confirm environment reuse and model modification without reload are preserved;
  confirm optional outputs cost nothing unless requested; confirm validation is not removed
  for speed]
- **Solver configuration audit**: [Principle I — for each solver invoked, enumerate the
  relevant parameters and defaults, cross-check each against the problem data profile
  (dimensions, sparsity, integrality, scale), and list any override with its rationale and
  source (vendor documentation, representative instance). N/A if no solver is invoked]
- **Solver parameters in one location**: [Principle VII-G — confirm every parameter this
  feature adds or touches is declared and defaulted only in the core options module, applied
  only by each backend's single parameter-applying function, reachable by raw name/value
  passthrough, never overriding a caller value, and reportable as part of the effective
  parameter set. Name the module and function. N/A if no solver parameter is involved]
- **Zero-copy inputs and bulk calls**: [Principle IV — for each backend, the native-interface
  route (solver's Rust API, optSolver's own C binding, or an existing binding crate) and why;
  for an existing crate, evidence that it exposes the bulk calls on borrowed slices. List every
  input array and whether it is borrowed as is or converted (and why); confirm only bulk load
  and slice calls are used; describe the fused validation-conversion pass and its bound]
- **Solver-independent contract**: [Principle XVI — (a) confirm every backend takes the generic
  problem type and options structure and returns the generic result type, with solver-specific
  information only in the separated fields (XVI-A); state any representation decision made by
  this feature. (b) Name each backend's status translation function and confirm it is total over
  the supported version's documented codes, with an explicit unknown status (XVI-B). (c) Confirm
  new behaviour is requested only through options-structure fields, shared fields documented per
  backend, and the rule for fields a backend cannot honour (XVI-C). (d) Name the verification
  measures applied to every solution and the tests that use them (XVI-D). N/A only if the feature
  touches no problem, options, result or status type]
- **Rust and FFI standards**: [Principle VII-A to VII-F, for code that crosses into C (for an
  existing binding crate, how it meets them or the accepted gap) — where `unsafe` lives and its SAFETY
  arguments; handle ownership, lifetimes and any Send/Sync with vendor citation; error versus
  status mapping and panic containment in callbacks; validation before the first FFI call;
  each vendor declaration and constant used, checked against the installed header (cite the
  header and date); crate hygiene (lints, `links`, build-script environment variables)]
- **Independence and additive features**: [Principle VIII — confirm no RunMat or sparse-crate
  dependency; feature additivity; the core still builds and tests with no vendor software]
- **Testing and behavioural evidence**: [Principle II — name the narrowest tests in each tier
  (unlicensed, licensed), the failing-input tests for each validated invariant, cross-solver
  agreement tests, and how a real caller is shown to reach the intended path]
- **Diagnostics and statuses**: [Principle III — confirm no direct printing, solver logs off by
  default, raw and normalised status both returned, vendor error code and message preserved]
- **Benchmarks and evidentiary preconditions**: [Principle XIV — (a) INSTANCES: name each
  benchmark instance with dimensions, nonzeros, class and provenance; criteria per instance.
  (b) CONFIGURATION: confirm the effective parameter set, solver versions, commit, machine and
  profile are recorded; for comparisons, confirm identical instances and equivalent parameters.
  (c) PHASES: environment creation, load, solve and extraction timed separately; cold versus
  warm distinguished; peak memory measured in a dedicated process. (d) POSITIVE CONTROL: if any
  finding could be "no difference", name the task showing the mechanism is engaged. N/A only if
  the feature makes no performance claim]
- **Reporting discipline**: [Principle XV — (a) the repetition count N and spread for every
  reported figure, and where it is set; claims scoped in the sentence. (b) for every guard or
  comparison harness added, the task exercising it against the failure it guards, including an
  "everything failed" case. (c) where run output is redirected and which log is complete.
  (d) any repository-wide rule added or widened, and the earlier decision searched for]
- **Notation**: [Principle XII-C — every mathematical object this feature names uses its symbol
  from the constitution's Notation Convention, in code, documents and discussion; new objects'
  symbols are added to the inventory through `/speckit-constitution`. N/A if none]
- **Provenance and licensing**: [Principle X — every external source, crate and test model with
  origin and licence; no vendor files or vendor-derived bindings committed]
- **Project parameters**: [Governance — every project-specific value this feature reads or
  writes is taken from `.specify/memory/project.yml`; no absolute repository path; machine
  locations recorded in run provenance only. N/A only if none]
- **Spec-driven scope control**: [Principle V — paths to edit, retired paths to avoid, any
  justified new dependency or abstraction]
- **Artifact placement**: [Principle IX — confirm every file created, moved or emitted goes
  where `docs/architecture/repo_folder_organisation.md` states for its role (do not restate
  destinations here); run output bundled per run with its logs; no path counted or stored
  (IX-I); large binaries kept out of Git history (IX-G). List any file whose placement changes]
- **Architecture conformance**: [Principle XIII — confirm `architecture-delta.yaml` exists and
  is approved before tasks; boundaries and dependency directions in `docs/architecture/`
  preserved; no cycle; solver-specific code behind its backend; public interfaces preserved or
  versioned deliberately; any ADR required named; no ownership asserted in agent files; any
  naming decision for a new layer recorded here and in the delta (XIII-F)]

## Architecture Impact

*Derived from `architecture-delta.yaml` — do not maintain a second, divergent
description here. Summarise; the YAML is authoritative.*

**Current architecture**: [Existing components, ownership boundaries, and
dependency directions relevant to this feature]

**Proposed delta**: [Added / modified / removed components; new and removed
dependency edges with a reason for each; public interfaces changed; persistent
data structures changed; runtime or deployment topology changed]

**Alternatives considered**: [Including the smallest conforming alternative, and
why it was not chosen]

**Reuse and adoption assessment** (Principles V, XIII-E, XIII-G — keep this
proportionate to the change; a small change needs a few lines, and an exhaustive
repository census is never required):

- *Owning component*: [the existing component that owns this responsibility]
- *Inspected*: [functions and contracts actually read, with `file:line` references.
  "No reusable implementation exists" must name the scope inspected and the evidence]
- *Disposition*: [reuse | extend | extract | introduce new — and why this is preferable
  to the smallest conforming alternative]
- *Duplication left in place*: [any duplicated algorithm, policy, default or scientific
  decision that remains, and why — including a deliberate independent verifier, which is
  not a defect]
- *If a shared capability is introduced or materially changed* (XIII-G):
  - *Named consumers and bounded adoption scope*: [...]
  - *Migrating now* / *deliberately remaining*: [...]
  - *Representative consumer test*: [the test showing required semantics survive adoption —
    isolated tests of the new capability do not discharge this]
  - *Disposition of replaced implementations*: [removed | retained for compatibility |
    deferred with named owner and reason]
- *Reachability and direction* (XIII-E): [which callers must reach it; Rust visibility
  (`pub`, `pub(crate)`) and crate-dependency constraints; whether extraction changes a
  dependency direction or creates a cycle; distinctions the shared interface must preserve]
- *Cross-boundary contract* (XIII-H): [if this component consumes another's effective
  configuration, results or metadata — the declared interface or data contract used, or the
  justification for depending on another component's source syntax or layout]
- *Behavioural integration evidence* (Principle II): [for a change to strategy selection,
  dispatch, a configuration control or a shared service — how a real caller is shown to
  reach the intended behaviour; or the exact limitation if execution is unavailable]

**Verdict**: [APPROVE | APPROVE_WITH_ADR | REVISE_ARCHITECTURE | REJECT — record
the verdict and its date; tasks may be generated only under the first two]

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
├── architecture-delta.yaml  # Principle XIII-C — MUST be approved before tasks exist
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)
<!--
  ACTION REQUIRED: Replace with the concrete crates and paths this feature touches.
  Destinations follow docs/architecture/repo_folder_organisation.md (Principle IX).
-->

```text
Cargo.toml
build.rs
src/
tests/
benches/
```

**Structure Decision**: [Document the selected structure and reference the real
directories captured above]

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |
