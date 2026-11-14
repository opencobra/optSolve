# Implementation Receipt: feature 001, implementation cycle 1

## Prompt

/speckit-implement "in parallel to the maximum extent possible with multiple agents" (maintainer, 2026-11-14), following Gate 2 approval of all tasks T001-T054.

## Final response

Feature 001 is implemented: all 57 approved tasks are done, with one success criterion not established. Speed criterion SC-005 isn't established, because the measurement also counts Gurobi's own model building. Nothing is committed yet; that decision comes with Gate 3 below.

**What now exists.** The crate `optsolver` provides:
- **Solver-independent core:** a borrowed problem type validated once before any solver call, the options structure, the result type with named statuses, and independent verification of every solution.
- **Gurobi LP backend:** about 25 C declarations of its own, checked against the installed header by a test. It bulk-loads in one call with `i32` row indices and in blocks through a small buffer with `usize` indices. A single function sets all solver parameters; every status code is translated; cancellation and progress callbacks are supported, and a panic in a callback is caught and re-raised.
- **Shared test harness:** each test is written once and runs on every backend.
- **Fixture extraction:** a MATLAB script builds the fixtures and checks them with a bit-for-bit round trip.
- **A benchmark.**

Three agents worked in parallel on the first round (corpus, verification, Gurobi declarations) and three on the second (parameters and callbacks, loader, US3 tests). I wrote the foundation, `solve.rs` and the benchmark, and integrated everything.

**Test results:**

| Tier | Library | Contract | Corpus | Header check | Standards |
|---|---|---|---|---|---|
| Licensed Gurobi 13.0.3 | 27 | 44 | 37 | 2 | 3 |
| No solver | 0 | 44 | 15 pass, 22 ignored | — | 3 |

All pass. In the no-solver tier the 22 Gurobi trials are listed as ignored with "gurobi: not compiled", never as passed. Formatting and clippy with warnings denied are clean, with and without the Gurobi feature. The Rust 1.85 build is unverified because that toolchain isn't installed.

**Benchmark** (iDopaNeuroC_problem_1, N = 5; details in `specs/001-core-gurobi-lp/benchmark-results.md`):
- **SC-004 memory: passes.** optSolver's heap peak during load is 78,536 B with `usize` indices and 16,384 B with `i32`, against a limit of 1,185,728 B.
- **SC-005 time: not established.** The measured (load + extraction + verification) ÷ solve is 0.057–0.061. That counts Gurobi's own model build inside "load", so it is only an upper bound. optSolver's own parts come to about 2% by derivation, not by direct measurement.

**Decisions made during implementation (all recorded in `human-loop.md`):**
- **Test corpus:** now problem 1 only, the flux-balance LP. Problems 2–5 are variational kinetics problems, and as stored, 2 and 3 are infeasible as LPs (research R18).
- **Reduced-cost check:** now scaled per column by the size of the terms summed. The duals reach 1.4 × 10¹⁰ on problems 4 and 5.
- **Constitution 2.4.1:** passing Gurobi's non-`const` input arrays directly is now justified by the manual's wording plus a recorded probe, which is kept as a test.

**Deviations** (listed in the receipt):
- **Contract shape:** `Problem` fields are private, with accessors. `Backend` takes a `ProblemRef`. There is no `new_unchecked`.
- **Farkas certificate sign:** chosen by observation; the vendor's documented convention isn't cited yet.
- **Test order:** for User Story 2 the code was written before the tests. A deliberately loosened threshold was used afterwards to confirm the tests can fail.
- **Callback threading:** the safety argument assumes Gurobi calls the callback from one thread at a time. This isn't confirmed in the manual.

**Things that need your attention:**
- **Licence path:** your `GRB_LICENSE_FILE` points to a file that doesn't exist (`/usr/local/bin/gurobi1303/gurobi.lic`). The real one is at `/usr/local/bin/gurobi1303/gurobi.lic2/gurobi.lic`. I passed it on the command line only and changed no configuration.
- **Fixture provenance:** the fixture's provenance names commit b4628b3 with uncommitted changes. Re-running the extraction after commit would record the new commit.

## Diff summary

- New: Cargo.toml, Cargo.lock, build.rs, LICENSE-MIT, LICENSE-APACHE, README.md; src/ (lib, error, problem with validate and convert, options with check, result, verify, backend, gurobi/{mod, ffi, env, load, params, status, callback, solve}); tests/ (common/{mod, mps, objectives, fixtures, fake_backend, us3}, contract, corpus, standards, header_conformance.rs and .c, fixtures/iDopaNeuroC_problem_1 MPS and provenance, fixtures/small/ four MPS files with provenance); benches/load_solve.rs; tools/corpus/extract_idopaneuroc_fixtures.m.
- Modified: .gitignore (target, results, in-flight marker, .serena); constitution 2.4.1 and changelog; project.yml (test corpus = problem 1); docs/architecture/components.yaml; feature spec, plan, research (R16-R18), data-model (reduced-cost scaling), contract reconciliation, quickstart, tasks, human-loop; new benchmark-results.md.

## Tests

- Licensed (Gurobi 13.0.3, GRB_LICENSE_FILE passed on the command line): lib 27, contract 44, corpus 37, header_conformance 2, standards 3 passed; 0 failed.
- No features: contract 44, corpus 15 passed and 22 ignored (gurobi: not compiled), standards 3; 0 failed.
- cargo fmt --check clean; cargo clippy --all-targets -D warnings clean with and without --features gurobi; cargo check --no-default-features and --features gurobi clean; MSRV 1.85 build not run (toolchain not installed).
- Benchmark: results/001-core-gurobi-lp/runs/20261114T211851Z-bench (SC-004 pass, SC-005 not established).
- Gate log: results/001-core-gurobi-lp/runs/20261114T211928Z-gate/gate.txt.

## Unresolved issues

- SC-005 not established: the load phase includes Gurobi model construction; optSolver time inside the load call needs separating.
- MSRV 1.85 unverified.
- Gurobi FarkasDual sign convention and single-threaded callback invocation not yet cited from vendor documentation (research R17).
- Fixture provenance records commit b4628b3 with uncommitted changes; re-extract after commit.
- Maintainer environment: GRB_LICENSE_FILE points to a missing file.

## Other information

- Deviations from the contract sketch and plan: private Problem fields with accessors; no new_unchecked; object-safe Backend with ProblemRef; solve_into returns a Solution with empty arrays; added problem::convert, options::check, verify::names, Capabilities::options, doc-hidden Env::load_report; dev-dependency dhat (MIT OR Apache-2.0) for exact heap measurement.
- Process deviation: User Story 2 code written before its tests; a mutation check (threshold loosened, 11 tests failed, restored) substituted for the failing-first record.
- Analysis findings C1 (solve_into tests), C2 (problem-class rejection tests), C3 (small-fixture provenance), C4 (results/ tracking declared in .gitignore) and U1 (standards test searches only; clippy in gate) were handled.
