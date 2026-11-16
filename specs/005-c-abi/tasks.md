# Tasks: A C Interface Over The Solver-Independent API

**Input**: `specs/005-c-abi/` — spec.md (Gate 1 PASS), plan.md, research.md, data-model.md,
contracts/c-interface.md, architecture-delta.yaml (Gate 2A: APPROVE_WITH_ADR, review 8,
`gate-reviews/gate-2A-20261116T004341Z.md`).

**Tests**: both tiers; benchmarks for the C path (Principles II, XIV). Tests are written before
or with the behaviour they verify. Every new licensed Rust test skips visibly (an ignored trial
naming the missing licence) when the backend's environment cannot be created, rather than
panicking. Header prototypes use descriptive parameter names (`result`, `caps`).

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

- [ ] T001 `Cargo.toml`: `[lib] crate-type = ["lib", "staticlib", "cdylib"]`; feature `capi = []`;
  `[[test]] name = "c_interface", harness = false, required-features = ["capi"]`; no new bench
  target (research R4, R7). Check: `cargo test --features capi --no-run` produces
  `liboptsolver.a` and `liboptsolver.so` in the target directory (recorded in the run log) before
  any C test relies on them.
- [ ] T002 `src/lib.rs`: `#[cfg(feature = "capi")] #[doc(hidden)] pub mod capi;` (research R6;
  symbols exported with `#[unsafe(no_mangle)]`). `tests/standards.rs`: admit `src/capi/` as an FFI module in the
  unsafe/extern test (definitions only, no extern blocks), and add the header's
  `optsolver_`/`OPTSOLVER_` names to the interface-document drift test.

## Phase 2: Foundational (blocks every story)

- [ ] T003 [US3] `src/options.rs`: `memory_limit_gb: Option<f64>`, listed in `set_shared_fields`
  and `Debug`; documented per backend (FR-011).
- [ ] T004 [US3] `src/gurobi/params.rs`: `MAPPINGS` entry `memory_limit_gb` → `SoftMemLimit`;
  `apply` sets it (FR-011, VII-G). Unit tests: applied value read back; raw `SoftMemLimit` with the
  option is `ParameterConflict`; raw `MemLimit` with the option is accepted.
- [ ] T005 [US3] MOSEK rejection test (no code change: absent from `mosek::params::MAPPINGS`):
  unit test in `src/mosek/params.rs` tests that `options::check::check` returns
  `Unsupported { field: "memory_limit_gb", backend: "mosek" }`.
- [ ] T006 [US2] `src/gurobi/env.rs`: private token type `Lifecycle`, constructed only by
  `pub(crate) fn lifecycle<T>(f: impl FnOnce(&Lifecycle) -> T) -> T`, which holds one process-wide
  `static Mutex<()>` (poison recovered with `into_inner`); `empty`, `start` and a new `free` take
  `&Lifecycle`; `Env::new` is one `lifecycle` call (create, `OutputFlag`, start, free on failure
  with the same token); `Drop` is one call; the unit tests in src/gurobi/env.rs and
  src/gurobi/params.rs go through `lifecycle` and `free`. `unsafe impl Send for Env` (Gurobi and
  MOSEK) with `SAFETY` citing research R3 and the accepted risk; module docs of both `env.rs`
  rewritten. `tests/standards.rs`: `GRBemptyenvinternal`, `GRBstartenv`, `GRBfreeenv` named only
  in `src/gurobi/env.rs` outside `ffi.rs`, with a negative test on synthetic source. Compile-time
  `Send` assertions. Licensed concurrency test: four threads each creating, solving a toy LP with,
  and freeing Gurobi environments twenty times concurrently; environments created on one thread
  and solved and freed on another, the freeing thread differing from the creating one (FR-008,
  VII-B; evidence for that run only).
- [ ] T007 `src/gurobi/solve.rs`: `solve_into` checks `out.s` length against k, as MOSEK does;
  unit test with a wrong-length `s` (FR-005 edge case; plan, duplication).
- [ ] T008 Header `src/capi/optsolver.h` per contracts/c-interface.md: every type, constant and
  prototype, ownership and thread rules, version rule, panic-profile note, link flags pointer.
