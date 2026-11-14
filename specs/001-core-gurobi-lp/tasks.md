---
description: "Task list for feature 001: solver-independent core and Gurobi LP backend"
---

# Tasks: Solver-Independent Core And Gurobi LP Backend

**Input**: `specs/001-core-gurobi-lp/` — plan.md, spec.md, research.md (R1–R15), data-model.md,
contracts/public-api.md, quickstart.md, architecture-delta.yaml (approved: APPROVE_WITH_ADR,
2026-11-14, ADR-0001)

**Tests**: REQUIRED (constitution II, XIV). Tests are written before the code they test and must
fail first. Every solver test is written once against the `Backend` trait in the shared test
module `tests/common/` and registered per backend (R14).

**Scope**: all code below (Rust, C conformance file, MATLAB extraction script) is inside this
feature under constitution VI.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1 corpus and harness; US2 contract and verification; US3 Gurobi backend;
  US4 benchmarks

## Path Conventions

Single crate at the repository root: `src/`, `tests/`, `benches/`, `tools/`, as stated in
`docs/architecture/repo_folder_organisation.md`.

---

## Phase 1: Setup

- [ ] T001 Create `Cargo.toml` for crate `optsolver` (edition 2024, `rust-version = "1.85"`, licence `MIT OR Apache-2.0`, feature `gurobi`, dependency `log`, dev-dependencies `libtest-mimic` and `cc`, `[[test]] harness = false` entries for `contract` and `corpus`, `[lints]` per constitution VII-F) and add `LICENSE-MIT`, `LICENSE-APACHE` at the repository root
- [ ] T002 Create `src/lib.rs` with module declarations (`problem`, `options`, `result`, `verify`, `backend`, `error`, `gurobi` behind `#[cfg(feature = "gurobi")]`), the 64-bit compile-time assertion, and `#![deny(unsafe_code)]` at crate level; each core module file (`problem.rs`, `options.rs`, `result.rs`, `verify.rs`, `backend.rs`, `error.rs`) carries `#![forbid(unsafe_code)]` (constitution VII-A; `forbid` cannot be relaxed below, so it is not used at crate level), and only `src/gurobi/` modules that call through `ffi` carry `#![allow(unsafe_code)]`
- [ ] T003 [P] Record the licences of `log`, `libtest-mimic` and `cc` (and their transitive dependencies) as they are added, with `cargo tree` output, in `specs/001-core-gurobi-lp/research.md` under a new section "R16. Dependency licences" (constitution X)
- [ ] T004 [P] Create `build.rs`: when feature `gurobi` is enabled, read `GUROBI_HOME` (declare `cargo:rerun-if-env-changed=GUROBI_HOME`), emit `rustc-link-search` for `$GUROBI_HOME/lib` and `rustc-link-lib=dylib=gurobi130` derived from the version in `.specify/memory/project.yml` `domain.solvers.gurobi`; add `links = "gurobi"` in `Cargo.toml`; embed no absolute path in any committed file

---

## Phase 2: Foundational (blocks every user story)

