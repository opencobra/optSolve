# Human Loop State

## Current State
- Status: Gate 1 PASS (review 4); Gate 2A reviews 1 to 3 REVISE_ARCHITECTURE, each repaired; Gate 2A review 4 requested.
- Active feature directory: specs/005-c-abi (worktree `../optSolver-005-c-abi`, branch `005-c-abi`)
- Last completed bundle: Bundle 1 (requirements preparation)
- Source code modified by this workflow: no
- Committed: yes, on branch 005-c-abi before each gate review (a first attempt was refused by the
  session's tool permissions; a later attempt succeeded).

## Core Command Ledger
- constitution:   amended to 3.0.0 on the maintainer's instruction (see changelog); checked
- specify:        invoked 2026-11-15 (subagent, under 2.5.1)
- clarify:        2026-11-15 — three questions answered by the feature agent under 3.0.0
                  (spec Clarifications), plus the unit of the memory limit
- checklist:      requirements checklist 15/16; the success-criteria item (CHK015) routed to Gate 3
- plan:           2026-11-15 — plan, research, data model, contract, quickstart, architecture delta, ADR-0004 (proposed)
- tasks:
- analyze:
- implement:

## Gate Reviews And Decisions
| Date (UTC) | Gate | Decision or verdict (with rationale) | Review file | Consequence |
|---|---|---|---|---|
| 2026-11-15 | (before Bundle 1) | Start feature 005 from request REQ-001 of the COBRA Toolbox fork, carried by a subagent the maintainer authorised | — | Branch 005-c-abi from devel (32e83ed) in its own worktree, because 004-milp-interface is open |
| 2026-11-15 | Gate 1 (decisions, first round) | MOSEK memory limit rejected as unsupported (vendor forbids MOSEK calls from the callback); Gurobi maps to SoftMemLimit only; environment handle usable from any thread, one at a time, Gurobi creation/freeing serialised. Rationale and evidence: spec Clarifications | (review below) | Thread-rule and memory-limit requirements (FR-008, FR-011) and the heap, wall-time, environment-reuse and link-demonstration criteria (SC-002, SC-003, SC-004, SC-007) revised |

| 2026-11-15 | Gate 1 review 1 | FAIL, seven findings: evidence not recorded; output-array rule deferred; comparison rule undefined; earlier non-Send ruling superseded silently; codes in place of names; M and N symbols; uncommitted work | gate-reviews/gate-1-20261115T232223Z.md | Repairs: dated vendor evidence in Clarifications; output-array and comparison rules decided; Env becomes Send (not Sync) with Gurobi creation lock, superseding the deferred-Send rulings of feature 001 research (R15) and feature 002 research (R2) (recorded); names beside codes; version rule without symbols; work committed (710ae9a, 2f62cce) |
| 2026-11-15 | Gate 1 review 2 | FAIL, six findings: comparison and reuse criteria assumed deterministic Gurobi runs; comparison rule used symbols outside the inventory; codes without names; commit 710ae9a omitted the in-flight governance changes; missing rationale for the structure-size mechanism and the factor ten; stale record of the constitution's commit state | gate-reviews/gate-1-20261115T232922Z.md | Repairs: deterministic comparison configuration with vendor rationale; rule stated in words; names beside codes; deviation recorded below; rationales added; record corrected |
| 2026-11-15 | Gate 1 review 3 | FAIL, three findings: a symbol outside the inventory left in the agreement criterion (SC-001); the memory-limit criterion (SC-005) not reliably testable; stale current-state record | gate-reviews/gate-1-20261115T233404Z.md | Repairs: criterion in words; memory-limit criterion fixes instance, one thread, a limit far below need and N = 3 with vendor rationale; state record updated |
| 2026-11-15 | Gate 1 review 4 | PASS | gate-reviews/gate-1-20261115T233939Z.md | Gate 1 passed; Bundle 2 (plan) starts. Observations routed to the plan: capabilities usability, crate types versus the feature, C-layer failing-input tests, thread-rule and no-output checks, machine class, version mapping, absent arrays. Wording observation (User Story 3 says "toy-sized" while the memory-limit criterion uses iDopaNeuroC_problem_LP) left for the next spec revision, which will need a Gate 1 re-review |
| 2026-11-15 | Gate 2A review 1 | REVISE_ARCHITECTURE (FAIL), three findings: the Gurobi creation lock placed in the C layer by ADR-0004, research R3 and the plan but in the backend by the spec and delta; the plan's delta summary missing `capi → gurobi.solve/mosek.solve` and `gurobi.solve`; symbols outside the inventory in the contract and plan. Names approved | gate-reviews/gate-2A-20261115T234459Z.md | Repairs: lock only in `gurobi::env`; plan summary matches the delta; symbols replaced by words and field names; MILP capability bit documented |
| 2026-11-15 | Gate 2A review 2 | REVISE_ARCHITECTURE (FAIL), four findings: ADR-0004 still said the Rust Env stays !Send; the inference that a lock suffices for Gurobi creation/freeing on any thread was not marked; codes without names in the plan; NaN-fill cost inconsistent | gate-reviews/gate-2A-20261115T235156Z.md | Repairs: ADR decisions 1 and 5 corrected; inference marked unverified, accepted risk with a licensed concurrency test, lock extended to Gurobi model creation and freeing; names beside codes; O(m + n + k). Observations routed to tasks: lazy name conversion, cone names from ConeKind::name, boundaries.yaml at closeout |
| 2026-11-15 | Gate 2A review 3 | REVISE_ARCHITECTURE (FAIL), three findings: the Gurobi lock did not cover two unit-test creation sites and had no crate-wide rule; codes without names in the plan; wrong research cross-reference | gate-reviews/gate-2A-20261115T235742Z.md | Repairs: crate-wide rule with the single helper gurobi::env::lifecycle, source test, poisoning recorded, sites added to delta and plan; names; R9. MOSEK concurrent creation stated unverified in research and header |

**Recorded deviation (Governance, "Commits Carry In-Flight Manual Work")**: commit 710ae9a
(feature artefacts) did not include or list the eleven tracked governance changes then in the
working tree (constitution 3.0.0 and its companions); they were committed 14 seconds later in
2f62cce, whose message lists them. The feature branch is not shared; the history is left as is
rather than rewritten.

## Human-Reserved Decisions Pending (constitution VI-A)
- Committing constitution 3.0.0 to the integration branch `devel` (it is written, uncommitted, in
  the worktree `../optSolver-devel`). It is committed on this feature branch (2f62cce); committing
  it to `devel` remains a human decision.
- After Gate 3, integrating this feature into `devel`.

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
- requirements.md, the success-criteria item (CHK015) → Gate 3 (closeout).
