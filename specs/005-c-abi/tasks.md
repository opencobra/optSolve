# Tasks: A C Interface Over The Solver-Independent API

**Input**: `specs/005-c-abi/` — spec.md (Gate 1 PASS), plan.md, research.md, data-model.md,
contracts/c-interface.md, architecture-delta.yaml (Gate 2A: APPROVE_WITH_ADR, review 8,
`gate-reviews/gate-2A-20261116T004341Z.md`).

**Tests**: both tiers; benchmarks for the C path (Principles II, XIV). Each implementing task
carries its own tests, written first or in the same change; a task is not done until its tests
pass. Licensed Rust tests live in `harness = false` integration tests (`tests/contract.rs` for
Rust-API behaviour, `tests/c_interface.rs` for the C interface) and skip visibly through the shared
registry (an ignored trial naming the missing backend or licence), never by panicking. Header
prototypes use descriptive parameter names (`result`, `caps`, `env`).

**Organisation**: Phase 2 is foundational for every story (the C layer and the shared C test
harness). Each story phase then carries its own independent test and needs only Phase 2.

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

- [ ] T001 `Cargo.toml`: `[lib] crate-type = ["lib", "staticlib", "cdylib"]`; feature `capi = []`;
  `[[test]] name = "c_interface", harness = false, required-features = ["capi"]`; no new bench
  target (research R4, R7). Check: `cargo test --features capi --no-run` produces
  `liboptsolver.a` and `liboptsolver.so` (recorded in the run log) before any C test relies on them.
- [ ] T002 `src/lib.rs`: `#[cfg(feature = "capi")] #[doc(hidden)] pub mod capi;` (research R6).
  `tests/standards.rs`: admit `src/capi/` as an FFI module (definitions only, no extern blocks) and
  extend the interface-document drift test to the header's `optsolver_`/`OPTSOLVER_` names; both
  checks shown to fail on a synthetic violation (XV-C).

## Phase 2: Foundational (blocks every story; each task with its tests)

- [ ] T003 Shared C test harness (owner: test harness). `tests/common/ccompiler.rs` extracted
  from tests/header_conformance.rs:20-75 and tests/header_conformance_mosek.rs:21-66; both header
  tests migrate and still pass. `tests/common/cdata.rs`: problem-file writer for toy instances and
  the iDopaNeuroC LP, result-file reader, compile-and-run of a `tests/c/` program against the
  static or shared library with captured standard output and error. `tests/c/driver.c`: generic C
  driver (owned or buffer path, handle or one-shot, options and raw parameters from the file,
  optional second thread for use and freeing, writes every result field and the error code,
  message and vendor code). `tests/common/compare.rs`: new function implementing the spec's
  comparison rule, with licence-free unit tests on synthetic results: a difference just above the
  bound is rejected; one within it is accepted and reported as not bit-identical; a status, limit
  reason, failed-measure-name or array-presence difference is rejected; an "everything failed"
  case is reported as uninterpretable (XV-C). The driver runner asserts empty standard output and
  error on every run.
- [ ] T004 Header `src/capi/optsolver.h` per contracts/c-interface.md (types with `rows`/`cols`,
  constants, prototypes, ownership, thread and aliasing rules, version rule, panic-profile and
  panic-hook notes, MOSEK creation and freeing unverified, link-flag pointer) together with its
  agreement test in `tests/c_interface.rs`: `tests/c/layout.c` prints `sizeof`/`offsetof` of every
  field and every constant, compared with `optsolver::capi` `size_of`/`offset_of!`/constants;
  prototypes parsed from the header compared with the `src/capi/` definitions through one type
  table; the test shown to fail on an altered copy of the header (FR-013).
