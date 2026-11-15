# Implementation Plan: A C Interface Over The Solver-Independent API

**Branch**: `005-c-abi` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/005-c-abi/spec.md`

## Summary

Add a C interface — header `src/capi/optsolver.h` and the static and shared libraries
`liboptsolver.a`, `liboptsolver.so` — over the existing solver-independent Rust API, so the COBRA
Toolbox MEX gateway (and any C caller) can solve LP, QP and conic problems on Gurobi and MOSEK with
borrowed CSC input, reusable environment handles that may move between threads (the Rust `Env` types become `Send`, not `Sync`, with Gurobi
environment creation serialised), error codes with per-thread messages, contained
panics, explicit ownership, a capabilities query and a versioned ABI (contract:
`contracts/c-interface.md`). Add the solver-independent option `memory_limit_gb` to the Rust API
(Gurobi `SoftMemLimit`; unsupported on MOSEK). Test with C programs compiled against the header and
compared with the Rust API through the shared test harness (research R6, R7).

## Technical Context

**Language/Version**: Rust 2024, MSRV 1.85 (unchanged); C11 for the header and C tests

**Primary Dependencies**: none new at run time; dev: existing `cc` (compiler discovery), `dhat`
(heap profile), `libtest-mimic`

**Storage**: N/A

**Testing**: `cargo test` — unlicensed tier everywhere (`--features capi`), licensed tier with
`--features capi,gurobi,mosek` where licences are present

**Target Platform**: 64-bit targets the crate supports; measured on the reference machine (x86_64
Linux)

**Project Type**: library

**Performance Goals**: SC-002 (≤ 4 KiB extra heap), SC-003 (≤ max(1 %, 1 ms) extra wall time)

**Constraints**: zero copies of input arrays (FR-003); no allocation proportional to nnz; O(m + n + k)
NaN writes on the caller-buffer path only; no printing (III)

**Scale/Scope**: about 45 C functions; one new module; header ~450 lines

## Constitution Check

- **Development scope** (VI): every file in Project Structure below, including the C test
  programs, the binary toy-instance writer, the benchmark and the header, is in scope; no
  `SPECKIT OVERRIDE`. A licensed probe of `SoftMemLimit` per model (research R2) is a test task,
  not a scratch script.
- **Performance and memory** (I): input path — no copy; the C layer allocates only the cone
  descriptor vector (linear in the number of cones, `num_cones`, with power-cone exponents) and the raw-parameter name
  strings (O(number of raw parameters)). Result path — owned arrays are read in place; per-result
  NUL-terminated copies of names only, made lazily on first read (O(number of names), nothing
  unless read); caller buffers are NaN-filled
  (O(m + n + k) writes, no allocation). No allocation proportional to nnz. Environment reuse is
  the point of the handle (US2); model modification without reload does not exist in the Rust API
  yet (spec Assumptions) and is not touched. Validation is not removed. Targets: the heap criterion (SC-002) and the wall-time criterion
  (SC-003).
- **Solver configuration audit** (I): research R8 — the only new parameter is `SoftMemLimit`,
  unset by default (vendor default Infinity, 13.0 parameter reference); the problem data passed is
  unchanged, so feature 001–003 audits stand.
- **Solver parameters in one location** (VII-G): `memory_limit_gb` is declared in
  `src/options.rs` (`Options`, no default override; `set_shared_fields`); applied only in
  `gurobi::params::apply` (`SoftMemLimit`), listed in `gurobi::params::MAPPINGS` so the conflict
  rule covers raw `SoftMemLimit`; absent from `mosek::params::MAPPINGS` (unsupported, research
  R1); recorded in the effective parameters by the existing recorder. The C layer sets no
  parameter (forbidden edge in the delta; `tests/standards.rs` setter test unchanged).
- **Zero-copy inputs and bulk calls** (IV): every input array (A's column pointers, row indices
  as `size_t` or `int32_t`, values; c, l, u; row sense and b or b_l and b_u; Q; F, g) is borrowed
  as is through `slice::from_raw_parts`; backend routes unchanged (optSolver's own bindings,
  features 001–002). Positive control: a unit test asserts every slice of the built `Problem`
  has the caller's address (XIV-D).
- **Solver-independent contract** (XVI): (a) the C layer passes the generic `Problem`/`Options`
  to `Backend` and exposes the generic `Solution`; (b) status translation untouched
  (`gurobi::status::translate`, `mosek::status::translate`; Gurobi `MEM_LIMIT` already maps to
  `StoppedAtLimit`/`Memory`); (c) `memory_limit_gb` is a shared field, documented per backend;
  MOSEK rejects it by the existing rule; (d) verification unchanged and reported through C; every
  licensed C test compares the verification verdict with the Rust API's.
- **Rust and FFI standards** (VII): `unsafe` only in `src/capi/` (an FFI module, `#![allow(unsafe_code)]`),
  every block with `// SAFETY:` and every `pub unsafe extern "C" fn` with `# Safety`; handles are
  `Box`ed Rust values behind opaque pointers, freed only by their free function; `Env` lifetimes
  unchanged; `unsafe impl Send` (not `Sync`) for `gurobi::Env` and `mosek::Env`, each citing the
  vendor documentation (research R3, superseding feature 001 R15 and feature 002 R2 as recorded
  in the spec's Clarifications); Gurobi environment and model creation and freeing under one lock in `gurobi::env`
  (`Env::new`, `Drop`, used by `gurobi::load`), the lock's sufficiency an accepted, recorded risk
  (research R3); errors as codes, panics contained with
  `catch_unwind` at every entry point (VII-C); validation before the first FFI call is the Rust
  API's, plus the C layer's null and length checks before building slices (VII-D); no vendor
  declaration added (VII-E); lints unchanged, `links` unchanged, no new build-script variable
  (VII-F). Exported symbols use `#[unsafe(no_mangle)]` (edition 2024).
- **Independence and additive features** (VIII): no RunMat dependency (the consumer links us);
  `capi` is additive and changes no Rust behaviour; the core builds and tests with no vendor
  software; feature combinations `capi`, `capi,gurobi`, `capi,mosek`, `capi,gurobi,mosek` checked.
- **Testing and behavioural evidence** (II): the C toy comparison is one trial per toy instance ×
  backend, generated from the shared harness (`tests/common/toy.rs`, `compare.rs`) and run
  unchanged on every backend supporting the instance; fixtures are the feature-003 toy instances
  with their provenance records (no new fixtures); `iDopaNeuroC_problem_LP` for benchmarks.
  Narrowest unlicensed: `cargo test --features capi --test c_interface -- unlicensed`; licensed:
  `-- toy`. Comparison trials use the spec's deterministic configuration (threads = 1; dual
  simplex for LP, barrier for QP and conic) on both paths, and the spec's tolerance rule,
  implemented once in the shared test module (`tests/common/cdata.rs`). Failing-input tests: every C-layer check (null pointer, both or neither row form,
  int32 and size_t index both given, bad sense, bad cone kind, bad parameter type, nnz overflow)
  plus representative Rust-API invalid inputs through C; also a structure-size prefix too small
  for the fields the library requires, an unknown backend, method, presolve or parameter-type
  code, and a too-small output buffer. Thread rule (FR-008): a compile-time assertion that both
  `Env` types are `Send`, and a licensed test creating an environment on one thread and solving
  with it on another (Rust API and C driver). No output (FR-018): the C driver's standard output
  and standard error are captured and must be empty on every trial. A real caller reaching the path: the C
  programs are real callers linked against the built libraries (II, behavioural integration).
