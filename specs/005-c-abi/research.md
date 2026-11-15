# Research: A C Interface Over The Solver-Independent API (feature 005)

Phase 0 of `/speckit-plan`. Vendor facts were checked on 2026-11-15 against the installed MOSEK
11.2 C API manual (`<MOSEK>/11.2/doc/capi.pdf`, located at run time, never stored), the installed
Gurobi 13.0.3 header (`$GUROBI_HOME/include/gurobi_c.h`), the Gurobi Optimizer Reference Manual
13.0 (docs.gurobi.com/projects/optimizer/en/13.0/) and Gurobi support article 360043111231
(constitution X). Unverified claims are marked.

## R1. MOSEK memory limit (spec Clarifications, FR-011)

- **Decision**: rejected as unsupported before any solver call.
- **Evidence**: `mosek.h` 11.2 has no memory-limit parameter; the only memory items are
  `MSK_getmemusagetask`, `MSK_RES_ERR_SPACE` and the optimisation server's
  `MSK_RES_TRM_SERVER_MAX_MEMORY` (Bundle 1 check). The C API manual, entry `MSKcallbackfunc`:
  "The user must not call any MOSEK function directly or indirectly from the callback function.
  The only exception is the possibility to retrieve a current best integer solution from the
  mixed-integer optimizer". Polling `MSK_getmemusagetask` from the progress callback (COBRA's
  preferred option B) is therefore prohibited; a probe that appeared to work would show only
  undefined behaviour not crashing, so no probe is run.
- **Mechanism**: none new. `memory_limit_gb` is absent from `mosek::params::MAPPINGS`, so the
  shared `options::check::check` returns `Error::Unsupported { field: "memory_limit_gb",
  backend: "mosek" }` before any MOSEK call (constitution XVI-C, one rule for every backend).
- **Alternatives rejected**: accepting it and reporting "not honoured" (spec option C) — the
  caller who asked for a limit would run unprotected; the error is the honest outcome.

## R2. Gurobi memory limit (FR-011)

- **Decision**: `memory_limit_gb` maps to `SoftMemLimit` only, set in `gurobi::params::apply`.
- **Evidence** (13.0 parameter reference): `SoftMemLimit`, double, default Infinity, minimum 0:
  "Limits the total amount of memory (in GB, i.e., 10^9 bytes) available to Gurobi. If more is
  needed, Gurobi will terminate with a MEM_LIMIT status code", "leads to a graceful exit of the
  optimization, such that it is possible to retrieve solution information afterwards", "only
  checked at places where optimization can be terminated gracefully, so memory use may exceed
  the limit between these checks", "allocated memory is tracked across all models within a
  Gurobi environment". `MemLimit`: "If more is needed, Gurobi will fail with an OUT_OF_MEMORY
  error ... it is not possible to retrieve solution information after an error termination".
  `GRB_MEM_LIMIT` = 17 is already translated by `gurobi::status::translate` to
  `StoppedAtLimit` with `LimitReason::Memory` (src/gurobi/status.rs:34).
- **Open, settled by a licensed probe in implementation (task list)**: whether `SoftMemLimit`
  set on the model's environment (where `params::apply` writes) is honoured, and which instance
  triggers it. If it is not honoured per model, the plan returns to Gate 2A.
- **Conflict rule**: the mapping lists `SoftMemLimit`, so the existing conflict check makes a raw
  `SoftMemLimit` alongside `memory_limit_gb` a `ParameterConflict`; raw `MemLimit` is untouched.

## R3. Thread rule for the environment handle (FR-008)

- **Decision**: any thread, one thread at a time; different handles concurrently; Gurobi
  environment creation and freeing serialised by one process-wide lock in `gurobi::env`
  (`Env::new` and `Drop`), so that Rust callers of the now-`Send` `Env` are protected too.
