# optSolver

`optsolver` is a Rust library for solving large optimisation problems that are already in matrix
form, through the native C interfaces of commercial solvers (Gurobi 13.0.3, MOSEK 11.2). It
offers:

- **zero-copy input**: the problem borrows the caller's sparse (CSC) arrays and vectors; data
  reach the solver through bulk calls without per-element copies wherever the solver allows;
- **one solver-independent contract**: the same `Problem`, `Options`, `Solution`, `Status` and
  `Backend` trait for every solver, with capabilities declared per backend and unsupported
  requests rejected before any solver call;
- **independent verification**: every solution can be checked (primal and dual feasibility,
  complementarity, conic membership, gap) against the problem, not the solver's own report;
- **no RunMat or MATLAB dependency**: usable from any Rust program; adapters (C interface,
  RunMat built-ins) sit on top.

This file is descriptive; the rules are in [the constitution](.specify/memory/constitution.md)
and project parameters in [.specify/memory/project.yml](.specify/memory/project.yml).

## Problem classes

| Class | Gurobi | MOSEK |
|---|---|---|
| Linear programme (LP) | yes | yes |
| Convex quadratic programme (QP) | yes | yes |
| Conic linear programme, Fx + g ∈ K | quadratic, rotated quadratic, nonnegative, zero cones | also exponential, power and semidefinite cones |
| Mixed-integer (MILP, MIQP, MI conic) | planned (feature 004) | planned (feature 004) |

## Status of development

Development follows Spec Kit with human-gated phases (specification, planning, architecture
review, implementation, verification); each feature has its own directory under `specs/` and its
own branch. The integration branch is `devel`.

| Feature | Content | State |
|---|---|---|
| [000](specs/000-solver-interface-exploration/) | Design exploration: why vendor MEX files fail under RunMat; native Rust chosen | done |
| [001](specs/001-core-gurobi-lp/) | Core contract, verification, Gurobi LP | integrated |
| [002](specs/002-mosek-lp-qp-clp/) | MOSEK LP, QP and conic programmes | integrated |
| [003](specs/003-llm-interface-toy-instances/) | Interface document for programs and LLMs, self-contained toy instances for every class and cone, Gurobi QP and second-order cones, MOSEK–Gurobi comparison | integrated |
| [004](specs/004-milp-interface/) | Mixed-integer problems (MILP, MIQP, mixed-integer conic) through the existing interface, optional fixed-model duals | specified and planned (Gates 1, 2A, 2 passed); implementation not started |
| 005 (`005-c-abi` branch) | C interface (header + library) over the solver-independent API, for a COBRA Toolbox C MEX gateway `optsolver_solve` under RunMat (request REQ-001) | implemented, Gate 3 passed; awaiting integration |

Open item: the manual LLM trial of the interface document (feature 003, task T024).

## Using optSolver

Read [docs/llm-interface.md](docs/llm-interface.md): the complete interface document, also the
crate-level Rust documentation (its examples are compiled tests). LLM tools can start from
[docs/llms.txt](docs/llms.txt). Test instances are described in
[docs/test-corpus.md](docs/test-corpus.md); architecture decisions are in
[docs/architecture/decisions/](docs/architecture/decisions/).

## Build and test

Minimum Rust version 1.85 (edition 2024).

```bash
cargo build                    # solver-independent core only; no vendor software needed
cargo test                     # tests that need a solver are reported as ignored
GUROBI_HOME=/path/to/gurobi/linux64 cargo test --features gurobi
MOSEK_BINDIR=/path/to/mosek/11.2/tools/platform/linux64x86/bin cargo test --features mosek
cargo test --features gurobi,mosek     # also compares the two solvers on every toy instance
OPTSOLVER_HEAVY_TESTS=1 cargo test --release --features gurobi,mosek   # heavy tier
```

The features are additive. `gurobi` links `libgurobi130` from `$GUROBI_HOME/lib`; `mosek` links
`libmosek64` from `$MOSEK_BINDIR`. Each library directory must be on the run-time library path
and a valid licence available. Vendor headers, libraries and licences are never part of this
repository.

## Repository layout

```text
src/            library: core contract, verification, src/gurobi, src/mosek
tests/          contract, corpus (toy and comparison trials), standards, header conformance
tests/fixtures/ toy instances (small/) and corpus problems with provenance
benches/        load and solve benchmarks
docs/           interface document, test corpus, architecture model and decisions
specs/          one directory per feature: spec, plan, research, tasks, reviews, receipts
tools/          instance generators (MATLAB)
literature/     open-licence reference texts used during development (see SOURCES.md)
.specify/       constitution, project parameters, Spec Kit workflow
```

## Licence

MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`).