- **Diagnostics and statuses** (III): the C layer prints nothing; messages are returned as data;
  raw statuses exposed; vendor code via `optsolver_last_error_solver_code`.
- **Capabilities usability** (FR-017; Gate 1 observation): `Capabilities` has no "usable" field;
  usability is derived in the C layer from `Env::new` — success is usable, an error is not usable
  with the error's message as the reason — so no Rust type changes.
- **Panic containment and the build profile** (VII-C, VII-F; Gate 1 observation): containment
  needs unwinding, which is Rust's default; the crate sets no profile and does not rely on one.
  The header documents that a library built with `panic = "abort"` aborts instead of returning
  the internal-error code, and the test asserts the internal-error code under the default
  profile.
- **Benchmarks and evidentiary preconditions** (XIV): (a) `iDopaNeuroC_problem_LP` (LP,
  m = 1344, n = 2942, nnz = 13748, provenance `tests/fixtures/` record, feature 001), seeded
  objective; criteria: the heap and wall-time criteria (SC-002, SC-003) per backend. (b) Each run bundle records the effective
  parameters, solver versions, optSolver tree state (commit plus content hash of the uncommitted
  tree), machine and profile. Both paths use identical instances and options. (c) Environment
  creation outside the timed region; the result's load, solve, extraction timings recorded for both
  paths; warm runs only for SC-003 (first run discarded); heap measured with `dhat` in a process
  per scenario. (d) Positive control: the pointer-identity test (task list) and a dhat check that
  the C path's allocation count differs from the Rust path's by the expected small constant.
