# Human Loop State

## Current State
- Status: Gate 1 passed; Bundle 2 (implementation preparation) in progress
- Active feature directory: specs/003-llm-interface-toy-instances
- Last completed bundle: Bundle 1 (requirements preparation)
- Source code modified by this workflow: no

## Core Command Ledger
- constitution:   checked (2.5.0)
- specify:        invoked 2026-11-15 (two clarifications resolved at specify time)
- clarify:        invoked 2026-11-15 (2 questions: readers test-only; SC-002 subagent trial at Gate 3)
- checklist:      invoked 2026-11-15 (requirements 16/16; contract 14/23, 9 routed below)
- plan:
- tasks:
- analyze:
- implement:

## Human Decisions
| Date (UTC) | Gate | Option chosen | Consequence |
|---|---|---|---|
| 2026-11-15 | Gate 1 | Continue and commit | Bundle 1 committed; Bundle 2 (plan) next |
| 2026-11-15 | (clarify) | SC-002 check: "Subagent trial at Gate 3" | SC-002 rewritten; trial recorded in the run bundle |
| 2026-11-15 | (clarify) | Readers: "Test-only" | FR-010a |
| 2026-11-15 | (specify) | Doc form: "Markdown + rustdoc" | FR-005: docs/llm-interface.md, llms.txt entry point, crate-level docs from the same source |
| 2026-11-15 | (specify) | Compare scope: "Add Gurobi QP + SOC" | FR-014–FR-016: Gurobi extended to QP and quadratic/rotated-quadratic conic problems |
| 2026-11-15 | (before Bundle 1) | Start feature 003: (1) re-extract fixtures from 32c29b4 [done in feature 002]; (2) LLM interface documentation; (3) self-contained toy instances in MPS and JSON for every problem type; (4) MOSEK–Gurobi comparison when both installed | Branch 003-llm-interface-toy-instances from devel (devel fast-forwarded to 7730efa: feature 002 integrated at Bundle 0) |

## Approved Implementation Scope
- Approved: no
- Scope:
- Tasks approved:
- Tasks deferred:
- Files allowed:
- Files not allowed:

## Pointers
- Exploration: none (direct request); background specs/002-mosek-lp-qp-clp
- Implementation receipt(s):
- Implementation review: specs/003-llm-interface-toy-instances/implementation-review.md

## Open Risks and Ambiguities
- Bundle 0: devel was behind the closed feature 002; repaired by fast-forward to 7730efa (no remotes).
- PSD cone in JTASK: MOSEK's JSON format documentation does not document an svec PSD domain (feature 002 reader); may need an in-code instance (spec Assumptions).

## Checklist Routing (constitution VI-B)
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
