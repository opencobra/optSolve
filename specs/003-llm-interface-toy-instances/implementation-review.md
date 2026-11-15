# Implementation Review

## Summary

Feature 003 adds an interface document for client authors and LLMs (one source: Markdown and
crate docs, compiled examples, drift test, `docs/llms.txt`), committed toy instances for every
problem class and cone kind (MPS, JTASK) built and checked with both solver tools, the Gurobi QP
and second-order-cone extension, and MOSEK–Gurobi comparison trials with a report.

## Embedded Core Commands Completed
- constitution: checked 2.5.0; amended to 2.5.1 (z_l, z_u) at Gate 2A
- specify: done (2 clarifications at specify time)
- clarify: done (2 questions)
- checklist: requirements 16/16; contract 23/23 (9 routed items closed in Bundle 2)
- plan: done (research R1–R12 with Gurobi C and MATLAB probes; data model; contract; quickstart)
- architecture review: Gate 2A APPROVE_WITH_ADR
- tasks: 26 tasks
- analyze: 0 critical, 1 medium (C1, applied to T017), 3 low (accepted)

## Architecture Verdict (Gate 2A)
- Verdict: APPROVE_WITH_ADR
- ADR required / written: ADR-0003 (Gurobi conic modelling and dual mapping, per-cone
  capabilities, interface document; llms.txt in docs/)
- Date (UTC): 2026-11-15

## Cross-Artifact Analysis Summary
All 17 functional requirements and 6 success criteria map to tasks; tests precede code; no
constitution conflict. C1 (certificates requested in comparisons of infeasible/unbounded
instances) applied to T017. Accepted: I1 (MATLAB interfaces count as independent tools), U1 (no
Gurobi QP callback-thread rerun), T1 (wording).

## Proposed Implementation Scope
- Tasks proposed: T001–T026
- First independently testable slice: Setup + Foundational + US1 (T001–T011): per-cone
  capabilities, readers, and the interface document with compiled examples and drift test
- Files likely to change: `.specify/memory/project.yml`; `src/{lib, conic, backend}.rs`,
  `src/options/check.rs`, `src/gurobi/{ffi, load, params, solve}.rs`, `src/mosek/solve.rs`;
  `tests/common/{fixtures, mps, jtask, f002, mod}.rs`, new `tests/common/{toy, compare}.rs`,
  `tests/{corpus, contract, standards}.rs`, `tests/contract/qp_conic.rs`,
  `tests/header_conformance.c`; `tests/fixtures/small/*`; `tools/corpus/make_toy_instances.m`;
  `docs/{llm-interface.md, llms.txt, test-corpus.md}`, `docs/architecture/components.yaml`;
  `README.md`; `benches/load_solve.rs`; feature 003 spec artifacts
- Files that should NOT change: other `.specify/` governance (constitution already amended),
  `literature/`, `specs/000-*`, `specs/001-*`, `specs/002-*`, the variational kinetics repository,
  the LP/QP/CLP corpus fixtures

## Tests and Validation Expected (name the narrowest relevant test first)
1. `cargo test --test contract` (ConeKind, per-kind capability check) — no licence
2. `cargo test --test corpus -- mps jtask` (reader self-tests) — no licence
3. `cargo test --test standards` (drift test and its self-test)
4. `cargo test --doc` and `cargo test --doc --features gurobi,mosek` (document examples)
5. `cargo test --features gurobi,mosek --test corpus -- toy f002 compare` (toy, Gurobi QP/SOC,
   comparison; report written)
6. Header conformance; full in-gate checks (T022); benchmark (T021); LLM trial (T024)

## Blocking Issues
None.

## Acceptable Risks
- Gurobi gives no certificates for QP and conic problems (vendor limitation): infeasible
  instances verify as PartiallyVerified on Gurobi when certificates are requested; documented.
- The JTASK domain names come from MOSEK's writer output, not its JSON documentation (R7).
- Gurobi's conic load builds a fixed-size block buffer of values as well as indices (Complexity
  Tracking).
- The LLM trial depends on the agents' model; recorded, not automated.

## Human Approval
- Approved: yes
- Approved option: Approve all tasks and commit
- Approved tasks/scope: T001–T026
- Required implementation invocation per constitution: `/speckit-implement`
- Date (UTC): 2026-11-15
