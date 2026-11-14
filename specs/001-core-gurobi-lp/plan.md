# Implementation Plan: Solver-Independent Core And Gurobi LP Backend

**Branch**: `001-core-gurobi-lp` | **Date**: 2026-11-14 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/001-core-gurobi-lp/spec.md`

## Summary

Create the single crate `optsolver` with a solver-independent core (borrowed CSC problem,
options structure, generic result with a named status vocabulary, independent verification)
and a Gurobi LP backend that bulk-loads through `GRBXloadmodel` / `GRBXaddvars` with
optSolver's own header-checked declarations. Extract the frozen iDopaNeuroC flux-balance LP instance (problem 1; corpus decision 2026-11-14)
from the variational kinetics repository into MPS fixtures with provenance, using that
repository's corpus locator and Gurobi's MATLAB writer, and test everything through one shared
harness that runs each test once per backend. Benchmark load, solve, extraction and
verification per instance. Design decisions: research.md R1–R15; entities: data-model.md.

## Technical Context

**Language/Version**: Rust, edition 2024, minimum supported version 1.85 (reference machine
1.91.1); MATLAB (extraction script only)

**Primary Dependencies**: none at run time beyond the Gurobi 13.0.3 shared library (feature
`gurobi`, linked at build time via `GUROBI_HOME`) and the `log` facade. Dev-dependencies:
`libtest-mimic` (harness with run-time ignore, R14), `cc` (header conformance test, R12).

**Storage**: MPS fixtures and JSON provenance records in the test fixtures directory; benchmark
run bundles in the generated-output home

**Testing**: `cargo test` (unlicensed tier), `cargo test --features gurobi` (licensed tier),
custom harness (R14)

**Target Platform**: 64-bit Linux (reference); 64-bit targets only, asserted at compile time

**Project Type**: library

**Performance Goals**: SC-004 (≤ 32 B per row and column + 1 MiB extra memory during load, no
per-nonzero term), SC-005 (optSolver's added time ≤ 5% of Gurobi solve time), per instance,
N = 5

**Constraints**: zero-copy inputs; fixed-size index buffer (2^18 entries) for 64-bit indices;
solver log off by default; no printing

**Scale/Scope**: corpus up to 1344 × 2942, 13,748 nonzeros; design valid to dimensions of
2^31 − 1 and nonzero counts beyond 2^32

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design. Answer each bullet;
write N/A only where stated.*

Pre-research check: PASS. Post-design re-check (after research.md, data-model.md, contracts):
PASS with two Complexity Tracking entries and one required amendment (Notation, below).

- **Development scope**: [VI] All code — library, FFI, build script, header conformance C file,
  MATLAB extraction script, MPS reader, harness, benchmarks — is inside this feature. No
  SPECKIT OVERRIDE.
- **Performance and memory**: [I] Targets SC-004, SC-005. Allocations proportional to nonzeros:
  none (64-bit indices converted through a fixed 1 MiB buffer, R2). O(m + n) allocations: result
  arrays (required), Gurobi sense array when rows are given as ranges, auxiliary columns for
  ranged rows (O(ranged rows)), infinite-bound mapping copy of l, u or b only when an infinite
  value is present. Environment reuse: `Env` created once, `solve` takes `&Env`. Model
  modification without reload: not in this feature (decision 19); the backend keeps the Gurobi
  model handle internal so a persistent model can be added without changing the result types.
  All available outputs returned by default; extraction cost measured (US4). Validation always
  on; `new_unchecked` is the only opt-out.
- **Solver configuration audit**: [I] research.md R10: corpus profile small, sparse, equality
  rows, finite bounds; no mismatched default; only default override `OutputFlag` = 0.
- **Solver parameters in one location**: [VII-G] Options type and defaults in
  `src/options.rs`; Gurobi parameters applied only by `gurobi::params::apply` in
  `src/gurobi/params.rs`, the only caller of `GRBsetintparam`/`GRBsetdblparam`/
  `GRBsetstrparam`; raw passthrough typed (R5); conflicts are errors; `apply` returns the
  effective parameter set recorded in every result. A test greps the crate for the setter
  symbols outside `params.rs`.
- **Zero-copy inputs and bulk calls**: [IV] Route: optSolver's own declarations (R1: all three
  existing crates fail IV). Arrays borrowed as is: column pointers (`usize` = `size_t`), values,
  c, l, u, b; `i32` row indices passed straight through. Converted: `usize` row indices (fixed
  buffer, fused with the range check), senses when rows are given as ranges (O(m)), infinite
  values (only when present). Bulk calls only: `GRBXloadmodel`, `GRBXaddvars`,
  `GRBgetdblattrarray`, `GRBgetintattrarray`.
- **Solver-independent contract**: [XVI] (a) one `Problem`, `Options`, `Solution` for every
  backend; solver-specific data only in `raw_status`, `solver`, `attributes`; representation
  decided here (borrowed struct, data-model.md). (b) `gurobi::status::translate`, total over
  codes 1–19 with `Unknown` otherwise (R4). (c) shared fields per data-model.md; unsupported
  field and double-route are errors. (d) verification measures per data-model.md, used by every
  licensed test through the shared harness.
- **Rust and FFI standards**: [VII-A–F] `unsafe` only in `src/gurobi/ffi.rs` and the callers in
  `src/gurobi/` that pass pointers; every other module carries `#![forbid(unsafe_code)]`.
  `Env` owns `NonNull<GRBenv>` and frees in `Drop`; the model is created and freed inside one
  solve call. No `Send`/`Sync` (R15). Callback body inside `catch_unwind` (R7). Validation in
  `CscRef::new` / `Problem::new` before any FFI. Vendor declarations checked against the header
  by the conformance test (R12); `GRBloadenv` is a macro, so `GRBloadenvinternal` is called
  with version numbers from `GRBversion` (design-notes §9). Non-`const` pointer inputs:
  **verify** input-only before use (task). Lints per VII-F in `Cargo.toml`; `links =
  "gurobi"`; `build.rs` reads only `GUROBI_HOME` and declares it.
