# Contract Requirements Quality Checklist: LLM Interface Documentation And Self-Contained Cross-Solver Test Instances

**Purpose**: Unit tests for the requirements of the interface document, the toy instance set,
the Gurobi QP and second-order-cone extension, and the cross-solver comparison
**Created**: 2026-11-15
**Feature**: [spec.md](../spec.md)
**Audience / depth**: reviewer at Gate 1; standard

Items left `[ ]` are explicitly routed in `../human-loop.md` (constitution VI-B).

## Requirement Completeness

- [x] CHK001 Are the topics the interface document must cover enumerated (dependency, notation, CSC rules, zero copy, backends, options, statuses, solution sign conventions, certificates, verification, buffers, errors, threading)? [Completeness, Spec §FR-001] — resolved
- [x] CHK002 Are the example programs the document must contain enumerated per problem class and per error path? [Completeness, Spec §FR-002] — resolved
- [x] CHK003 Is the toy instance set enumerated per problem class, outcome and cone type? [Completeness, Spec §FR-007] — resolved
- [x] CHK004 Is the exact Gurobi formulation of each supported cone (quadratic and rotated quadratic as second-order-cone constraints on auxiliary variables or quadratic constraints), and how its duals map onto s, specified? [Gap, Spec §FR-014] — resolved in Bundle 2: research R2, R3; data-model.md
- [x] CHK005 Are the certificates Gurobi can return for QP and second-order-cone problems (Farkas and unbounded ray availability) specified, including what is reported when Gurobi provides none? [Gap, Spec §FR-014] — resolved in Bundle 2: research R4
- [x] CHK006 Is the provenance record of a toy instance specified field by field (construction, status, objective, x where unique, derivation, independent-tool check)? [Completeness, Spec §FR-006, FR-009] — resolved in Bundle 2: research R9; data-model.md
- [x] CHK007 Are the comparison report's fields enumerated? [Completeness, Spec §FR-013] — resolved

## Requirement Clarity

- [x] CHK008 Is "mentioned in the document" (the drift test of FR-004) defined operationally — which public items, matched how? [Ambiguity, Spec §FR-004] — resolved in Bundle 2: research R10
- [x] CHK009 Is the single-source rule for Markdown and crate documentation stated unambiguously (one source, not two copies)? [Clarity, Spec §FR-005] — resolved
- [x] CHK010 Is the QP file convention specified (which MPS quadratic section, ½ factor, lower or full triangle) so that both solver tools and the test reader read the same Q? [Ambiguity, Spec §FR-006] — resolved in Bundle 2: research R6
- [x] CHK011 Is the agreement tolerance referenced to a defined rule? [Clarity, Spec §FR-011, Assumptions] — resolved (feature 002 R10 with R19 normalisation)
- [x] CHK012 Is "not comparable" distinguished from "skipped" and from "passed"? [Clarity, Spec §FR-012, US3-2, US3-3] — resolved

## Requirement Consistency

- [x] CHK013 Is the test-only status of the readers (FR-010a) consistent with the document showing caller-array construction only (FR-002)? [Consistency] — resolved
- [x] CHK014 Are the supported Gurobi cone types consistent between FR-014, FR-015, SC-006 and Key Entities? [Consistency] — resolved
- [x] CHK015 Is the comparison of corpus instances (FR-011 "toy and corpus") consistent with the heavy tier (which corpus instances, which tier)? [Consistency, Spec §FR-011] — resolved in Bundle 2: research R11

## Acceptance Criteria Quality

- [x] CHK016 Are SC-001 to SC-006 each measurable with a stated count or percentage? [Measurability] — resolved
- [x] CHK017 Is the LLM trial (SC-002) specified as a reproducible protocol (agents, inputs, attempts, record)? [Measurability, Spec §SC-002, Clarifications] — resolved

## Scenario And Edge Case Coverage

- [x] CHK018 Are non-unique optima, infeasible/unbounded comparisons and indefinite statuses addressed? [Edge Case, Spec §Edge Cases] — resolved
- [x] CHK019 Is the behaviour defined for a toy instance that an independent solver tool reads differently from the test reader (who is right, what is recorded)? [Edge Case, Spec §FR-009] — resolved in Bundle 2: research R9
- [x] CHK020 Is the positive semidefinite toy instance's form (file or in-code) decided, with the evidence about MOSEK's JTASK encoding? [Assumption, Spec §Assumptions] — resolved in Bundle 2: research R7

## Dependencies And Assumptions

- [x] CHK021 Are the solver versions and licences assumed stated? [Assumption, Spec §Assumptions] — resolved
- [x] CHK022 Are the Gurobi features the extension relies on (QP, second-order-cone constraints, their dual and certificate attributes, minimum version) confirmed against the vendor documentation? [Dependency, Spec §FR-014] — resolved in Bundle 2: research R1–R5
- [x] CHK023 Is the independence of toy tests from the variational kinetics repository stated? [Assumption, Spec §FR-010] — resolved