- [ ] T011 (foundational; placed here so that every licensed trial runs with the lock) `src/gurobi/env.rs`: private token `Lifecycle` created only by
  `pub(crate) fn lifecycle<T>(f: impl FnOnce(&Lifecycle) -> T) -> T` (one process-wide
  `static Mutex<()>`, poison recovered with `into_inner`); `empty`, `start`, new `free` take
  `&Lifecycle`; `Env::new` one `lifecycle` call (free on failure with the same token); `Drop` one
  call; the unit tests in src/gurobi/env.rs and src/gurobi/params.rs go through it. `unsafe impl
  Send for Env` in src/gurobi/env.rs and src/mosek/env.rs with `SAFETY` citing research R3 and
  the accepted risk; module docs rewritten. Tests: compile-time `Send` assertions;
  `tests/standards.rs` — `GRBemptyenvinternal`, `GRBstartenv`, `GRBfreeenv` named only in
  src/gurobi/env.rs outside `ffi.rs`, with a negative test on synthetic source; licensed
  concurrency trial in `tests/contract.rs` (four threads creating, solving a toy LP with and
  freeing Gurobi environments twenty times concurrently; environments created on one thread, used
  and freed on another) (FR-008, VII-B; evidence for that run only).
- [ ] T005 `src/capi/mod.rs` with its tests: constants (`pub const`), error codes, per-thread last
  error, `guard` (catch_unwind, thread-local in-entry flag, `Error` → code), the silent wrapping
  panic hook (research R4), `optsolver_abi_version`, `optsolver_last_error_message`
  (caller-owned copy) and `optsolver_string_free`, `optsolver_last_error_solver_code`, the name
  functions (from the core's own names, cached), `optsolver_test_panic`,
  `optsolver_environments_created`. Unit tests: version equals the header macros; every `Error`
  variant → its code, `Solver` keeps the vendor code; code-by-code translation of every Rust enum
  (status, limit reason, solution returned, basis, verification overall and verdict, cone kind,
  problem class) to its C constant and back, unknown codes → `""` or UNKNOWN; every name function
  for every code equals the core's name; a panic outside an entry point reaches a previously
  installed hook; `optsolver_test_panic` returns INTERNAL with a message and writes nothing; two
  threads failing with different errors (one a solver error from a fake backend error value) each
  read back their own message and vendor code (FR-010).
- [ ] T006 `src/options.rs`: `memory_limit_gb: Option<f64>` (in `set_shared_fields`, `Debug`,
  per-backend documentation; unit test), needed by the C options mapping. `src/capi/problem.rs`
  with its tests: `optsolver_options_init`; C problem →
  `Problem` (both index widths, both row forms, Q, F/g/cones with `exponents`), C options →
  `Options`; every slice bounded (`len · size ≤ isize::MAX`, `cols + 1` overflow → TOO_LARGE); NULL
  with length 0 → empty slice; built and validated before any environment. Tests: pointer identity
  of every slice (positive control, SC-002, XIV-D); every C-layer check of the contract fails with
  INVALID (or TOO_LARGE) naming the field — NULL with nonzero length, both and neither row forms,
  both and neither index widths with nonzeros, negative `int32` row index, bad sense, backend,
  cone-kind, method, presolve and parameter-type codes, `struct_size` too small, truncated
  options accepted with the rest unset (FR-020), `cols + 1` and nnz overflow, `threads` above
  `u32::MAX`, `random_seed` outside `int32_t`, exponential-cone `dim` not 3 or 0, non-UTF-8 name.
- [ ] T007 `src/capi/env.rs` with its tests: `optsolver_env_create`, `optsolver_env_free`,
  `optsolver_backend_compiled`, `optsolver_capabilities`, `optsolver_env_capabilities`,
  `optsolver_caps_*`, `optsolver_caps_free`; usability from `Env::new`. Tests: not-compiled backend
  → UNSUPPORTED (unlicensed); licensed: capabilities from C equal the Rust `Capabilities`
  (classes, cones, certificates, basis, option names, solver version), usable with empty reason;
  compiled in but not usable: the C driver run as a subprocess with the vendor licence variable
  pointing at a nonexistent file reports not usable with the vendor's message as the reason and
  `optsolver_env_create` fails with OPTSOLVER_ERR_SOLVER (if the vendor library does not honour
  the variable reliably, the behaviour is recorded as unverified).
- [ ] T008 `src/capi/result.rs` with its tests: `optsolver_solve`, `optsolver_solve_once`,
  `optsolver_solve_into` (NaN fill, lengths to `SolutionBuffers`), every `optsolver_result_*`
  accessor (names converted lazily), `optsolver_result_free`. Tests (in-process through
  `optsolver::capi`, licensed where a solve is needed): every accessor on a toy LP equals the Rust
  `Solution` field; absent arrays NULL/0; wrong-length output buffer → INVALID naming the array,
  before any solver call, and every supplied array then holds NaN (FR-005 edge case, FR-006); a raw parameter Gurobi rejects (`TimeLimit` = −1) →
  `OPTSOLVER_ERR_SOLVER` with `optsolver_last_error_solver_code` equal to the vendor code (FR-010).
- [ ] T009 `src/gurobi/solve.rs`: `solve_into` checks the conic-dual buffer length against k, as
  MOSEK does; licensed test in `tests/contract.rs` (written once, every backend): a wrong-length `s`
  is `Error::Invalid` naming `out.s`, no panic.

## Phase 3: User Story 1 — LP and QP from C (P1)

- [ ] T010 [US1] Toy comparison trials in `tests/c_interface.rs`, written once over the shared
  registry: every LP and QP toy instance × backend supporting it × {owned, buffers} × {sense and
  right-hand side, ranges} × {`size_t`, `int32` row indices}, through `tests/c/driver.c` linked to
  the static and to the shared library, compared with the Rust API by the comparison rule under the
  deterministic configuration; options reaching the solver (effective parameters equal), raw
  parameters, parameter conflict, a non-optimal toy (infeasible, unbounded) as a status, and on the
  buffer path its quantities the backend does not return read as NaN (NaN rule); standard output
  and error of the driver empty on every trial (checked by the shared driver runner of T003, so
  every trial of every story checks it) (SC-001, acceptance scenarios 1 to 5, FR-018).

## Phase 4: User Story 2 — zero copy, reuse and threads (P1)

The lock and `Send` change this story relies on is T011 (foundational).

- [ ] T012 [US2] Reuse and thread trials in `tests/c_interface.rs`: the driver solves twenty
  problems through one handle (every toy instance the backend supports, repeated in order until
  twenty; the count per backend recorded) and the same problems one-shot; `optsolver_environments_created`
  increases by one for the handle sequence; results agree by the comparison rule; wall time of the
  sequence and the one-shot solves recorded (informational, N = 1) (SC-004); a handle created on
  one thread is used and freed on another (C driver, `pthread`) (FR-008). The pointer-identity test
  of T006 is this story's zero-copy evidence (acceptance scenario 1).

## Phase 5: User Story 3 — memory limit (P2)

- [ ] T013 [US3] `src/gurobi/params.rs`: `MAPPINGS` entry and `apply` → `SoftMemLimit` (the
  option itself is added in T006). Tests: unit — mapping; licensed, in `tests/contract.rs`: value read
  back from the model, raw `SoftMemLimit` with the option → `ParameterConflict`, raw `MemLimit`
  with it accepted; MOSEK → `Unsupported { field: "memory_limit_gb", backend: "mosek" }` before
  any solver call.
- [ ] T014 [US3] Memory-limit trials (SC-005): Gurobi, `iDopaNeuroC_problem_LP`, one thread,
  `memory_limit_gb` = 1e-6, N = 3, through the Rust API (`tests/contract.rs`) and the C driver
  (`tests/c_interface.rs`) → `StoppedAtLimit`/`Memory` with `SoftMemLimit` in the effective
  parameters; without the limit → Optimal; MOSEK through C → UNSUPPORTED naming the field. If
  Gurobi does not stop, record the result and return to Gate 2A (research R2).

## Phase 6: User Story 4 — conic from C (P2)

- [ ] T015 [US4] Conic toy comparison trials in `tests/c_interface.rs`: every conic toy instance ×
  backend supporting its cone kinds × {owned, buffers}, conic duals included, by the comparison
  rule; an unsupported cone kind → UNSUPPORTED naming it before any solver call (SC-001,
  acceptance scenarios 1 and 2).

## Phase 7: User Story 5 — errors, ownership, versioning (P1)

- [ ] T016 [US5] `tests/c/errors.c` and its trials (unlicensed): version equals header macros;
  every C-layer failing input of T006 from C; every Rust-API invalid input a C caller can express —
  each `expect_invalid` case of `tests/contract.rs` mapped to a C call, or recorded as not
  expressible from C with the reason — returns INVALID (or TOO_LARGE) with a message naming the
  input, and the environment-creation count shows no solver was reached (SC-006);
  `optsolver_test_panic` → INTERNAL, the process continues, standard error empty; free functions
  accept NULL; message copies freed with `optsolver_string_free`; the program runs under
  AddressSanitizer's leak checker with zero leaks (if the sanitizer is unavailable, recorded
  unverified).

