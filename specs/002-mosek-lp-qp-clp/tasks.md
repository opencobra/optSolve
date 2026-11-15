---
description: "Task list for feature 002: MOSEK backend for LP, QP and CLP"
---

# Tasks: MOSEK Backend For LP, QP And CLP

**Input**: `specs/002-mosek-lp-qp-clp/` — plan.md, spec.md, research.md (R1–R15), data-model.md,
contracts/public-api.md, quickstart.md, architecture-delta.yaml (APPROVE_WITH_ADR, ADR-0002)

**Tests**: REQUIRED (constitution II, XIV). Tests first, failing before the code. Every solver
test is written once against `Backend` in `tests/common/` and runs on every backend supporting
its class; backend-specific tests only for status translation and header conformance.

**Licences on this machine**: Gurobi needs `GRB_LICENSE_FILE=/usr/local/bin/gurobi1303/gurobi.lic2/gurobi.lic`
on the command line; MOSEK uses `MOSEKLM_LICENSE_FILE` as set.

## Format: `[ID] [P?] [Story] Description`

US1 corpus, documentation, Rust 1.85; US2 contract behaviour; US3 MOSEK LP; US4 MOSEK QP;
US5 MOSEK CLP.

---

## Phase 1: Setup

- [X] T001 Add feature `mosek` to `Cargo.toml`; extend `build.rs` to link `libmosek64` from `MOSEK_BINDIR` (declared with `rerun-if-env-changed`; version from `project.yml` `domain.solvers.mosek`) when the feature is on, keeping one `links` key per vendor library in the crate (if Cargo forbids two `links` values in one crate, record the decision in research.md and use `cargo:rustc-link-lib` without `links` for the second vendor)
- [X] T002 [P] Update `.specify/memory/project.yml`: `domain.solvers.mosek: "11.2"` already present (confirm); split `test_corpus` into `lp_instances` (iDopaNeuroC_problem_LP), `qp_instances` (iDopaNeuroC_problem_LP_minNorm), `clp_instances` (iCoreED_problem_1..5), `clp_heavy_instances` (iDopaNeuroC_problem_1, 2, 2_fullRankFR, 3, 3_fullRankFR, 5) with the source-instance mapping for the LP fixture; update `tests/common/fixtures.rs` readers accordingly

---

## Phase 2: Foundational (blocks every story)

