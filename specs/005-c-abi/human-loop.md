# Human Loop State

## Current State
- Status: Bundle 1 complete; Gate 1 compliance review requested.
- Active feature directory: specs/005-c-abi (worktree `../optSolver-005-c-abi`, branch `005-c-abi`)
- Last completed bundle: Bundle 1 (requirements preparation)
- Source code modified by this workflow: no
- Committed: no. Constitution 3.0.0 makes feature-branch commits agent actions, but the agent
  session's tool permissions refused `git commit`; artefact revisions are therefore identified in
  each gate review by SHA-256 content hashes (constitution VI-A permits this where uncommitted).

## Core Command Ledger
- constitution:   amended to 3.0.0 on the maintainer's instruction (see changelog); checked
- specify:        invoked 2026-11-15 (subagent, under 2.5.1)
- clarify:        2026-11-15 — three questions answered by the feature agent under 3.0.0
                  (spec Clarifications), plus the unit of the memory limit
- checklist:      requirements checklist 15/16; CHK015 routed to Gate 3
- plan:
- tasks:
- analyze:
- implement:

## Gate Reviews And Decisions
| Date (UTC) | Gate | Decision or verdict (with rationale) | Review file | Consequence |
|---|---|---|---|---|
| 2026-11-15 | (before Bundle 1) | Start feature 005 from request REQ-001 of the COBRA Toolbox fork, carried by a subagent the maintainer authorised | — | Branch 005-c-abi from devel (32e83ed) in its own worktree, because 004-milp-interface is open |
| 2026-11-15 | Gate 1 (decisions) | MOSEK memory limit rejected as unsupported (vendor forbids MOSEK calls from the callback); Gurobi maps to SoftMemLimit only; environment handle usable from any thread, one at a time, Gurobi creation/freeing serialised. Rationale and evidence: spec Clarifications | (review below) | FR-008, FR-011 and SC-002/3/4/7 revised |

## Human-Reserved Decisions Pending (constitution VI-A)
- Committing constitution 3.0.0 to the integration branch `devel` (it is written, uncommitted, in
  the worktree `../optSolver-devel`; carried, uncommitted, into this worktree).
- Committing this feature's work (agent action under 3.0.0, refused by the session's tool
  permissions) and, after Gate 3, integrating it into `devel`.

## Approved Implementation Scope
- Approved: no

## Pointers
- Exploration: none (direct request; see spec Exploration field)
- Gate reviews: specs/005-c-abi/gate-reviews/
- Implementation receipt(s):
- Implementation review:

## Open Risks and Ambiguities
- Bundle 0 (2026-11-15): features 001–003 are contained in devel (32e83ed); 004-milp-interface is
  open and unmerged (its checkout at the main working tree is untouched); no remotes.
- Placement gaps (constitution IX-F), resolved in the plan by amending
  `docs/architecture/repo_folder_organisation.md`: public C header, C test sources, incoming
  cross-repository requests.
- Architecture: a new C-exporting module and new library crate types — ADR expected at Gate 2A.
- Interaction with feature 004 (MILP, open): both change `Options`; whichever merges second
  rebases. Variable types are a later minor version of the C interface (FR-020).
- The RunMat binary on the reference machine does not start (missing libhdf5.so.103), so the
  `runmat mex` build is demonstrated by the COBRA consumer (SC-007).

## Checklist Routing (constitution VI-B)
- requirements.md CHK015 (meets success criteria) → Gate 3 (closeout).
