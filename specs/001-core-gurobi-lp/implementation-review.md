# Implementation Review

## Summary

Feature 001 creates the `optsolver` crate: a solver-independent core (validated borrowed
problem, options structure, generic result with named statuses, independent verification) and a
Gurobi LP backend with header-checked declarations and bulk loading; seven iDopaNeuroC MPS
fixtures extracted from the variational kinetics corpus; one shared test harness; benchmarks.
No source code exists yet; no source code has been modified by this workflow.

## Embedded Core Commands Completed

- constitution / specify / clarify / checklist / plan / architecture review / tasks / analyze:
  checked (2.3.0, amended to 2.4.0 at Gate 2A) / invoked / invoked (3 questions) / invoked
  (contract.md 33/33) / invoked / APPROVE_WITH_ADR / invoked (57 tasks) / invoked (0 critical,
  4 medium, 5 low)

## Architecture Verdict (Gate 2A)

- Verdict: APPROVE_WITH_ADR
- ADR required / written: yes / `docs/architecture/decisions/ADR-0001-crate-layering-and-solver-independent-contract.md`
- Date (UTC): 2026-11-14

## Cross-Artifact Analysis Summary

| Finding | Class | Proposed handling |
|---|---|---|
| C1 caller buffers (`solve_into`) untested | should-fix | add a per-backend test in US3 |
| C2 problem-class rejection untested | should-fix | add a per-backend test in US3 |
| C3 small fixtures lack provenance | should-fix (constitution II, X) | T031 writes provenance JSON |
| C4 tracking of `results/` undeclared | should-fix (constitution IX-C) | add a setup task declaring it in `.gitignore` |
| U1 clippy inside a test; `--all-features` needs `GUROBI_HOME` | should-fix | standards test searches only; clippy in T050 |
| U2 MATLAB reading `project.yml` | acceptable | use `project_params.py --json` |
| I1 "skipped" vs "ignored" | acceptable | wording note |
| I2 `DualReductions` override | acceptable | covered by conflict rule |
| F1 lettered task IDs | acceptable | none |

## Proposed Implementation Scope

- Tasks proposed: T001–T054 (57 tasks), after the should-fix edits above
- First independently testable slice: Setup + Foundational + User Story 1 (T001–T019): the
  frozen corpus and harness, testable with no solver
- Files likely to change: `Cargo.toml`, `build.rs`, `LICENSE-MIT`, `LICENSE-APACHE`,
  `README.md`, `.gitignore`, `src/**`, `tests/**`, `benches/load_solve.rs`,
  `tools/corpus/extract_idopaneuroc_fixtures.m`, `docs/architecture/components.yaml`,
  `specs/001-core-gurobi-lp/{research.md, benchmark-results.md, agent-runs/}`,
  `results/001-core-gurobi-lp/`
- Files that should NOT change: `.specify/` (governance), `literature/`, `specs/000-*`, the
  variational kinetics repository (read-only)

## Tests and Validation Expected (name the narrowest relevant test first)

1. `cargo test --test corpus` (fixtures, provenance, MPS reader, seeded objectives) — no solver
2. `cargo test --test contract` (validation, verification, options rules) — no solver
3. `cargo test --features gurobi` (licensed tier: corpus solves, certificates, status
   translation, parameters, callbacks, header conformance)
4. `cargo bench --features gurobi --bench load_solve` (SC-004, SC-005)
5. In-gate checks: fmt, clippy with warnings denied, feature combinations, MSRV

## Blocking Issues

None.

## Acceptable Risks

- Vendor-documentation facts marked **verify** in research.md are checked by T038 before the
  FFI code that depends on them.
- The `Backend` trait is provisional (ADR-0001).
- MSRV 1.85 may not be installed; then recorded as unverified.

## Human Approval

- Approved: yes (intent recorded; edits await the constitution's implementation invocation)
- Approved option: Approve all tasks
- Approved tasks/scope: T001–T054 as they stand; the should-fix findings C1–C4 and U1 are
  handled during implementation and recorded as deviations in the receipt
- Required implementation invocation per constitution: `/speckit-implement` (or "Run the Spec
  Kit implementation phase for the active feature")
- Date (UTC): 2026-11-14
