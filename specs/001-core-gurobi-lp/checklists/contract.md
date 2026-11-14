# Contract Requirements Quality Checklist: Solver-Independent Core And Gurobi LP Backend

**Purpose**: Unit tests for the requirements of the solver-independent contract, the Gurobi LP
backend, the frozen test corpus and harness, and the performance criteria
**Created**: 2026-11-14
**Feature**: [spec.md](../spec.md)
**Audience / depth**: reviewer at Gate 1; standard

Items left `[ ]` are explicitly routed in `../human-loop.md` (constitution VI-B).

## Requirement Completeness

- [x] CHK001 Are all input validation rules enumerated, including non-finite values in A and c and NaN bounds? [Completeness, Spec §FR-006] — resolved: FR-006 amended 2026-11-14
- [x] CHK002 Is the meaning of an infinite bound defined for both row-bound forms (infinite right-hand side with a sense; ±infinity in a range)? [Clarity, Spec §Edge Cases] — resolved in Bundle 2: data-model.md, Problem
- [x] CHK003 Is the role of the objective constant in the reported objective value and in the duality gap specified? [Clarity, Spec §FR-005] — resolved in Bundle 2: data-model.md, Problem
- [x] CHK004 Are the names and meanings of the solver-independent statuses enumerated? [Completeness, Spec §FR-010] — resolved in Bundle 2: data-model.md, Status vocabulary
- [x] CHK005 Are the limit reasons enumerated? [Completeness, Spec §FR-010] — resolved in Bundle 2: data-model.md, Status vocabulary
- [x] CHK006 Are the shared option fields enumerated with their solver-independent definitions? [Completeness, Spec §FR-008] — resolved in Bundle 2: data-model.md, Options; research.md R5
- [x] CHK007 Is the rule for a setting supplied both as a shared field and as a raw parameter specified? [Clarity, Spec §FR-008, Edge Cases] — resolved: an error
- [x] CHK008 Is "every output the solver makes available" enumerated for the LP backend? [Clarity, Spec §FR-009] — resolved: Key Entities, Result
- [x] CHK009 Is the library's behaviour without a Gurobi installation or licence specified, not only the tests'? [Coverage, Spec §Edge Cases] — resolved: edge case amended 2026-11-14

## Requirement Clarity

- [x] CHK010 Is "the relevant data" whose infinity norm scales each relative verification measure defined per measure? [Ambiguity, Spec §FR-012, US2-3] — resolved in Bundle 2: data-model.md, Verification
- [x] CHK011 Is it specified which effective Gurobi tolerance judges which verification measure? [Clarity, Spec §FR-012] — resolved in Bundle 2: research.md R5
- [x] CHK012 Is the verification report's verdict structure defined (per measure, and whether an overall verdict exists)? [Clarity, Spec §Key Entities] — resolved in Bundle 2: data-model.md, Verification
- [x] CHK013 Is the behaviour of dual-side verification defined when the caller switches dual outputs off? [Coverage, Spec §FR-009, FR-012] — resolved in Bundle 2: data-model.md, Verification
- [x] CHK014 Is the extent of reusing variational kinetics extraction code bounded ("where it can be reused")? [Ambiguity, Spec §FR-001] — resolved in Bundle 2: research.md R11; plan.md reuse assessment
- [x] CHK015 Are the random objectives' distribution and generator specified precisely enough for bit-for-bit reproducibility across platforms? [Clarity, Spec §FR-002] — resolved in Bundle 2: research.md R13
- [x] CHK016 Is "available backend" defined for the harness (compiled in, versus licensed at run time)? [Clarity, Spec §US1-4] — resolved in Bundle 2: research.md R14
- [x] CHK017 Is the number of seeded objectives per instance fixed? [Clarity, Spec §Assumptions, SC-003] — resolved: three

## Requirement Consistency

- [x] CHK018 Is verification on by default (FR-012) consistent with the 5% time budget (SC-005)? [Consistency] — resolved: SC-005 counts verification inside the 5%
- [x] CHK019 Are SC-004's memory target and FR-007's fixed-size conversion buffer consistent? [Consistency] — resolved after clarification 1
- [x] CHK020 Is solver logging off (FR-018) consistent with the progress callback (FR-019)? [Consistency] — resolved: the callback receives data; nothing is printed
- [x] CHK021 Is the one-environment-per-thread assumption consistent with environment reuse (FR-015)? [Consistency, Spec §Assumptions] — resolved

## Acceptance Criteria Quality

- [x] CHK022 Is "beyond Gurobi's own" memory in SC-004 measurable, with the baseline defined? [Measurability, Spec §SC-004] — resolved in Bundle 2: research.md R8
- [x] CHK023 Is "Gurobi solve time" in SC-005 defined (what is timed, which settings, threads)? [Clarity, Spec §SC-005] — resolved in Bundle 2: research.md R9
- [x] CHK024 Are the deterministic solver settings behind SC-003 specified? [Clarity, Spec §SC-003] — resolved in Bundle 2: research.md R10
- [x] CHK025 Do the cancellation flag and progress callback have acceptance scenarios? [Coverage, Spec §FR-019] — resolved: US3 scenario 7 added 2026-11-14
- [x] CHK026 Is every success criterion stated per instance with its N? [Measurability, constitution XIV-A, XV-A] — resolved

## Scenario And Edge Case Coverage

- [x] CHK027 Are requirements for ranged rows through Gurobi, whose native form is sense plus right-hand side, specified? [Gap] — resolved in Bundle 2: research.md R3
- [x] CHK028 Are the MPS conventions of the fixtures specified (format variant, objective sense, bounds and ranges sections)? [Gap, Spec §FR-001] — resolved in Bundle 2: research.md R11
- [x] CHK029 Are empty problems, empty columns and infinite bounds listed as edge cases? [Coverage, Spec §Edge Cases] — resolved
- [x] CHK030 Is the behaviour for MILP or QP fields given to the LP-only backend defined? [Coverage, Spec §Edge Cases, FR-013] — resolved: rejected through capabilities

## Dependencies And Assumptions

- [x] CHK031 Are out-of-scope items (MILP, QP, MOSEK, persistent models, warm starts) declared? [Scope, Spec §Assumptions] — resolved
- [x] CHK032 Is the licence decision on the iDopaNeuro data recorded with its consequence for provenance? [Assumption, constitution X] — resolved: Assumptions; provenance states "licence not verified"
- [x] CHK033 Is the dependency on the variational kinetics repository and MATLAB limited to extraction time? [Dependency, Spec §Assumptions] — resolved
