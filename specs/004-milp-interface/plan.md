# Implementation Plan: Mixed-Integer Programmes Through The Existing Interface

**Branch**: `004-milp-interface` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/004-milp-interface/spec.md`

## Summary

Admit integer and binary variables through the existing problem (`with_var_types`), options,
result, verification, backend trait, tests, toy instances, comparison, document and benchmark,
extending each only where integrality requires: per-backend mixed-integer capabilities expressed
with the existing `ProblemClass` and `ConeKind` types; five MIP option fields and a fixed-model
duals switch on `Options`; the gap computed from the existing `objective` and `bound`;
integrality and gap verification measures; bulk loading of variable types (zero copy on Gurobi);
fixed-model duals through the solvers' own facilities and the existing extraction code. Both
backends solve MILP, MIQP and mixed-integer conic problems (Gurobi: its four cone kinds; MOSEK:
every kind except PSD). Decisions: research.md R1–R12.

## Technical Context

**Language/Version**: Rust 2024, minimum 1.85; MATLAB (toy generation only)

**Primary Dependencies**: Gurobi 13.0.3, MOSEK 11.2; no new dependency

**Storage**: toy files in `tests/fixtures/small/`; derived MILP provenance in
`tests/fixtures/milp/`; run bundles in `results/004-milp-interface/runs/`

**Testing**: existing tiers and harness (contract, corpus with toy and comparison trials,
standards, doctests, header conformance, heavy, Rust 1.85)

**Target Platform**: 64-bit Linux

**Project Type**: library

**Performance Goals**: load memory: SC-004 (at most 1 byte per column beyond features 001–003;
zero on Gurobi, which takes the caller's bytes); overhead rule of feature 001 on the MILP benchmark

**Constraints**: no per-column vendor calls; fixed-model duals cost one extra continuous solve,
only when requested

**Scale/Scope**: toy MI instances (n ≤ 10); benchmark: derived iDopaNeuroC_problem_LP_int
(1344 × 2942)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Pre-research: PASS. Post-design: PASS; one Complexity Tracking entry (MOSEK integer column list).

- **Development scope**: [VI] all code in this feature; no override.
- **Performance and memory**: [I] Gurobi: variable types borrowed (zero copy); MOSEK: index and
  type arrays for integer columns only (8 B per integer column) and bound copies only when a
  binary column's bounds lie outside [0, 1]; fixed-model duals only on request (one extra
  continuous solve, a model/task copy made by the solver).
- **Solver configuration audit**: [I] research R4–R5: solver defaults for gap tolerances and
  limits; `DualReductions` = 0 not set for mixed-integer problems (it corrupts unbounded MILP
  results, *probe*); `NonConvex` = 0 kept for QP (research R1 notes Gurobi may still solve a
  non-convex MIQP).
- **Solver parameters in one location**: [VII-G] every new parameter is set only in each backend's
  `params::apply`; raw route can override.
- **Zero-copy inputs and bulk calls**: [IV] Gurobi vtype from the caller's slice (`GRBXloadmodel`,
  `GRBXaddvars` per block); MOSEK `MSK_putvartypelist` once.
- **Solver-independent contract**: [XVI] (a) extensions of existing types only
  (contracts/public-api.md); (b) status translations extended (MOSEK INTEGER_OPTIMAL); (c) new
  shared fields mapped on both backends; (d) integrality and gap measures; fixed-problem duals
  judged against the fixed problem.
- **Rust and FFI standards**: [VII] new declarations (`GRBfixmodel`, `GRBsetdblattrlist`, Gurobi
  MIP callback constants; `MSK_putvartypelist`, `MSK_clonetask`, `MSK_putvarboundlist`, MIO
  constants) added to each `ffi.rs` and its header-conformance file; Gurobi's vtype pointer from a
  shared slice uses the VII-A exception (manual + probe, licensed test).
- **Independence and additive features**: [VIII] unchanged.
- **Testing and behavioural evidence**: [II] toy and comparison trials (existing) gain MI
  instances; per-backend tests for limits, fixed-model duals and unsupported combinations are
  written once in `tests/common/`; guard self-test: an integrality violation and a gap violation
  fail verification.
- **Diagnostics and statuses**: [III] unchanged; raw codes kept.
- **Benchmarks and evidentiary preconditions**: [XIV] (a) iDopaNeuroC_problem_LP_int (derived,
  provenance); (b) configuration recorded; (c) phases and dedicated-process memory as before; (d)
  positive control: the load report states the number of integer columns loaded.
- **Reporting discipline**: [XV] N = 5; comparison report per run.
- **Notation**: [XII-C] no new mathematical symbols (gap and bound are named, not new symbols in
  the inventory; b is not reused for the bound in code, which uses `bound`).
- **Provenance and licensing**: [X] new toy instances project-authored with provenance.
- **Project parameters**: `test_corpus.toy_instances` extended; `milp_instances` added.
- **Spec-driven scope control**: [V] no new dependency; the reuse assessment below.
- **Artifact placement**: [IX] as features 002–003.
- **Architecture conformance**: [XIII] architecture-delta.yaml.

## Architecture Impact

**Current architecture**: components.yaml after feature 003.

**Proposed delta** (architecture-delta.yaml): no new components; modifies core.options,
core.backend, core.result, core.verify, core.lib (document), gurobi.{ffi, load, params, solve,
callback}, mosek.{ffi, load, params, solve, status, callback}, test.harness (readers, toy,
compare), test.corpus, tools.toy_instances, docs, bench.load_solve.

**Alternatives considered**: new problem-class variants per mixed-integer combination (R2);
a separate MILP result type with gap fields (rejected: the bound field and a method suffice);
optSolver-side convexity checks (not done, feature 002 decision).

**Reuse and adoption assessment**:

- *Owning component*: core.backend (capabilities), core.options (shared check and options),
  core.verify (measures), each backend's load/params/solve, the test harness.
- *Inspected*: src/problem.rs (`with_var_types`, validation), src/backend.rs, src/options.rs,
  src/options/check.rs, src/result.rs, src/verify.rs, src/gurobi/{load, load_qc, params, solve,
  status, callback}.rs, src/mosek/{load, params, solve, status, callback}.rs,
  tests/common/{mps, jtask, toy, compare, f002}.rs, tools/corpus/make_toy_instances.m.
- *Disposition*: extend every one; no parallel code path. New public items and why reuse was not
  possible (SC-003): `Capabilities::integer_classes`/`integer_cones` (new fields of existing
  types; no field could hold per-class integer support), five option fields (no existing field
  maps to gap tolerances or node and solution limits), `fixed_model_duals` (new behaviour),
  `Counts::nodes` (no count of nodes), `Tolerances::mip_relative_gap`/`mip_absolute_gap` (no gap
  tolerance), `ProgressPhase::BranchAndBound` (no phase fits), `Solution::mip_gap` (a method over
  existing fields), measure names `INTEGRALITY`, `MIP_GAP`. No new type, module or test framework.
- *Duplication left in place*: none new; fixed-model extraction reuses each backend's extraction.
- *If a shared capability is introduced*: the mixed-integer capability fields — consumers:
  `check_problem` (every backend), toy and comparison trials (comparability), the document.
- *Reachability and direction*: unchanged.
- *Cross-boundary contract*: backends declare mixed-integer support only through `Capabilities`.
- *Behavioural integration evidence*: toy and comparison trials call `Backend::solve` with
  mixed-integer problems on both backends.

**Verdict**: APPROVE_WITH_ADR, 2026-11-15 (maintainer). ADR-0004.

## Project Structure

```text
src/options.rs, src/options/check.rs, src/backend.rs, src/result.rs, src/verify.rs   # extended
src/gurobi/{ffi, load, params, solve, callback}.rs                                    # vtype, MIP, fixmodel
src/mosek/{ffi, load, params, solve, status, callback}.rs                             # vartypes, ITG, clone
tests/common/{mps, jtask, toy, compare, f002}.rs, tests/{corpus, contract, standards}.rs
tests/header_conformance{.c, _mosek.c}
tests/fixtures/small/toy_mi*  tests/fixtures/milp/iDopaNeuroC_problem_LP_int.provenance.json
tools/corpus/make_toy_instances.m, docs/{llm-interface, test-corpus}.md, benches/load_solve.rs
.specify/memory/project.yml
```

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| MOSEK integer-column index and type arrays (8 B per integer column) | `MSK_putvartypelist` takes index and enum arrays; MOSEK has no byte type array | Per-column calls violate IV |
