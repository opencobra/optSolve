# Human Loop State

## Current State
- Status: Gate 1 PASS (review 4); Gate 2A PASS (review 8, APPROVE_WITH_ADR); tasks and analysis written; Gate 2 PASS (review 7); implementation T001–T022 complete and validated; Gate 3 reviews 1 to 3 FAIL, each repaired; Gate 3 review 4 requested.
- Active feature directory: specs/005-c-abi (worktree `../optSolver-005-c-abi`, branch `005-c-abi`)
- Last completed bundle: Bundle 3 (implementation); Bundle 4 (verification and closeout) at Gate 3
- Source code modified by this workflow: yes (approved scope T001–T022)
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
- implement:      2026-11-16, T001–T022 (agent-runs/20261116T022035Z-implement/implementation-receipt.md);
                  repairs after Gate 3 review 3 (agent-runs/20261116T033623Z-gate3-repairs-2/implementation-receipt.md);
                  repairs after Gate 3 reviews 1 and 2 (agent-runs/20261116T030638Z-gate3-repairs/implementation-receipt.md)

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
| 2026-11-16 | Gate 2 review 3 | FAIL, three findings: certificates through C untested; new output and leak checks without negative controls; `unsafe` would be needed outside src/capi/ | gate-reviews/gate-2-20261116T010611Z.md | Repairs: certificate trials; runner self-test and deliberate-leak control; driver-based accessor tests and a safe hidden helper for the bench. Observations folded in (upward library search, every option through C, per-code tests, verdicts in the concurrency trial, repr(C) note) |
| 2026-11-16 | Gate 2 review 4 | FAIL, two findings: leak check did not cover environment and result and its control could not run unlicensed; drift test could not pass at the end of the foundational phase | gate-reviews/gate-2-20261116T011329Z.md | Repairs: two leak checks with working controls; names documented in the introducing task; observations folded in (guard source check, timing rule, empty problem, bench paths). Pending for Gate 3: maintainer's confirmation of the instruction behind constitution 3.0.0 |
| 2026-11-16 | Gate 2 review 5 | FAIL, two findings: too-large dimension case not covered through C; link demonstration ordered before its flags were documented | gate-reviews/gate-2-20261116T012147Z.md | Repairs: T016 and T006 validation order; T018 documents flags first; contract states the validation order (A first). Observations folded in (UTF-8 string value, verification off, verdicts, leak-run stderr exception, MOSEK "before any solver call" defined) |
| 2026-11-16 | Gate 2 review 6 | FAIL, two findings: too-large column case untested; principle numerals without names in tasks and analysis | gate-reviews/gate-2-20261116T012618Z.md | Repairs: column case in T016 with dimensions-first order; names beside numerals; contract `cols + 1`; verdict assertions added to T012 and T016 |
| 2026-11-16 | Gate 2 review 7 | PASS; scope all tasks T001–T022 | gate-reviews/gate-2-20261116T013252Z.md | Implementation authorised for the approved scope. Observations routed to implementation (NULL top-level pointers, minimal.c prints the verdict, feature-003 benchmark comparison, scoped lint allowances, history section in env.rs, suppression-list home) and closeout (stale plan text) |
| 2026-11-16 | Implementation | T001–T022 done; in-gate checks pass for all eight feature combinations; benchmark defect found and fixed (C path minimised a maximisation instance); wall-time comparison switched to alternating runs in one process | — | Gate 3 review requested |

**Architecture reconciliation (constitution XIII-D)**: approved and implemented — capi,
capi.header, test.c_interface; modified core.options, gurobi.env, mosek.env, gurobi.params,
gurobi.solve, test.harness, bench.load_solve, docs.llm_interface, placement document, manifest,
test.standards, test.header_conformance. Approved but not implemented — none. Implemented but not
approved — none (the safe in-process helper `solve_lp_through_c` and `CEnv` live inside `capi`,
as planned for T008/T017). Existing violations worsened — none. components.yaml and
boundaries.yaml reconciled (T020).

