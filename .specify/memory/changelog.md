# Changelog

Tier 4: records of change only; originates no rule and is not cited as authority
(constitution, "Records Of Changes Live In One Changelog"). One section per file, newest
entry first, sections ordered alphabetically by path.

## .specify/memory/constitution.md

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
