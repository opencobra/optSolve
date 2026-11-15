---
description: "Task list for feature 003: LLM interface document, toy instances, cross-solver comparison"
---

# Tasks: LLM-Usable Interface Documentation And Self-Contained Cross-Solver Test Instances

**Input**: `specs/003-llm-interface-toy-instances/` — plan.md, spec.md, research.md (R1–R12),
data-model.md, contracts/public-api.md, quickstart.md, architecture-delta.yaml (APPROVE_WITH_ADR,
ADR-0003)

**Tests**: REQUIRED (constitution II, XIV). Tests first, failing before the code. Every solver
test is written once against `Backend` in `tests/common/` and runs on every backend supporting
its class (and, for conic problems, its cone kinds).

**Licences on this machine**: `GRB_LICENSE_FILE=/usr/local/bin/gurobi1303/gurobi.lic2/gurobi.lic`
and `MOSEK_BINDIR=/usr/local/bin/mosektoolslinux64x86/mosek/11.2/tools/platform/linux64x86/bin`
on the command line.

## Format: `[ID] [P?] [Story] Description`

US1 interface document; US2 toy instances; US3 cross-solver comparison (with the Gurobi QP and
second-order-cone extension).

---

## Phase 1: Setup

- [X] T001 Declare `test_corpus.toy_instances` in `.specify/memory/project.yml` (the 20 names of data-model.md: 4 existing small LP fixtures, 2 LP, 3 QP, 11 conic) and add `toy_instances()` and `toy_path(name)` (MPS or JTASK by existing file) to `tests/common/fixtures.rs` with a self-test
- [X] T002 [P] Add Gurobi declarations and constants used by T017–T018 (`GRBaddqpterms`, `GRBaddqconstr`, `GRB_ERROR_Q_NOT_PSD`, `GRB_ERROR_QCP_EQUALITY_CONSTRAINT`, `GRB_ERROR_NOT_SUPPORTED`, and any attribute-name constants) to `src/gurobi/ffi.rs` and `tests/header_conformance.c`; header conformance passes

---

## Phase 2: Foundational (blocks every story)

- [X] T003 [P] Contract tests in `tests/contract/qp_conic.rs`: `Cone::kind` for every cone; `ConeKind::name` stable names; `check_problem` rejects a cone kind missing from `Capabilities::cones` with `Unsupported { field: <name> }` and accepts listed kinds (fake backend)
- [X] T004 Implement `ConeKind`, `Cone::kind`, `ConeKind::name` in `src/conic.rs`, `Capabilities::cones` in `src/backend.rs`, the per-kind rule in `src/options/check.rs`, re-export in `src/lib.rs`; MOSEK lists all kinds, Gurobi none (until T018), fake backends updated; make T003 pass
- [X] T005 [P] Test MPS reader (`tests/common/mps.rs`): QUADOBJ, QSECTION <objective row>, QMATRIX (research R6: one-triangle sections mirrored to the lower triangle with duplicates summed; QMATRIX lower triangle taken), comment lines and trailing-blank headers; `MpsProblem` gains Q (lower-triangle CSC) and `problem()` applies `with_Q`; self-tests on hand-written strings for each section, the duplicate trap, and malformed input
- [X] T006 [P] Test JTASK reader (`tests/common/jtask.rs`): `["svecpsd", d]` → PSD cone of order r with r(r+1)/2 = d (non-triangular d rejected); power weights normalised; header comment corrected (domain names observed in MOSEK 11.2.0 writer output, research R7); self-tests

**Checkpoint**: per-cone capabilities and both readers ready.

---

## Phase 3: User Story 1 — LLM interface document (P1)

- [X] T007 [US1] Drift test in `tests/standards.rs` per research R10 (public items of the solver-independent modules plus `gurobi::Env`, `mosek::Env`, each in backticks in `docs/llm-interface.md`), with a self-test showing it fails on an unmentioned identifier; fails before T008
- [X] T008 [US1] Write `docs/llm-interface.md` with the sections of data-model.md (Interface document) and complete examples (LP, QP, conic, unsupported class, non-optimal status, caller buffers), each building over borrowed arrays; examples select backends by `#[cfg(feature = …)]` and return gracefully without a licence (research R10)
- [X] T009 [US1] Make the document the crate-level documentation (`#![doc = include_str!("../docs/llm-interface.md")]` in `src/lib.rs`, replacing the current crate doc text); `cargo test --doc` passes with no features and with `--features gurobi,mosek` (examples run); T007 passes
- [X] T010 [P] [US1] Write `docs/llms.txt` (llmstxt.org form) linking `docs/llm-interface.md`, `docs/test-corpus.md`, `README.md`
- [X] T011 [P] [US1] Link the document from `README.md`

---

## Phase 4: User Story 2 — Self-contained toy instances (P2)