**Knowledge promotion (Gate 3 question)**: (1) "A benchmark comparing two call paths must build
both from one problem description" — the sense defect; feature-specific to this bench, recorded
in the receipt, no promotion. (2) The C-compiler discovery helper is now shared (tests/common).
(3) The `#[doc(hidden)] pub mod` pattern for a second interface is recorded in ADR-0004. Nothing
generalises into a rule.
| 2026-11-16 | Gate 3 review 1 | FAIL, ten findings: relink race in the leak trials; incomplete zero-copy control; safe benchmark helper without length checks; bit-identity not reported for conic and certificate runs; benchmark claims broader than measured; unbundled wall-time claim, unstated resolution, no regression check; CHK015 open; history prose inline; incomplete bundles; Gurobi library name restated | gate-reviews/gate-3-20261116T023304Z.md | Repairs in c9cf95b; evidence re-run at f9a30b2 (clean tree); claim withdrawn (below); CHK015 resolved; Gate 3 review 2 requested |

**Withdrawn claim (constitution XV-A)**: the first implementation receipt said separate-process
wall-time runs "passed 4 of 6"; their run bundles had been deleted after a benchmark defect (the C
path minimised a maximisation instance) was fixed, so the claim had no evidence. Withdrawn
2026-11-16; the alternating-run measurements in the receipt replace it.
| 2026-11-16 | Gate 3 review 2 | FAIL, four findings: receipt ledger (Final response no longer verbatim, no receipt for the repair run); an attribution deleted without being withdrawn and no record of why the method changed; Gurobi version in the suppression list; incomplete reconciliation | gate-reviews/gate-3-20261116T030058Z.md | First receipt restored verbatim; repair receipt written; withdrawals and method change recorded (below); version-free suppression list, leak trials re-run; reconciliation completed (below); Gate 3 review 3 requested |

**Withdrawn attribution and method change (constitution XV-A)**: the first receipt attributed the
failed separate-process wall-time runs to "the solvers' own solve phase, not the C layer's work".
That reading rested on unretained runs and is withdrawn (2026-11-16). What is recorded: in those
runs the per-phase load and extraction medians of the two paths agreed while the solve phase
differed by a few milliseconds, and the C layer adds no code inside the solve phase; two readings
were possible — machine drift between separate processes, or a real cost — and the method was
switched to alternating Rust and C calls in one process precisely to separate them. The
alternating runs (corrected at Gate 3 review 3: the three bundles at f9a30b2 gave median
differences between −0.79 and +0.29 ms; across all nine alternating bundles recorded so far the
range is −5.44 to +1.00 ms — the −5.44 ms value is a C path faster than the Rust path — all
within the limit, the largest positive difference 0.9993 ms)
support the first reading; the failed runs themselves are not evidence either way.

**Closeout reconciliation, additions after Gate 3 review 2 (constitution XIII-D)**:
- Duplication remaining: each backend's `solve_into` checks its buffer lengths itself
  (src/gurobi/solve.rs, src/mosek/solve.rs); Gurobi's now mirrors MOSEK's conic-dual check. Kept
  under ADR-0002 decision 2; owner: the maintainer, if a later feature extracts a shared check
  into `core.result`.
- Documentation invalidated: the spec's FR-021 list omits the Gurobi conic-dual check, and User
  Story 3 says "toy-sized" where the memory-limit criterion uses iDopaNeuroC_problem_LP; a spec
  revision needs a Gate 1 re-review — owner: the next feature that revises this spec, or the
  maintainer.
- Unowned components: `tests/contract.rs` and `tests/contract/` (including the new
  `tests/contract/f005.rs`) have no component in components.yaml (pre-existing gap); owner: the
  maintainer's next architecture reconciliation.
- Behavioural integration evidence (Principle II): the C programs are real callers linked against
  the static and shared libraries; see the implementation receipts and the gate run directories.

