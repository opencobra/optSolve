# Human Loop State

## Current State
- Status: Gate 2 passed (approve all tasks); awaiting the implementation invocation (/speckit-implement)
- Active feature directory: specs/001-core-gurobi-lp
- Last completed bundle: Bundle 2 (implementation preparation)
- Source code modified by this workflow: no

## Core Command Ledger
- constitution:   checked (2.3.0)
- specify:        invoked 2026-11-14
- clarify:        invoked 2026-11-14 (3 questions: performance targets, verification factor, fixture layout)
- checklist:      invoked 2026-11-14 (checklists/contract.md, 33 items: 16 resolved, 17 routed)
- plan:           invoked 2026-11-14 (plan.md, research.md, data-model.md, contracts/public-api.md, quickstart.md, architecture-delta.yaml)
- tasks:          invoked 2026-11-14 (57 tasks)
- analyze:        invoked 2026-11-14 (0 critical; should-fix C1 C2 C3 C4 U1; low U2 I1 I2 F1)
- implement:

## Human Decisions
| Date (UTC) | Gate | Option chosen | Consequence |
|---|---|---|---|
| 2026-11-14 | Gate 2 | Approve all tasks; commit authorised | Scope T001–T054; should-fix findings handled during implementation as recorded deviations; implementation awaits /speckit-implement |
| 2026-11-14 | Gate 2A | APPROVE_WITH_ADR; accept all names; approve both amendments; commit authorised | ADR-0001 written; constitution 2.4.0 (notation); placement role "Maintenance tooling"; tasks may be generated |
| 2026-11-14 | Gate 1 | "Speckit human loop" (free text; read as: continue the human loop to Bundle 2); commit authorised | Bundle 1 committed on 001-core-gurobi-lp; speckit-plan next |
| 2026-11-14 | (before Bundle 1) | Start the human loop for feature 001; ignore the licence of the iDopaNeuro data | Feature 001 started on branch 001-core-gurobi-lp from devel; data licence not verified and not blocking (constitution X check waived by the maintainer for this data) |

## Approved Implementation Scope
- Approved: yes (Gate 2, 2026-11-14); edits only after the implementation invocation
- Scope: all
- Tasks approved: T001–T054
- Tasks deferred: none
- Files allowed: as listed in implementation-review.md, "Files likely to change"
- Files not allowed: .specify/, literature/, specs/000-*, the variational kinetics repository

## Pointers
- Exploration: specs/000-solver-interface-exploration/ (founding exploration; design-notes.md, open-questions.md)
- Implementation receipt(s):
- Plan: specs/001-core-gurobi-lp/plan.md
- Architecture delta: specs/001-core-gurobi-lp/architecture-delta.yaml
- Implementation review: specs/001-core-gurobi-lp/implementation-review.md

## Open Risks and Ambiguities
- Bundle 0 integration check: integration branch devel; no remotes; no closed features; working tree held only the untracked .serena/ (local tool files).

## Checklist Routing (constitution VI-B)

checklists/requirements.md: 16/16 resolved. checklists/contract.md: the 17 items below were routed
to the plan at Gate 1; all 17 resolved in Bundle 2 (33/33), with the closing artifact named on
each item in the checklist:

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
