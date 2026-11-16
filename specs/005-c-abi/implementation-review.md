# Implementation Review

## Summary

Feature 005 adds a C interface (header `src/capi/optsolver.h`, static and shared libraries) over
the solver-independent Rust API, the solver-independent memory limit `memory_limit_gb` (Gurobi
`SoftMemLimit`; unsupported on MOSEK), and `Send` (not `Sync`) environments with serialised Gurobi
environment creation and freeing. 26 tasks in `tasks.md`.

## Embedded Core Commands Completed

- constitution: 3.0.0 (amended on the maintainer's instruction; checked)
- specify, clarify, checklist: Gate 1 PASS (review 4, `gate-reviews/gate-1-20261115T233939Z.md`)
- plan and architecture review: Gate 2A APPROVE_WITH_ADR (review 8,
  `gate-reviews/gate-2A-20261116T004341Z.md`)
- tasks: 2026-11-16
- analyze: 2026-11-16, below

## Architecture Verdict (Gate 2A)

- Verdict: APPROVE_WITH_ADR
- ADR required / written: ADR-0004, Accepted
- Date (UTC): 2026-11-16

## Cross-Artifact Analysis Summary

Performed by the feature agent with the `speckit-analyze` checks (coverage, consistency,
ambiguity, constitution alignment). Classification: blocking / should-fix / acceptable / deferred.

Requirement → task coverage (every functional requirement and success criterion has a task):

| Requirement | Tasks |
|---|---|
| Thin translation (FR-001) | T013, T014, T015 |
| Problem input (FR-002), zero copy (FR-003) | T013 (pointer-identity positive control), T017 |
| Options (FR-004) | T013, T017 |
| Result fields (FR-005), two result paths (FR-006) | T015, T017, T020 |
| Environments and one-shot (FR-007) | T014, T018 |
| Thread rule (FR-008) | T006, T018 |
| Panic containment (FR-009), error codes and messages (FR-010) | T009, T012 |
| Memory limit (FR-011) | T003, T004, T005, T019 |
| Libraries and flags (FR-012) | T001, T022, T023 |
| Header agreement (FR-013) | T011 |
| C-level tests (FR-014) | T010 to T012, T016 to T020 |
| Versioning (FR-015), ownership (FR-016) | T009, T012 |
| Capabilities (FR-017) | T014, T012, T017 |
| No output (FR-018) | T009 (silent hook), T012, T017 |
| Interface document (FR-019) | T002, T023 |
| Extension point (FR-020) | T012 |
| Rust API preserved; listed changes (FR-021) | T003, T006, T007 |
| Narrowest checks (FR-022) | T012, T017 |
| Agreement (SC-001) | T017, T020 |
| Heap (SC-002), wall time (SC-003) | T013, T021 |
| Environment reuse (SC-004) | T018 |
| Memory limit (SC-005) | T019 |
| Errors, panic, leaks (SC-006) | T012 |
| Link demonstration (SC-007) | T022 |

Findings:

- Blocking: none.
- Should-fix (folded into tasks before this review): capabilities had no test (added to T017 and
  T012); constants must be `pub` for the layout test (T009); header parameter names (`result`,
  `caps`) aligned in the contract.
- Acceptable: Gurobi `solve_into` conic-dual length check (T007) is a repaired defect not listed in
  FR-021's Rust-API changes (it removes a panic on user input); reported at closeout.
- Deferred to closeout (Gate 3): the success-criteria checklist item (CHK015); User Story 3's
  "toy-sized" wording (a spec revision with Gate 1 re-review if changed); components and
  boundaries reconciliation; consumer adoption (COBRA gateway, routed to the COBRA feature);
  existing Gurobi unit tests that panic without a licence (pre-existing Principle II gap).

Constitution alignment: no conflict found; the gate reviews record the per-principle findings.

## Proposed Implementation Scope

- Tasks proposed: all, T001 to T026.
- First independently testable slice: Phases 1 to 3 (T001 to T015; unlicensed tier green).
- Files likely to change: Cargo.toml; src/lib.rs; src/capi/ (new: mod.rs, problem.rs, env.rs,
  result.rs, optsolver.h); src/options.rs; src/gurobi/env.rs, params.rs, solve.rs; src/mosek/env.rs,
  params.rs (test); tests/c_interface.rs, tests/c/, tests/common/ (cdata.rs, ccompiler.rs,
  compare.rs, mod.rs), tests/header_conformance.rs, tests/header_conformance_mosek.rs,
  tests/standards.rs; benches/load_solve.rs; docs/llm-interface.md; docs/architecture/.
- Files that should NOT change: the core's validation, status translation, parameter application
  outside the memory-limit mapping, verification; vendor declarations (`ffi.rs`); build.rs;
  project.yml; the constitution.

## Tests and Validation Expected

Narrowest first: `cargo test --features capi --test c_interface` (unlicensed: agreement, version,
errors, panic, leaks); then `cargo test --features capi,gurobi,mosek --test c_interface`
(licensed toy comparison, reuse, threads, memory limit, conic); then the in-gate checks (T025).

## Blocking Issues

None.

## Acceptable Risks

- Gurobi environment creation and freeing on any thread under a lock is an inference from vendor
  text (research R3), exercised by a concurrency test, not proven.
- MOSEK concurrent environment creation and freeing unverified; documented in the header.

## Gate 2 Approval (compliance review)

- Approved: pending the Gate 2 review
- Proposed scope (all | slice): all
- Approved tasks/scope:
- Gate 2 review file and verdict:
- Date (UTC):
