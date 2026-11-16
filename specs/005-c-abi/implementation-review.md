# Implementation Review

## Summary

Feature 005 adds a C interface (header `src/capi/optsolver.h`, static and shared libraries) over
the solver-independent Rust API, the solver-independent memory limit `memory_limit_gb` (Gurobi
`SoftMemLimit`; unsupported on MOSEK), and `Send` (not `Sync`) environments with serialised Gurobi
environment creation and freeing. 22 tasks in `tasks.md`.

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

Requirement → task coverage (revised after the first Gate 2 review, which found test ordering,
story independence, five coverage gaps, an untested comparison rule and incomplete additivity
checks; the first version of this analysis overstated coverage and said no conflict was found):

| Requirement | Tasks |
|---|---|
| Thin translation (FR-001) | T005 to T008 |
| Problem input (FR-002), zero copy (FR-003) | T006 (pointer-identity positive control), T010, T015 |
| Options (FR-004) | T006, T010 |
| Result fields (FR-005), two result paths (FR-006) | T008, T010, T015 |
| Environments and one-shot (FR-007) | T007, T012 |
| Thread rule (FR-008) | T011, T012 |
| Panic containment (FR-009) | T005, T016 |
| Error codes and messages, vendor code (FR-010) | T005, T008 (solver error), T016 |
| Memory limit (FR-011) | T006, T013, T014 |
| Libraries and flags (FR-012) | T001, T018, T019 |
| Header agreement (FR-013) | T004 |
| C-level tests (FR-014) | T003, T004, T010, T012, T014 to T016 |
| Versioning (FR-015) | T005, T016 |
| Ownership (FR-016) | T005, T008, T016 (leaks, NULL frees, message copies) |
| Capabilities (FR-017) | T007 |
| No output (FR-018) | T005, T010, T016 |
| Interface document (FR-019) | T002, T019 |
| Extension point (FR-020) | T006, T016 |
| Rust API preserved; listed changes (FR-021) | T006, T009, T011, T013 |
| Narrowest checks (FR-022) | T016, T010 |
| Agreement (SC-001) | T010 (LP, QP), T015 (conic), both paths |
| Heap (SC-002), wall time (SC-003) | T006, T017 |
| Environment reuse (SC-004) | T012 |
| Memory limit (SC-005) | T014 |
| Errors, panic, leaks (SC-006) | T016 (every expressible invalid input), T006 |
| Link demonstration (SC-007) | T018 |

Test ordering: every implementing task carries its own tests (written first or in the same
change); story phases need only the foundational phase, and each carries its own independent
test. The comparison rule is shown to reject disagreements (T003). Additivity covers all eight
feature combinations (T021).

Findings:

- Blocking: none remaining.
- Repaired after the first Gate 2 review: test ordering; story independence (driver and harness
  foundational, foundational tasks unlabelled); invalid-input coverage (all `expect_invalid`
  cases mapped or recorded as not expressible); wrong-length buffer, solver-error code, name
  functions, conic on both paths; negative tests of the comparison rule; additivity combinations;
  C structure fields `rows`/`cols` and the `caps` parameter name in the contract.
- Acceptable: Gurobi `solve_into` conic-dual length check (T009) is a repaired defect not listed in
  FR-021's Rust-API changes (it removes a panic on user input); reported at closeout.
- Deferred to closeout (Gate 3): the success-criteria checklist item (CHK015); User Story 3's
  "toy-sized" wording; components and boundaries reconciliation (T020); consumer adoption
  (COBRA gateway, routed to the COBRA feature); build-time cost of the new crate types (T021);
  existing Gurobi unit tests that panic without a licence (pre-existing Principle II gap).

## Proposed Implementation Scope

- Tasks proposed: all, T001 to T022.
- First independently testable slice: Phases 1 and 2 (T001 to T009).
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