- **Evidence**: MOSEK manual §10.3 "Sharing a task between threads is safe, as long as it is not
  accessed from more than one thread at a time. Multiple tasks can be created and used in
  parallel without any problems"; §10.6 "Calling MSK_optimize from different threads using the
  same MOSEK environment only consumes one license token". Gurobi 13.0 C reference (environment
  functions, quoted in feature 001 research R17-7): "Multiple threads cannot work simultaneously
  within the same environment", "environment initialization is itself not thread-safe"; support
  article 360043111231: "Gurobi's environment creation and deletion are not thread-safe", "In a
  multithreaded application, create and delete environments in the main thread", "Pass each
  additional thread an initialized environment for its exclusive use".
- **Rust types**: `gurobi::Env` and `mosek::Env` become `Send` (not `Sync`) through
  `unsafe impl Send`, each citing the evidence above in its `SAFETY` comment (constitution VII-B).
  This supersedes feature 001 research R15 and feature 002 research R2, which deferred `Send` to
  exactly this check (spec Clarifications, constitution XV-E). Because Gurobi says environment
  creation and deletion are not thread-safe, `gurobi::env` gains one process-wide `Mutex<()>`
  held across `GRBemptyenvinternal` … `GRBstartenv` in `Env::new` and across `GRBfreeenv` in
  `Drop`; this also closes a hazard that existed before (two threads each creating their own
  environment concurrently). MOSEK states no such restriction; no lock is added there
  (unverified that MOSEK environment creation is thread-safe; not needed by the C interface,
  whose consumer creates environments one at a time). Not `Sync`: "Multiple threads cannot work
  simultaneously within the same environment". A compile-time test asserts `Send` and a
  doc-test-style compile-fail note records `!Sync`.
- **Inference, unverified** (Gate 2A review 2): the Gurobi article says "create and delete
  environments in the main thread" and gives no reason. The design reads the instruction as
  "never concurrently with another creation or deletion", which a process-wide lock guarantees, and
  allows creation and freeing on any thread under that lock. The vendor does not say a lock
  suffices; this is recorded as an accepted risk in the `SAFETY` comment of `unsafe impl Send for
  gurobi::Env`, and a licensed test creates, uses and frees environments on several threads
  concurrently (task list) — evidence of absence of a fault in that run only, not proof. Whether
  the model's own environment copy (made when `GRBXloadmodel` creates a model, freed by
  `GRBfreemodel`) counts as environment creation is not stated by the vendor; the same lock is
  therefore also held around model creation and model freeing in `gurobi::load` and in the test
  helper model of `gurobi::callback` (no lock around optimisation, so concurrent solves on
  different environments still run in parallel).
- **One locked helper** (Gate 2A review 3): the rule is crate-wide — every call of
  `GRBemptyenvinternal`, `GRBstartenv`, `GRBfreeenv`, `GRBXloadmodel` and `GRBfreemodel`, in
  library code and in unit tests (which run on parallel threads of one process), is made inside
  `gurobi::env::lifecycle(|| …)`, a `pub(crate)` function that holds the one `static Mutex<()>`
  for the duration of the closure. `Env::new` makes its whole creation sequence (create, set
  `OutputFlag`, start, free on failure) one `lifecycle` call, so the non-re-entrant mutex is never
  taken twice; `Drop` is one call. Present sites: `Env::new` and `Drop` (src/gurobi/env.rs), the
  unit test creating an empty environment (src/gurobi/env.rs:160-165), the unit test of
  `configure_new_environment` (src/gurobi/params.rs:389-396), model creation and freeing in
  src/gurobi/load.rs, and the test-helper model in src/gurobi/callback.rs. A source test in
  `tests/standards.rs` fails if any line naming one of the five functions (outside `ffi.rs`) is
  not inside a `lifecycle(` call. Poisoning: a panic while the lock is held cannot leave the
  vendor's state half-changed by Rust (the vendor calls do not unwind), so a poisoned lock is
  recovered with `PoisonError::into_inner` and used normally.
