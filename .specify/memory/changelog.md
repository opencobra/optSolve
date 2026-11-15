# Changelog

Tier 4: records of change only; originates no rule and is not cited as authority
(constitution, "Records Of Changes Live In One Changelog"). One section per file, newest
entry first, sections ordered alphabetically by path.

## .specify/memory/constitution.md

### 2.5.0 — 2026-11-15

Sync Impact Report.

- Version change: 2.4.1 → 2.5.0 (MINOR: Notation Convention inventory expanded).
- Trigger: feature 002 Gate 2A (maintainer approval of the symbols in
  specs/002-mosek-lp-qp-clp/data-model.md).
- Modified: Notation Convention — added F, g, K, s, k.
- Companions: templates unchanged.

### 2.4.1 — 2026-11-14

Sync Impact Report.

- Version change: 2.4.0 → 2.4.1 (PATCH: clarification of an existing rule's evidence standard).
- Trigger: feature 001 task T038 found that the Gurobi 13.0 manual describes the
  `GRBXloadmodel` / `GRBXaddvars` arrays only as initialisation data; the maintainer accepted
  that wording plus a recorded probe as the basis (decision recorded in
  specs/001-core-gurobi-lp/human-loop.md).
- Modified: VII-A — non-`const` pointers from shared slices also allowed when the vendor
  documents the array only as input data and a recorded probe on the supported version shows it
  unchanged; the probe is kept as a licensed test.
- Companions: templates unchanged.

### 2.4.0 — 2026-11-14

Sync Impact Report.

- Version change: 2.3.0 → 2.4.0 (MINOR: Notation Convention inventory expanded).
- Trigger: feature 001 Gate 2A (maintainer approval of the symbols proposed in
  specs/001-core-gurobi-lp/data-model.md).
- Modified: Notation Convention — added b_l, b_u, y, z, γ, σ.
- Companions: ✅ docs/architecture/repo_folder_organisation.md (Maintenance tooling role, same
  gate); templates unchanged.

### 2.3.0 — 2026-11-14

Sync Impact Report.

- Version change: 2.2.0 → 2.3.0 (MINOR: materially expanded guidance in II).
- Modified: II Testing, Benchmarks, And Behavioural Evidence — added "Tests are written once and
  reused" (one solver-independent test run against every backend, one shared test module) and
  "A frozen test corpus" (fixtures with provenance, seeded synthesised data, corpus declared in
  project.yml). Adopted from the variational kinetics practice at the maintainer's direction.
- Companions: ✅ project.yml (test_corpus: iDopaNeuroC instances, MPS fixtures, seeded random
  objectives); ✅ plan-template.md (testing bullet); ✅ tasks-template.md (T010a shared harness).
- Deferred: licence of the iDopaNeuroC / Recon3D-derived data to be verified before fixtures are
  committed (X); fixture extraction is code development under VI (feature 001 or a dedicated
  test-corpus feature).

### 2.2.0 — 2026-11-14

Sync Impact Report.

- Version change: 2.1.0 → 2.2.0 (MINOR: new sub-principle and section, expanded guidance).
- Trigger: the maintainer's design decisions recorded in
  specs/000-solver-interface-exploration/open-questions.md.
- Modified: I (all available outputs returned by default, extraction cost measured, optional
  switch-off; behaviour-changing outputs only on request); VII-A (unsafe confined to FFI
  modules; forbid at crate or module level); VII-F (`non_snake_case` allowed only on public
  problem and result types); VII-G, XVI-B, XVI-D ("core crate" → "solver-neutral core");
  XII renamed Names, Notation And History; Technical Constraints (single crate; licence
  MIT OR Apache-2.0 decided).
- Added: XII-C One Symbol Per Object, In Householder Notation; Notation Convention section.
- Companions: ✅ project.yml (structure for a single crate); ✅
  docs/architecture/repo_folder_organisation.md; ✅ plan-template.md (notation bullet, single
  crate tree); ✅ tasks-template.md (single crate paths).

### 2.1.0 — 2026-11-14

Sync Impact Report.

- Version change: 2.0.0 → 2.1.0 (MINOR: new principle, materially expanded guidance).
- Added: XVI A Solver-Independent Contract — XVI-A Problems And Results Have Generic Formats,
  XVI-B A Solver-Independent Status Says How The Solve Went, XVI-C Non-Default Solver Behaviour
  Is Requested Through A Structure, XVI-D Every Solution Is Verified Independently Of The Solver.
- Modified: IV (solvers reached through their native interface, Rust API or C API through a
  binding crate; existing wrapper crates allowed only if they expose bulk calls on borrowed
  slices); VII introduction, VII-E and VII-F (FFI rules apply to code crossing into C, in full
  to optSolver-owned bindings; existing binding crates assessed in the plan); VII-G (options
  type is the XVI-C structure); III (status refers to XVI-B); Technical Constraints; review
  checklist.
- Templates: ✅ plan-template.md (contract bullet, native-interface route); ✅
  tasks-template.md (status translation and verification tasks); ✅ spec-template.md (FR-009);
  ✅ checklist and exploration templates unchanged; ✅ human-loop workflow unchanged.
- Deferred: generic type representations, status vocabulary details, verification tolerances
  and run-time default — recorded as open questions in specs/000-solver-interface-exploration.

### 2.0.0 — 2026-11-14

Sync Impact Report.

- Version change: 1.0.0 → 2.0.0 (MAJOR: principles renumbered and redefined; governance
  replaced).
- Principles now: I Performance And Memory First; II Testing, Benchmarks, And Behavioural
  Evidence; III Bounded Diagnostics And Preserved Statuses; IV Zero-Copy Borrowed Inputs And
  Bulk Native Calls; V Spec-Driven Scope Control And Reuse; VI Strict Spec-Driven Development
  Gate (VI-A human-loop gates, VI-B checklist resolution); VII Rust And FFI Coding Standards
  (VII-A to VII-G, with VII-G Solver Parameters Have One Location); VIII Independence And
  Additive Backends; IX Repository File Organisation And Artifact Placement (IX-A to IX-J);
  X Provenance, Licensing And Verified Facts; XI Problem-Space Exploration Before
  Specification; XII Descriptive Names And History At The End; XIII Architecture Conformance
  And Component Ownership (XIII-A to XIII-H); XIV Evidentiary Preconditions For Performance
  Claims; XV Agent Working Discipline And Result Reporting (XV-A to XV-E).
- Mapping from 1.0.0: I → I; II → IV; III and IV → VIII; V → VII-A to VII-D; VI → II and
  XIV; VII → X.
- Adopted from the variational kinetics repository constitution 2.4.0 (2026-11-08), adapted:
  V, VI, VI-A, VI-B, IX, XI, XIII, XV; XII from its XVIII and XIX; XIV from its XIV; VII-G from
  its VII-G; III from its III and VII-J; the configuration audit in I from its IV; governance
  sections on commits, branches, agent instruction files, project knowledge, project
  parameters, the Markdown control surface, tracking and the changelog.
- Not adopted (domain-specific): mathematical code quality, notation, MATLAB standards other
  than VII-G and VII-J, MATLAB and Julia co-evolution, fitting to prior data, formalised
  results, manuscript exceptions, the derived code index section.
- Rules lifted from guide drafts written earlier on 2026-11-14 (not kept in the repository):
  native-handle ownership and opaque types (VII-B); panics at FFI boundaries and
  `#[non_exhaustive]` (VII-C); fused validation and the unsafe unchecked constructor (VII-D);
  macro entry points and generated-binding licensing (VII-E); edition, lints, `links`, build
  scripts, no global allocator, no reliance on own profile (VII-F); raw parameter passthrough
  (VII-G); separate phase timing and per-process peak memory (XIV-C); two test tiers (II).
- Templates: plan, spec, tasks, checklist and exploration templates replaced by adapted
  variational kinetics versions; human-loop and implement-continuation extensions added;
  agent-context hooks disabled; `.claude/skills` for constitution, plan, git and the new
  extensions are pointers to neutral definitions.
- Agent files: `CLAUDE.md` and `AGENTS.md` reduced to identical delegators.
- Integration branch: `devel`. Deferred: optSolver licence
  confirmed in feature 001.

### 1.0.0 — 2026-11-14

- Initial ratification from the template: seven principles (performance and memory first;
  zero-copy borrowed inputs and bulk native calls; independence; additive solver backends;
  contained unsafe code; tests and benchmarks as acceptance criteria; provenance, licensing and
  verified facts), Technical Constraints, Development Workflow, Governance.
- Plan template Constitution Check filled; tasks template made tests and benchmarks required.