- [ ] T005 Implement `Error` (variants per contracts/public-api.md) in `src/error.rs`
- [ ] T006 Implement `RowIndex` (sealed; `usize`, `i32`), `CscRef`, `RowBounds`, `Sense`, `Problem` type definitions (fields `A`, `c`, `gamma`, `sense`, `rows`, `l`, `u`, reserved `var_types`, `Q`; `#[allow(non_snake_case)]` on these types only) in `src/problem.rs` — constructors are stubs returning `Error::Invalid` until US2
- [ ] T007 Implement `Options`, `Method`, `Presolve`, `ParamValue`, `Progress` and the defaults module (the single declaration point of optSolver defaults: `solver_log = false` with its rationale, constitution VII-G) in `src/options.rs`
- [ ] T008 Implement `Solution`, `SolutionBuffers`, `SolutionView`, `Status`, `LimitReason`, `SolutionReturned`, `Basis`, `Counts`, `Timings`, `Certificate`, `SolverIdentity` in `src/result.rs` (data-model.md, Result and Status vocabulary)
- [ ] T009 Implement the provisional `Backend` trait and `Capabilities` in `src/backend.rs` (contracts/public-api.md; doc comment states it is provisional per ADR-0001)
- [ ] T010 Create the shared test module skeleton `tests/common/mod.rs` with the backend registry (compiled-in backends via `cfg(feature)`), `Trial` construction through `libtest-mimic`, and run-time availability check producing ignored trials with reason (research R14)
- [ ] T010a Shared test harness (Principle II): add to `tests/common/mod.rs` the helpers every test must use — `for_each_backend(name, |backend| …)`, assertion helpers for statuses and verification verdicts — and a test in `tests/contract.rs` that, with no backend compiled, every backend-dependent trial is reported ignored, never passed (SC-002)
- [ ] T010b [P] Verify Rust and FFI standards (Principle VII): add `tests/standards.rs` checking that `unsafe` and `extern` appear only in `src/gurobi/`, that `extern "C"` declarations appear only in `src/gurobi/ffi.rs`, and that `cargo clippy --all-targets --all-features -- -D warnings` passes (run in gate)
- [ ] T010c Solver parameter location check (Principle VII-G): in `tests/standards.rs`, fail if `GRBsetintparam`, `GRBsetdblparam` or `GRBsetstrparam` appear in any file other than `src/gurobi/params.rs` and `src/gurobi/ffi.rs`

**Checkpoint**: types compile; harness reports every backend trial as ignored.

---

## Phase 3: User Story 1 — Frozen test corpus and reusable harness (Priority: P1) 🎯 MVP

**Goal**: seven iDopaNeuroC MPS fixtures with provenance, seeded objectives, and the shared
harness running corpus tests per backend.

**Independent Test**: with no solver, fixtures load, provenance hashes match, objectives
regenerate identically, backend trials are ignored with reason.

### Tests for User Story 1 (write first, must fail)

- [ ] T011 [P] [US1] Test in `tests/corpus.rs`: every instance listed in `project.yml` `test_corpus.instances` has an MPS file and a provenance JSON in `tests/fixtures/`, every required provenance field is present (data-model.md, Fixture provenance record), and the recorded MPS SHA-256 matches the file
- [ ] T012 [P] [US1] Test in `tests/corpus.rs`: each fixture parses with the shared MPS reader into dimensions m, n, nnz equal to its provenance record, all rows equality, objective sense maximise
- [ ] T013 [P] [US1] Test in `tests/corpus.rs`: SplitMix64 objective for each recorded seed is identical across two generations and matches a stored reference checksum for seed 1 of `iDopaNeuroC_problem_2`
- [ ] T014 [P] [US1] Test in `tests/common/mps.rs` unit tests: the MPS reader handles `OBJSENSE`, `ROWS`, `COLUMNS`, `RHS`, `RANGES`, `BOUNDS` (UP, LO, FX, FR, MI, PL) on small hand-written strings, and rejects malformed input with a message

### Implementation for User Story 1

- [ ] T015 [US1] Write `tools/corpus/extract_idopaneuroc_fixtures.m`: read instance names from `.specify/memory/project.yml` (no restated list), locate the corpus with the variational kinetics `vkProblemInstanceDir` (path to that repository taken from an environment variable, never stored), load each `model`, build a Gurobi model structure (A = S, rhs = b, sense from csense, lb, ub, obj = c, modelsense from osenseStr), write `tests/fixtures/<instance>.mps` with `gurobi_write`, read it back with `gurobi_read` and require bit-for-bit equality of every array, then write `tests/fixtures/<instance>.provenance.json` (fields per data-model.md; licence "not verified (maintainer decision 2026-11-14)"; seeds 1, 2, 3)
- [ ] T016 [US1] Run the extraction script in MATLAB (output redirected to `results/001-core-gurobi-lp/runs/<UTC>-extract/log.txt`), inspect the first written MPS file's sections and record the observed MPS conventions in research.md R11 (closing its **verify** note)
- [ ] T017 [US1] Implement the shared MPS reader in `tests/common/mps.rs` (sections per T014; produces CSC with `usize` indices and sense + b rows)
- [ ] T018 [US1] Implement SplitMix64 and `seeded_objective(seed, n)` in `tests/common/objectives.rs` (research R13)
- [ ] T019 [US1] Implement provenance loading and hash checking in `tests/common/fixtures.rs`; make T011–T014 pass

