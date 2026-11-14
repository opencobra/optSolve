# Implementation Receipt: feature 001, implementation cycle 2 (slice 2)

## Prompt

/speckit-implement (maintainer, 2026-11-14), after Gate 3 chose "Next slice: establish SC-005" and approved T055-T058 (commit not authorised).

## Final response

Slice 2 is done except T058, and SC-005 is now established: it passes. T058, re-extracting the fixture so its provenance names a clean commit, is waiting for you to authorise a commit. Nothing is committed yet.

**T055 (loader):** the loader now times Gurobi's own load calls (`GRBXloadmodel`, `GRBXaddvars`, `GRBupdatemodel`) separately from its whole run, and a licensed test checks that Gurobi's share is measured and never exceeds the total.

**T056 (benchmark):** the benchmark now also times validation, and judges SC-005 on optSolver's parts only: (validation + optSolver's own load work + extraction + verification) ÷ solve. A record missing any of these fields is refused.

**T057 (results):** on iDopaNeuroC_problem_1, N = 5:

| Row indices | SC-005 ratio, median | Range | SC-004 heap peak during load |
|---|---|---|---|
| `usize` | 0.019 | 0.015–0.031 | 78,536 B |
| `i32` | 0.019 | 0.013–0.026 | 16,384 B |

Both pass both limits (0.05 for SC-005; 1,185,728 B for SC-004). optSolver's own load work takes 0.02–0.03 ms, against about 1–1.5 ms inside Gurobi's calls. `benchmark-results.md` now holds the new figures; the earlier 0.057–0.061 measurement, which counted Gurobi's model build, is marked superseded.

One oddity is recorded there unexplained: Gurobi's share measured in load-only calls is larger than the "load" phase timed inside full solves. They are different calls with different environment state, and SC-005 doesn't depend on it.

**Tests:** both tiers pass. With licensed Gurobi: library 27, contract 44, corpus 37, header check 2, standards 3. Without a solver: contract 44, corpus 15 passed and 22 ignored, standards 3. Clippy with warnings denied and `cargo fmt --check` are clean.

## Diff summary

- src/gurobi/load.rs: vendor and total load timing (`Loaded::vendor_seconds`, `total_seconds`) and a test assertion.
- src/gurobi/solve.rs: `LoadReport` gains `vendor_seconds`, `total_seconds` (doc-hidden).
- benches/load_solve.rs: validation timing, load split, SC-005 on optSolver parts, record validation of the new fields.
- specs/001-core-gurobi-lp/benchmark-results.md, tasks.md, human-loop.md updated.

## Tests

- Licensed: lib 27, contract 44, corpus 37, header_conformance 2, standards 3 passed; 0 failed.
- No features: contract 44, corpus 15 passed and 22 ignored, standards 3; 0 failed.
- clippy --all-targets --features gurobi -D warnings clean; fmt --check clean.
- Benchmark run bundle: results/001-core-gurobi-lp/runs/20261114T220238Z-bench.

## Unresolved issues

- T058 not run: needs a commit authorisation.
- Unexplained: Gurobi load-call time in load-only calls exceeds the load phase timed inside solves.
- Carried from cycle 1: MSRV 1.85 unverified; FarkasDual sign convention and callback threading not yet cited from vendor documentation; GRB_LICENSE_FILE misconfigured on this machine.
