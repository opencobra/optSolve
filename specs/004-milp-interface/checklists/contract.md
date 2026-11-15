# Contract Requirements Quality Checklist: Mixed-Integer Programmes Through The Existing Interface

**Purpose**: Unit tests for the requirements of the mixed-integer extension of the contract, the
backends, the tests and the document
**Created**: 2026-11-15
**Feature**: [spec.md](../spec.md)
**Audience / depth**: reviewer at Gate 1; standard

Items left `[ ]` are explicitly routed in `../human-loop.md` (constitution VI-B).

## Requirement Completeness

- [x] CHK001 Are the new options fields enumerated (relative and absolute gap, node and solution limits, integrality tolerance, fixed-model duals)? [Completeness, Spec §FR-002, FR-003a] — resolved
- [ ] CHK002 Is the gap definition specified exactly (relative gap normalisation, sign for maximise, behaviour when the objective or bound is zero or infinite)? [Clarity, Spec §FR-003, FR-004]
- [ ] CHK003 Is the representation of per-backend mixed-integer combinations in capabilities specified, reusing the existing capability fields where possible (SC-003)? [Consistency, Spec §FR-009, SC-003]
- [ ] CHK004 Are the cone kinds MOSEK allows with integer variables confirmed against vendor documentation? [Dependency, Spec §FR-009, Clarifications]
- [ ] CHK005 Is the status mapping for mixed-integer outcomes specified per backend (optimal within gap, solution limit, node limit, infeasible or unbounded, feasible solution found at a limit)? [Completeness, Spec §FR-007]
- [x] CHK006 Are the verification measures for mixed-integer problems enumerated (row, bound, cone, integrality, gap; no duals unless fixed-model)? [Completeness, Spec §FR-004, FR-009a] — resolved

## Requirement Clarity

- [ ] CHK007 Is "fixed at the returned values" specified for integer values that are within tolerance but not exactly integral (rounded before fixing, or fixed as returned)? [Ambiguity, Spec §FR-003a]
- [ ] CHK008 Is how the result states that duals belong to the fixed problem specified (field, attribute or status)? [Clarity, Spec §FR-003a]
- [x] CHK009 Is "Optimal" defined for mixed-integer problems (within the gap tolerances)? [Clarity, Spec §FR-003] — resolved

## Requirement Consistency

- [ ] CHK010 Are binary-variable bound rules (intersection with [0, 1]) and fractional integer bounds specified identically for both backends? [Consistency, Spec §Edge Cases]
- [x] CHK011 Are tests required to reuse the existing harness, toy set and comparison (no parallel framework)? [Consistency, Spec §FR-010, FR-011] — resolved

## Acceptance Criteria Quality

- [x] CHK012 Are SC-001 to SC-007 measurable with counts or percentages? [Measurability] — resolved
- [ ] CHK013 Is the MILP benchmark instance (and its provenance) named so SC-004 is reproducible? [Measurability, Spec §SC-004, FR-012]

## Scenario And Edge Case Coverage

- [x] CHK014 Are limit-stopped searches covered (status, reason, incumbent, bound, gap)? [Coverage, Spec §US1-3] — resolved
- [x] CHK015 Are unsupported combinations covered (rejected before solving)? [Coverage, Spec §FR-009, US3-3] — resolved
- [ ] CHK016 Is the comparison rule for mixed-integer objectives specified (which gap tolerance, both solvers' bounds)? [Clarity, Spec §FR-011, SC-002]

## Dependencies And Assumptions

- [x] CHK017 Are out-of-scope items stated (MIP starts, incumbent callbacks, lazy constraints, SOS, semi-continuous)? [Assumption] — resolved
- [ ] CHK018 Is the MPS integer-marker and binary-bound convention specified so both tools and the reader agree? [Dependency, Spec §FR-010]
