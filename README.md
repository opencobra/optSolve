# optSolver

Rust library (`optsolver`) for solving large optimisation problems that are already in matrix
form through native solver interfaces, with zero-copy borrowed inputs, one solver-independent
problem, options and result format, and independent verification of every solution.

This file is descriptive (Tier 3); the rules are in
[the constitution](.specify/memory/constitution.md).

## Status

Feature 001: linear programmes through Gurobi 13.0.3. MOSEK and further problem classes are
later features.

## Build

```bash
cargo build                    # solver-independent core only; no vendor software needed
cargo test                     # tests that need a solver are reported as ignored
GUROBI_HOME=/path/to/gurobi/linux64 cargo test --features gurobi
```

The Gurobi feature links `libgurobi130` from `$GUROBI_HOME/lib` at build time; the library
directory must be on the run-time library path and a valid Gurobi licence available.

Validation scenarios: [specs/001-core-gurobi-lp/quickstart.md](specs/001-core-gurobi-lp/quickstart.md).

## Licence

MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`).
