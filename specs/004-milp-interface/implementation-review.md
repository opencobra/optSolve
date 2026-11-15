# Implementation Review

## Summary

Feature 004 admits integer and binary variables through the existing interface for every
mixed-integer class the solvers support (MILP, MIQP and mixed-integer conic problems on Gurobi
with its four cone kinds and on MOSEK with every kind except PSD), with optional fixed-model duals,
extending existing types, backends, tests, toys, comparison, document and benchmark only.

## Embedded Core Commands Completed
- constitution: checked 2.5.1
- specify: done (scope clarification); spec revised at Gate 1 (SC-003: reuse where possible) and
  corrected in planning (MOSEK supports MIQP)
- clarify: done (fixed-model duals optional)
- checklist: requirements 16/16; contract 18/18 (10 routed items closed in Bundle 2)
- plan: done (research R1–R12 with Gurobi and MOSEK C probes)
- architecture review: Gate 2A APPROVE_WITH_ADR (ADR-0004)
- tasks: 24 tasks
- analyze: 0 critical, 0 high; 2 low (accepted)

## Architecture Verdict (Gate 2A)
- Verdict: APPROVE_WITH_ADR
- ADR required / written: ADR-0004 (mixed-integer through the existing interface)
- Date (UTC): 2026-11-15

## Cross-Artifact Analysis Summary
Coverage: FR-001 → T005–T007; FR-002 → T005, T010, T011; FR-003 → T005, T010, T011; FR-003a →
T016–T018; FR-004 → T004–T005; FR-005 → T005; FR-006 → T020; FR-007 → T009–T011; FR-008 → T010,
T011; FR-009/009a → T005, T019; FR-010 → T006, T007, T013; FR-011 → T014; FR-012 → T021. SC-001 →
T013–T014; SC-002 → T014; SC-003 → plan reuse assessment (review); SC-004 → T021; SC-005 → T020;
SC-006 → T013, T019; SC-007 → T016. 100% coverage; tests precede code (T004 → T005, T008 → T009–
T012, T016 → T017–T018). Low findings accepted: (L1) the spec's edge-case wording on solver bound
conventions is satisfied by documentation (T020) rather than a dedicated test beyond T004's binary
case; (L2) the benchmark instance's tractability is decided in T021 (research R12 allows revising
the derivation rule).

## Proposed Implementation Scope
- Tasks proposed: T001–T024
- First independently testable slice: Setup + Foundational + US1 (T001–T012): MILP on both
  backends with limits and verification
- Files likely to change: `.specify/memory/project.yml`; `src/{options, backend, result,
  verify}.rs`, `src/options/check.rs`, `src/gurobi/{ffi, load, params, solve, callback}.rs`,
  `src/mosek/{ffi, load, params, solve, status, callback}.rs`; `tests/common/{fixtures, mps, jtask,
  toy, compare, f002, fake_backend}.rs`, `tests/{corpus, contract, standards}.rs`,
  `tests/contract/*.rs`, `tests/header_conformance.c`, `tests/header_conformance_mosek.c`;
  `tests/fixtures/small/*`, `tests/fixtures/milp/*`; `tools/corpus/make_toy_instances.m`;
  `docs/{llm-interface, test-corpus}.md`, `docs/architecture/components.yaml`, `README.md`;
  `benches/load_solve.rs`; feature 004 artifacts
- Files that should NOT change: other `.specify/` governance, `literature/`, `specs/000-*` to
  `specs/003-*`, the variational kinetics repository, existing corpus fixtures (LP, QP, CLP data)

## Tests and Validation Expected (name the narrowest relevant test first)
1. `cargo test --test contract` (MI capability rule, gap, integrality, fixed-problem duals)
2. `cargo test --test corpus -- mps jtask` (readers)
3. `cargo test --features gurobi,mosek --test corpus -- f004 toy compare`
4. `cargo test --doc --features gurobi,mosek`; `cargo test --test standards`
5. Header conformance; full in-gate checks (T022); benchmark (T021)

## Blocking Issues
None.

## Acceptable Risks
- Gurobi may solve a non-convex MIQP that MOSEK rejects (documented; optSolver does not check
  convexity).
- MOSEK reports an unbounded MILP as InfeasibleOrUnbounded; treated as compatible in comparisons.
- The derived MILP benchmark instance may be easy or hard; time-limited at 60 s.

## Human Approval
- Approved: yes
- Approved option: Approve all tasks and commit
- Approved tasks/scope: T001–T024
- Required implementation invocation per constitution: `/speckit-implement`
- Date (UTC): 2026-11-15