**Validation after the Gate 3 review-2 repairs**: in-gate run at commit f4af454 with a clean tree,
`results/005-c-abi/runs/20261116T030644Z-gate/summary.txt` (generated output, untracked): fmt, clippy `-D warnings` and `cargo test` ok for
all eight feature combinations; five parallel C-interface repeats with fresh C builds ok; 1.85.0
build ok; three C-path benchmark bundles (heap difference 0 B, N = 3; wall-time median differences
−0.01, +0.49, −0.36 ms Gurobi and +0.06, +0.05, −0.58 ms MOSEK, N = 5, all within the limit);
default benchmark ok (the regression comparison is recorded below, after the Gate 3 review-3
repairs).
| 2026-11-16 | Gate 3 review 3 | FAIL, three findings: C-layer checks without failing-input tests (and the misalignment check missing from the contract); the regression comparison no longer recorded; benchmark figures misreported (bundle count, missing spreads, the C path's environment creation not timed) | gate-reviews/gate-3-20261116T032912Z.md | Tests and contract/header/document text added (da7bf81); the benchmark times each path's environment and reports spreads; validation re-run at da7bf81; regression comparison and corrected figures recorded (below); Gate 3 review 4 requested |

**Validation after the Gate 3 review-3 repairs**: in-gate run at commit da7bf81 with a clean tree,
`results/005-c-abi/runs/20261116T033246Z-gate/summary.txt` (generated output, untracked): fmt, clippy `-D warnings` and `cargo test` ok for
all eight feature combinations; five parallel C-interface repeats with fresh C builds ok; 1.85.0
build ok; link demonstration re-run (static and shared, both backends: Optimal, objective 5,
Verified). C-path benchmark, three bundles (`capi-bench-1..3.log`): heap difference 0 B, Rust and C
each N = 3 with zero spread; wall-time median differences +0.86, −0.15, −0.09 ms (Gurobi) and
+0.78, −0.32, +0.46 ms (MOSEK), N = 5 each, all within max(1 %, 1 ms); environment creation now
timed per path (N = 1): Gurobi 1.19–1.27 ms Rust, 1.25–1.30 ms C; MOSEK 6.5–7.5 ms Rust,
5.9–6.5 ms C.

**Regression comparison (constitution II; Development Workflow)** — the default benchmark at
da7bf81 (`results/005-c-abi/runs/20261116T033538Z-bench/summary.md`) against feature 003's
release-profile bundle (main checkout,
`results/003-llm-interface-toy-instances/runs/20261115T160715Z-bench/summary.md`),
iDopaNeuroC_problem_LP with `usize` row indices, N = 5 each: heap peak during load 78 536 B in
both (zero spread); overhead ratio (validation + optSolver load + extraction + verification) /
solve, Gurobi 0.023 (min 0.015, max 0.025) against 0.018 (min 0.014, max 0.023), MOSEK 0.015
(min 0.010, max 0.019) against 0.018 (min 0.017, max 0.019) — both under the 0.05 limit;
validation phase, Gurobi run 1.60e-4 s (min 1.52e-4, max 1.72e-4) against 7.90e-5 s (min 7.23e-5,
max 1.75e-4), MOSEK run 1.46e-4 s against 1.50e-4 s. Reading: no regression. The validation code
(`CscRef::new`, `Problem::new`) is unchanged by this feature and independent of the backend, and
feature 003's own MOSEK run measured it at 1.50e-4 s against 7.90e-5 s in its Gurobi run of the
same session, so the Gurobi difference is variation between runs and sessions, not a change.

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
- Approved: yes (Gate 2 review 7, 2026-11-16)
- Scope: all
- Tasks approved: T001–T022

## Pointers
- Exploration: none (direct request; see spec Exploration field)
- Gate reviews: specs/005-c-abi/gate-reviews/
- Implementation receipt(s): specs/005-c-abi/agent-runs/20261116T022035Z-implement/implementation-receipt.md
  (T001–T022); specs/005-c-abi/agent-runs/20261116T033623Z-gate3-repairs-2/implementation-receipt.md (Gate 3 review-3 repairs); specs/005-c-abi/agent-runs/20261116T030638Z-gate3-repairs/implementation-receipt.md (Gate 3 repairs)
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
- requirements.md, the success-criteria item (CHK015): resolved at Gate 3 (see the checklist).