**Checkpoint**: `cargo test` passes on a machine without Gurobi; corpus solve trials listed as ignored.

---

## Phase 4: User Story 2 — Solver-independent contract and verification (Priority: P2)

**Goal**: validated borrowed problems, options rules, and the verifier, all without a solver.

**Independent Test**: invalid problems rejected with named defects; verifier reports correct
measures on hand-built solutions.

### Tests for User Story 2 (write first, must fail)

- [ ] T020 [P] [US2] Tests in `tests/contract.rs`: one failing input per validation rule of FR-006 and data-model.md (lengths, decreasing pointers, out-of-range, unsorted, duplicate indices, non-finite A or c, NaN bounds, invalid infinite bounds in both row forms, dimension > 2^31 − 1 via `TooLarge`), each asserting the error names the defect
- [ ] T021 [P] [US2] Test in `tests/contract.rs`: a problem with `i32` indices exposes the caller's slice pointer unchanged (no copy), and with `usize` indices the conversion iterator yields blocks of at most 2^18 entries
- [ ] T022 [P] [US2] Verifier tests in `tests/contract.rs` on hand-built LPs: known row and bound violations (absolute and relative values), reduced-cost inconsistency, dual sign violations for minimise and maximise, complementary slackness, primal-dual gap including γ, Farkas certificate and unbounded ray checks, NotEvaluated when duals are absent, overall verdicts Verified / Failed / PartiallyVerified
- [ ] T023 [P] [US2] Guard self-test (Principle XV-C) in `tests/contract.rs`: the verifier reports Failed for an all-zero "everything failed" solution on a corpus fixture, and for a solution perturbed by 10 × tolerance in one row
- [ ] T024 [P] [US2] Options tests in `tests/contract.rs`: unsupported field for a backend's capabilities is `Unsupported`; a shared field and a raw parameter setting the same vendor parameter is `ParameterConflict` (using a test backend in `tests/common/` that implements `Backend` with fixed capabilities)

### Implementation for User Story 2

- [ ] T025 [US2] Implement `CscRef::new`, `new_unchecked` and `Problem::new` validation (single pass, fused range check) in `src/problem.rs`; implement the `usize` → `i32` block conversion iterator with a reusable fixed buffer
- [ ] T026 [US2] Implement canonical row-bound views (sense + b and ranges → b_l, b_u without copying) in `src/problem.rs`
- [ ] T027 [US2] Implement `verify` and `VerificationReport` in `src/verify.rs` per data-model.md, Verification (computing Ax and Aᵀy itself)
- [ ] T028 [US2] Implement capability and options checks shared by all backends (unsupported fields, double-route conflicts) in `src/options.rs`
- [ ] T029 [US2] Make T020–T024 pass

**Checkpoint**: contract and verification complete without any solver.

---

## Phase 5: User Story 3 — Solve with Gurobi (Priority: P3)

**Goal**: the Gurobi backend passing every corpus test through the harness.

**Independent Test**: with Gurobi and a licence, all corpus trials pass; without, ignored.

### Tests for User Story 3 (write first, must fail)

