# Implementation Plan: MOSEK Backend For LP, QP And CLP

**Branch**: `002-mosek-lp-qp-clp` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/002-mosek-lp-qp-clp/spec.md`

## Summary

Add a MOSEK backend (feature `mosek`) behind the existing solver-independent contract for LP, QP
and conic linear programmes with affine conic constraints, using optSolver's own header-checked
declarations (research R1). Extend the core problem with a validated quadratic term and conic
data (F, g, cones from a broad cone set), the result with conic duals, and verification with QP
and conic measures. Rename the LP fixture to iDopaNeuroC_problem_LP, document the corpus, add
variational kinetics CLP fixtures (CBF) and a derived QP fixture, and add contract-level tests of
the certificate sign and callback threading that run on both backends. Make every target build
and pass on Rust 1.85 and add that build to the in-gate checks. Decisions: research.md R1–R15;
entities: data-model.md.

**Amended during implementation (2026-11-15, maintainer decisions, research R19)**: the CLP
fixture format is MOSEK JTASK, not CBF (CBF does not round-trip the linear block and bounds), so
every "CBF" below reads "JTASK" (`tests/common/jtask.rs`, `tests/fixtures/clp/<instance>.jtask`);
the result gains split bound multipliers z_l, z_u; the gap uses MOSEK's normalisation; conic
complementarity is judged per cone; per-item complementary slackness is required only for
basic solutions.

## Technical Context

**Language/Version**: Rust 2024, minimum 1.85.0 (installed 2026-11-15); MATLAB (extraction only)

**Primary Dependencies**: Gurobi 13.0.3 (feature `gurobi`), MOSEK 11.2 (feature `mosek`,
`libmosek64` from `MOSEK_BINDIR` at build time); dev-dependencies unchanged (`libtest-mimic`,
`cc`, `dhat`)

**Storage**: test fixtures (MPS for LP, CBF for conic, provenance JSON); run bundles in
`results/`

**Testing**: `cargo test` tiers: no solver; licensed (`--features gurobi,mosek`); heavy
(`OPTSOLVER_HEAVY_TESTS=1`); minimum version (`+1.85.0`)

**Target Platform**: 64-bit Linux

**Project Type**: library

**Performance Goals**: MOSEK SC-004 and SC-005 as for Gurobi (spec SC-004)

**Constraints**: zero copy except MOSEK bound keys (O(m + n)), infinite-bound copies (O(m + n),
only when present), QP triplet indices (O(nnz(Q)), unavoidable, Complexity Tracking); F loaded
through a fixed-size buffer

**Scale/Scope**: CLP up to n = 38,763, k = 56,159 (iDopaNeuroC); LP corpus 1344 × 2942

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Pre-research: PASS. Post-design: PASS with three Complexity Tracking entries and two required
amendments (Notation symbols F, g, K, s, k; feature 001's `random_seed` test generalised).

- **Development scope**: [VI] all code (Rust, C conformance file, MATLAB extraction scripts,
  CBF reader, tests, benchmark changes) inside this feature; no override.
- **Performance and memory**: [I] targets spec SC-004; allocations proportional to nonzeros:
  QP triplet indices only (Complexity Tracking); F through a fixed buffer; LP as feature 001.
  MOSEK environment reused; task per solve; outputs all returned by default.
- **Solver configuration audit**: [I] research R9: interior point default for QP and conic;
  basis identification on for LP; tolerances from MOSEK defaults (1e-8); log 0. The variational
  kinetics parameters (`CO_TOL_PFEAS 1e-8`, `MAX_TIME 600`, conic optimizer, presolve free) are
  recorded per CLP fixture; tests use MOSEK defaults plus threads 1 for determinism (M§10.3).
- **Solver parameters in one location**: [VII-G] `mosek::params::apply` is the only caller of
  `MSK_put*param`/`MSK_putna*param`; the standards test extends to MOSEK setter symbols.
- **Zero-copy inputs and bulk calls**: [IV] LP: `putacolslice64` with borrowed column starts and
  values, i32 rows direct or blocked conversion; bound keys built; QP: `putqobj` triplets;
  conic: `putafefentrylist` per block, `putafegslice` borrowed, `appendaccsseq` once.
- **Solver-independent contract**: [XVI] (a) one problem/result type; additions in
  contracts/public-api.md; (b) `mosek::status::translate` total (research R6); (c) shared
  fields mapped per R9, unsupported ones are errors; (d) QP and conic verification measures
  (data-model.md).
- **Rust and FFI standards**: [VII] `unsafe` only in `src/mosek/` (ffi declarations only in
  `ffi.rs`); `Env` owns `MSKenv`, task created and deleted inside each solve and tied by
  lifetime; no Send/Sync; callback with `catch_unwind`; header conformance test for MOSEK.
- **Independence and additive features**: [VIII] `mosek` feature additive; core and Gurobi
  unaffected; `cargo check` for each feature combination.
- **Testing and behavioural evidence**: [II] every existing per-backend test runs on MOSEK
  unchanged; new tests written once in `tests/common/`; backend-specific tests only for status
  translation and header conformance; cross-solver LP agreement via a two-backend trial.
- **Diagnostics and statuses**: [III] MOSEK silent by default (no stream; `LOG` 0); log stream
  forwarded to `log` when requested.
- **Benchmarks and evidentiary preconditions**: [XIV] LP benchmark extended to MOSEK, same
  instance and N; positive control of the load path.
- **Reporting discipline**: [XV] N stated; guards (CBF reader, conic verifier) tested against
  failures; heavy runs logged to the run bundle.
- **Provenance and licensing**: [X] CLP fixtures with provenance (variational kinetics commit,
  source SHA-256, MOSEK version and parameters, expected status, origStat); variational kinetics
  data licence: not verified, as for the LP fixture (maintainer decision 2026-11-14 extends to
  this corpus: to be confirmed at Gate 2A).
- **Project parameters**: corpus lists (LP, QP, CLP, heavy) in `project.yml`; MOSEK version
  declared there; build script reads `MOSEK_BINDIR` only.
- **Notation**: [XII-C] F, g, K, s, k added through `/speckit-constitution` before tasks.
- **Artifact placement**: [IX] `src/conic.rs`, `src/mosek/`, `tests/fixtures/clp/`,
  `tools/corpus/extract_vk_clp_fixtures.m`, `docs/test-corpus.md`.
- **Architecture conformance**: [XIII] `architecture-delta.yaml`; ADR-0002 required (second
  solver coupling; re-examination of the provisional `Backend` trait per ADR-0001).

## Architecture Impact

**Current architecture**: components.yaml (feature 001): core, gurobi, test harness, corpus,
tools, bench.

**Proposed delta** (architecture-delta.yaml): adds `core.conic`; `mosek.ffi`, `mosek.env`,
`mosek.load`, `mosek.params`, `mosek.status`, `mosek.callback`, `mosek.solve`;
`tools.vk_clp_extraction`; `docs.test_corpus`. Modifies `core.problem` (Q validation, conic),
`core.result` (s), `core.verify` (QP and conic measures), `core.backend` (ProblemClass::Conic;
the provisional trait is re-examined now that a second backend implements it),
`test.harness` (CBF reader, heavy tier, two-backend trials), `test.corpus`, `bench.load_solve`,
`tools.corpus_extraction` (rename). Dependency directions unchanged: backends → core.

**Alternatives considered**: the official `mosek` crate (R1); MPS for conic fixtures (no conic
support); loading F with `putafefcol` (per-column calls, violates IV); a CSR conic input
(would push a transpose onto every CSC caller).

**Reuse and adoption assessment**:

- *Owning component*: core.problem / core.verify (extended), gurobi.* as the pattern for mosek.*.
- *Inspected*: src/problem.rs, src/problem/convert.rs (IndexBuffer reused for MOSEK row-index
  blocks), src/verify.rs, src/options/check.rs, src/gurobi/*.rs; mosek crate 11.2.3 lib.rs
  (`put_a_col_slice` lib.rs:15790, `handle_res` :10236, `callback_proxy` :4932); variational
  kinetics optimizeVKmodel.m, solveCobraCLP.m, COBRA parseMskResult.m, optimizeCbModel.m.
- *Disposition*: extend the core; new backend module following the Gurobi structure; reuse
  `IndexBuffer`; reuse variational kinetics `optimizeVKmodel` (assembly) and MOSEK's CBF writer
  for extraction.
- *Duplication left in place*: per-backend `ffi/env/load/params/status/callback/solve` modules
  repeat a structure (not code); a shared abstraction is not extracted (constitution V) beyond the
  `Backend` trait.
- *If a shared capability is introduced*: `Backend` gains its second implementation — consumers:
  the shared harness (all tests), the benchmark; representative consumer test: every corpus test
  now runs on both backends.
- *Reachability and direction*: unchanged; `pub mod mosek` behind its feature.
- *Cross-boundary contract*: backends consume the core only through public types.
- *Behavioural integration evidence*: every corpus test calls `Backend::solve` on MOSEK; the
  parameter test reads effective parameters back from MOSEK.

**Verdict**: APPROVE_WITH_ADR, 2026-11-15 (maintainer). ADR-0002. Names accepted; constitution
2.5.0 (F, g, K, s, k); CLP problem = the `solveCobraCLP` input; iDopaNeuroC_problem_4 excluded;
the data-licence decision of 2026-11-14 extends to the variational kinetics CLP data.

## Project Structure

```text
src/conic.rs                          # cones, ConicRef, cone verification helpers
src/mosek/{mod,ffi,env,load,params,status,callback,solve}.rs
src/problem.rs, src/result.rs, src/verify.rs, src/backend.rs, src/options/check.rs  # extended
build.rs                              # + MOSEK linking (feature mosek)
tests/common/{cbf.rs, us3.rs (extended), mod.rs (heavy tier, pair trials)}
tests/fixtures/iDopaNeuroC_problem_LP.{mps,provenance.json}
tests/fixtures/qp/iDopaNeuroC_problem_LP_minNorm.provenance.json   # derived fixture
tests/fixtures/clp/<instance>.{cbf,provenance.json}
tests/header_conformance_mosek.{rs,c}
tools/corpus/extract_vk_clp_fixtures.m
docs/test-corpus.md
benches/load_solve.rs                 # + MOSEK
```

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| QP index arrays O(nnz(Q)) | MOSEK accepts the quadratic term only as triplets in one call | No slice or CSC QP call exists; chunking would replace terms |
| MOSEK bound-key arrays O(m + n) | MOSEK's bound encoding | Cannot be borrowed; within the 32 B per row and column budget |
| Two corpus file formats (MPS, CBF) | MPS cannot express affine conic constraints | A custom binary format loses interoperability with solver tools |
