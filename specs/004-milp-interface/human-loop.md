# Human Loop State

## Current State
- Status: Gate 1 passed; Bundle 2 (implementation preparation) in progress
- Active feature directory: specs/004-milp-interface
- Last completed bundle: Bundle 1 (requirements preparation)
- Source code modified by this workflow: no

## Core Command Ledger
- constitution:   checked (2.5.1)
- specify:        invoked 2026-11-15 (one clarification at specify time)
- clarify:        invoked 2026-11-15 (1 question: fixed-model duals optional)
- checklist:      invoked 2026-11-15 (requirements 16/16; contract 8/18, 10 routed below)
- plan:
- tasks:
- analyze:
- implement:

## Human Decisions
| Date (UTC) | Gate | Option chosen | Consequence |
|---|---|---|---|
| 2026-11-15 | Gate 1 | Continue and commit | Bundle 1 committed; Bundle 2 next |
| 2026-11-15 | Gate 1 | Revise the specification: "I do not mean 'no new public types' I mean that there should be reuse where possible." | SC-003 rewritten (reuse where possible, new items justified in the plan); clarification recorded; CHK003 reworded |
| 2026-11-15 | (clarify) | MILP duals: "Optional fixed-model duals" | FR-003a, FR-004 amended, SC-007 |
| 2026-11-15 | (specify) | MI scope: "All MI classes the solvers support" | FR-009, FR-009a, US3, SC-006 |
| 2026-11-15 | (before Bundle 1) | Start feature 004: "create an interface that admits MILP problems. reuse the existing interface wherever possible rather than writing completely separate code" | Branch 004-milp-interface from devel (32e83ed) |
| 2026-11-15 | Gate 3 of 003 (before 004) | Close as is, integrate | devel fast-forwarded to 32e83ed |

## Approved Implementation Scope
- Approved: no
- Scope:
- Tasks approved:
- Tasks deferred:
- Files allowed:
- Files not allowed:

## Pointers
- Exploration: none (direct request)
- Implementation receipt(s):
- Implementation review: specs/004-milp-interface/implementation-review.md

## Open Risks and Ambiguities
- Bundle 0: features 001–003 closed and integrated in devel; no remotes; clean tree.
- Carried debt: feature 003 T024 (LLM trial, maintainer).

## Checklist Routing (constitution VI-B)
Unchecked items of checklists/contract.md, each routed to Bundle 2 (plan):
- CHK002 gap definition → data-model.md
- CHK003 capability representation vs SC-003 → research.md / data-model.md (may need a spec decision)
- CHK004 MOSEK cone kinds with integers → research.md (vendor docs, probe)
- CHK005 status mapping per backend → research.md
- CHK007 fixing near-integral values → research.md
- CHK008 marking fixed-model duals → data-model.md
- CHK010 binary and fractional bounds → research.md (probe both)
- CHK013 benchmark instance → plan
- CHK016 comparison rule → research.md
- CHK018 MPS integer markers → research.md (both tools)
