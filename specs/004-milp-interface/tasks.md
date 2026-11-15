---
description: "Task list for feature 004: mixed-integer programmes through the existing interface"
---

# Tasks: Mixed-Integer Programmes Through The Existing Interface

**Input**: `specs/004-milp-interface/` — plan.md, spec.md, research.md (R1–R12), data-model.md,
contracts/public-api.md, quickstart.md, architecture-delta.yaml (APPROVE_WITH_ADR, ADR-0004)

**Tests**: REQUIRED (constitution II). Tests first, failing before the code; written once against
`Backend` in `tests/common/`, extending the existing toy and comparison trials (no new framework).

**Licences**: `GRB_LICENSE_FILE=/usr/local/bin/gurobi1303/gurobi.lic2/gurobi.lic` and
`MOSEK_BINDIR=/usr/local/bin/mosektoolslinux64x86/mosek/11.2/tools/platform/linux64x86/bin` on
the command line.

## Format: `[ID] [P?] [Story] Description`

US1 MILP through the existing interface; US2 tests, toys, comparison reused; US3 mixed-integer QP
and conic, fixed-model duals; US4 the document.

---

## Phase 1: Setup

- [ ] T001 [P] Gurobi declarations and constants (`GRBfixmodel`, `GRBsetdblattrlist`, `GRB_CB_MIP`, `GRB_CB_MIP_OBJBST`, `GRB_CB_MIP_OBJBND`, `GRB_CB_MIP_NODCNT`, `GRB_CONTINUOUS`/`GRB_INTEGER`/`GRB_BINARY`) in `src/gurobi/ffi.rs` and `tests/header_conformance.c`
- [ ] T002 [P] MOSEK declarations and constants (`MSK_putvartypelist`, `MSK_clonetask`, `MSK_putvarboundlist`, `MSK_VAR_TYPE_CONT`/`INT`, `MSK_SOL_ITG`, `MSK_SOL_STA_INTEGER_OPTIMAL` (exists), MIO parameters, `MSK_DINF_MIO_OBJ_BOUND`/`MIO_OBJ_INT`, `MSK_IINF_MIO_NUM_BRANCH`/`MIO_NUM_RELAX`, `MSK_CALLBACK_IM_MIO`/`NEW_INT_MIO`, `MSK_RES_ERR_GLOBAL_INV_CONIC_PROBLEM`) in `src/mosek/ffi.rs` and `tests/header_conformance_mosek.c`
- [ ] T003 Declare the new toy names and `milp_instances` (iDopaNeuroC_problem_LP_int) in `.specify/memory/project.yml`; `milp_instances()` and `milp_provenance_path()` in `tests/common/fixtures.rs`

---

## Phase 2: Foundational — the contract extended (blocks every story)

- [ ] T004 [P] Contract tests in `tests/contract/qp_conic.rs` (or a sibling module): mixed-integer capability rule (base class and cone kinds, all-continuous types need nothing; fake backend); `Solution::mip_gap` (definition incl. p = 0, no bound); verification: integrality measure (pass, fail, binary bounds ∩ [0, 1]), MIP gap measure (either test passes; NotEvaluated without a bound), no dual measures for MI without duals, fixed-problem dual measures; guard self-test: an integrality violation and a gap violation fail
- [ ] T005 Implement in the core: `Options` fields (research R4) and `set_shared_fields`; `Capabilities::integer_classes`, `integer_cones` and the rule in `check_problem`; `Counts::nodes`; `Solution::mip_gap`; `Tolerances::mip_relative_gap`, `mip_absolute_gap`; `ProgressPhase::BranchAndBound` (`#[non_exhaustive]`); verification measures `INTEGRALITY`, `MIP_GAP` and fixed-problem duals (`src/verify.rs`); update every constructor of the extended types; T004 passes
- [ ] T006 [P] Test MPS reader: `'MARKER' 'INTORG'/'INTEND'`, bound types `BV`, `LI`, `UI` (and integer without bounds = [0, 1], research R10); `MpsProblem::var_types`, `problem()` applies `with_var_types`; self-tests
- [ ] T007 [P] Test JTASK reader: `"type": "int"` → `b'I'`; `JtaskProblem::var_types`; self-tests

**Checkpoint**: contract, readers ready; no solver needed.

---

## Phase 3: User Story 1 — MILP through the existing interface (P1)

