# ADR-0001: Crate Layering And The Solver-Independent Contract

- **Status**: Accepted
- **Date**: 2026-11-14
- **Decided by**: the maintainer, at Gate 2A of feature 001 (`specs/001-core-gurobi-lp/`)
- **Constitution**: VIII (independence, additive backends), XVI (solver-independent contract),
  VII-A and VII-G (FFI confinement, single parameter location), II (tests written once)

## Context

optSolver has no source. Feature 001 introduces the crate, its first layers, a shared data
representation (problem, options, result), and its first coupling to a solver (Gurobi). The
constitution requires an ADR for each of these (XIII-C).

## Decision

1. One crate, `optsolver`, at the repository root. Solver backends are modules behind additive
   Cargo features (`gurobi` first).
2. Two layers: the **solver-independent core** (problem, options, result, verification,
   backend trait, error) and **backends**. Backends depend on the core; the core never depends
   on a backend.
3. Within a backend, one module declares every C function and constant (`ffi`), and one
   function applies every solver parameter (`params`). All other modules forbid `unsafe` or
   use it only to call through `ffi`.
4. Backends are reached through a `Backend` trait. It has one implementation in feature 001 and
   is **provisional**: its shape is re-examined, and may change, when the MOSEK backend
   implements it.
5. Tests reach backends only through `Backend`, from one shared test module, so every test runs
   against every backend that supports its problem class.
6. Bindings are optSolver's own declarations, checked against the installed vendor header by a
   test; no vendor files or generated bindings are committed.

## Consequences

- Adding a backend adds one module and one feature; no test is rewritten.
- The provisional trait is accepted architectural debt until a second backend exists
  (plan.md Complexity Tracking, feature 001).
- The architecture model (`components.yaml`) gains the feature-001 components at closeout,
  reconciled against `specs/001-core-gurobi-lp/architecture-delta.yaml`.

## Alternatives considered

- A workspace with `-sys` and backend crates: rejected by the maintainer in favour of a single
  crate (`specs/000-solver-interface-exploration/open-questions.md`, decisions).
- Existing Gurobi binding crates: rejected; none exposes 64-bit bulk loading on borrowed slices
  (feature 001 research R1).
- A backend enum instead of a trait: rejected; it requires editing the test harness for every
  backend.
