# Implementation Plan: LLM-Usable Interface Documentation And Self-Contained Cross-Solver Test Instances

**Branch**: `003-llm-interface-toy-instances` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/003-llm-interface-toy-instances/spec.md`

## Summary

Write one interface document (`docs/llm-interface.md`) that is also the crate-level Rust
documentation, with every example a documentation test and a drift test tying it to the public
items; add `docs/llms.txt` as an entry point. Add committed toy instances (MPS for LP and QP, JTASK for
conic problems, one per cone kind including PSD) built and checked against both solver tools by
one MATLAB script, with provenance. Extend the Gurobi backend to convex QP and to quadratic,
rotated quadratic, nonnegative and zero cones (per-cone capabilities in the shared check), so
that MOSEK and Gurobi are compared on every instance both support, with a report per run.
Decisions: research.md R1–R12; entities: data-model.md.

## Technical Context

**Language/Version**: Rust 2024, minimum 1.85.0; MATLAB (toy instance generation only)

**Primary Dependencies**: Gurobi 13.0.3 (`gurobi`), MOSEK 11.2 (`mosek`); dev-dependencies
unchanged (`libtest-mimic`, `cc`, `dhat`)

**Storage**: toy instance files and provenance in `tests/fixtures/small/`; comparison reports and
the LLM trial in `results/003-llm-interface-toy-instances/runs/`

**Testing**: documentation tests (`cargo test --doc`), drift test (`tests/standards.rs`), toy and
comparison trials (`tests/corpus.rs`), all tiers as feature 002 (no solver, licensed, heavy,
Rust 1.85)

**Target Platform**: 64-bit Linux

**Project Type**: library

**Performance Goals**: Gurobi QP load: SC-004 rule of features 001/002 (heap ≤ 32 B·(m + n) +
1 MiB plus the documented O(nnz(Q)) triplets) on the derived corpus QP; conic loading on Gurobi
bounded by a fixed buffer

**Constraints**: zero copy for LP and the A part of QP on Gurobi (unchanged); QP triplets and the
stacked [A; F] block buffer are the only proportional allocations (Complexity Tracking)

**Scale/Scope**: toy instances: n ≤ 6; corpus comparison: LP 1344 × 2942, QP same

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Pre-research: PASS. Post-design: PASS with two Complexity Tracking entries and one required
amendment (Notation Convention: z_l, z_u, added by feature 002 without one; maintainer decision
at Gate 2A).

- **Development scope**: [VI] all code (Rust, MATLAB generator, document, tests) in this
  feature; no override.
- **Performance and memory**: [I] Gurobi QP: O(nnz(Q)) triplets (12 B per entry); Gurobi conic:
  fixed-size block buffer (row indices and values of [A; F] per column block) plus O(k) for
  auxiliary-column data and O(d) per cone for its quadratic constraint; no change to the LP path
  or to MOSEK. Optional outputs unchanged.
- **Solver configuration audit**: [I] research R1–R5: Gurobi `NonConvex` = 0 with Q (fail on
  non-convexity instead of branching), `QCPDual` = 1 with cones (duals), `DualReductions` = 0
  with certificates (unchanged), barrier default for QP/SOCP (crossover unavailable), effective
  tolerances by class (R5). MOSEK unchanged.
- **Solver parameters in one location**: [VII-G] `gurobi::params::apply` sets `NonConvex` and
  `QCPDual` (conditioned on the problem class, passed in as MOSEK's is); raw route can override;
  reported in effective parameters.
- **Zero-copy inputs and bulk calls**: [IV] Gurobi QP: `GRBaddqpterms` one call (triplets built:
  scaled diagonal, Complexity Tracking). Gurobi conic: stacked blocked `GRBXaddvars` load,
  `GRBXaddvars` for auxiliaries, one `GRBaddqconstr` per quadratic/rotated cone (no bulk call
  exists, R2).
- **Solver-independent contract**: [XVI] (a) `ConeKind`, `Capabilities::cones` added
  (contracts/public-api.md); (b) Gurobi status translation unchanged (13 SUBOPTIMAL stays
  NumericalDifficulty, R5); (c) no new options fields; (d) feature 002 verification applies to
  Gurobi QP/conic solutions; Gurobi QP/conic infeasibility gives PartiallyVerified when
  certificates are requested (R4), documented.
- **Rust and FFI standards**: [VII] new Gurobi declarations (`GRBaddqpterms`, `GRBaddqconstr`,
  `GRBXaddconstrs` if used, constants `GRB_ERROR_Q_NOT_PSD` …) added to `src/gurobi/ffi.rs` and
  `tests/header_conformance.c`; unsafe only in `src/gurobi/`.
- **Independence and additive features**: [VIII] unchanged; the toy and comparison tests run in
  every feature combination (ignored where a backend is missing).
- **Testing and behavioural evidence**: [II] toy trials and comparison trials are written once
  in `tests/common/` against `Backend`; feature 002 per-backend QP and cone tests now run on
  Gurobi for its supported kinds (the dual-sign mapping R3 is asserted there); guard tests: the
  drift test fails on an unmentioned item; the comparison harness reports a forced disagreement
  as failed and a one-backend registry as ignored.
- **Diagnostics and statuses**: [III] unchanged.
- **Benchmarks and evidentiary preconditions**: [XIV] (a) iDopaNeuroC_problem_LP_minNorm (QP,
  1344 × 2942, nnz 13,748, nnz(Q) 2942; provenance tests/fixtures/qp) on Gurobi and MOSEK, both
  index types; (b) configuration recorded as feature 002; (c) phases and dedicated-process heap
  as feature 002; (d) positive control: the load report states the QP path (triplet count).
- **Reporting discipline**: [XV] N = 5 for benchmarks; comparison reports per run; the LLM trial
  records attempts and outcomes.
- **Notation**: [XII-C] no new mathematical objects except the internal Gurobi auxiliary u
  (internal, not in the interface); z_l, z_u (feature 002) added to the Notation Convention
  before tasks (maintainer, Gate 2A).
- **Provenance and licensing**: [X] toy instances project-authored (MIT OR Apache-2.0), each with
  provenance and independent-tool checks; no vendor files.
- **Project parameters**: `test_corpus.toy_instances` added to `project.yml`.
- **Spec-driven scope control**: [V] no new dependency; readers stay test-only (FR-010a).
- **Artifact placement**: [IX] `docs/llm-interface.md` (docs), toy files in `tests/fixtures/small/`,
  generator in `tools/corpus/`, reports in `results/`, `docs/llms.txt` in docs (maintainer, Gate
  2A: not at the root; the architecture model's root list is unchanged).
- **Architecture conformance**: [XIII] architecture-delta.yaml; the shared capability check
  gains cone kinds (core.options, core.backend), the Gurobi backend gains QP/conic loading.

## Architecture Impact

**Current architecture**: components.yaml after feature 002 (core, gurobi.*, mosek.*, test
harness with MPS and JTASK readers, corpus, tools, bench).

**Proposed delta** (architecture-delta.yaml): adds `docs.llm_interface`, `docs.llms_txt`,
`tools.toy_instances` (and `docs.llms_txt`); modifies `core.conic` (ConeKind), `core.backend` (Capabilities::cones),
`core.options` (per-cone check), `gurobi.load` / `gurobi.params` / `gurobi.solve` / `gurobi.ffi`
(QP and second-order cones), `test.harness` (QUADOBJ/QSECTION/QMATRIX, svecpsd, toy and comparison
trials), `test.corpus` (toy instances), `core.lib` (crate docs from the document),
`bench.load_solve` (QP scenario). No dependency direction changes.

**Alternatives considered**: public readers (rejected, FR-010a); Gurobi cones through
`GRBaddgenconstrNorm` or undocumented `GRBaddcone` (R2); duals via `QCPi` (R3); two copies of the
document (clarification).

**Reuse and adoption assessment**:

- *Owning component*: core.options owns the capability check (extended, not duplicated);
  gurobi.load owns loading (blocked path reused for the stacked matrix).
- *Inspected*: src/options/check.rs, src/backend.rs, src/conic.rs, src/gurobi/{load, solve,
  params, ffi}.rs, tests/common/{mps, jtask, mod, f002}.rs, feature 002 T034 cases.
- *Disposition*: extend; the feature 002 hand-built cone cases supply the toy optima.
- *Duplication left in place*: none new.
- *If a shared capability is introduced*: `Capabilities::cones` — consumers: `check_problem`
  (every backend), the comparison trials (comparability), the interface document.
- *Reachability and direction*: unchanged.
- *Cross-boundary contract*: backends expose cone kinds through `Capabilities` only.
- *Behavioural integration evidence*: every toy trial calls `Backend::solve` on both backends.

**Verdict**: APPROVE_WITH_ADR, 2026-11-15 (maintainer). ADR-0003. `llms.txt` in `docs/`;
Notation Convention gains z_l, z_u.

## Project Structure

```text
docs/llm-interface.md                  # the interface document = crate-level docs
docs/llms.txt                          # LLM entry point
src/lib.rs                             # #![doc = include_str!(...)]
src/conic.rs, src/backend.rs, src/options/check.rs        # ConeKind, Capabilities::cones
src/gurobi/{ffi, load, params, solve}.rs                  # QP and second-order cones
src/mosek/solve.rs                     # Capabilities::cones = all kinds
tests/common/{mps, jtask, toy, mod}.rs # readers extended; toy and comparison trials
tests/standards.rs                     # drift test
tests/fixtures/small/                  # toy instances + provenance
tests/header_conformance.{rs, c}       # new Gurobi declarations
tools/corpus/make_toy_instances.m      # generator and independent checks
.specify/memory/project.yml            # test_corpus.toy_instances
docs/test-corpus.md                    # toy instances described
benches/load_solve.rs                  # QP scenario
```

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Gurobi QP triplets O(nnz(Q)) | `GRBaddqpterms` takes triplets only and the diagonal must be halved | No slice form; scaling in place would modify the caller's data |
| Gurobi conic stacked block buffer (indices and values) | Gurobi has no affine conic rows; F's entries must join A's columns | Per-row calls violate IV; `GRBaddcone` is undocumented |