- **Independence and additive features**: [VIII] No RunMat or sparse-crate dependency. One
  feature, `gurobi`, additive. Core builds and tests with no Gurobi.
- **Testing and behavioural evidence**: [II] All corpus, contract and certificate tests written
  once in the shared harness and registered per backend (R14); backend-specific tests: status
  translation table and header conformance (outside the contract, stated). Fixtures: corpus MPS
  files with provenance (R11). Failing-input tests for every validation rule. Cross-solver
  agreement: N/A until a second backend. Real-caller evidence: corpus tests call
  `Backend::solve` exactly as a user would.
- **Diagnostics and statuses**: [III] `log` facade only; `OutputFlag` 0 by default; raw and
  solver-independent statuses both returned; Gurobi error code and `GRBgeterrormsg` text read
  immediately.
- **Benchmarks and evidentiary preconditions**: [XIV] (a) the named corpus instance(s) with
  dimensions and provenance. (b) effective parameters, Gurobi version, commit, machine, profile
  recorded per run. (c) environment creation, load, solve, extraction, verification timed
  separately; cold (first solve in a fresh process) and warm labelled; memory per instance in a
  dedicated process (R8). (d) positive control: the benchmark records which load path ran
  (single `GRBXloadmodel` vs blocked `GRBXaddvars`) and the buffer high-water mark.
- **Reporting discipline**: [XV] N = 5 per benchmark figure, median and spread, set in the
  benchmark binary; corpus solves N = 1, stated. Guards: the verifier is tested against
  deliberately violated solutions and against an all-zero "everything failed" solution; the
  harness's skip path is tested by running with the feature off. Run output redirected to the
  run bundle's log. No repository-wide rule added except the placement amendment below.
- **Provenance and licensing**: [X] Fixtures: provenance records (data-model.md); licence "not
  verified" by maintainer decision. Crates `log`, `libtest-mimic`, `cc`: MIT OR Apache-2.0
  (**verify** at dependency addition). No vendor files or generated bindings committed.
- **Project parameters**: corpus instances and fixture directory from `project.yml`
  (`test_corpus`); Gurobi version from `domain.solvers.gurobi`; no absolute paths; Gurobi and
  variational kinetics locations resolved at run time.
- **Spec-driven scope control**: [V] Paths edited: `Cargo.toml`, `build.rs`, `src/`, `tests/`,
  `benches/`, the extraction script directory (below), `docs/architecture/`. Read-only: the
  variational kinetics repository. New dependencies justified above.
- **Notation**: [XII-C] New symbols γ, b_l, b_u, y, z, σ added to the Notation Convention
  (constitution 2.4.0, Gate 2A).
- **Artifact placement**: [IX] The placement document has no role for maintenance scripts
  such as the MATLAB extraction script. Per IX-F the gap is reported here: the architecture
  delta proposes a new role "Maintenance tooling → `tools/<purpose>/`", to be added to
  `docs/architecture/repo_folder_organisation.md` on approval. Fixtures go to the test fixtures
  directory; benchmark output to the generated-output home, bundled per run.
- **Architecture conformance**: [XIII] `architecture-delta.yaml` written; ADR required
  (introduces the crate and its first layers; couples to a solver) — `APPROVE_WITH_ADR`
  expected. No ownership asserted in agent files. Naming decisions for new elements (`Backend`,
  `Env`, `verify`) recorded in the delta for the maintainer to confirm (XIII-F).

## Architecture Impact

*Derived from `architecture-delta.yaml` — the YAML is authoritative.*

**Current architecture**: no source exists; `components.yaml` is empty.

