# Human Loop State

## Current State
- Status: Gate 2 passed (approve all); awaiting the implementation invocation (/speckit-implement)
- Active feature directory: specs/003-llm-interface-toy-instances
- Last completed bundle: Bundle 2 (implementation preparation)
- Source code modified by this workflow: no

## Core Command Ledger
- constitution:   checked (2.5.0); amended to 2.5.1 at Gate 2A (z_l, z_u)
- specify:        invoked 2026-11-15 (two clarifications resolved at specify time)
- clarify:        invoked 2026-11-15 (2 questions: readers test-only; SC-002 subagent trial at Gate 3)
- checklist:      invoked 2026-11-15 (requirements 16/16; contract 14/23, 9 routed below)
- plan:           invoked 2026-11-15 (plan.md, research.md R1-R12, data-model.md, contracts/public-api.md, quickstart.md, architecture-delta.yaml; two read-only research agents with Gurobi C and MATLAB probes)
- tasks:          invoked 2026-11-15 (26 tasks)
- analyze:        invoked 2026-11-15 (0 critical; 1 medium applied to T017; 3 low accepted)
- implement:

## Human Decisions
| Date (UTC) | Gate | Option chosen | Consequence |
|---|---|---|---|
| 2026-11-15 | Gate 2 | Approve all tasks and commit | Scope T001–T026; implementation on /speckit-implement |
| 2026-11-15 | Gate 2A | APPROVE_WITH_ADR; llms.txt in docs/ (not root); add z_l, z_u to the Notation Convention | ADR-0003; delta, plan, spec FR-005 updated; /speckit-constitution amendment next |
| 2026-11-15 | Gate 1 | Continue and commit | Bundle 1 committed; Bundle 2 (plan) next |
| 2026-11-15 | (clarify) | SC-002 check: "Subagent trial at Gate 3" | SC-002 rewritten; trial recorded in the run bundle |
| 2026-11-15 | (clarify) | Readers: "Test-only" | FR-010a |
| 2026-11-15 | (specify) | Doc form: "Markdown + rustdoc" | FR-005: docs/llm-interface.md, llms.txt entry point, crate-level docs from the same source |
| 2026-11-15 | (specify) | Compare scope: "Add Gurobi QP + SOC" | FR-014–FR-016: Gurobi extended to QP and quadratic/rotated-quadratic conic problems |
| 2026-11-15 | (before Bundle 1) | Start feature 003: (1) re-extract fixtures from 32c29b4 [done in feature 002]; (2) LLM interface documentation; (3) self-contained toy instances in MPS and JSON for every problem type; (4) MOSEK–Gurobi comparison when both installed | Branch 003-llm-interface-toy-instances from devel (devel fast-forwarded to 7730efa: feature 002 integrated at Bundle 0) |

## Approved Implementation Scope
- Approved: yes (Gate 2, 2026-11-15); edits only after the implementation invocation
- Scope: all
- Tasks approved: T001–T026
- Tasks deferred: none
- Files allowed: as in implementation-review.md, "Files likely to change"
- Files not allowed: other .specify/ governance, literature/, specs/000-*, specs/001-*, specs/002-*, the variational kinetics repository, the LP/QP/CLP corpus fixtures

## Pointers
- Exploration: none (direct request); background specs/002-mosek-lp-qp-clp
- Implementation receipt(s):
- Implementation review: specs/003-llm-interface-toy-instances/implementation-review.md

## Open Risks and Ambiguities
- Bundle 0: devel was behind the closed feature 002; repaired by fast-forward to 7730efa (no remotes).
- PSD cone in JTASK: MOSEK's JSON format documentation does not document an svec PSD domain (feature 002 reader); may need an in-code instance (spec Assumptions).

## Checklist Routing (constitution VI-B)
All nine routed items are closed in Bundle 2 (research R2–R5, R6, R7, R9, R10, R11; data-model.md); checklist updated.

Unchecked items of checklists/contract.md, each routed to Bundle 2 (plan):
- CHK004 Gurobi cone formulation and dual mapping → research.md (vendor docs) and data-model.md
- CHK005 Gurobi certificates for QP and SOC → research.md
- CHK006 toy provenance record fields → data-model.md
- CHK008 drift test definition → research.md / plan
- CHK010 QP MPS convention (QUADOBJ, ½, triangle) → research.md, checked against both solver tools
- CHK015 which corpus instances are compared, which tier → plan
- CHK019 reader-versus-tool disagreement → data-model.md
- CHK020 PSD instance form (JTASK encoding evidence) → research.md (probe with MOSEK)
- CHK022 Gurobi features and version confirmed → research.md
