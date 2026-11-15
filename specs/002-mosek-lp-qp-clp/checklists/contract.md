# Contract Requirements Quality Checklist: MOSEK Backend For LP, QP And CLP

**Purpose**: Unit tests for the requirements of the QP and conic extensions of the contract, the
MOSEK backend, the corpus and the toolchain items
**Created**: 2026-11-15
**Feature**: [spec.md](../spec.md)
**Audience / depth**: reviewer at Gate 1; standard

Items left `[ ]` are explicitly routed in `../human-loop.md` (constitution VI-B).

## Requirement Completeness

- [x] CHK001 Is the QP objective convention (½ xᵀQx + cᵀx + γ; Q given by its lower triangle) stated consistently for input, objective value and dual objective? [Clarity, Spec §FR-006, Key Entities] — resolved in Bundle 2: data-model.md, Problem
- [x] CHK002 Are the conic dual conventions (sign of s, the dual cone of each cone type, dual-exponential and dual-power definitions) specified? [Gap, Spec §FR-008, FR-009] — resolved in Bundle 2: data-model.md, Cones and Verification; research.md R5
- [x] CHK003 Is the vectorised positive semidefinite cone layout (ordering of the lower triangle, √2 scaling) specified precisely enough to be verified? [Clarity, Spec §FR-007] — resolved in Bundle 2: data-model.md, PSD layout; research.md R5
- [x] CHK004 Are the power-cone exponent conventions (how many leading coordinates, normalisation of the exponents) specified? [Gap, Spec §FR-007] — resolved in Bundle 2: data-model.md, Cones; research.md R5
- [x] CHK005 Are the verification measures and their relative scalings defined for each cone type, including positive semidefinite membership? [Gap, Spec §FR-009] — resolved in Bundle 2: data-model.md, Verification
- [x] CHK006 Are the toolchain requirements (minimum Rust build of library, tests and benchmarks; in-gate) complete? [Completeness, Spec §FR-003] — resolved
- [x] CHK007 Is the content of the corpus document enumerated? [Completeness, Spec §FR-002] — resolved

## Requirement Clarity

- [x] CHK008 Is "first inner conic problem" of a variational kinetics instance defined operationally (which call, which parameters, which variational kinetics commit)? [Ambiguity, Spec §FR-014] — resolved in Bundle 2: research.md R11 (operational definition; maintainer confirmation at Gate 2A)
- [x] CHK009 Is how the expected status of each CLP problem is obtained (variational kinetics' single direct MOSEK route) specified, including what happens when that route fails? [Clarity, Spec §FR-014] — resolved in Bundle 2: research.md R11
- [x] CHK010 Is the closed-form infeasible problem for the certificate-sign test, and the expected sign per row, specified? [Clarity, Spec §FR-004] — resolved in Bundle 2: data-model.md, Contract behaviour tests
- [x] CHK011 Is "never concurrently" in the callback test defined as an observable (for example an in-flight counter never exceeding one)? [Measurability, Spec §FR-005] — resolved in Bundle 2: data-model.md, Contract behaviour tests
- [x] CHK012 Is the MOSEK status translation's source of truth (response codes, problem status, solution status, which solution type is read) specified? [Gap, Spec §FR-011] — resolved in Bundle 2: research.md R6

## Requirement Consistency

- [x] CHK013 Is the rename (FR-001) consistent with the constitution's rule that records of change are not rewritten (historical records keep the old name)? [Consistency, Spec §US1-1] — resolved: "no reference outside historical records"
- [x] CHK014 Are SC-004 targets for MOSEK the same as feature 001's for Gurobi? [Consistency, Spec §SC-004] — resolved
- [x] CHK015 Is the cross-solver agreement tolerance (SC-001) consistent with each solver's tolerance definitions (Gurobi simplex versus MOSEK interior point)? [Consistency, Spec §SC-001] — resolved in Bundle 2: research.md R10

## Acceptance Criteria Quality

- [x] CHK016 Is every success criterion stated per named problem with N? [Measurability, Spec §SC-001..SC-006] — resolved
- [x] CHK017 Is the driver-built QP (quadratic weight, which variables are penalised) specified so SC-003 is reproducible? [Clarity, Spec §SC-003, Assumptions] — resolved in Bundle 2: research.md R12

## Scenario And Edge Case Coverage

- [x] CHK018 Are unsupported-class rejections (Gurobi with QP or CLP) covered? [Coverage, Spec §US5-3, FR-013] — resolved
- [x] CHK019 Are empty conic and empty linear parts covered? [Edge Case, Spec §Edge Cases] — resolved
- [x] CHK020 Is the behaviour for a non-convex QP (Q not positive semidefinite) specified beyond "the solver's status is reported" — is it rejected, or passed to the solver? [Ambiguity, Spec §Edge Cases] — resolved in Bundle 2: data-model.md, Problem; research.md R4

## Dependencies And Assumptions

- [x] CHK021 Is the MOSEK binding route (official crate versus own declarations) to be decided against constitution IV recorded as a research item? [Assumption, Spec §Assumptions] — resolved in Bundle 2: research.md R1
- [x] CHK022 Is the MOSEK version and licence location an explicit assumption? [Assumption, Spec §Assumptions] — resolved
- [x] CHK023 Is the heavy-tier scope (which tests, when run) specified? [Completeness, Spec §FR-017] — resolved