- **Version mapping for the consumer** (versioning requirement FR-015; Gate 1 observation): the
  research record's points carried from Gate 1 (R10).
- **Reporting discipline** (XV): N and spread stated per figure (heap criterion SC-002: N = 3; wall-time criterion SC-003:
  N = 5);
  the agreement test is exercised against an altered header and the comparison against an
  "everything failed" case (a C result file reporting no results is reported as uninterpretable,
  not agreement); run output to `results/005-c-abi/runs/<UTC>-<label>/` with its log. No
  repository-wide rule widened except `tests/standards.rs` admitting `src/capi/` as an FFI module
  (earlier decision searched: the BACKENDS list in `tests/standards.rs`, constitution VII-A
  "confined to FFI modules" — admitted, not reversed).
- **Notation** (XII-C): C field names use the inventory symbols (`A`, `c`, `l`, `u`, `b`, `b_l`,
  `b_u`, `Q`, `F`, `g`, `x`, `y`, `z`, `z_l`, `z_u`, `s`, `gamma`, `m`, `n`, `k`); no new symbol.
- **Provenance and licensing** (X): no new crate or fixture; vendor documentation cited in
  research with dates; no vendor header or library copied; the header is optSolver's own (MIT OR
  Apache-2.0).
- **Project parameters** (Governance): the build reads the declared solver versions as before; no
  new parameter; no absolute path (the C tests find the built library from the test executable's
  location, IX-I).
- **Spec-driven scope control** (V): paths in Project Structure only; no retired path; no new
  dependency. The two test-support entry points (`optsolver_environments_created`,
  `optsolver_test_panic`) ship in every `capi` build: they change no solving behaviour, the count
  also lets a caller confirm its own environment reuse, and a test-only build variant would mean
  the shipped library is not the tested one.