- [X] T012 [US2] Write `tools/corpus/make_toy_instances.m` (research R9): definitions of every new toy instance (data-model.md table) with known solution and derivation; write with Gurobi (`gurobi_write`, MPS: LP, QP) or MOSEK (JTASK: conic); read back; solve with both tools where both support the class (MOSEK only otherwise); stop on any disagreement with the analytic optimum; write provenance (data-model.md fields); also add independent checks to the four existing small LP fixtures' provenance
- [X] T013 [US2] Run T012 (log to `results/003-llm-interface-toy-instances/runs/<UTC>-toy/`); commit-ready files in `tests/fixtures/small/`
- [X] T014 [P] [US2] Toy trials in `tests/common/toy.rs`, registered in `tests/corpus.rs`: per instance and backend: provenance hash and dimensions; backend without the class or a cone kind → `Unsupported`; otherwise expected status, objective within feature 002 R10 of `expected_objective`, Verified (certificate measure where the backend provides certificates); conic instances also solved in maximise form (c negated, sense flipped, objective −p*); independent of the variational kinetics corpus
- [X] T015 [US2] Describe the toy instances in `docs/test-corpus.md` (feature 002's corpus-document test covers the new names)

---

## Phase 5: User Story 3 — MOSEK and Gurobi compared (P3)

### Tests first

- [X] T016 [P] [US3] Adapt feature 002 per-backend tests in `tests/common/f002.rs` to per-kind capabilities: `t034_conic_cases` runs each case on a backend listing its cone kind and expects `Unsupported` otherwise; `t034_conic_infeasible` and `t029_*` accept "no certificate" where the backend documents none (status still required); fail on Gurobi before T017–T018
- [X] T017 [P] [US3] Comparison harness in `tests/common/compare.rs` (research R11): pair trials over toy instances, the LP corpus (seeds 1–3, generalising feature 002 T019) and the derived QP; certificates requested for instances expected Infeasible or Unbounded (so statuses are definite, research R4); equal status; Optimal → R10 agreement and both Verified; not comparable → ignored with the reason; report line appended to `results/003-llm-interface-toy-instances/runs/<UTC>-compare/report.tsv`; guard self-tests: a forced disagreement fails, a one-backend registry yields ignored trials

### Implementation

- [X] T018 [US3] Gurobi QP (research R1): `GRBaddqpterms` with halved diagonal; `NonConvex` = 0 in `gurobi::params::apply` when Q is present; effective tolerances by class (R5); capabilities gain `Qp`; feature 002 QP tests (t029) pass on Gurobi; non-convex Q gives `Error::Solver` 10020
- [X] T019 [US3] Gurobi cones (research R2, R3): stacked conic rows through the blocked load (fixed buffer of indices and values), auxiliary columns and bounds, one `GRBaddqconstr` per quadratic or rotated cone, nonnegative/zero as rows; `QCPDual` = 1; s from `Pi` (sign asserted by the cone tests); auxiliaries hidden from x and row activity; capabilities gain `Conic` with the four kinds; T016 passes on Gurobi for its kinds
- [X] T020 [US3] Run the comparison (T017) with both solvers; every comparable instance agrees or the disagreement is investigated and recorded in research.md before closeout

---

## Phase 6: Benchmarks

- [X] T021 Extend `benches/load_solve.rs` with the derived QP (iDopaNeuroC_problem_LP_minNorm) on both backends, both index types, N = 5; positive control (QP triplet count in the load report); evaluate the SC-004 rule plus the documented O(nnz(Q)); write `specs/003-llm-interface-toy-instances/benchmark-results.md`

---

## Phase 7: Polish

- [ ] T022 In-gate checks (fmt; clippy -D warnings ×4 feature combinations; doc tests with and without features; tests in every tier incl. heavy; Rust 1.85; feature-combination check), logged to `results/003-llm-interface-toy-instances/runs/<UTC>-gate/`
- [X] T023 [P] Reconcile `docs/architecture/components.yaml` with the delta; README
- [ ] T024 LLM trial (SC-002, research R12) with three independent agents; results recorded in the run bundle (executed at verification, before Gate 3) — **DEFERRED (debt, maintainer 2026-11-15): manual run; protocol in llm-trial.md**
- [X] T025 Report files edited, checks run, pass/fail and unverified behaviour
- [X] T026 Implementation receipt `specs/003-llm-interface-toy-instances/agent-runs/<UTC>-implement-003/implementation-receipt.md`

---

## Dependencies & Execution Order

- Setup (T001–T002) → Foundational (T003–T006) → stories.
- US1 (T007–T011) depends only on Foundational (the document describes `ConeKind`).
- US2: T012–T013 (MATLAB) can run in parallel with US1; T014 needs T005, T006, T013.
- US3: T016–T017 tests first; T018 before T019; T020 needs T013, T014, T019.
- Benchmarks after T018; polish last; T024 after the document is final.

## Parallel Opportunities

- T002, T005, T006 alongside T003–T004 (different files).
- US1 writing (T008) alongside the MATLAB generation (T012–T013).
- T016 and T017 in parallel (different files).

## Implementation Strategy

1. MVP: Setup, Foundational, US1 (the document with compiled examples).
2. US2 toy instances; then US3 Gurobi extension and comparison.
3. Benchmark, polish, LLM trial, Gate 3.
