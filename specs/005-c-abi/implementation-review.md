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
| Options (FR-004) | T006, T008 (every option through C), T010 |
| Result fields incl. certificates (FR-005), two result paths and the NaN rule (FR-006) | T008 (every accessor, NaN after error), T010 (NaN for absent quantities; certificates on infeasible and unbounded toys), T015 |
| Environments and one-shot (FR-007) | T007, T012 |
| Thread rule (FR-008) | T011, T012 |
| Panic containment (FR-009) | T005, T016 |
| Error codes and messages, vendor code, per-thread messages (FR-010) | T005 (two-thread test), T008 (solver error), T016 |
| Memory limit (FR-011) | T006, T013, T014 |
| Libraries and flags (FR-012) | T001, T018, T019 |
| Header agreement (FR-013) | T004 |
| C-level tests (FR-014) | T003, T004, T010, T012, T014 to T016 |
| Versioning (FR-015) | T005, T016 |
| Ownership (FR-016) | T005, T008, T016 (unlicensed leak check of error paths and message copies; licensed leak check of environment, result and capabilities; positive controls for both) |
| Capabilities, usable or not with reason (FR-017) | T007 (not compiled, usable, compiled but not usable; the last possibly unverified) |
| No output (FR-018) | T005, T010, T016 |
| Interface document (FR-019) | T002, T019 |
| Extension point (FR-020) | T006, T016 |
| Rust API preserved; listed changes (FR-021) | T006, T009, T011, T013 |
| Narrowest checks (FR-022) | T016, T010 |
| Agreement (SC-001) | T010 (LP, QP), T015 (conic), both paths |
| Heap (SC-002), wall time (SC-003) | T006, T017 |
| Environment reuse (SC-004) | T012 |
| Memory limit (SC-005) | T014 |
| Errors, panic, leaks (SC-006) | T016 (every `expect_invalid` case mapped or recorded, and the too-large dimension cases for rows and columns; leak checks in both tiers with positive controls), T006 (validation order) |
| Link demonstration (SC-007) | T018 (documents the flags first, then builds with exactly them) |

Test ordering: every implementing task carries its own tests (written first or in the same
change); story phases need only the foundational phase, and each carries its own independent
test. The comparison rule is shown to reject disagreements (T003). Additivity covers all eight
feature combinations (T021). The Gurobi lifecycle lock (T011) is foundational, so every licensed
trial runs with it.

Findings:

- Repaired after the sixth Gate 2 review: the too-large column case (now tested from C, with the
  dimensions-first validation order); principle numerals given their names.
- Repaired after the fifth Gate 2 review: the too-large dimension case was not mapped through C
  (now T016, with T006's validation order); the link demonstration preceded the documentation of
  its flags (now written first in T018).
- Repaired after the fourth Gate 2 review: the leak check covered no environment or result and
  its control could not run without a licence (now two leak checks, unlicensed and licensed, each
  with a working control); the interface-document drift test could not pass at the end of the
  foundational phase (names now documented in the task that introduces them, prose in T019).
- Repaired after the third Gate 2 review: certificates through C untested; no negative control for
  the empty-output assertion or the leak checker; in-process C calls would have needed `unsafe`
  outside `src/capi/` (now a safe hidden helper for the bench, and driver-based tests).
- Blocking: none remaining after the second Gate 2 review's repairs (NaN rule, per-thread
  messages and the not-usable backend were untested in the previous version; the previous
  analysis claimed them covered).
- Repaired after the first Gate 2 review: test ordering; story independence (driver and harness
  foundational, foundational tasks unlabelled); invalid-input coverage (all `expect_invalid`
  cases mapped or recorded as not expressible); wrong-length buffer, solver-error code, name
  functions, conic on both paths; negative tests of the comparison rule; additivity combinations;
  C structure fields `rows`/`cols` and the `caps` parameter name in the contract.
- Acceptable: stale plan text — plan.md lists four feature combinations (tasks run eight, T021)
  and names the C structure fields `m`/`n` (the contract now uses `rows`/`cols`); the tasks and the
  contract govern; corrected at closeout with the reconciliation (T020).
- Acceptable: Gurobi `solve_into` conic-dual length check (T009) is a repaired defect not listed in
  FR-021's Rust-API changes (it removes a panic on user input); reported at closeout.
- Deferred to closeout (Gate 3): the success-criteria checklist item (CHK015); User Story 3's
  "toy-sized" wording; components and boundaries reconciliation (T020); consumer adoption
  (COBRA gateway, routed to the COBRA feature); build-time cost of the new crate types (T021);
  existing Gurobi unit tests that panic without a licence (pre-existing gap against Principle II, Testing, Benchmarks, And Behavioural Evidence).

## Proposed Implementation Scope

- Tasks proposed: all, T001 to T022.
- First independently testable slice: Phases 1 and 2 (T001 to T009).
- Files likely to change: Cargo.toml; src/lib.rs; src/capi/ (new: mod.rs, problem.rs, env.rs,
  result.rs, optsolver.h); src/options.rs; src/gurobi/env.rs, params.rs, solve.rs; src/mosek/env.rs;
  tests/contract.rs (licensed Rust-API trials of T009, T011, T013, T014; not listed in the plan's
  Project Structure — a known omission of the plan's file list, recorded here, not an architecture
  change); tests/c_interface.rs, tests/c/, tests/common/ (cdata.rs, ccompiler.rs,
  compare.rs, mod.rs), tests/header_conformance.rs, tests/header_conformance_mosek.rs,
  tests/standards.rs; benches/load_solve.rs; docs/llm-interface.md; docs/architecture/.
- Files that should NOT change: the core's validation, status translation, parameter application
  outside the memory-limit mapping, verification; vendor declarations (`ffi.rs`); build.rs;
  project.yml; the constitution.

## Tests and Validation Expected

Narrowest first: `cargo test --features capi --test c_interface` (unlicensed: agreement, version,
errors, panic, leaks); then `cargo test --features capi,gurobi,mosek --test c_interface`
(licensed toy comparison, reuse, threads, memory limit, conic); then the in-gate checks (T021).

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