- [ ] T008 [US1] Per-backend MILP tests in `tests/common/` (written once): limits (node limit → StoppedAtLimit Nodes with incumbent, bound and gap; solution limit; time limit), unsupported combination (PSD with integers) rejected before solving, `random_seed` accepted for MI on both, options mapped and reported in effective parameters; fail before T009–T012
- [ ] T009 [US1] Gurobi: vtype from the caller's slice in `src/gurobi/load.rs` (single call and blocked paths; auxiliary columns `'C'`); VII-A probe kept as a licensed test (array unchanged after load)
- [ ] T010 [US1] Gurobi: MIP parameters and effective tolerances in `src/gurobi/params.rs` (no `DualReductions` for MI); capabilities, bound (`ObjBound`), nodes, MI rules (no duals) in `src/gurobi/solve.rs`; `GRB_CB_MIP` progress in `src/gurobi/callback.rs`
- [ ] T011 [US1] MOSEK: `MSK_putvartypelist` and binary bounds ∩ [0, 1] in `src/mosek/load.rs`; MIO parameters, MIO seed for MI, effective tolerances in `src/mosek/params.rs`; INTEGER_OPTIMAL in `src/mosek/status.rs` (unit tests); ITG solution, bound, nodes, capabilities in `src/mosek/solve.rs`; IM_MIO / NEW_INT_MIO progress in `src/mosek/callback.rs`
- [ ] T012 [US1] Make T008 and the feature 001–003 per-backend tests pass on both backends (`cargo test --features gurobi,mosek`)

---

## Phase 4: User Story 2 — toys, trials and comparison reused (P2)

- [ ] T013 [US2] Extend `tools/corpus/make_toy_instances.m` with the mixed-integer toy instances of research R11 (Gurobi writes MPS with explicit integer bounds; MOSEK writes JTASK for the conic ones; two-tool checks; `accepted_statuses` where solvers differ by design); run it (log to `results/004-milp-interface/runs/<UTC>-toy/`)
- [ ] T014 [US2] Extend `tests/common/toy.rs` (accepted statuses; integrality-aware outcome; no maximise form for MI conic unless meaningful) and `tests/common/compare.rs` (compatible statuses; mixed-integer objective rule, research R11); the trials pick up the new names from `project.yml`
- [ ] T015 [US2] Describe the mixed-integer toy instances in `docs/test-corpus.md`

---

## Phase 5: User Story 3 — mixed-integer QP and conic, fixed-model duals (P3)

- [ ] T016 [P] [US3] Per-backend tests: fixed-model duals requested → duals present, marked in attributes, verified against the fixed problem, for a MILP, a MIQP and a mixed-integer second-order-cone toy; not requested → empty and no extra solve; fixed problem infeasible → no duals, status recorded
- [ ] T017 [US3] Gurobi fixed-model duals (`GRBfixmodel`, rounded bounds via `GRBsetdblattrlist`, solve, existing extraction on the fixed model, free) in `src/gurobi/solve.rs`
- [ ] T018 [US3] MOSEK fixed-model duals (`MSK_clonetask`, types continuous, bounds fixed at rounded values, optimise, BAS or ITR extraction with the existing code) in `src/mosek/solve.rs`
- [ ] T019 [US3] MIQP and mixed-integer conic loading verified on both backends (Gurobi QP terms and cone constraints with integer columns; MOSEK putqobj / ACCs with integer columns); T016 and the MI toy trials pass

---

## Phase 6: User Story 4 — the document (P4)

- [ ] T020 [US4] Extend `docs/llm-interface.md`: variable types, mixed-integer capabilities, MIP options, statuses for MI, `mip_gap`, integrality and gap verification, fixed-model duals, progress phase, a MILP example (doctest); drift test passes

---

## Phase 7: Benchmark and polish

- [ ] T021 Derived MILP provenance `tests/fixtures/milp/iDopaNeuroC_problem_LP_int.provenance.json` (research R12) and a MILP scenario in `benches/load_solve.rs` (both backends, both index types, N = 5, time limit 60 s, positive control: integer columns loaded); `specs/004-milp-interface/benchmark-results.md`
- [ ] T022 In-gate checks (fmt; clippy ×4; doctests both tiers; all test tiers incl. heavy; Rust 1.85), logged to `results/004-milp-interface/runs/<UTC>-gate/`
- [ ] T023 [P] Reconcile `docs/architecture/components.yaml`; README
- [ ] T024 Report and implementation receipt `specs/004-milp-interface/agent-runs/<UTC>-implement-004/implementation-receipt.md`

---

## Dependencies & Execution Order

- Setup (T001–T003) → Foundational (T004–T007) → US1 (T008–T012) → US2 (T013–T015; T013 can
  start after T003) → US3 (T016–T019) → US4 (T020) → T021–T024.

## Parallel Opportunities

- T001, T002 together; T006, T007 alongside T004–T005; T013 (MATLAB) alongside US1.

## Implementation Strategy

MVP: Setup + Foundational + US1 (MILP on both backends with limits and verification); then the
reused tests and toys, the mixed-integer QP/conic and fixed-model duals, the document, benchmark.
