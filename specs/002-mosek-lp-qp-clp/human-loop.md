# Human Loop State

## Current State
- Status: Gate 1 passed; Bundle 2 (implementation preparation) in progress
- Active feature directory: specs/002-mosek-lp-qp-clp
- Last completed bundle: Bundle 1 (requirements preparation)
- Source code modified by this workflow: no

## Core Command Ledger
- constitution:   checked (2.4.1)
- specify:        invoked 2026-11-15 (1 clarification: CLP test problems)
- clarify:        invoked 2026-11-15 (2 questions: cone types broader set; heavy test tier)
- checklist:      invoked 2026-11-15 (checklists/contract.md, 23 items: 9 resolved, 14 routed)
- plan:
- tasks:
- analyze:
- implement:

## Human Decisions
| Date (UTC) | Gate | Option chosen | Consequence |
|---|---|---|---|
| 2026-11-15 | (before Bundle 1) | Start feature 002: MOSEK LP, QP, CLP; rename LP fixture and document corpus; verify certificate sign and callback threading as code behaviour; build on Rust 1.85 | Branch 002-mosek-lp-qp-clp from devel |
| 2026-11-15 | Gate 1 | Continue to Bundle 2; commit authorised | Bundle 1 committed; speckit-plan next |
| 2026-11-15 | (specify) | CLP test problems: first inner conic problem of each VK corpus instance, iCoreED and iDopaNeuroC | Spec FR-014 |

## Approved Implementation Scope
- Approved: no
- Scope:
- Tasks approved:
- Tasks deferred:
- Files allowed:
- Files not allowed:

## Pointers
- Exploration: none (direct request); background specs/000 and specs/001 closeout
- Implementation receipt(s):
- Implementation review: specs/002-mosek-lp-qp-clp/implementation-review.md

## Open Risks and Ambiguities
- Bundle 0: on devel, clean tree, no remotes; feature 001 integrated; MOSEK 11.2 and licence present.
- Rust 1.85 check (2026-11-15, before this feature): library and integration tests build and pass on 1.85; the loader's unit tests and the benchmark do not compile (cast_unsigned, stable from 1.87).

## Checklist Routing (constitution VI-B)

checklists/requirements.md 16/16. checklists/contract.md open items, routed to the plan
(Bundle 2, step 1), closed in research.md or data-model.md before Gate 2A:

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