- **MOSEK environment creation**: MOSEK documents no restriction and no guarantee for concurrent
  `MSK_makeenv`; unverified. No lock is added; the header states that MOSEK environments should
  be created one at a time, which the consumer (one environment per backend, created once) does.
- **C handle**: a `Box` of an enum holding one of the two `Env`s; the C caller may move the
  pointer between threads under the same rule.

## R4. Library form and crate types

- **Decision**: one crate (constitution, Technical Constraints), `crate-type = ["lib",
  "staticlib", "cdylib"]`; the C entry points live in a new module `src/capi/` behind a new
  additive Cargo feature `capi`. Without `capi` the static and shared libraries are built but
  export no C symbol; the Rust API is unchanged.
- **Alternatives rejected**: a second crate `optsolver-capi` (cleaner build, but the constitution
  states "One crate, optsolver" — changing that is an amendment, a human decision); generating
  the header with cbindgen (a new tool and build dependency for about forty declarations; drift is
  caught by the agreement test instead, R6).
- **Static library**: links no vendor library; the caller adds `-lgurobi130 -lmosek64` and the
  Rust runtime's native libraries (`cargo rustc --crate-type staticlib -- --print
  native-static-libs` gives them; on x86_64 Linux: `-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc`,
  recorded in the run log in implementation).
- **Shared library**: `liboptsolver.so` records `libgurobi130.so` and `libmosek64.so.11.2` as
  needed libraries (the build script's `rustc-link-lib`); at load time they are found through
  the dynamic loader's search (`LD_LIBRARY_PATH` or the system configuration). No rpath with an
  absolute machine path is written (constitution VII-F, Governance).
- **Panics**: the crate does not set `panic = "abort"` (VII-F: no reliance on its own profile);
  every entry point contains unwinding with `catch_unwind`. A caller that builds the library with
  `panic = "abort"` loses containment; the header says so.
- **Crate types and the additive feature** (Gate 1 observation): Cargo cannot select crate types
  by feature, so `[lib] crate-type = ["lib", "staticlib", "cdylib"]` is unconditional; what is
  behind the `capi` feature is the C entry points. Without `capi`, `cargo build` also produces
  `liboptsolver.a` and `liboptsolver.so` exporting no `optsolver_` symbol, and the Rust API (the
  `lib` crate type, an rlib) is unchanged; enabling `capi` adds symbols and changes no Rust
  behaviour (VIII additivity). Rejected: `cargo rustc --crate-type` per build (works, but the
  integration tests then could not rely on Cargo having built the libraries). The `links` key and
  the build script's environment variables are unchanged (VII-F).

## R5. Interface shape (FR-002 to FR-017)

- Input structures are `#[repr(C)]`, filled by the caller and passed by pointer, each starting
  with `struct_size` so that later minor versions can append fields (FR-015, FR-020).
- Arrays are borrowed as `&[T]` built with `slice::from_raw_parts` directly from the caller's
  pointers, lengths derived from the dimensions and `col_ptrs[n]`; nothing is copied (FR-003).
  Per-cone descriptors become a `Vec<Cone>` (one entry per cone, plus the power-cone exponents):
  linear in the number of cones (`num_cones`).
- Owned results stay as the Rust `Solution` inside an opaque `optsolver_result`; arrays are
  returned as pointers into its `Vec`s (no copy). Strings the Rust result holds as `String`
  (solver name and version, raw status names, attribute and parameter names) are converted to
  NUL-terminated copies lazily, on the first accessor call that needs them (a once-initialised
  cell in the result object), so they cost nothing unless read (constitution I): O(number of
  names), independent of m, n and nnz. Cone-kind names come from the existing `ConeKind::name`
  (with a static NUL-terminated table derived from it and checked against it by a unit test), not
  a second list. Basis statuses are read per index (no converted array).
- Caller buffers: the C layer fills each supplied buffer with NaN (O(m + n + k) writes, no
  allocation) and then calls `Backend::solve_into`; an entry the backend did not write is NaN, so
  undefined content is never presented as data (spec Edge Cases).
