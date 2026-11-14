---

description: "Task list template for feature implementation"
---

# Tasks: [FEATURE NAME]

**Input**: Design documents from `/specs/[###-feature-name]/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/, and an **approved** architecture-delta.yaml — tasks MUST NOT be generated before the architecture delta is approved (Principle XIII-C)

**Tests**: Behavioural changes MUST include the narrowest relevant tests in both tiers
(unlicensed and licensed) from the specification and plan, and any load, solve or
result path change MUST include benchmarks against its stated targets (Principles II,
XIV). Pure documentation or planning changes may state that no executable test is
required.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

**Scope**: Include temporary and inline code, benchmark harnesses and bindings under
Principle VI, not just retained source changes. Only an explicit `SPECKIT OVERRIDE`
exempts code development from this workflow.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- Single crate: `src/`, `tests/`, `benches/`, as stated in
  `docs/architecture/repo_folder_organisation.md`

<!--
  ============================================================================
  IMPORTANT: The tasks below are SAMPLE TASKS for illustration purposes only.

  The /speckit-tasks command MUST replace these with actual tasks based on:
  - User stories from spec.md (with their priorities P1, P2, P3...)
  - Feature requirements from plan.md
  - Entities from data-model.md
  - Endpoints from contracts/

  Tasks MUST be organized by user story so each story can be:
  - Implemented independently
  - Tested independently
  - Delivered as an MVP increment

  DO NOT keep these sample tasks in the generated tasks.md file.
  ============================================================================
-->

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [ ] T001 Create project structure per implementation plan
- [ ] T002 Initialize [language] project with [framework] dependencies
- [ ] T003 [P] Configure linting and formatting tools

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

Examples of foundational tasks (adjust based on your project):

- [ ] T004 Setup database schema and migrations framework
- [ ] T005 [P] Implement authentication/authorization framework
- [ ] T006 [P] Setup API routing and middleware structure
- [ ] T007 Create base models/entities that all stories depend on
- [ ] T008 Configure error handling and logging infrastructure
- [ ] T009 Setup environment configuration management
- [ ] T010 Define project-specific reproducibility command, fixture, or
  diagnostic check from the Constitution Check
- [ ] T010b [P] Verify Rust and FFI standards for this feature (Principle VII): `unsafe`
  confined with SAFETY comments; handles owned with lifetimes; no panic across FFI;
  validation before the first FFI call; vendor declarations checked against the installed
  header; workspace lints pass with warnings denied
- [ ] T010c Solver parameter location check (Principle VII-G) — REQUIRED whenever a solver
  parameter is added or touched: confirm vendor parameter-setting symbols appear only in each
  backend's single applying function, and test that a caller-supplied value reaches the
  solver unchanged and that the effective parameter set is reported
- [ ] T010d Status translation test (Principle XVI-B) — REQUIRED for any backend whose status
  translation is added or changed: map every documented code of the supported solver version
  and an unrecognised code, without a licence
- [ ] T010e Solution verification (Principle XVI-D) — REQUIRED whenever a backend returns
  solutions: every licensed test verifies the returned solution with the solver-independent
  measures, and the verification itself is tested against deliberately violated solutions
- [ ] T010f Positive control (Principle XIV-D) — REQUIRED whenever a finding could be
  "no difference": an executable check that the mechanism under test (chunked path,
  zero-copy path, warm start) is actually engaged. It MUST precede the comparison
- [ ] T010g Guard self-test (Principle XV-C) — REQUIRED for every check, gate or comparison
  harness this feature adds: exercise it against the failure it guards, including an
  "everything failed" case reported as uninterpretable. Omit only if no guard is added
- [ ] T010h Benchmark setup (Principles XIV, XV-A) — declare instances with provenance,
  record the effective parameter set, versions, commit, machine and profile, set N, time
  phases separately, measure peak memory in a dedicated process, and redirect output to the
  run bundle's log. Omit only if the feature makes no performance claim

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - [Title] (Priority: P1) 🎯 MVP

**Goal**: [Brief description of what this story delivers]

**Independent Test**: [How to verify this story works on its own]

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T011 [P] [US1] Contract or interface compatibility test for [behavior] in tests/[path]
- [ ] T012 [P] [US1] Licensed-solver test for [user journey] in tests/[name].rs (skipped visibly without a licence)

### Implementation for User Story 1

- [ ] T013 [P] [US1] Create [Entity1] model in src/[entity1].rs
- [ ] T014 [P] [US1] Create [Entity2] model in src/[entity2].rs
- [ ] T015 [US1] Implement [Service] in src/[service].rs (depends on T013, T014)
- [ ] T016 [US1] Implement [endpoint/feature] in src/[file].rs
- [ ] T017 [US1] Add validation and error handling
- [ ] T018 [US1] Add verbosity-gated diagnostics for user story 1 operations (Principle III)
- [ ] T019 [US1] Run benchmark against the wall-time and peak-memory targets in plan.md

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - [Title] (Priority: P2)

**Goal**: [Brief description of what this story delivers]

**Independent Test**: [How to verify this story works on its own]

### Tests for User Story 2

- [ ] T020 [P] [US2] Contract or interface compatibility test for [behavior] in tests/[path]
- [ ] T021 [P] [US2] Licensed-solver test for [user journey] in tests/[name].rs

### Implementation for User Story 2

- [ ] T022 [P] [US2] Create [Entity] model in src/[entity].rs
- [ ] T023 [US2] Implement [Service] in src/[service].rs
- [ ] T024 [US2] Implement [endpoint/feature] in src/[file].rs
- [ ] T025 [US2] Integrate with User Story 1 components (if needed)
- [ ] T026 [US2] Run benchmark against the wall-time and peak-memory targets in plan.md

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: User Story 3 - [Title] (Priority: P3)

**Goal**: [Brief description of what this story delivers]

**Independent Test**: [How to verify this story works on its own]

### Tests for User Story 3

- [ ] T027 [P] [US3] Contract or interface compatibility test for [behavior] in tests/[path]
- [ ] T028 [P] [US3] Licensed-solver test for [user journey] in tests/[name].rs

### Implementation for User Story 3

- [ ] T029 [P] [US3] Create [Entity] model in src/[entity].rs
- [ ] T030 [US3] Implement [Service] in src/[service].rs
- [ ] T031 [US3] Implement [endpoint/feature] in src/[file].rs
- [ ] T032 [US3] Run benchmark against the wall-time and peak-memory targets in plan.md

**Checkpoint**: All user stories should now be independently functional

---

[Add more user story phases as needed, following the same pattern]

---

## Phase N-1: Consumer Adoption And Retirement *(only if this feature introduces or materially changes a shared capability — Principle XIII-G; omit this phase entirely otherwise)*

**Purpose**: Show that the shared capability is adopted, and dispose of what it replaces.
Scope is bounded by the adoption scope approved at Gate 2A. Do NOT migrate callers outside
the approved feature scope to satisfy these tasks; a named deferral is the correct outcome.

- [ ] TXXX Migrate consumer `<name>` to the shared capability [one task per consumer named
  as migrating in the plan]
- [ ] TXXX Representative consumer test proving the required semantics survive adoption
  (not an isolated test of the capability itself)
- [ ] TXXX Behavioural evidence that a real caller reaches the intended path
  (Principle II) — or record the exact limitation and mark the behaviour unverified
- [ ] TXXX Dispose of each replaced implementation: remove it, retain it for compatibility
  with a stated reason, or defer with a named owner
- [ ] TXXX Record consumers deliberately remaining on their existing implementation, with
  the reason

---

## Phase N: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] TXXX [P] Documentation updates in docs/, including any architecture prose, source
  reference or contract this change invalidated (Principle XIII-D)
- [ ] TXXX Code cleanup and refactoring
- [ ] TXXX Performance optimization across all stories
- [ ] TXXX [P] Additional unit tests in 
- [ ] TXXX Security hardening
- [ ] TXXX Run quickstart.md validation
- [ ] TXXX Report files edited, checks run, pass/fail results, and unverified
  behavior
- [ ] TXXX Create implementation receipt in
  `agent-runs/<UTC-timestamp>-<short-task-or-run-name>/implementation-receipt.md`,
  with the actual final user-facing agent completion response copied verbatim
  into the `Final response` section

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → P3)
- **Polish (Final Phase)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - May integrate with US1 but should be independently testable
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - May integrate with US1/US2 but should be independently testable

### Within Each User Story

- Tests or reproducibility checks MUST be written or defined before implementation
- Models before services
- Services before endpoints
- Core implementation before integration
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- Once Foundational phase completes, all user stories can start in parallel (if team capacity allows)
- All tests for a user story marked [P] can run in parallel
- Models within a story marked [P] can run in parallel
- Different user stories can be worked on in parallel by different team members

---

## Parallel Example: User Story 1

```bash
# Launch all tests for User Story 1 together (if tests requested):
Task: "Contract test for [endpoint] in tests/[name].rs"
Task: "Integration test for [user journey] in tests/[name].rs"

# Launch all models for User Story 1 together:
Task: "Create [Entity1] model in src/[entity1].rs"
Task: "Create [Entity2] model in src/[entity2].rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Test User Story 1 independently
5. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (MVP!)
3. Add User Story 2 → Test independently → Deploy/Demo
4. Add User Story 3 → Test independently → Deploy/Demo
5. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1
   - Developer B: User Story 2
   - Developer C: User Story 3
3. Stories complete and integrate independently

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