- [ ] T009 `src/capi/mod.rs`: constants (one Rust `pub const` per header constant, reachable as `optsolver::capi::…` for the
  layout test), error
  codes, per-thread last error (`thread_local!` `CString`), `guard` helper wrapping each entry
  point in `catch_unwind` with the thread-local in-entry flag and mapping `Error` → code; the
  silent wrapping panic hook installed once (research R4) with a Rust unit test that a panic outside
  an entry point still reaches a previously installed hook; `optsolver_abi_version`,
  `optsolver_last_error_message` (caller-owned copy), `optsolver_string_free`,
  `optsolver_last_error_solver_code`, name functions (status, limit-reason,
  returned, verification, cone-kind and measure names converted from the core's own names from `ConeKind::name` / `Measure::name` into a process-wide cache),
  `optsolver_test_panic`, `optsolver_environments_created` (atomic counter).

## Phase 3: User Story 5 — errors, ownership, versioning (P1, unlicensed)

### Tests first

- [ ] T010 [US5] `tests/common/ccompiler.rs` extracted from tests/header_conformance.rs:20-75 and
  tests/header_conformance_mosek.rs:21-66 (both migrate, header tests still pass);
  `tests/common/cdata.rs` (shared harness): compile a C program from `tests/c/` through it, `-I src/capi`, linked to the static or shared library Cargo
  built (found from the test executable's directory), run it as a process with captured standard
  output and error; writer of a toy instance in a binary layout and reader of the C driver's
  result file; the comparison rule of the spec's Clarifications as one new function in
  `tests/common/compare.rs`.
- [ ] T011 [US5] `tests/c/layout.c` + agreement test in `tests/c_interface.rs`: prints `sizeof`,
  `offsetof` of every field and every constant; compared with Rust `size_of`/`offset_of!` and
  constants through `optsolver::capi`; prototype check parsing header prototypes against `src/capi/` definitions through one
  type table; the check is shown to fail on an altered copy of the header (FR-013, XV-C).
- [ ] T012 [US5] `tests/c/errors.c` + trials: version equals header macros; every C-layer
  failing input (null required pointer, both and neither row-bound forms, both index widths, bad
  sense, unknown backend, cone kind, method, presolve and parameter-type codes, `struct_size` too
  small, nnz overflow, interior NUL in a parameter name, negative int32 row index) returns
  `OPTSOLVER_ERR_INVALID` (or TOO_LARGE) with a message naming the input, before any solver call;
  representative Rust-API invalid inputs (unsorted row indices, non-finite value, Q above the
  diagonal) likewise; backend not compiled in → UNSUPPORTED; `optsolver_test_panic` → INTERNAL
  and the process continues; free functions accept NULL; a shorter, earlier-size options
  structure (options truncated after `time_limit`) is accepted with the rest unset, and a
  too-small problem or buffers structure is INVALID (FR-020); run under AddressSanitizer's leak checker with zero leaks
  (SC-006); standard output and error empty, including after the forced panic (FR-018); the
  conversion checks of the contract (threads above `u32::MAX`, `random_seed` outside `int32_t`,
  exponential-cone `dim` not 3 or 0, non-UTF-8 parameter name or value) each INVALID; NULL with
  length 0 accepted as empty.

### Implementation

- [ ] T013 [US5] `src/capi/problem.rs`: `optsolver_options_init`; every slice bounded by
  `len · size ≤ isize::MAX` and `n + 1` overflow rejected; the problem is built and validated
  before any environment is created (`solve_once`), so invalid-input tests need no licence;
  buffer aliasing documented as undefined behaviour in the header and `# Safety`; C problem →
  `Problem` (both
  index widths, both row forms, Q, F/g/cones) borrowing every array in place; C options →
  `Options`. Unit test: every slice of the built `Problem` has the caller's address (positive
  control, SC-002, XIV-D).
- [ ] T014 [US5] `src/capi/env.rs`: `optsolver_env_create`, `optsolver_env_free`,
  `optsolver_backend_compiled`, capabilities (`optsolver_capabilities`,
  `optsolver_env_capabilities`, `optsolver_caps_*`, `optsolver_caps_free`), usability from
  `Env::new` (FR-017).
- [ ] T015 [US5] `src/capi/result.rs`: `optsolver_solve`, `optsolver_solve_once`,
  `optsolver_solve_into` (NaN fill, lengths passed to `SolutionBuffers`), every
  `optsolver_result_*` accessor (names converted to NUL-terminated strings lazily on first read),
  `optsolver_result_free` (FR-005 to FR-007).

## Phase 4: User Story 1 — LP and QP from C (P1, licensed)

- [ ] T016 [US1] `tests/c/driver.c`: reads a problem file, solves through the C interface (owned
  or buffer path, one-shot or handle, options from the file), writes every result field.
- [ ] T017 [US1] Toy comparison trials in `tests/c_interface.rs`: every LP and QP toy instance ×
  backend supporting it × {owned, buffers} × LP in both row-bound forms and both index widths,
  compared with the Rust API by the shared rule under the deterministic configuration; options
  reaching the solver (effective parameters equal), raw parameters, parameter conflict
  (SC-001, acceptance scenarios 1 to 5); "everything failed" case reported as uninterpretable.
  Capabilities (FR-017): from C, per compiled backend, usable, reason, problem classes, cone kinds,
  certificates, basis, option names and solver version equal the Rust `Capabilities` of the same
  backend; a backend not compiled in gives UNSUPPORTED (unlicensed, in T012).

## Phase 5: User Story 2 — zero copy and environment reuse (P1, licensed)

- [ ] T018 [US2] Reuse trial: the driver solves every toy instance a backend supports through one
  handle and one-shot; `optsolver_environments_created` increases by one for the sequence;
  results agree (SC-004). Thread trial: handle created on one thread, used on another (C driver
  with `pthread`, Rust API with `std::thread`) (FR-008).

## Phase 6: User Story 3 — memory limit (P2, licensed)

- [ ] T019 [US3] Gurobi: `iDopaNeuroC_problem_LP`, one thread, `memory_limit_gb` = 1e-6, N = 3,
  through the Rust API and the C driver → `StoppedAtLimit`/`Memory`, effective parameters list
  `SoftMemLimit`; without the limit → Optimal. MOSEK: `Unsupported` naming the field (SC-005).
  If Gurobi does not stop, record the result and return to Gate 2A (research R2).

## Phase 7: User Story 4 — conic from C (P2, licensed)

- [ ] T020 [US4] Conic toy comparison trials (both backends where supported; unsupported cone kind
  → UNSUPPORTED naming it before solving), conic duals included.

## Phase 8: Benchmarks and link demonstration

- [ ] T021 `benches/load_solve.rs` extended with C-path scenarios through `optsolver::capi`: SC-002 (dhat heap, C path versus Rust path, process per
  scenario, N = 3) and SC-003 (end-to-end wall time, N = 5 warm, spread; per-phase timings), both
  backends, `iDopaNeuroC_problem_LP`; environment creation and the cold run recorded as their own
  phases, separate from the warm runs (XIV-C); run bundle under the generated-output home for 005 (resolved through `project.yml`, `structure.generated`
  plus the feature name, replacing the hard-coded feature-003 path at benches/load_solve.rs:630;
  C-path scenarios under `#[cfg(feature = "capi")]`), with
  effective parameters, versions, tree state, machine, profile.
- [ ] T022 SC-007: `tests/c/minimal.c`; release build; compile and run static and shared with
  the documented flags; commands and output in `results/005-c-abi/runs/<UTC>-link/`.

## Phase 9: Polish

- [ ] T023 `docs/llm-interface.md`: C interface section (build, flags, every function, type and
  constant, ownership, threads, versioning), `memory_limit_gb`, Threading section (`Send`).
- [ ] T024 `docs/architecture/components.yaml` reconciled; ADR-0004 status Accepted; the delta's
  forbidden edges considered for `boundaries.yaml`.
- [ ] T025 In-gate checks: fmt; clippy -D warnings for `""`, `capi`, `capi,gurobi`, `capi,mosek`,
  `capi,gurobi,mosek`; tests in every tier; doc tests; Rust 1.85 build; logged to
  `results/005-c-abi/runs/<UTC>-gate/`.
- [ ] T026 Implementation receipt; human-loop.md; hand-back facts for the consumer.

## Dependencies & Execution Order

T001–T002 → T003–T009 → US5 (T010–T015) → US1 (T016–T017) → US2, US3, US4 (T018–T020, any
order) → T021–T022 → T023–T026. Tests in each phase precede or accompany its implementation.

## Implementation Strategy

First independently testable slice: Phases 1–3 (unlicensed tier green). Approved scope proposed
at Gate 2: all tasks.