- **Artifact placement** (IX): three roles missing from the placement document — public C header
  (`src/capi/`), C test sources (`tests/c/`), incoming cross-repository requests
  (`specs/<feature>/requests/`) — are added to `docs/architecture/repo_folder_organisation.md` in
  this feature before any such file is created (IX-F), each under a location `project.yml`
  already declares, so `project.yml` is unchanged (research R8); a record of the COBRA request REQ-001 (origin, date,
  pointer; its text is quoted in the spec's Input, not copied) is then placed in its new home. Run output under the generated-output home.
- **Architecture conformance** (XIII): `architecture-delta.yaml` declares the new `capi`
  component, forbidden edges (core and backends never depend on it; it never calls vendor
  parameter setters or declarations); ADR-0004 required (new layer, new crate types, external
  convention); names decided in the delta (XIII-F) for the Gate 2A review.

## Architecture Impact

**Current architecture**: solver-neutral core (`core.*`) with backends `gurobi.*`, `mosek.*`
behind features, all in crate `optsolver` (components.yaml); no C-exported symbols; the founding
design's RunMat adapter lives outside the repository.

**Proposed delta**: see `architecture-delta.yaml` — added `capi` (src/capi/), `capi.header`
(src/capi/optsolver.h), `test.c_interface`, `bench.c_interface`; modified `core.options`, `gurobi.env` (Send; creation/freeing lock), `mosek.env` (Send),
`gurobi.params`, `gurobi.solve` (conic-dual buffer length check), `test.harness`, `docs.llm_interface`, the placement document, the manifest and
`tests/standards.rs`. New edges capi → core.*, capi → gurobi.env/mosek.env (environment creation and
ownership) and capi → gurobi.solve/mosek.solve (solving through their `Backend` implementations). Public interfaces: `Options::memory_limit_gb`; `Send` for both `Env` types; C interface ABI
1.0.

**Alternatives considered**: (1) a separate crate `optsolver-capi` — cleaner crate types but
contradicts "One crate" (Technical Constraints; an amendment is a human decision); (2) cbindgen
generation — adds a tool for ~45 declarations, while the agreement test gives the same safety;
(3) copying input into owned Rust buffers in the C layer — violates IV; (4) the smallest
conforming alternative is the plan itself: a translation module and a hand header.

**Reuse and adoption assessment**:

- *Owning component*: `core.backend` owns solving through `Backend`; `core.problem`,
  `core.conic`, `core.options` own construction and validation; `core.result` the result. The C
  layer owns only translation.
- *Inspected* (plus `gurobi::env` and `mosek::env` in full for the `Send` change): `Backend` (src/backend.rs:78-88), `CscRef::new` (src/problem.rs:69), `Problem::new`,
  `with_gamma`, `with_Q`, `with_conic` (src/problem.rs:190-265), `ConicRef::new` (src/conic.rs:366),
  `Options` (src/options.rs:94-125), `options::check::check` (src/options/check.rs:20), `Solution`
  and `SolutionBuffers` (src/result.rs:151-312), `gurobi::Env::new` (src/gurobi/env.rs:115),
  `mosek::Env::new` (src/mosek/env.rs:108), `gurobi::params::apply` (src/gurobi/params.rs:189),
  `gurobi::status::translate` (src/gurobi/status.rs:34), `solve_into` buffer checks
  (src/gurobi/solve.rs:56-91, src/mosek/solve.rs:55-75), the shared harness (tests/common/toy.rs,
  tests/common/compare.rs).
- *Disposition*: reuse — every behaviour is reached through the existing API; extend `Options`
  by one field and `gurobi::params::apply` by one mapping; new only for the C translation.
- *Duplication left in place*: the C layer re-checks null pointers and the row-bound form, which
  the Rust API cannot see (it receives slices); no validation rule of the Rust API is repeated.
  One gap in the Rust API is closed where it lives rather than in the C layer: Gurobi's
  `solve_into` does not check the length of the conic-dual buffer `s` (src/gurobi/solve.rs:56-91;
  MOSEK's does, src/mosek/solve.rs:63-70); the check is added there (FR-001: no validation in the
  C layer).
- *If a shared capability is introduced or materially changed* (XIII-G):
  - *Named consumers and bounded adoption scope*: the COBRA Toolbox MEX gateway
    `optsolver_solve` (consumer repository, contract `optsolver-mex.md`); within this repository,
    the C test programs.
  - *Migrating now* / *deliberately remaining*: no existing caller migrates; Rust callers stay on
    the Rust API.
  - *Representative consumer test*: `tests/c/` driver, a C program linked like the gateway
    (static and shared), solving every toy instance and compared with the Rust API; the
    link-demonstration criterion (SC-007) builds a minimal program with the documented flags.
    Adoption by the real consumer (the COBRA gateway built with `runmat mex`) cannot be shown in
    this repository (the RunMat binary does not start on the reference machine, and the gateway
    lives in the consumer repository); it is routed to the COBRA feature
    20261115-173718-runmat-testall-optsolver (its gateway build and test tasks, T032–T034, and its optSolver-interface success
    criterion, SC-008), and
    the hand-back to the COBRA session names the header, libraries and flags.
  - *Disposition of replaced implementations*: the founding design's RunMat adapter (never
    created) is superseded; recorded in ADR-0004.
