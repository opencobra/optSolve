# Human Loop State

## Current State
- Status: Gate 3 passed (commit, demote iDopaNeuroC_problem_5); feature 002 implemented and committed
- Active feature directory: specs/002-mosek-lp-qp-clp
- Last completed bundle: Bundle 2 (implementation preparation)
- Source code modified by this workflow: yes (uncommitted)

## Core Command Ledger
- constitution:   checked (2.4.1)
- specify:        invoked 2026-11-15 (1 clarification: CLP test problems)
- clarify:        invoked 2026-11-15 (2 questions: cone types broader set; heavy test tier)
- checklist:      invoked 2026-11-15 (checklists/contract.md, 23 items: 9 resolved, 14 routed)
- plan:           invoked 2026-11-15 (plan.md, research.md R1-R15, data-model.md, contracts/public-api.md, quickstart.md, architecture-delta.yaml; two read-only research agents)
- tasks:          invoked 2026-11-15 (40 tasks)
- analyze:        invoked 2026-11-15 (0 critical; 1 low fixed)
- implement:      invoked 2026-11-15 (T001–T040; two MATLAB/reader agents; receipt specs/002-mosek-lp-qp-clp/agent-runs/20261115T103954Z-implement-002/implementation-receipt.md)

## Human Decisions
| Date (UTC) | Gate | Option chosen | Consequence |
|---|---|---|---|
| 2026-11-15 | (before Bundle 1) | Start feature 002: MOSEK LP, QP, CLP; rename LP fixture and document corpus; verify certificate sign and callback threading as code behaviour; build on Rust 1.85 | Branch 002-mosek-lp-qp-clp from devel |
| 2026-11-15 | Gate 3 | Commit; demote iDopaNeuroC_problem_5 to an expected verification failure (verifier must catch it) | project.yml clp_expected_verification_failures; test asserts the row-bound failure |
| 2026-11-15 | (implementation) | Per-item complementary slackness: required for basic solutions only (after comparing MOSEK and Gurobi) | verify.rs; research R19 |
| 2026-11-15 | (implementation) | Gap: research MOSEK's way; then split bound multipliers z_l, z_u and MOSEK's normalisation | result.rs, verify.rs, both backends; research R19 |
| 2026-11-15 | (implementation) | Conic complementarity per cone, not summed | verify.rs; research R19 |
| 2026-11-15 | (implementation) | CLP fixture format JTASK (CBF does not round-trip) | project.yml, tests/common/jtask.rs; research R19 |
| 2026-11-15 | Gate 2 | Approve all tasks; commit authorised | Scope T001–T040 |
| 2026-11-15 | Gate 2A | APPROVE_WITH_ADR; CLP problem = solveCobraCLP input; exclude iDopaNeuroC_problem_4; accept names, notation (2.5.0), extend licence decision to VK CLP data; commit | ADR-0002; constitution 2.5.0; spec FR-014 amended; tasks may be generated |
| 2026-11-15 | Gate 1 | Continue to Bundle 2; commit authorised | Bundle 1 committed; speckit-plan next |
| 2026-11-15 | (specify) | CLP test problems: first inner conic problem of each VK corpus instance, iCoreED and iDopaNeuroC | Spec FR-014 |

## Approved Implementation Scope
- Approved: yes (Gate 2, 2026-11-15); edits only after the implementation invocation
- Scope: all
- Tasks approved: T001–T040
- Tasks deferred: none
- Implementation baseline (2026-11-15, /speckit-implement): T001–T040, 40 unchecked
- Files allowed: as in implementation-review.md, "Files likely to change"
- Files not allowed: .specify/ governance (except project.yml), literature/, specs/000-*, specs/001-*, the variational kinetics repository

## Pointers
- Exploration: none (direct request); background specs/000 and specs/001 closeout
- Implementation receipt(s): specs/002-mosek-lp-qp-clp/agent-runs/20261115T103954Z-implement-002/implementation-receipt.md
- Implementation review: specs/002-mosek-lp-qp-clp/implementation-review.md

## Open Risks and Ambiguities
- Bundle 0: on devel, clean tree, no remotes; feature 001 integrated; MOSEK 11.2 and licence present.
- Rust 1.85 check (2026-11-15, before this feature): library and integration tests build and pass on 1.85; the loader's unit tests and the benchmark do not compile (cast_unsigned, stable from 1.87).

## Checklist Routing (constitution VI-B)

checklists/requirements.md 16/16. checklists/contract.md: the 14 routed items below are all
resolved in Bundle 2 (23/23); closing artifact named on each item:

| Item | Question | Destination |
|---|---|---|
| CHK001 | QP objective convention | data-model.md |
| CHK002 | Conic dual conventions per cone | data-model.md |
| CHK003 | PSD vectorised layout | data-model.md |
| CHK004 | Power-cone exponents | data-model.md |
| CHK005 | Verification measures per cone type | data-model.md |
| CHK008 | Operational definition of "first inner conic problem" | research.md |
| CHK009 | How expected CLP statuses are obtained | research.md |
| CHK010 | Closed-form infeasible problem and expected signs | data-model.md |
| CHK011 | Observable for "never concurrently" | data-model.md |
| CHK012 | MOSEK status translation source of truth | research.md |
| CHK015 | Cross-solver agreement tolerance | research.md |
| CHK017 | Driver-built QP definition | research.md |
| CHK020 | Non-convex QP behaviour | data-model.md |
| CHK021 | MOSEK binding route | research.md |

## Implementation Continuation Log
- 2026-11-15, cycle 1: unchecked approved tasks 40 → 0. Stop-and-report conditions: none
  recorded. Tests introduced: all pass except one heavy-tier test
  (`f002::t033_clp_fixture_heavy::iDopaNeuroC_problem_5::mosek`, a genuine feasibility finding,
  research R19), so predicate 3 does not fully hold. Decision: no resumption (no task
  remains); in-flight marker removed; reported to the maintainer at Gate 3 with that condition.
