# Implementation Receipt: feature 003, implementation cycle 1

## Prompt

/speckit-implement (maintainer, 2026-11-15), after Gate 2 approved T001–T026; later "complete
remaining tasks, will do LLM trial manually so leave as debt for now", then "commit".

## Final response

25 of 26 tasks are done; T024 (LLM trial, SC-002) is deferred as debt for a manual run
(protocol in `llm-trial.md`). Every in-gate check passes.

- US1: `docs/llm-interface.md` is the crate-level documentation (`include_str!`), with 6
  examples run as doctests in both tiers; a drift test fails if any of 260 public items is not
  named; `docs/llms.txt` entry point; README link.
- US2: 16 new toy instances (LP, QP in MPS; one conic JTASK per cone kind incl. PSD, plus a conic
  infeasible case) and the 4 existing small fixtures, all with provenance and two-tool checks (34
  checks agree with the closed-form optima); toy trials on both backends (conic also in
  maximise form).
- US3: Gurobi solves QP and quadratic, rotated quadratic, nonnegative and zero cones
  (`src/gurobi/load_qc.rs`; `NonConvex` = 0, `QCPDual` = 1 in `params::apply`; s = Pi);
  per-kind capabilities (`ConeKind`, `Capabilities::cones`); 18 MOSEK–Gurobi comparisons agree,
  6 not comparable; report per run.
- Found and fixed: both backends read the vendor error message after removing the callback,
  which clears it (feature 001–002 bug); now read immediately (research R13).
- Benchmark: QP on both backends, N = 5, all PASS (benchmark-results.md).

## Diff summary

- New: `docs/llm-interface.md`, `docs/llms.txt`, `src/gurobi/load_qc.rs`, `tests/common/{toy,
  compare}.rs`, `tools/corpus/make_toy_instances.m`, 16 toy files and records in
  `tests/fixtures/small/`, feature artifacts (benchmark-results.md, llm-trial.md, receipt).
- Changed: `src/{lib, conic, backend}.rs`, `src/options/check.rs`, `src/gurobi/{ffi, load,
  params, solve, callback, mod}.rs`, `src/mosek/{load, solve}.rs`; `tests/common/{fixtures, mps,
  jtask, f002, fake_backend, mod}.rs`, `tests/{corpus, contract, standards}.rs`,
  `tests/contract/qp_conic.rs`, `tests/header_conformance.c`; the 4 small fixtures' provenance
  (independent checks added); `benches/load_solve.rs`; `.specify/memory/project.yml`;
  `docs/test-corpus.md`, `docs/architecture/components.yaml`, `README.md`; feature spec, research
  (R13), tasks, human-loop.

## Tests (gate run results/003-llm-interface-toy-instances/runs/20261115T160741Z-gate)

- fmt; cargo check and clippy -D warnings for none, gurobi, mosek, gurobi+mosek: pass.
- Doctests: 6 pass without solvers (compile, return early) and with both (solve).
- No solver: 151 passed, 147 ignored. Licensed: 323 passed, 18 ignored (heavy and not
  comparable). Heavy: 12 passed. Rust 1.85: 323 passed.
- Comparison report: results/003-llm-interface-toy-instances/runs/20261115T160427Z-compare.
- Benchmark: results/003-llm-interface-toy-instances/runs/20261115T160715Z-bench.

## Unresolved issues

- **T024 / SC-002 unverified**: the LLM trial is deferred to the maintainer (debt).
- Toy provenance records name commit 89776f4 with `optsolver_worktree_clean: false`; re-run
  `tools/corpus/make_toy_instances.m` on a clean commit to record it (as done for feature 002).
- Plan deviation: Gurobi conic rows are added after the linear model (`GRBXaddconstrs` +
  `GRBXchgcoeffs` in fixed blocks) rather than stacked in the column loader (same bound;
  research R13).
- Gurobi provides no certificates for QP and conic problems (vendor limitation): those
  infeasible instances verify as PartiallyVerified on Gurobi.
- Not measured: Gurobi conic loading memory (no suitable corpus instance).
