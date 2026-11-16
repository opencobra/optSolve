# Human Loop State

## Current State
- Status: Gate 1 PASS (review 4); Gate 2A PASS (review 8, APPROVE_WITH_ADR); tasks and analysis written; Gate 2 reviews 1 and 2 FAIL, repaired; Gate 2 review 3 requested.
- Active feature directory: specs/005-c-abi (worktree `../optSolver-005-c-abi`, branch `005-c-abi`)
- Last completed bundle: Bundle 2 (up to Gate 2)
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
- tasks:          2026-11-16, after the Gate 2A PASS
- analyze:        2026-11-16 (feature agent, speckit-analyze checks; summary in implementation-review.md)
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
| 2026-11-16 | Gate 2A review 4 | REVISE_ARCHITECTURE (FAIL), five findings: lock check could not catch calls through helpers and had no negative test; lock cost on loading unjustified; duplicated buffer-length checks unrecorded; error-message ownership contradicted the spec; ADR did not name the decisions it changes | gate-reviews/gate-2A-20261116T000525Z.md | Repairs: lock narrowed to environment creation and freeing, enforced by a token type plus a source test with a negative case, cost stated; duplication recorded with reason (ADR-0002 decision 2); caller-owned message copy with optsolver_string_free; ADR decision 7; duplicates merged; observations (minimum structure sizes, NUL names, NaN before the check, build cost, env docs) folded in |
| 2026-11-16 | Gate 2A review 5 | REVISE_ARCHITECTURE (FAIL), three findings: contained panics still print through Rust's panic hook; the plan's lock sentence contradicted itself; the Notation answer missed `alpha` and `r` | gate-reviews/gate-2A-20261116T001606Z.md | Repairs: silent wrapping panic hook (research R4, ADR decision 4, two tests); lock sentence fixed; C names `exponents` and `OPTSOLVER_CERT_RAY`, inventory gap routed to the maintainer. Observations folded in: C-layer checks listed with tests, names from the core's strings, test-support functions in Complexity Tracking, empty-slice rule, MOSEK freeing unverified |
| 2026-11-16 | Gate 2A review 6 | REVISE_ARCHITECTURE (FAIL), two findings: reuse not assessed for the new test and benchmark components (comparison rule, compiler discovery, benchmark machinery); the private capi module unreachable by its tests and benchmark | gate-reviews/gate-2A-20261116T002649Z.md | Repairs: comparison rule as a new function in compare.rs; compiler discovery extracted to tests/common/ccompiler.rs with both header-conformance tests migrating; load_solve bench extended instead of a new bench; optsolver::capi doc(hidden) pub, declared in the delta with its edges. Observations routed to tasks |
| 2026-11-16 | Gate 2A review 7 | REVISE_ARCHITECTURE (FAIL), one finding: delta and plan still declared a [[bench]] c_interface target contradicting the extend-load_solve decision | gate-reviews/gate-2A-20261116T003521Z.md | Repair: no new bench target; C-path scenarios under cfg(feature = "capi") in load_solve, its [[bench]] entry unchanged; [[test]] c_interface entry stated. Observations folded in: contract states aliasing, slice bounds, validate-before-environment and core-derived names; bench output path through project.yml; mosek.h memory items corrected; boundaries.yaml in scope. Closeout: FR-021 omits the Gurobi conic-dual length check (report as repaired defect) |
| 2026-11-16 | Gate 2A review 8 | APPROVE_WITH_ADR (PASS); names `capi`, "C interface", `src/capi/optsolver.h` approved (XIII-F) | gate-reviews/gate-2A-20261116T004341Z.md | ADR-0004 accepted; delta verdict set; task generation permitted. Observations routed to tasks (header parameter names `result`/`caps`, freeing on another thread, cold and environment-creation phases in the bench, visible skips for new licensed tests, early library-build check) and to closeout (conic-dual check as a repaired defect; components and boundaries reconciliation; consumer adoption; User Story 3 wording) |
| 2026-11-16 | Gate 2 review 1 | FAIL, six findings: test ordering; stories not independently testable; coverage gaps (all invalid inputs, wrong-length buffer, solver-error code, name functions, conic on both paths); comparison rule never shown to reject; additivity combinations incomplete; analysis overstated | gate-reviews/gate-2-20261116T005056Z.md | Repairs: tasks restructured (each task with its tests; foundational harness and driver; story phases independent); gaps covered; negative comparison tests; eight feature combinations; analysis corrected; contract fields `rows`/`cols`, `caps` |
| 2026-11-16 | Gate 2 review 2 | FAIL, two findings: NaN rule, per-thread messages and not-usable backend untested; analysis still overstated (coverage, task reference, file list) | gate-reviews/gate-2-20261116T005909Z.md | Repairs: tests added to T005, T007, T008, T010; analysis corrected; observations folded in (lock foundational, explicit comparison tests, no-output in the shared runner, twenty problems, phase names in dependencies) |

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
- Confirming the instruction behind constitution 3.0.0: it reached this repository relayed by the
  COBRA session (Gate 2A review 7 observation); the maintainer confirms it when committing 3.0.0
  to `devel`.
- Notation inventory (amendment, constitution XII-C): the Rust API names the power-cone exponents
  `alpha` (features 002–003) and the unbounded ray `r` (feature 001), neither in the Notation
  Convention, and `r` is also used informally for the order of a positive semidefinite cone. The
  C interface avoids both (`exponents`, `OPTSOLVER_CERT_RAY`); whether to add symbols for them is
  the maintainer's decision.

## Approved Implementation Scope
- Approved: no

## Pointers
- Exploration: none (direct request; see spec Exploration field)
- Gate reviews: specs/005-c-abi/gate-reviews/
- Implementation receipt(s):
- Implementation review: specs/005-c-abi/implementation-review.md

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
