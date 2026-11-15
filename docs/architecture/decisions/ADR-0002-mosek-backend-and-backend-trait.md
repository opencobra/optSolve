# ADR-0002: MOSEK Backend And The Backend Trait

- **Status**: Accepted
- **Date**: 2026-11-15
- **Decided by**: the maintainer, at Gate 2A of feature 002 (`specs/002-mosek-lp-qp-clp/`)
- **Constitution**: IV, VII, VIII, XVI; supersedes nothing; extends ADR-0001

## Context

Feature 002 couples optSolver to a second solver, MOSEK 11.2, for LP, QP and conic problems,
and gives the `Backend` trait introduced as provisional in ADR-0001 its second implementation.

## Decision

1. MOSEK is reached through optSolver's own declarations in `src/mosek/ffi.rs`, checked against
   the installed header by a test, linked at build time behind the additive feature `mosek`.
   The official `mosek` crate is not used (feature 002 research R1: unchecked slice lengths
   reachable from safe code, panics on input, error codes discarded, no handle lifetimes, no
   panic containment in callbacks, build-time download).
2. The MOSEK backend has the same module structure as the Gurobi backend (ffi, env, load,
   params, status, callback, solve); no code is shared between backends.
3. The `Backend` trait keeps its shape (`capabilities`, `solve`, `solve_into` over
   `ProblemRef`). Its re-examination at this second implementation is recorded at feature 002
   closeout; until then it stays provisional.
4. Problem classes are declared per backend in `Capabilities`; a problem of an undeclared class
   is rejected before any solver call.

## Consequences

- Each backend's unsafe surface is confined to its own `ffi` module and checked against its own
  header.
- Every test written against `Backend` runs on both solvers, giving cross-solver evidence.

## Alternatives considered

- The official `mosek` crate, wrapped (R1 gaps become accepted risks).
- A shared FFI helper layer across backends (premature; constitution V).