- [X] T003 [P] Tests in `tests/contract.rs` (written once, no solver): `Cone` and `ConicRef` validation — dimensions summing to k, each cone type's minimum dimension, power exponents positive and fewer than the dimension, PSD dimension r(r+1)/2, unknown/zero-size cones, F row indices < k; `with_Q` rejects upper-triangle entries, wrong size, non-finite values; `check_problem` rejects conic and QP problems for a backend without those classes (fake backend)
- [X] T004 [P] Verifier tests in `tests/contract.rs`: QP measures (reduced costs with Qx, gap with ½xᵀQx) on a hand-built QP with known optimum; conic measures per cone type (a point inside, on the boundary and outside each cone; dual cone membership with σ for minimise and maximise; complementarity; gap with −gᵀs; conic Farkas certificate); guard self-test: an all-zero conic solution claimed optimal is Failed
- [X] T005 Implement `src/conic.rs` (`Cone`, `ConicRef`, validation, block violation functions per data-model.md including closure cases of the exponential cones and λ_min for PSD via a small symmetric eigenvalue routine or Cholesky test, no new dependency) and wire `pub mod conic` in `src/lib.rs`
- [X] T006 Extend `src/problem.rs` (`with_Q` validation, `with_conic`, `conic()`), `src/backend.rs` (`ProblemClass::Conic`), `src/options/check.rs` (`check_problem` for Q and conic), `src/result.rs` (`Solution::s`, `Certificate::Farkas::s`, `SolutionBuffers::s`), and `src/verify.rs` (QP and conic measures, data-model.md); make T003–T004 pass
- [X] T007 [P] Extend the shared harness `tests/common/mod.rs`: MOSEK as a declared backend (`optsolver::mosek::Env::new`), heavy-tier registration (trials ignored with reason unless `OPTSOLVER_HEAVY_TESTS=1`), and two-backend trials (`pair_trials(name, fn(&dyn Backend, &dyn Backend))`) ignored with reason unless both are available; harness self-tests for both in `tests/contract.rs`
- [X] T008 [P] CBF reader in `tests/common/cbf.rs` (objective sense, variables with domains, constraints with domains incl. affine conic rows, OBJACOORD/OBJBCOORD, ACOORD/BCOORD, the cone codes used by MOSEK's writer for F, L+, L-, L=, Q, QR, EXP, EXP*, POW/POW* with parameters, SVPSD) producing `optsolver::Problem` + `ConicRef`; `self_tests()` on hand-written strings including malformed input
- [X] T009 Extend `tests/standards.rs`: MOSEK `extern` blocks only in `src/mosek/ffi.rs`; MOSEK parameter setters (`MSK_putintparam`, `MSK_putdouparam`, `MSK_putstrparam`, `MSK_putnaintparam`, `MSK_putnadouparam`, `MSK_putnastrparam`, `MSK_putparam`) only in `src/mosek/params.rs` and `ffi.rs`

**Checkpoint**: core QP and conic contract complete without a solver.

---

## Phase 3: User Story 1 — Named, documented corpus on Rust 1.85 (P1)

- [X] T010 [US1] Rust 1.85: replace `cast_unsigned` in `src/gurobi/load.rs` tests with an explicit conversion; diagnose and fix the benchmark's failure on 1.85; confirm `cargo +1.85.0 test --all-targets --features gurobi` builds and passes
- [X] T011 [US1] Rename the LP fixture: extend `tools/corpus/extract_idopaneuroc_fixtures.m` to map the source instance iDopaNeuroC_problem_1 to the fixture name iDopaNeuroC_problem_LP (from `project.yml`) and record `source_instance` and `renamed_from` in provenance; re-run it (log to `results/002-mosek-lp-qp-clp/runs/<UTC>-extract-lp/`); remove the old fixture files; update every non-historical reference (tests, quickstart of feature 002, README) — feature 001 records keep the old name
- [X] T012 [P] [US1] Write `docs/test-corpus.md` (Tier 3): every fixture in `project.yml` with class, dimensions, source, extraction script, objective rule, expected status; why variational kinetics problems 2–5 are not LP test problems (feature 001 research R18); the derived QP; the CLP definition (research R11) and why iDopaNeuroC_problem_4 is excluded
- [X] T013 [P] [US1] Test in `tests/corpus.rs`: every fixture declared in `project.yml` appears in `docs/test-corpus.md`, and every fixture named there exists

---

## Phase 4: User Story 2 — Contract behaviour verified on every backend (P2)

- [X] T014 [P] [US2] Per-backend test in `tests/common/us3.rs`: `tests/fixtures/small/infeasible.mps` with certificates → Infeasible, Farkas certificate with y_F,1 > 0 and z_F,j < 0 for both columns (data-model.md CHK010) and verification passing
- [X] T015 [P] [US2] Per-backend test: progress closure records `std::thread::current().id()` and an in-flight counter; threads = 4, method Barrier, iDopaNeuroC_problem_LP; assert every id equals the caller's, maximum in-flight 1, at least one call (data-model.md CHK011)
- [X] T016 [US2] Run T014–T015 on Gurobi (both must pass before MOSEK exists); record results; remove the "observed, not cited" notes in feature 001's `src/gurobi/solve.rs` comment and research R17 by pointing to these tests

---

## Phase 5: User Story 3 — MOSEK LP (P3)

### Tests first

- [X] T017 [P] [US3] Backend-specific `src/mosek/status.rs` unit tests: every MSK_SOL_STA_*, MSK_PRO_STA_* and MSK_RES_TRM_* value and an unknown response code translate per research R6 (no licence)
- [X] T018 [P] [US3] `tests/header_conformance_mosek.rs` + `.c`: every declaration in `src/mosek/ffi.rs` and every constant checked against `mosek.h`; skipped visibly without `MOSEK_BINDIR`
- [X] T019 [P] [US3] Two-backend trial in `tests/corpus.rs`: iDopaNeuroC_problem_LP seeds 1, 2, 3, Gurobi and MOSEK both Optimal and Verified, objective agreement per research R10 (SC-001)
- [X] T020 [P] [US3] Generalise feature 001's `t033_raw_parameter_effective` to use any mapped integer vendor parameter (MOSEK maps no seed), keeping its Gurobi behaviour

### Probes (licensed; findings recorded in research.md before dependent code)

- [X] T021 [US3] MOSEK probes (unit tests in `src/mosek/` kept as licensed tests): ±∞ on a bound side the key marks infinite (ignored or error); which solution type carries an LP infeasibility certificate when ITR and BAS both exist; solution and problem status after `MSK_RES_TRM_USER_CALLBACK`; zero-length slices and NULL pointers for m = 0, k = 0; record each in research.md

### Implementation

- [X] T022 [US3] `src/mosek/ffi.rs` (about 45 functions, 15 enums; research R2–R9), `src/mosek/env.rs` (`MSK_makeenv`, `MSK_checkoutlicense`, errors via `MSK_getcodedesc`/`MSK_getlasterror`, `Drop`), `src/mosek/mod.rs`
- [X] T023 [US3] `src/mosek/params.rs`: the single applying function per research R9 (unsupported shared fields → `Error::Unsupported`), log stream to the `log` facade when requested, effective tolerances by problem class, `MAPPINGS`
- [X] T024 [US3] `src/mosek/status.rs`: total translation and solution-returned rule (research R6)
- [X] T025 [US3] `src/mosek/callback.rs`: progress records from `douinf`/`intinf`, cancellation by non-zero return, `catch_unwind`, panic resumed after optimise
- [X] T026 [US3] `src/mosek/load.rs` (LP): presize, append, `putacolslice64` single call (i32) or blocks through `IndexBuffer` (usize), bounds with bound keys and infinity copies only when needed, objective, constant, sense; load timing split as for Gurobi
- [X] T027 [US3] `src/mosek/solve.rs`: `Backend for Env` (capabilities LP, QP, Conic), checks, load, parameters, callback, `MSK_optimizetrm`, solution type choice, extraction (x, y, z = s_l − s_u, basis from skx/skc, objective, certificates per research R7), verification, `LoadReport`
- [X] T028 [US3] Make every per-backend test of feature 001 and T014–T020 pass on MOSEK (`cargo test --features gurobi,mosek`)

---

## Phase 6: User Story 4 — MOSEK QP (P4)

- [X] T029 [P] [US4] Derived QP fixture `tests/fixtures/qp/iDopaNeuroC_problem_LP_minNorm.provenance.json` (Q = 10⁻⁴ I on iDopaNeuroC_problem_LP, c = 0, reference objectives from research R12) and per-backend tests: that QP and two hand-built QPs with closed-form optima → Optimal, objective within tolerance, Verified; Gurobi reports Unsupported
- [X] T030 [US4] `src/mosek/load.rs` (QP): `MSK_putqobj` from the lower-triangle CSC (qosubj built, qosubi converted when usize), non-convex Q error surfaced; make T029 pass

---

## Phase 7: User Story 5 — MOSEK CLP (P5)

- [X] T031 [US5] `tools/corpus/extract_vk_clp_fixtures.m` per research R11: stored param + `solveStrategy = 'single'`, `printLevel = 0`, `saveProb = 1` in a temporary folder; `sol.prob`; checks (cone dimensions sum, domain codes, bit-equality with the saved MOSEK problem); write CBF with MOSEK, read back and require bit-for-bit equality (else stop and report); provenance JSON (variational kinetics commit, source SHA-256, MOSEK version and parameters, expected `stat`/`origStat`/objective, statement that this is the `solveCobraCLP` input)
- [X] T032 [US5] Run T031 for iCoreED_problem_1..5 and the heavy iDopaNeuroC instances (log to `results/002-mosek-lp-qp-clp/runs/<UTC>-extract-clp/`)
- [X] T033 [P] [US5] Per-backend tests: each CLP fixture reaches its expected status and, when optimal, verifies (iCoreED in the licensed tier, iDopaNeuroC in the heavy tier); Gurobi reports Unsupported; `solve_into` with an `s` buffer on iCoreED_problem_1 matches `solve`
- [X] T034 [P] [US5] Hand-built conic problems with closed-form optima for every cone type (quadratic, rotated quadratic, primal and dual exponential, primal and dual power, nonnegative, zero, PSD of order 2 and 3), minimise and maximise, each verified; one conic infeasible problem with a verified certificate
- [X] T035 [US5] `src/mosek/load.rs` (conic): `appendafes`, F via `putafefentrylist` per column block through a fixed buffer, `putafegslice`, one domain per distinct cone shape, `appendaccsseq`; `solve.rs`: conic duals s from `getaccdotys` (research R5); make T033–T034 pass

---

## Phase 8: Benchmarks

- [X] T036 Extend `benches/load_solve.rs` with MOSEK scenarios (both index types, N = 5, dedicated processes) and evaluate spec SC-004 for MOSEK; write `specs/002-mosek-lp-qp-clp/benchmark-results.md`

---

## Phase 9: Polish

- [X] T037 In-gate checks: fmt; clippy -D warnings for each feature combination (none, gurobi, mosek, both); tests in every tier including `OPTSOLVER_HEAVY_TESTS=1`; `cargo +1.85.0 test --all-targets --features gurobi,mosek`; feature-combination `cargo check`; log to `results/002-mosek-lp-qp-clp/runs/<UTC>-gate/`
- [X] T038 [P] Update `docs/architecture/components.yaml` (reconcile with the delta), `README.md` (MOSEK, test tiers, Rust 1.85)
- [X] T039 Report files edited, checks run, pass/fail and unverified behaviour
- [X] T040 Implementation receipt `specs/002-mosek-lp-qp-clp/agent-runs/<UTC>-implement-002/implementation-receipt.md`

---

## Dependencies & Execution Order

- Setup (T001–T002) → Foundational (T003–T009) → stories.
- US1 (T010–T013) and US2 (T014–T016) depend only on Foundational; US2 runs on Gurobi first.
- US3 (T017–T028) depends on Foundational; T021 before T026–T027.
- US4 and US5 depend on US3's backend (T022–T027); US5's extraction (T031–T032) can run in parallel with US3.
- Benchmarks after US3; Polish last.

## Parallel Opportunities

- Foundational: T003, T004, T007, T008, T009 in parallel (different files); T005–T006 after T003–T004.
- After Foundational: US1, US2, US3's tests (T017–T020) and US5 extraction (T031–T032) in parallel.
- US3 implementation: ffi/env, params, status, callback in parallel after T022's declarations.

## Implementation Strategy

1. MVP: Setup, Foundational, US1, US2 (no MOSEK needed).
2. US3 MOSEK LP; then US4 QP; then US5 CLP.
3. Benchmarks, polish, Gate 3.