- Errors: the entry point returns a code; the message is stored per thread (thread-local) and
  read with `optsolver_last_error_message()`, valid until the next call on that thread
  (FR-010: one thread's failure cannot overwrite another's message).
- Usability (FR-017): `optsolver_capabilities(backend, &caps)` creates and frees a temporary
  environment and reports usable or the vendor's reason; `optsolver_env_capabilities(env, &caps)`
  reads a kept environment without creating another.

## R6. Header agreement (FR-013)

The header is written by hand (`src/capi/optsolver.h`). A licence-free test
`tests/c_interface.rs` (i) compiles and runs a C program printing `sizeof`, `offsetof` of every
field and every constant, and compares each with the Rust `size_of`, `offset_of!` and constants;
(ii) parses the header's prototypes and the Rust `extern "C"` definitions in `src/capi/` and
compares names and parameter and return types through one fixed type table; (iii) is shown to
fail on a deliberately altered copy of the header (XV-C).

## R7. Tests (FR-014, constitution II)

- C test programs live in `tests/c/`. The Rust harness compiles them with the `cc` crate's
  compiler with `-I src/capi` and links them to the library Cargo built for the test run (found
  from the test executable's directory, not a stored path, IX-I).
- Data exchange: the toy instances are read by the shared harness (`tests/common/toy.rs`, MPS and
  JTASK readers); the Rust test writes each instance in a small binary layout to a per-test
  directory under `CARGO_TARGET_TMPDIR`; the C driver reads it, solves through the C interface and
  writes every result field; the Rust test compares with the Rust API's result through the shared
  comparison helpers. Tests are written once and run on every backend supporting the instance.
- Unlicensed tier: version, constants and layout, validation and error codes, panic containment,
  leak check (AddressSanitizer's leak checker on the C error-path program; verify availability
  in implementation, else record unverified).
- Licensed tier: toy comparison (both row-bound forms for LP, both row-index widths), owned and
  caller-buffer paths, environment reuse with the creation count, memory limit on both backends.

## R8. Placement (constitution IX-F; Gate 1 observation)

The placement document gains three roles, each under a location `project.yml` already declares,
so no declared value changes and no kind of parameter is added (both human-reserved, VI-A):
public C header → `src/capi/` (`structure.source`; the header is source of the module that
implements it); C test sources → `tests/c/` (`structure.tests`); incoming cross-repository
requests → `specs/<feature>/requests/` (`structure.specs`; Tier 2, binding on the feature that
answers them). A record of the consumer's REQ-001 (origin, date, pointer to the text in the COBRA repository) is placed in `specs/005-c-abi/requests/`; the text itself is not copied, so no question of the COBRA repository's licence arises. The COBRA process
expected `requests/from-cobratoolbox/`; that would need a new declared location, a human decision,
and is not needed.

## R9. Solver configuration audit (constitution I)

No new problem data profile is introduced: the C layer passes the same problems, and the only new
parameter is `SoftMemLimit` (R2), unset by default (Gurobi default Infinity). No default changes.

## R10. Points carried from the Gate 1 reviews

- **Absent arrays on the caller-buffer path** (FR-005 "absent state distinguishable"): an array
  the backend did not write is all NaN; NaN is never a value a solver returns for x, y, z, z_l,
  z_u, s or row activity (finite values are checked by verification), so the absent state is
  distinguishable from data. The owned-result path reports absence as a NULL pointer and length 0.
- **The consumer's version check**: the COBRA contract says the gateway "refuses a mismatch" of
  `abiVersion`. Under the versioning rule a gateway built against 1.0 works with any 1.x; the
  hand-back to COBRA states that the gateway should compare the major part for equality and the
  minor part for at least its own. The library does not depend on which the consumer chooses.
- **Machine class for the performance criteria**: every run bundle records the CPU model, core
  count, memory and operating system of the machine (XIV-B); the criteria are relative (C path
  against Rust path on the same machine in the same session), so no machine class is fixed.