**Proposed delta**: adds components `core.problem`, `core.options`, `core.result`,
`core.verify`, `core.backend` (provisional trait), `gurobi.ffi`, `gurobi.env`, `gurobi.load`,
`gurobi.params`, `gurobi.status`, `gurobi.callback`, `gurobi.solve`, `test.harness`,
`test.corpus`, `tools.corpus_extraction`, `bench.load_solve`. Dependency direction: `gurobi.*`
→ `core.*`; `core.*` never → `gurobi.*`; only `gurobi.ffi` declares C functions; only
`gurobi.params` calls vendor parameter setters. Public interfaces introduced: data
contracts/public-api.md.

**Alternatives considered**: existing binding crates (rejected, R1); separate `-sys` crate
(rejected: single-crate decision); a backend enum instead of a trait (smaller, but the harness
then needs editing for every backend — rejected in favour of a provisional trait, Complexity
Tracking).

**Reuse and adoption assessment**:

- *Owning component*: none in optSolver (no source). Fixture extraction reuses the variational
  kinetics corpus locator.
- *Inspected*: variational kinetics `src/matlab/experiment/corpus/vkProblemInstanceDir.m`
  (whole file); `docs/solver-internals.md` "The Frozen Problem Corpus"; instance contents
  (MATLAB `whos`, field list); COBRA `src/base/io/writeLPProblem.m`, `convertCobraLP2mps.m`
  (`writeLPProblem` → `BuildMPS` read 2026-11-14: 12-character number fields, L/G/E row
  reordering; not reused, maintainer decision, research R11); Gurobi MATLAB `gurobi_write`,
  `gurobi_read` (present); Rust crates
  `gurobi-rs` 1.1.0, `gurobi-sys` 0.3.0, `grb` 3.0.1, `grb-sys` 0.2.1 (sources).
- *Disposition*: reuse `vkProblemInstanceDir`, `gurobi_write`, `gurobi_read`; introduce new
  Rust code for everything else (nothing reusable passes IV).
- *Duplication left in place*: the MPS reader in the test module duplicates what Gurobi can
  read; it is a deliberate independent path (fixtures are checked by a reader that is not the
  solver under test).
- *If a shared capability is introduced*: the `Backend` trait and the shared test harness.
  *Named consumers*: the Gurobi backend and the corpus tests (this feature); MOSEK (next
  feature). *Representative consumer test*: corpus solve tests. *Replaced implementations*:
  none.
- *Reachability and direction*: all in one crate; core modules `pub`, backend `pub mod gurobi`
  behind the feature; FFI module `pub(crate)`. No cycle.
- *Cross-boundary contract*: the harness consumes backends only through `Backend`.
- *Behavioural integration evidence*: corpus tests drive `Backend::solve`; the parameter test
  reads back effective parameters from Gurobi.

**Verdict**: APPROVE_WITH_ADR, 2026-11-14 (maintainer, Gate 2A). ADR:
`docs/architecture/decisions/ADR-0001-crate-layering-and-solver-independent-contract.md`. Names
accepted as proposed; Notation Convention amended (constitution 2.4.0); placement role
"Maintenance tooling" added.

## Project Structure

### Documentation (this feature)

```text
specs/001-core-gurobi-lp/
├── plan.md, research.md, data-model.md, quickstart.md
├── contracts/public-api.md
├── architecture-delta.yaml
├── checklists/requirements.md, checklists/contract.md
├── human-loop.md
└── tasks.md             # after Gate 2A approval
```

### Source Code (repository root)

```text
Cargo.toml
build.rs                      # feature gurobi: link libgurobi130 from GUROBI_HOME
src/
├── lib.rs
├── problem.rs  options.rs  result.rs  verify.rs  backend.rs  error.rs
└── gurobi/
    ├── mod.rs  ffi.rs  env.rs  load.rs  params.rs  status.rs  callback.rs  solve.rs
tests/
├── common/                   # the one shared test module: MPS reader, SplitMix64, harness, assertions
├── contract.rs  corpus.rs    # harness = false (libtest-mimic)
├── header_conformance.rs     # feature gurobi, needs GUROBI_HOME
└── fixtures/                 # 7 MPS files + provenance JSON + small hand-built fixtures
benches/
└── load_solve.rs             # one scenario per process for memory
tools/
└── corpus/extract_idopaneuroc_fixtures.m   # proposed placement role (Gate 2A)
```

**Structure Decision**: single crate at the repository root (maintainer decision); backend as a
feature-gated module.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| `Backend` trait with one implementation (constitution V: no cross-backend abstraction before two backends) | Constitution II requires tests written once and run against every backend; the harness needs one call shape now | A backend enum needs harness edits per backend; testing Gurobi directly would have to be rewritten for MOSEK. Kept provisional: re-examined when MOSEK implements it |
| New dev-dependencies `libtest-mimic`, `cc` | Run-time ignored reporting (SC-002) and header conformance (VII-E) | `#[ignore]` cannot depend on a run-time licence check; hand-maintained declarations without a header check violate VII-E |
