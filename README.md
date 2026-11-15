# optSolver

Rust library (`optsolver`) for solving large optimisation problems that are already in matrix
form through native solver interfaces, with zero-copy borrowed inputs, one solver-independent
problem, options and result format, and independent verification of every solution.

This file is descriptive (Tier 3); the rules are in
[the constitution](.specify/memory/constitution.md).

## Status

- Feature 001: linear programmes (LP) through Gurobi 13.0.3.
- Feature 002: MOSEK 11.2 for LP, convex quadratic programmes (QP) and conic linear programmes
  (affine conic constraints Fx + g ∈ K with quadratic, rotated quadratic, exponential, power,
  nonnegative, zero and semidefinite cones). Gurobi remains LP only.

**Using optSolver from another program (or asking an LLM to)**: read
[docs/llm-interface.md](docs/llm-interface.md), the complete interface document (also the
crate-level Rust documentation; its examples are compiled tests). LLM tools can start from
[docs/llms.txt](docs/llms.txt). The test corpus is described in
[docs/test-corpus.md](docs/test-corpus.md).

## Build

Minimum Rust version: 1.85 (`rust-version` in `Cargo.toml`), checked in the in-gate checks
with `cargo +1.85.0 test --all-targets`.

```bash
cargo build                    # solver-independent core only; no vendor software needed
cargo test                     # tests that need a solver are reported as ignored
GUROBI_HOME=/path/to/gurobi/linux64 cargo test --features gurobi
MOSEK_BINDIR=/path/to/mosek/11.2/tools/platform/linux64x86/bin cargo test --features mosek
OPTSOLVER_HEAVY_TESTS=1 cargo test --release --features gurobi,mosek   # heavy tier
```

The features are additive. `gurobi` links `libgurobi130` from `$GUROBI_HOME/lib`; `mosek`
links `libmosek64` from `$MOSEK_BINDIR`. Each library directory must be on the run-time library
path and a valid licence available. Test tiers: no solver; licensed (`--features …`); heavy
(`OPTSOLVER_HEAVY_TESTS=1`: the large variational kinetics conic problems); minimum Rust
version.

Validation scenarios: [specs/001-core-gurobi-lp/quickstart.md](specs/001-core-gurobi-lp/quickstart.md),
[specs/002-mosek-lp-qp-clp/quickstart.md](specs/002-mosek-lp-qp-clp/quickstart.md).

## Licence

MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`).