## Phase 8: Benchmarks and link demonstration

- [ ] T017 `benches/load_solve.rs`: C-path scenarios under `#[cfg(feature = "capi")]` through
  `optsolver::capi`; heap criterion (SC-002: dhat, process per scenario, N = 3, plus the
  allocation-count comparison as positive control) and wall-time criterion (SC-003: N = 5 warm with
  spread; environment creation and the cold run as their own phases; per-phase timings), both
  backends, `iDopaNeuroC_problem_LP`; output location resolved through `project.yml`
  (`structure.generated` plus the feature name) instead of the hard-coded feature-003 path; record
  effective parameters, versions, tree state, machine, profile.
- [ ] T018 Link demonstration (SC-007): `tests/c/minimal.c`; release build; compile and run with
  the documented flags, static and shared; commands and output in the run bundle.

## Phase 9: Polish and closeout

- [ ] T019 `docs/llm-interface.md`: C interface section (building; compile, link and run-time
  flags; vendor libraries needed at load time and never shipped; every function, type and
  constant; ownership; threads; versioning; panic hook), `memory_limit_gb`, Threading (`Send`).
- [ ] T020 `docs/architecture/components.yaml` reconciled with the delta; `boundaries.yaml` gains
  the delta's forbidden edges; ADR-0004 checked against what was built.
- [ ] T021 In-gate checks: `cargo fmt --check`; clippy `-D warnings` and `cargo test` for every
  feature combination — `""`, `gurobi`, `mosek`, `gurobi,mosek`, `capi`, `capi,gurobi`,
  `capi,mosek`, `capi,gurobi,mosek` (additivity, VIII); doc tests; Rust 1.85 build; extra build
  time of the new crate types measured; all logged to the run bundle.
- [ ] T022 Implementation receipt; `human-loop.md`; closeout items (conic-dual check reported as a
  repaired defect; consumer adoption routed; User Story 3 wording); hand-back facts for the
  consumer.

## Dependencies & Execution Order

Setup (T001–T002) → Foundational (T003, T004, T011, then T005–T009) → each story phase — LP and QP
from C (T010), reuse and threads (T012), memory limit (T013–T014), conic from C (T015), errors and
versioning (T016) — needs only the foundational phase and may run in any order → benchmarks and
link demonstration (T017–T018) → polish and closeout (T019–T022).

## Implementation Strategy

First independently testable slice: Phases 1 and 2 (unlicensed tier green, licensed accessor
tests green). Scope proposed at Gate 2: all tasks.
