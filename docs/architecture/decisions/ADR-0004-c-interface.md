# ADR-0004: A C Interface Over The Solver-Independent API

- **Status**: Accepted — Gate 2A compliance review 8 of feature 005 issued `APPROVE_WITH_ADR` on
  2026-11-16 (`specs/005-c-abi/gate-reviews/gate-2A-20261116T004341Z.md`)
- **Date**: 2026-11-15
- **Decided by**: the feature-005 agent under constitution 3.0.0 (VI-A, XIII-F), checked by the
  Gate 2A compliance review
- **Constitution**: I, III, IV, VII, VIII, IX, XIII, XVI; extends ADR-0001

## Context

The COBRA Toolbox calls optSolver from RunMat through a C MEX gateway in the COBRA repository.
MATLAB-language code reaches compiled code only across a C boundary, and Rust has no stable ABI
between separately built binaries, so optSolver must offer a C header and a library. The founding
design placed a Rust RunMat adapter outside this repository; that adapter becomes the C gateway,
still outside this repository (VIII).

## Decision

1. One crate still (Technical Constraints): `crate-type = ["lib", "staticlib", "cdylib"]`; the C
   entry points live in `src/capi/` behind an additive feature `capi`. The Rust API is unchanged
   except two additive changes: `Options::memory_limit_gb`, and `Send` (not `Sync`) for
   `gurobi::Env` and `mosek::Env`; and Gurobi's `solve_into` gains the conic-dual buffer length
   check MOSEK's already has.
2. The C layer is a translation only: it builds the Rust `Problem` and `Options` from caller
   structures that borrow the caller's arrays in place, calls `Backend::solve` or `solve_into`,
   and exposes the Rust `Solution` through an opaque object read in place. Validation, status
   translation, parameter application and verification stay where they are.
3. The public header `src/capi/optsolver.h` is written by hand and kept in agreement with the
   library by a licence-free test (layout, constants, prototypes). Its version is
   major.minor, with input structures carrying `struct_size` so minor versions can append fields.
4. Errors cross as codes with a per-thread message; panics are contained at every entry point,
   silently: a panic hook installed once on first use writes nothing for panics inside entry
   points (their message becomes the internal-error message) and passes every other panic to
   the previously installed hook (feature 005 research R4).
5. Environment handles may move between threads (one user at a time), on vendor evidence
   (feature 005 research R3): the Rust `gurobi::Env` and `mosek::Env` become `Send`, not `Sync`,
   superseding feature 001 research R15 and feature 002 research R2, which deferred `Send` to
   that check. Gurobi environment creation and freeing are serialised by one lock in
   `gurobi::env`, enforced by a token type only the locked helper creates, protecting Rust callers
   too; model creation and loading are not locked. Gurobi's article advises creating and deleting
   environments in the main thread; that a lock suffices instead, and that a model's internal
   environment copy is not covered by that advice, are inferences, unverified, recorded as an
   accepted risk and exercised by a licensed concurrency test (research R3).
6. The memory limit is a solver-independent option: Gurobi `SoftMemLimit`; unsupported on MOSEK.
7. Earlier decisions changed: ADR-0001 decision 2 ("two layers") gains a third, the C layer
   `capi`, above the core and the backends, depending on both and depended on by neither;
   ADR-0001 decision 3 and ADR-0002's consequence that `unsafe` code lives only in backend
   modules are widened: `src/capi/` is also an FFI module (it exports `extern "C"` functions and
   dereferences caller pointers), with the same `SAFETY` discipline and no `extern` declaration
   blocks.

## Alternatives considered

A separate `optsolver-capi` crate, header generation with cbindgen, and copying input in the C
layer were rejected; see `specs/005-c-abi/plan.md`, Architecture Impact, and research R4.

## Consequences

- The Rust side of the C interface is reachable from Rust as `optsolver::capi`
  (`#[doc(hidden)] pub`, feature `capi`) so that tests and the benchmark can use it; it is not part
  of the documented Rust API and may change with any release.

- Building the library also builds static and shared libraries; without `capi` they export no C
  symbol. A shared library built with a backend needs that vendor library at load time.
- A Rust-API change that affects the C interface must update the header, the contract and the
  ABI version in the same change, or the agreement test fails.
- The RunMat adapter of the founding design (specs/000) is superseded by the consumer's C
  gateway; the boundary "no optSolver crate depends on RunMat" is unchanged.