- *Reachability and direction* (XIII-E): C callers reach `#[unsafe(no_mangle)] extern "C"`
  symbols exported from the static and shared libraries; the module is private in Rust (no Rust
  visibility change); dependency direction capi → core/backends only, no cycle.
- *Cross-boundary contract* (XIII-H): the C layer consumes only public Rust items (`Backend`,
  `Problem`, `Options`, `Solution`, `Capabilities`, `Env::new`); no private detail.
- *Behavioural integration evidence* (Principle II): the C programs are linked against the
  libraries Cargo built and run as separate processes; the Gurobi memory-limit test shows the
  option reaching `SoftMemLimit` (effective-parameter list) and the status `StoppedAtLimit`/
  `Memory` (licensed probe; if no instance triggers the soft limit, recorded unverified).

**Verdict**: issued by the Gate 2A compliance review (see `gate-reviews/` and `human-loop.md`).

## Project Structure

### Documentation (this feature)

```text
specs/005-c-abi/
├── spec.md, plan.md, research.md, data-model.md, quickstart.md
├── contracts/c-interface.md
├── architecture-delta.yaml
├── gate-reviews/
├── checklists/requirements.md
└── tasks.md
```

### Source Code (repository root)

```text
Cargo.toml                    # crate-type, feature capi, [[test]] c_interface, [[bench]] c_interface
src/capi/optsolver.h           # new: public C header (placement: public C header)
src/lib.rs                    # mod capi behind feature
src/capi/mod.rs               # new: codes, constants, errors, panic containment, version
src/capi/problem.rs           # new: C structures → Problem (borrowed), Options
src/capi/env.rs               # new: environment handle, capabilities
src/capi/result.rs            # new: solve entry points and result accessors
src/options.rs                # memory_limit_gb
src/gurobi/env.rs             # Send; creation/freeing lock
src/gurobi/load.rs            # model creation and freeing under the lock
src/gurobi/callback.rs        # test-helper model creation and freeing under the lock
src/mosek/env.rs              # Send
src/gurobi/params.rs          # SoftMemLimit
src/gurobi/solve.rs           # solve_into checks the conic-dual buffer length (as MOSEK does)
tests/c_interface.rs          # new: agreement, unlicensed and licensed C tests
tests/c/                      # new: C test programs (placement: C test sources)
tests/common/cdata.rs         # new: toy-instance writer, C-result reader (shared harness)
tests/standards.rs            # src/capi/ admitted as FFI module; header names in drift test
benches/c_interface.rs        # new: SC-002, SC-003
docs/llm-interface.md         # C interface section
docs/architecture/            # components.yaml, repo_folder_organisation.md, ADR-0004
specs/005-c-abi/requests/from-cobratoolbox-REQ-001.md   # record of the consumer's request
```

**Structure Decision**: one crate, one new module, one public header (research R4).

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| NaN-fill of caller buffers (O(m + n + k) writes) | spec Edge Cases: unwritten buffer entries must not look like data | leaving them untouched would present stale caller memory as results |
| NUL-terminated copies of result names | C needs terminated strings; Rust `String`s are not | changing the Rust result to `CString` would change the Rust API |
