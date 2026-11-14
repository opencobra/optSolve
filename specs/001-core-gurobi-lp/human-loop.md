# Human Loop State

## Current State
- Status: Gate 1 passed; Bundle 2 (implementation preparation) in progress
- Active feature directory: specs/001-core-gurobi-lp
- Last completed bundle: Bundle 1 (requirements preparation)
- Source code modified by this workflow: no

## Core Command Ledger
- constitution:   checked (2.3.0)
- specify:        invoked 2026-11-14
- clarify:        invoked 2026-11-14 (3 questions: performance targets, verification factor, fixture layout)
- checklist:      invoked 2026-11-14 (checklists/contract.md, 33 items: 16 resolved, 17 routed)
- plan:
- tasks:
- analyze:
- implement:

## Human Decisions
| Date (UTC) | Gate | Option chosen | Consequence |
|---|---|---|---|
| 2026-11-14 | Gate 1 | "Speckit human loop" (free text; read as: continue the human loop to Bundle 2); commit authorised | Bundle 1 committed on 001-core-gurobi-lp; speckit-plan next |
| 2026-11-14 | (before Bundle 1) | Start the human loop for feature 001; ignore the licence of the iDopaNeuro data | Feature 001 started on branch 001-core-gurobi-lp from devel; data licence not verified and not blocking (constitution X check waived by the maintainer for this data) |

## Approved Implementation Scope
- Approved: no
- Scope:
- Tasks approved:
- Tasks deferred:
- Files allowed:
- Files not allowed:

## Pointers
- Exploration: specs/000-solver-interface-exploration/ (founding exploration; design-notes.md, open-questions.md)
- Implementation receipt(s):
- Implementation review: specs/001-core-gurobi-lp/implementation-review.md

## Open Risks and Ambiguities
- Bundle 0 integration check: integration branch devel; no remotes; no closed features; working tree held only the untracked .serena/ (local tool files).

## Checklist Routing (constitution VI-B)

checklists/requirements.md: 16/16 resolved. checklists/contract.md: open items, each routed to
the plan (Bundle 2, step 1), to be closed in research.md or data-model.md before Gate 2A:

| Item | Question | Destination |
|---|---|---|
| CHK002 | Meaning of infinite bounds in both row-bound forms | data-model.md |
| CHK003 | Role of the objective constant in objective value and duality gap | data-model.md |
| CHK004 | Names and meanings of the solver-independent statuses | data-model.md |
| CHK005 | Enumeration of limit reasons | data-model.md |
| CHK006 | Shared option fields and their definitions | data-model.md, research.md (Gurobi mapping) |
| CHK010 | Data whose norm scales each relative verification measure | data-model.md |
| CHK011 | Which Gurobi tolerance judges which measure | research.md |
| CHK012 | Verification verdict structure | data-model.md |
| CHK013 | Dual-side verification when dual outputs are switched off | data-model.md |
| CHK014 | Extent of reuse of variational kinetics extraction code | plan.md, reuse assessment (constitution V) |
| CHK015 | Objective distribution and generator for reproducibility | research.md |
| CHK016 | Definition of "available backend" for the harness | plan.md |
| CHK022 | Baseline for "beyond Gurobi's own" memory | research.md (constitution XIV-C) |
| CHK023 | Definition of "Gurobi solve time" | research.md |
| CHK024 | Deterministic solver settings for SC-003 | research.md (solver configuration audit, constitution I) |
| CHK027 | Ranged rows through Gurobi | research.md |
| CHK028 | MPS conventions of the fixtures | research.md |
