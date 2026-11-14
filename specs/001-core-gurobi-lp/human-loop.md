# Human Loop State

## Current State
- Status: closed (Gate 3 accept and close, 2026-11-14); integrated into devel
- Active feature directory: specs/001-core-gurobi-lp
- Last completed bundle: Bundle 4 (verification and closeout)
- Source code modified by this workflow: yes (implementation cycle 1)

## Core Command Ledger
- constitution:   checked (2.3.0)
- specify:        invoked 2026-11-14
- clarify:        invoked 2026-11-14 (3 questions: performance targets, verification factor, fixture layout)
- checklist:      invoked 2026-11-14 (checklists/contract.md, 33 items: 16 resolved, 17 routed)
- plan:           invoked 2026-11-14 (plan.md, research.md, data-model.md, contracts/public-api.md, quickstart.md, architecture-delta.yaml)
- tasks:          invoked 2026-11-14 (57 tasks)
- analyze:        invoked 2026-11-14 (0 critical; should-fix C1 C2 C3 C4 U1; low U2 I1 I2 F1)
- implement:      invoked 2026-11-14 (cycle 1; receipt agent-runs/20261114T212100Z-implement-001)

## Human Decisions
| Date (UTC) | Gate | Option chosen | Consequence |
|---|---|---|---|
| 2026-11-14 | Gate 3 (after slice 2) | Commit, T058, accept & close | Commit on 001-core-gurobi-lp; T058 re-extraction; commit; integrate into devel; switch to devel |
| 2026-11-14 | Gate 3 (follow-up) | Approve T055–T058; commit: not yet | Slice-2 scope recorded; cycle-1 work stays uncommitted; T058 waits for a commit |
| 2026-11-14 | Gate 3 | Next slice: establish SC-005 | Feature stays open; nothing integrated; tasks T055–T058 added for approval |
| 2026-11-14 | (implementation) | Corpus: "Use the FBA LP problem only. That is the first problem." | project.yml test_corpus = iDopaNeuroC_problem_1; other six fixtures removed; spec FR-001, SC-001, SC-003 amended; research R18 |
| 2026-11-14 | (implementation) | Reduced-cost consistency scaled by term magnitude | data-model.md and verify.rs amended |
| 2026-11-14 | (implementation) | Gurobi input arrays: accept manual wording + recorded probe as the VII-A basis | Constitution 2.4.1 (VII-A clarified); loader passes pointers directly; probe kept as a licensed test |
| 2026-11-14 | (after Gate 2) | MPS writer for T015: keep gurobi_write (COBRA writeLPProblem considered: 12-character numbers, row reordering) | No task change; research R11 and plan reuse assessment record the reason |
| 2026-11-14 | Gate 2 | Approve all tasks; commit authorised | Scope T001–T054; should-fix findings handled during implementation as recorded deviations; implementation awaits /speckit-implement |
| 2026-11-14 | Gate 2A | APPROVE_WITH_ADR; accept all names; approve both amendments; commit authorised | ADR-0001 written; constitution 2.4.0 (notation); placement role "Maintenance tooling"; tasks may be generated |
| 2026-11-14 | Gate 1 | "Speckit human loop" (free text; read as: continue the human loop to Bundle 2); commit authorised | Bundle 1 committed on 001-core-gurobi-lp; speckit-plan next |
| 2026-11-14 | (before Bundle 1) | Start the human loop for feature 001; ignore the licence of the iDopaNeuro data | Feature 001 started on branch 001-core-gurobi-lp from devel; data licence not verified and not blocking (constitution X check waived by the maintainer for this data) |

## Approved Implementation Scope
- Approved: yes (Gate 2, 2026-11-14); edits only after the implementation invocation
- Scope: all
- Tasks approved: T001–T054
- Tasks deferred: none
- Slice 2 approved 2026-11-14: T055–T058 (T058 runs only after a commit is authorised)
- Files allowed: as listed in implementation-review.md, "Files likely to change"
- Files not allowed: .specify/, literature/, specs/000-*, the variational kinetics repository

## Implementation Cycle Baseline
- Cycle 1 started 2026-11-14 by /speckit-implement ("in parallel to the maximum extent possible with multiple agents")
- Approved task IDs: T001–T054 (57 tasks including T010a–T010c); unchecked at start: 57
- Continuation check (after_implement), 2026-11-14: unchecked 57 → 0; no stop-and-report condition fired; required tests pass (receipt). Decision: finished; in-flight marker removed; hand to Gate 3.

- Cycle 2 (slice 2) started 2026-11-14 by /speckit-implement; approved task IDs T055–T058; unchecked at start: 4 (T058 conditional on a commit authorisation, not yet given)
- Continuation check, 2026-11-14: unchecked 4 → 1 (T058); T058 needs a human commit authorisation, so the run stops and hands to Gate 3; in-flight marker removed. Receipt: specs/001-core-gurobi-lp/agent-runs/20261114T220319Z-implement-001-slice2/implementation-receipt.md

## Pointers
- Exploration: specs/000-solver-interface-exploration/ (founding exploration; design-notes.md, open-questions.md)
- Implementation receipt(s): specs/001-core-gurobi-lp/agent-runs/20261114T212100Z-implement-001/implementation-receipt.md
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

## Closeout (Bundle 4)

Architecture reconciliation against architecture-delta.yaml (constitution XIII-D):
- Approved and implemented: every component in the delta (core.problem, core.options,
  core.result, core.verify, core.backend, core.error, gurobi.ffi/env/load/params/status/
  callback/solve, test.harness, test.corpus, tools.corpus_extraction, bench.load_solve).
- Approved but not implemented: none.
- Implemented but not approved: sub-modules src/problem/{validate,convert}.rs and
  src/options/check.rs (inside approved components); tests/common/us3.rs (inside test.harness);
  doc-hidden `gurobi::Env::load_report` / `LoadReport` (benchmark support, inside gurobi.solve).
  No new component or dependency edge.
- Existing violations worsened: none. Forbidden edges hold (tests/standards.rs).
- Model validity: components.yaml updated; source conforms by inspection.

Knowledge promotion (Governance, "Project Knowledge Lives In The Repository"): candidates
recorded for the maintainer — (1) the fact that only iDopaNeuroC_problem_1 is a flux-balance LP
(currently research R18, feature-level; promote to docs/ if other features use the corpus);
(2) no new rule for the constitution. Decision pending.

T058 (2026-11-14): fixture re-extracted after commit b2e7732; provenance now names that commit
with a clean worktree; MPS SHA-256 unchanged; licensed tier passed (27, 44, 37, 2, 3).

Integration: devel fast-forwarded to the feature branch tip; working branch switched to devel;
feature branch 001-core-gurobi-lp kept. No remotes exist, so nothing was published.
