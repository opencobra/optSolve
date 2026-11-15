# Implementation Review

## Summary

Feature 002 adds a MOSEK backend (own header-checked declarations) for LP, QP and conic problems
behind the existing contract; extends the core for QP and a broad cone set with independent
verification; renames the LP fixture to iDopaNeuroC_problem_LP and documents the corpus; adds
variational kinetics CLP fixtures (CBF; iDopaNeuroC in a heavy tier) and a derived QP fixture;
verifies certificate signs and callback threading on every backend; and makes every target build
on Rust 1.85. No source code has been modified by this workflow.

## Embedded Core Commands Completed

- constitution (2.4.1 → 2.5.0 at Gate 2A) / specify / clarify (2) / checklist (23/23) / plan /
  architecture review (APPROVE_WITH_ADR, ADR-0002) / tasks (40) / analyze (0 critical; one low
  finding fixed: conic `solve_into` test added to T033)

## Architecture Verdict (Gate 2A)

- Verdict: APPROVE_WITH_ADR
- ADR required / written: yes / docs/architecture/decisions/ADR-0002-mosek-backend-and-backend-trait.md
- Date (UTC): 2026-11-15

## Cross-Artifact Analysis Summary

Every FR and SC maps to at least one task (FR-001 T011; FR-002 T012–T013; FR-003 T010, T037;
FR-004 T014; FR-005 T015; FR-006 T003, T006, T030; FR-007 T003, T005–T006; FR-008 T006, T035;
FR-009 T004, T006; FR-010 T022–T027, T030, T035; FR-011 T017, T022–T024; FR-012 T014–T015,
T025, T027; FR-013 T006, T029, T033; FR-014 T029, T031–T032; FR-015 T019, T007; FR-016 T036;
FR-017 T007, T033). Known risk carried into T001: Cargo allows one `links` value per package.

## Proposed Implementation Scope

- Tasks proposed: T001–T040
- First independently testable slice: Setup + Foundational + US1 + US2 (T001–T016), no MOSEK
  needed
- Files likely to change: `Cargo.toml`, `build.rs`, `.specify/memory/project.yml`, `src/**`,
  `tests/**`, `benches/load_solve.rs`, `tools/corpus/*.m`, `docs/test-corpus.md`,
  `docs/architecture/components.yaml`, `README.md`, feature 002 spec artifacts
- Files that should NOT change: `.specify/` governance other than `project.yml`, `literature/`,
  `specs/000-*`, `specs/001-*` (historical), the variational kinetics repository

## Tests and Validation Expected (narrowest first)

1. `cargo test --test contract` (conic and QP validation and verification, harness self-tests)
2. `cargo test --features gurobi` (certificate sign and callback thread on Gurobi)
3. `cargo test --features gurobi,mosek` (all per-backend tests on MOSEK, LP agreement, QP, iCoreED CLP)
4. `OPTSOLVER_HEAVY_TESTS=1 cargo test --features gurobi,mosek` (iDopaNeuroC CLP)
5. `cargo +1.85.0 test --all-targets --features gurobi,mosek`; benchmark

## Blocking Issues

None.

## Acceptable Risks

- MOSEK behaviours not documented (bound values on infinite sides, certificate solution type,
  status after user-callback termination, callback thread) are settled by probes (T021) and the
  contract tests (T015) before dependent code.
- CBF round-trip precision of MOSEK's writer is unverified; T031 stops and reports if it fails.
- The `Backend` trait stays provisional until closeout records its re-examination (ADR-0002).

## Human Approval

- Approved: yes (intent; edits await /speckit-implement)
- Approved option: Approve all tasks
- Approved tasks/scope: T001–T040
- Required implementation invocation per constitution: `/speckit-implement`
- Date (UTC): 2026-11-15
