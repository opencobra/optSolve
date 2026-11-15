# Implementation Receipt: feature 002, implementation cycle 1

## Prompt

/speckit-implement (maintainer, 2026-11-15), after Gate 2 approved all tasks T001–T040.
Commits were not authorised for implementation; nothing is committed.

## Final response

All 40 tasks are done. Every in-gate check passes except one heavy-tier test, which fails on a
genuine feasibility finding: MOSEK on iDopaNeuroC_problem_5. Five maintainer decisions were
taken during implementation (research R19).

- US1: LP fixture renamed iDopaNeuroC_problem_LP (byte-identical MPS, all array hashes match);
  docs/test-corpus.md written and checked against project.yml by a test; every target builds
  and passes on Rust 1.85 (the benchmark had no incompatibility of its own).
- US2: certificate sign (y_F,1 > 0, z_F < 0) and callback threading (calling thread only,
  never overlapping, four threads, barrier) are contract tests on every backend; both pass on
  Gurobi and MOSEK; the Gurobi comments now cite them.
- US3: MOSEK backend (own header-checked declarations: 62 functions, 100 constants, checked by
  a C compile against mosek.h plus a parameter-by-parameter Rust comparison, with negative
  controls). Every feature 001 per-backend test passes on MOSEK; LP agreement with Gurobi on
  three seeds passes (R10).
- US4: QP: derived fixture iDopaNeuroC_problem_LP_minNorm within R10 of a reference; two
  closed-form QPs; Gurobi reports Unsupported.
- US5: CLP: JTASK fixtures for 11 variational kinetics instances; every cone type (10 cases)
  minimise and maximise verified on MOSEK, including the dual sign mapping of R5; conic
  infeasibility certificate verified; licensed tier (iCoreED 1–5) passes; heavy tier 5 of 6
  pass on MOSEK (all 6 Unsupported on Gurobi as required).
- Benchmarks (N = 5): MOSEK SC-004 heap peak 78,536 B (usize) and 11,768 B (i32), limit
  1,185,728 B; SC-005 ratio 0.013 and 0.016, limit 0.05. Gurobi unchanged.

## Maintainer decisions during implementation (research R19)

1. CLP fixture format JTASK (CBF does not round-trip the linear block and bounds).
2. Conic complementarity per cone, not summed over all cones.
3. Gap the MOSEK way: separate bound multipliers z_l, z_u in the result (public API addition)
   and normalisation max(1, min(|p|, |d|)).
4. Per-item complementary slackness required only for basic solutions; reported, not
   deciding, for interior-point solutions.
5. (Following from 3) CLP reference objectives compared with the larger of the two runs'
   relative-gap tolerances.

## Diff summary

- New: src/conic.rs; src/mosek/{mod, ffi, env, load, params, status, callback, solve,
  probes}.rs; tests/contract/qp_conic.rs; tests/common/{f002, jtask}.rs;
  tests/header_conformance_mosek.{rs, c}; tests/fixtures/iDopaNeuroC_problem_LP.{mps,
  provenance.json}; tests/fixtures/qp/iDopaNeuroC_problem_LP_minNorm.provenance.json;
  tests/fixtures/clp/ (11 × .jtask + provenance, 12.7 MB); tools/corpus/
  extract_vk_clp_fixtures.m; docs/test-corpus.md; specs/002-…/benchmark-results.md.
- Changed: Cargo.toml (feature mosek), build.rs (MOSEK linking), .specify/memory/project.yml
  (corpus split, JTASK); src/{lib, problem, backend, result, verify}.rs,
  src/options/check.rs; src/gurobi/{solve, callback, load}.rs (Farkas s field, z_l/z_u,
  comments citing tests, Rust 1.85 fix); tests (harness: MOSEK, closures, heavy tier, pairs;
  standards for MOSEK; fixture readers; renamed references; generalised seed use);
  benches/load_solve.rs (per-solver scenarios); tools/corpus/extract_idopaneuroc_fixtures.m
  (rename); README.md; docs/architecture/components.yaml; feature 002 plan (amendment note),
  research (R15 finding, R16–R19), data-model, contracts/public-api, tasks, human-loop.
- Deleted: tests/fixtures/iDopaNeuroC_problem_1.{mps, provenance.json}.

## Tests (gate run results/002-mosek-lp-qp-clp/runs/20261115T103741Z-gate)

- fmt --check; cargo check and clippy -D warnings for none, gurobi, mosek, gurobi+mosek: all
  pass.
- Licensed (gurobi + mosek): unit 41, contract 72, corpus 133 (12 heavy ignored), header
  conformance 2 + 2, standards 3: all pass.
- No solver: unit 2, contract 72, corpus 64 passed and 81 ignored, standards 3: all pass.
- Rust 1.85.0, all targets, gurobi + mosek: same counts as licensed, all pass.
- Heavy (`OPTSOLVER_HEAVY_TESTS=1`): 11 passed, 1 failed (below).
- Benchmark run bundle: results/002-mosek-lp-qp-clp/runs/20261115T103501Z-bench.

## Unresolved issues

- **iDopaNeuroC_problem_5 on MOSEK fails verification**: MOSEK reports OPTIMAL with a
  row-bound violation of 1.97e-7 relative (limit 1e-7). Tighter tolerances don't change it,
  and with the variational kinetics parameters MOSEK reproduces that run's objective exactly
  with the same violation, so the recorded reference solution itself is about 20× outside
  MOSEK's stated feasibility tolerance (research R19). The test was not loosened.
- Fixture provenance records name commit 2b3be46 with `optsolver_worktree_clean: false`
  (extracted before any commit of this feature), as in feature 001 before T058.
- Not done: QP and CLP loading are not benchmarked; the MOSEK loader still copies infinite
  bound values although probe R18.1 shows MOSEK ignores them on infinite sides.
- Process: tests for T003/T004 were written after the code they test, not before (all passed
  on first run; the T034 cases did catch a real verifier bug for m = 0).
- Ranged rows: the exact dual objective uses split multipliers for variable bounds only; row
  bounds still use y alone, which is exact for equality and one-sided rows (all fixtures).

## Gate 3 (2026-11-15)

Maintainer: "Commit, demote instance". iDopaNeuroC_problem_5 is declared in `project.yml`
(`clp_expected_verification_failures`); its heavy-tier test now asserts that MOSEK reports
Optimal and that the verifier fails exactly the row-bound measure. docs/test-corpus.md
records the finding. Final gate run results/002-mosek-lp-qp-clp/runs/20261115T105344Z-gate:
every check passes (heavy tier 12 of 12). Fixture provenance still names commit 2b3be46 with
a dirty worktree; re-extraction from a clean commit was not requested.

## Re-extraction from the clean commit (2026-11-15, maintainer request)

All 13 provenance records (LP, derived QP, 11 CLP) now name 32c29b4 with a clean worktree.
LP from a clean tree; the CLP script was then re-run on a clean tree (the LP record stashed)
because it reads the worktree state before writing. Every MPS and JTASK file is bit-identical
to the committed one (round trips bit-for-bit); only provenance (commit, clean flag, timings)
changed. Log: results/002-mosek-lp-qp-clp/runs/20261115T110608Z-reextract-clean/. Corpus
tests: 133 passed, 12 heavy ignored.