- [ ] T030 [P] [US3] Corpus solve test in `tests/corpus.rs`, registered per backend: each of the 7 instances × seeds 1, 2, 3 with Method DualSimplex, Threads 1, Seed 0 → status Optimal, returned Optimal, verification Verified (SC-003, N = 1 per solve); plus one solve per instance with empty options
- [ ] T031 [P] [US3] Small hand-built fixtures in `tests/fixtures/small/` (infeasible, unbounded, ranged rows, free row, empty column, zero rows, zero columns) and per-backend tests in `tests/corpus.rs`: statuses Infeasible and Unbounded with certificates requested and verified; ranged-row problem Optimal with x of length n (auxiliary columns removed)
- [ ] T032 [P] [US3] Per-backend test in `tests/corpus.rs`: time limit 0 → StoppedAtLimit with reason Time and the solution-returned field consistent with the solver
- [ ] T033 [P] [US3] Per-backend test in `tests/corpus.rs`: a raw parameter (`Seed` = 7) appears unchanged in `effective_parameters`; the solver log is off by default (nothing written to stdout or stderr during the solve)
- [ ] T034 [P] [US3] Per-backend test in `tests/corpus.rs`: cancellation flag set from the progress callback after its first call gives Interrupted; progress records are solver-independent; a panic in the callback is resumed after the solve without crashing the process
- [ ] T035 [P] [US3] Per-backend test: one environment solves three problems in sequence (licence check counted once by the test via `Env` construction count)
- [ ] T036 [P] [US3] Backend-specific test `src/gurobi/status.rs` unit tests: codes 1–19 map per research R4 and code 99 maps to Unknown with raw code kept; no licence needed (SC-006; outside the contract, so not in the shared harness — stated in the test's doc comment)
- [ ] T037 [P] [US3] Header conformance test `tests/header_conformance.rs` with `tests/header_conformance.c`: compiles against `$GUROBI_HOME/include/gurobi_c.h` and checks every declared function's type and every constant used (research R12); skipped visibly when `GUROBI_HOME` is unset

### Implementation for User Story 3

- [ ] T038 [US3] Before writing FFI calls, verify in the Gurobi 13.0 reference manual that every array argument of `GRBXloadmodel` and `GRBXaddvars` is input-only, that `InfUnbdInfo` and `DualReductions` behave as research R5/R6 assume, that `RC = c − Aᵀ Pi` holds for both objective senses, and Gurobi's determinism conditions (R10); record each finding with date and source in research.md, replacing its **verify** marker
- [ ] T039 [US3] Declare the Gurobi C functions and constants used (about 25; design-notes §9, research R2–R7) in `src/gurobi/ffi.rs` with opaque handle types; this is the only module with `extern "C"`
- [ ] T040 [US3] Implement `Env` (create via `GRBloadenvinternal` with numbers from `GRBversion`, `Drop` → `GRBfreeenv`, error text via `GRBgeterrormsg`) in `src/gurobi/env.rs`; `!Send`, `!Sync`
- [ ] T041 [US3] Implement `gurobi::params::apply` in `src/gurobi/params.rs`: the single function applying shared fields (research R5) and typed raw parameters through `GRBgetparamtype` and the setters; returns the effective parameter set; reads back effective tolerances for verification
- [ ] T042 [US3] Implement loading in `src/gurobi/load.rs`: `i32` path via one `GRBXloadmodel`; `usize` path via `GRBXloadmodel` with zero columns plus blocked `GRBXaddvars` through the fixed buffer; sense conversion for ranges; auxiliary columns for finite ranged rows; ±∞ → ±`GRB_INFINITY` only when present; record which path ran (for the US4 positive control)
- [ ] T043 [US3] Implement `gurobi::status::translate` in `src/gurobi/status.rs` (research R4) and the solution-returned rule from `SolCount`
- [ ] T044 [US3] Implement the callback in `src/gurobi/callback.rs` (research R7): relaxed load of the flag, `GRBterminate`, progress records, `catch_unwind` with panic resumed after `GRBoptimize`
- [ ] T045 [US3] Implement `Backend for Env` in `src/gurobi/solve.rs`: capabilities (LP only), validation and capability checks, load, apply parameters, optimise, extract all available outputs with `GRBgetdblattrarray` / `GRBgetintattrarray` into owned vectors or caller buffers (dropping auxiliary columns), attributes (`Runtime`, `Work`, `MaxMemUsed`, `IterCount`, `BarIterCount`), raw status, solver identity, timings, verification on by default; free the model in all paths
- [ ] T046 [US3] Make T030–T037 pass with `GUROBI_HOME` set and a licence present; record the run in `results/001-core-gurobi-lp/runs/<UTC>-tests/`

**Checkpoint**: licensed tier green; unlicensed tier still green.

---

## Phase 6: User Story 4 — Benchmarks (Priority: P4)

**Goal**: per-instance phase timings and memory, recorded as the baseline.

**Independent Test**: a run bundle per instance with every required field.

### Tests for User Story 4 (write first, must fail)

- [ ] T047 [P] [US4] Test in `tests/corpus.rs` for the benchmark record writer: a record missing any field required by plan.md (instance, N, medians and spreads per phase, Rust heap peak, VmHWM rise, `MaxMemUsed`, effective parameters, Gurobi version, commit, machine, profile, load path) is refused (Principle XV-C)

### Implementation for User Story 4

- [ ] T048 [US4] Implement `benches/load_solve.rs` (harness = false): counting global allocator (benchmark binary only), one instance per process (the binary re-executes itself per instance), N = 5, phases timed separately, cold and warm labelled, positive control printing the load path and buffer high-water mark before any comparison (Principle XIV-D), records written to `results/001-core-gurobi-lp/runs/<UTC>-bench/`
- [ ] T049 [US4] Run the benchmark for both the `usize` and `i32` index paths, evaluate SC-004 and SC-005 per instance, and write the results with their N and scope in `specs/001-core-gurobi-lp/benchmark-results.md`

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T050 Run the in-gate checks (constitution, Development Workflow And Quality Gates): `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `cargo test --features gurobi`, feature-combination check (`cargo check --no-default-features` and `--features gurobi`), and an MSRV build with Rust 1.85 if installed (otherwise record as unverified)
- [ ] T051 [P] Update `docs/architecture/components.yaml` with the components actually implemented (reconciled against `architecture-delta.yaml` at closeout, constitution XIII-D)
- [ ] T052 [P] Write `README.md` (Tier 3: what the crate does, how to build with and without Gurobi, link to quickstart.md)
- [ ] T053 Report files edited, checks run, pass/fail results and unverified behaviour
- [ ] T054 Create the implementation receipt `specs/001-core-gurobi-lp/agent-runs/<UTC-timestamp>-implement-001/implementation-receipt.md` with the actual final user-facing completion response verbatim

---

## Dependencies & Execution Order

- Setup (T001–T004) → Foundational (T005–T010c) → user stories.
- **US1** (T011–T019) depends only on Foundational; it is the MVP.
- **US2** (T020–T029) depends on Foundational; independent of US1 except T023, which uses a
  corpus fixture (after T016).
- **US3** (T030–T046) depends on US1 (fixtures, harness) and US2 (validation, verification).
- **US4** (T047–T049) depends on US3.
- Polish after all stories.

Within each story: tests first and failing, then implementation; T038 before T039–T045.

## Parallel Opportunities

- Setup: T003, T004 alongside T001–T002.
- US1: T011–T014 together; T017, T018 together after T015–T016.
- US2: T020–T024 together.
- US3: T030–T037 together; T040–T044 are separate files and can proceed in parallel after T039.

## Implementation Strategy

1. **MVP**: Setup, Foundational, US1 — the frozen corpus and harness, testable with no solver.
2. Add US2 — the full contract, still without a solver.
3. Add US3 — Gurobi; licensed tier.
4. Add US4 — baseline benchmarks.
5. Polish and closeout (Gate 3).
