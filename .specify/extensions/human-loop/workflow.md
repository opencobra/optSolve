---
name: "speckit-human-loop"
description: "Orchestrate GitHub Spec Kit's core commands as bundled phases, each gate passed by an independent fresh-context compliance review: requirements preparation, implementation preparation, an architecture gate, an approval gate before any source-code change, scoped implementation, then verification and closeout. This workflow embeds and invokes the installed core Spec Kit skills (constitution, specify, clarify, checklist, plan, tasks, analyze, implement) rather than replacing them."
argument-hint: "<explore <question> | feature request | continue | prepare | implement approved | closeout> [--scope all|slice:<phase>|tests-only]"
compatibility: "Requires a Spec Kit project initialized with .specify/ and the core speckit skills installed for the active agent. Honors the project constitution at .specify/memory/constitution.md, including its implementation gate and implementation-receipt ledger."
metadata:
  author: "local"
  source: "custom:speckit-human-loop"
---

<!--
AUTHORITATIVE, AGENT-NEUTRAL DEFINITION.

This file is the single definition of the human-loop workflow. Per-agent skill
files (.claude/skills/speckit-human-loop/SKILL.md, .agents/skills/speckit-human-loop/SKILL.md,
and the equivalent for any future agent) are THIN POINTERS to this file and MUST
NOT restate or extend it. Constitution: "Agent Instruction Files Delegate To
Spec Kit" (Governance) and Principle XIII-B.

Agent-native affordances named below (the Agent tool, AskUserQuestion, the task
list, MCP servers) are Claude's names for generic capabilities. Under another
agent, substitute that agent's equivalent for each: its fresh-context subagent
mechanism for a gate's compliance review, an interactive prompt for a decision
the constitution reserves for a human, its progress/task mechanism and its tool
integrations. Where an agent cannot spawn a fresh-context reviewer, it stops at
the gate and reports what the reviewer must be given (constitution VI-A) — the
gate itself is never optional.
-->


## User Input

For substantive discussions of an incomplete feature, including status questions
outside a formal gate, apply constitution XI-4 before the final reply: refresh the
single `Current conversation summary` below the title of the existing feature's
`exploration.md`. Read the latest decisions and evidence first, preserve human stop
conditions, and link detailed records rather than copying logs. This is bookkeeping,
not a new gate or permission to resume implementation. Use the exploration template
only if the file is absent; keep the existing feature directory and provenance.

```text
$ARGUMENTS
```

You **MUST** consider the user input before proceeding (if not empty).

# Spec Kit Human Loop

Run the ordinary GitHub Spec Kit workflow as a small number of bundles, each
ending at a gate passed by an independent compliance review (constitution VI-A), so the user does not chain eight commands by hand. This skill is an
**orchestrator**: it must not replace, summarize away, or silently skip the core
Spec Kit commands. At each phase it invokes the installed core skill and follows
its instructions.

This workflow supports each agent through its native tools and the shared
Spec Kit artifacts. Where this skill and the project constitution overlap, **the
constitution wins** and this skill defers to it (see "Constitution reconciliation").

## Non-negotiable rules

1. **No source edits before Gate 2 passes.** Do not modify source code, tests,
   build files, or other implementation artifacts before the Gate 2 compliance
   review has recorded a PASS for the current spec, plan and tasks, and then only
   in the implementation phase (see Constitution reconciliation). Spec Kit
   artifacts under `specs/<feature>/` and the orchestration state file are not
   "source code" and may be written during preparation.
2. **Embed, never emulate, core commands.** Each bundle invokes the corresponding
   installed core skill using the runtime mapping below (e.g. `speckit-specify`). Do not
   hand-write `spec.md`, `plan.md`, or `tasks.md` unless the core skill is
   unavailable and the user explicitly asks for a temporary fallback.
3. **Gates are compliance reviews.** Run the compliance review (see "Gate
   discipline") at Gates E, 1, 2A, 2 and 3 as applicable. Stop for a human only
   on a decision the constitution reserves for one (VI-A: publication,
   constitution amendments, `project.yml` values, legal and licensing questions,
   `SPECKIT OVERRIDE`, reversing a recorded maintainer ruling) or on a blocker
   no permitted agent decision resolves. Do not ask between routine phases.
4. **Record every gate verdict and every decision** in the orchestration state
   file, with its rationale and the review file.
5. **`spec.md` is the authority for requirements.** When it changes, re-derive the
   downstream artifacts (`plan.md`, `tasks.md`, analysis) rather than editing them
   in isolation.
6. **Stop and report** if the repository is not a Spec Kit project, a required core
   command is missing, the active feature cannot be determined, or the approved
   scope cannot be enforced. Do not invent replacements for missing core commands.

## Constitution reconciliation (read first)

Before Bundle 0, read `.specify/memory/constitution.md` if present and obey it.
This skill is explicitly subordinate to it. In particular:

- **Implementation gate.** Implementation runs only through `speckit-implement`
  (or the agent's equivalent), only after the Gate 2 compliance review has
  recorded a PASS for the current spec, plan and tasks, and only within the task
  scope that review passed (constitution VI, VI-A).
- **Run record / receipt.** If the constitution mandates an implementation-receipt
  ledger at a specific path (for example
  `<FEATURE_DIR>/agent-runs/<UTC-timestamp>-<short-name>/implementation-receipt.md`),
  that receipt is the **canonical** record of an implementation run. Do not invent
  a competing record. This skill's `human-loop.md` is a lightweight orchestration
  index only, and must cross-reference (not duplicate) the constitutional receipt.
  Follow the receipt's required sections and concision rules exactly.
- **Language/coding standards.** Surface the constitution's coding standards
  (Principle VII, including the single location for solver parameters, VII-G) as
  checks in the implementation and verification bundles.
- **Run-directory naming.** If the mandated receipt directory name does not match the
  active agent, **do not silently rename it** — the path is fixed by the
  constitution. Follow it; renaming is a `speckit-constitution` change, not an
  ad-hoc edit. The directory is the agent-neutral `agent-runs/`.

## Runtime mapping (how to run each step)

This maps tools under Principle VI-A; it adds no permissions or alternative gates.
It also applies when a core skill or extension is invoked directly.

| Capability | Claude | Codex |
|---|---|---|
| Core skill | Invoke the Skill tool | Read the registered `SKILL.md`, follow its authoritative pointers, and execute its procedure with available tools |
| Gate compliance review | Agent tool, a new fresh-context subagent (never a fork, never one that produced or reviewed the artefacts before) | Available collaboration tools that start a subagent with an independent context |
| Human-reserved decision (VI-A) | `AskUserQuestion` | Use an available structured question tool only when it permits this kind of question; otherwise ask explicitly in the response and wait |
| Progress | `TaskCreate` / `TaskUpdate` | Available progress tools and concise updates; durable state remains in feature artifacts |
| Review artifacts | `present_files` | Clickable file links to the actual artifacts |

On Codex, `request_user_input` may be restricted to Plan mode or optional
clarifications: do not use it for approval when the tool instructions prohibit
that. If `request_user_input_async` is available for approval, a returned tool
call is not the answer; wait for the user's actual response before dependent
work. A preselected choice, silence or elapsed time never passes a gate. An
already recorded approval remains valid for its unchanged scope; do not ask again
solely because the agent or session changed.

Resolve dotted hook commands to the registered hyphenated skill name. An
`EXECUTE_COMMAND:` line is a dispatch description, not evidence of execution:
actually run the skill procedure and wait for its result. Apply the same mapping
to implementation-continuation hooks. Report missing tools or failed dispatch;
do not report an unexecuted hook as completed. The continuation extension's
separate harness-trigger limitation still applies: this document installs no
automatic end-of-turn trigger.

The following Claude names use the equivalents above on other runtimes:

- **Invoke core skills with the Skill tool**, not shell aliases. Use
  `speckit-constitution`, `speckit-specify`, `speckit-clarify`, `speckit-checklist`,
  `speckit-plan`, `speckit-tasks`, `speckit-analyze`, `speckit-implement`.
- **Track bundles and tasks with the task list.** At the start of a run, create a
  task per bundle with `TaskCreate`; mark each `in_progress`/`completed` with
  `TaskUpdate`. During implementation, mirror `tasks.md` entries as tasks so the
  user sees live progress.
- **Run each gate as a compliance review with the Agent tool** (see "Gate
  discipline"). Use `AskUserQuestion` only for a decision the constitution
  reserves for a human.
- **Verify with Cargo**: `cargo fmt --check`, `cargo clippy` with warnings denied,
  the narrowest relevant `cargo test` first (unlicensed tier everywhere, licensed tier
  where a solver licence is present), feature-combination checks, and benchmarks for
  any load, solve or result path. Redirect long output to the run bundle's log
  (Principle XV-D). Capture raw solver status codes faithfully.
- **Surface review artifacts** to the user with `present_files`
  (`implementation-review.md`, comparison reports, diffs).

## Hook awareness

If `.specify/extensions.yml` exists with `auto_execute_hooks: true`, each embedded
core command may trigger `before_*`/`after_*` hooks (for example git auto-commit
and agent-context refresh). Because a bundle runs several phases in sequence, this
can produce several commit prompts. At the start of a bundle, tell the user that
per-phase hooks will fire, and offer to (a) let each hook prompt as configured, or
(b) defer commits and make one commit at the end of the bundle. Either way, the
feature's work MUST be committed to its own branch before the next gate's compliance
review (constitution, "Feature Branches Begin And End At The Integration Branch";
feature-branch commits are agent actions, integration-branch commits are not). Never
disable hooks silently.

## Orchestration state file

Maintain `specs/<feature>/human-loop.md` once the feature directory exists (create
it immediately after `speckit-specify` creates the directory). Keep it lightweight;
it indexes the run and points at the constitutional receipt rather than copying it.

```markdown
# Human Loop State

## Current State
- Status:
- Active feature directory:
- Last completed bundle:
- Source code modified by this workflow: yes/no

## Core Command Ledger
- constitution:   (checked | invoked | n/a)
- specify:
- clarify:
- checklist:
- plan:
- tasks:
- analyze:
- implement:

## Gate Reviews And Decisions
| Date (UTC) | Gate | Decision or verdict (with rationale) | Review file | Consequence |
|---|---|---|---|---|

## Human-Reserved Decisions Pending (constitution VI-A)

## Approved Implementation Scope
- Approved: no
- Scope: (all | slice:<phase/task ids>)
- Tasks approved:
- Tasks deferred:
- Files allowed:
- Files not allowed:

## Pointers
- Exploration: specs/<feature>/exploration.md (+ exploration/ supporting material), or "none"
- Implementation receipt(s): <path(s) under the constitution's receipt ledger>
- Implementation review: specs/<feature>/implementation-review.md

## Open Risks and Ambiguities
```

## Bundle 0: project and phase detection

1. Confirm the current directory is inside a Git repository and `.specify/` exists.
   If not, stop and ask the user to run `specify init` or give the correct root.
2. Read `.specify/memory/constitution.md` and apply "Constitution reconciliation".
3. **Integration check** (constitution, "Feature Branches Begin And End At The
   Integration Branch": entry checks the loop is closed). The integration branch is
   `branches.integration` in `.specify/memory/project.yml`. Check `git status`, then confirm:
   - **Remotes agree** (only if remotes exist): the local integration branch and the
     integration branch on every remote are the same commit. Ask each host with
     `git ls-remote`; a tracking ref is not evidence.
   - **Closed features are integrated.** Every feature whose `human-loop.md` records an
     accept-and-close is contained in the integration branch
     (`git merge-base --is-ancestor <branch> <integration>`).
   - **The working tree is this feature's alone.** It holds no uncommitted work belonging to
     another feature, and it is not checked out on another open feature's branch.

   Report any failure with the commits or files involved, and repair it before new work
   starts, unless the maintainer explicitly defers it.
4. Determine the active feature from the current branch and `specs/`.
5. Confirm the core skills in the active agent's registry: `.agents/skills/`
   for Codex, `.claude/skills/` for Claude. Follow their shared Spec Kit pointers.
   Do not prefer another agent's adapter over the active agent's registered skill.
6. Decide the requested mode: `explore`, `start`, `prepare`, `implement`,
   `continue`, or `closeout`, from the request and the feature's recorded state
   (the last PASS in `gate-reviews/`). If the request does not determine it,
   ask the requester:
   - Explore a candidate feature before specifying (Bundle E — problem-space
     research only, no spec yet).
   - Start a new gated workflow from this request.
   - Continue the active feature workflow.
   - Prepare the implementation review for the active feature.
   - Implement the approved task scope.
   - Verify and close out the active feature.

## Bundle E: problem-space exploration (pre-feature)

Runs BEFORE any feature directory exists. Purpose: record the rationale and
conceptualisation that decides whether a feature should be specified at all —
upstream of, and distinct from, `/speckit-plan`'s Phase 0 solution-space
`research.md`. Entered via `explore <question>`, or from Gate 3's "open an
exploration" option at close-out of the previous feature.

State-externalization rule (adapted from the community Research Harness /
Harness-1 design): the exploration file, not this conversation, is the working
memory. Any LLM — and several different LLMs concurrently — may contribute;
every finding is written to the file at the moment it is learned, so a session
is disposable and any agent can resume from the file alone.

1. **Locate or create the artifact.** An exploration is the FIRST artifact in
   a feature directory, created before any spec exists, instantiated from
   `.specify/templates/exploration-template.md`. At exploration start the
   directory is provisional: `specs/<YYYYMMDD-HHMMSS>-<slug>/exploration.md`
   with a full UTC timestamp — exactly that form, because
   `create-new-feature.sh` excludes `^[0-9]{8}-[0-9]{6}-` from its
   sequential-number scan (a bare date prefix would be parsed as a feature
   number and corrupt the sequence). The directory receives its mature
   `<###-slug>` name only at the "go" decision (Gate E). Never overwrite an
   existing exploration: scan `specs/*/exploration.md` for an open one
   matching the question first, and resume it (append a new mission `M#` if
   the question is genuinely new). Multi-file supporting material goes in an
   `exploration/` subdirectory beside it.
2. **Obey the template's multi-agent rules**: append-only ledgers (Evidence,
   Claims & verdicts, Observations, Dead ends) with agent identity + UTC on
   every entry; unique sequential IDs (re-read the file on collision); dedup
   against the Observations log before searching; refuted entries are marked,
   never deleted.
3. **Explore loop** (decide -> act -> bookkeep): each iteration choose one of
   SEARCH (repo; prior `specs/*/exploration.md` and `specs/*/exploration/`
   material; generated output; the literature home and its sources index; vendor
   headers and documentation),
   INSPECT (a known candidate source), CURATE (promote a finding into the
   Evidence ledger with an importance tag and source pointer), or STOP. After
   every action append one Observations row (<=3 lines).
4. **Verify before believing.** Load-bearing claims get an adversarial check
   against the primary source (the installed vendor header, vendor documentation,
   or a read-only probe) and a verdict row: verified / refuted / unverifiable,
   with confidence. Refuted directions go to Dead ends.
5. **Stop rule**: mission answered, or 3 consecutive actions yielding no new
   live evidence, or human interrupt.
6. **Regenerate Synthesis** at every session end (and before Gate E): rewrite
   the Synthesis section whole — identity, UTC, highest entry ID incorporated,
   contradictions carried forward — so the evolving summary of the exploration
   is itself the artifact, whichever agent wrote last.
7. Code, tests and build edits remain prohibited in this bundle (XI-5).

Then **Gate E**.

### Gate E — exploration decision (compliance review)

The agent chooses one outcome, records it with its rationale and the request it
answers, and passes Gate E only on a PASS from the compliance review. Outcomes:
- Go: mature the directory name, then seed specify (proceed to Bundle 1).
  Allocate the next sequential feature number (max over `specs/` dirs and
  feature branches, + 1), `git mv specs/<timestamp>-<slug>
  specs/<###-slug>`, then invoke `create-new-feature.sh` with
  `--number <###> --short-name <slug> --allow-existing-branch` so `spec.md`
  lands beside `exploration.md`; seed `speckit-specify` from the Decision
  section's distilled request and fill the spec's **Exploration** field.
- Continue exploring (name the open mission or add a new one).
- Park: record Status `parked` with rationale; resumable later.
- No-go: record Status `no-go` with rationale; the timestamped directory
  remains, holding the exploration, as the recorded dead end (no feature
  number is consumed).

The agent fills the Decision section and names the review file (constitution
XI-4). Record the choice in the exploration file (there is no `human-loop.md`
yet — it begins at specify); the review goes to the exploration directory's
`gate-reviews/`, which moves with it on "go".

## Bundle 1: requirements preparation

**Before `speckit-specify` creates the branch**, satisfy the start half of the
constitution's "Feature Branches Begin And End At The Integration Branch": check out
the integration branch (`branches.integration` in `project.yml`), update it from its
remotes if any exist, and create the feature branch **from there**.

**If another feature is still open** — its branch unmerged into the integration branch,
or its work uncommitted in the current checkout — do not start the new feature in this
working tree. Create the new feature's own worktree from the updated integration branch
and run every later bundle of the new feature there:

```bash
git worktree add -b <NNN-slug> <worktree-path> <integration>
```

`create-new-feature.sh` runs `git checkout -q -b <branch>` with **no base**, so it
inherits whatever `HEAD` happens to be. The script does not enforce the base, so
**this step is the enforcement.**

Embedded core skills, in order:

1. `speckit-constitution` — **check, do not regenerate**. If the constitution is
   missing, stop and ask for governing principles (or invoke it if the user
   supplied enough detail). If present, read it and record `constitution: checked`.
   Only invoke it to create/revise principles when the user explicitly asks.
2. `speckit-specify` with the feature request.
3. `speckit-clarify`. If it raises blocking questions, stop at Gate 1.
4. `speckit-checklist`. If failures are blocking, stop at Gate 1.

Ensure `human-loop.md` exists and records completed commands. Then **Gate 1**.

### Gate 1 — requirements decision (compliance review)

The agent answers any open clarification questions itself (recording each answer
and its rationale in the spec's Clarifications section), chooses one outcome, and
passes Gate 1 only on a PASS. Outcomes:
- Continue to implementation-preparation bundle.
- Revise the specification, then re-run clarify and checklist.
- Split this into smaller features.

No source code has been modified. Record the outcome and the review file.

## Bundle 2: implementation preparation

Bundle 2 is split by an architecture gate. Steps 1-2 run, the workflow **stops at
Gate 2A**, and steps 3-5 run only on an approving verdict.

1. `speckit-plan` (use the user's technical constraints; ask only for missing
   required constraints). The plan's **Benchmarks and evidentiary preconditions**
   Constitution Check bullet (Principle XIV) MUST be answered here, not deferred: the
   benchmark instances with provenance, the recorded effective configuration, separate
   phase timing and dedicated-process memory measurement, and — if any finding could be
   "no difference" — the task that demonstrates the mechanism is engaged. `N/A` is
   permitted only when the feature makes no performance claim. A feature that cannot
   answer these has not established that its measurements can bear its claims, and the
   honest outcome at Gate 2A is to revise the plan.
2. Write `specs/<feature>/architecture-delta.yaml` and the plan's **Architecture
   Impact** section (Principle XIII-C). Declare added / modified / removed
   components, every new and removed dependency edge with its reason, changed
   public interfaces and persistent data structures, and any edge the feature
   declares forbidden. Run the conformance check against `docs/architecture/`
   (`repo_folder_organisation.md`, `components.yaml`, `boundaries.yaml`) and all accepted ADRs
   under `docs/architecture/decisions/`, and report: components affected, responsibilities
   added or moved, new dependency edges, any cycle or boundary violation,
   whether the same behaviour is achievable with less coupling, and the smallest
   conforming alternative. No source edits.

   When an accepted/proposed architecture preview is shown, colour added and
   modified components amber (`#ffcc33`) in the proposed view by default.
   Keep the accepted view unchanged and use the same saved coordinates for
   unchanged elements. Identify removed components separately in the delta,
   since they cannot appear in the proposed view. This highlight is a
   presentation aid; the declared model and `architecture-delta.yaml` remain
   authoritative. Record any exception in the viewing instructions.

`present_files` the delta, then **always** run Gate 2A.

### Gate 2A — architecture delta review (compliance review)

The Gate 2A reviewer issues the verdict. Names for a new layer or a renamed
element are decided by the agent in the delta and checked by this review
(constitution XIII-F).

Before choosing a verdict, examine three things alongside the delta itself
(Principle XIII-C, XIII-E, XIII-G). Each is grounds for `REVISE_ARCHITECTURE`; none
is a separate gate:

- **Semantic duplication** the delta leaves in place — a second implementation of a
  responsibility an existing component already owns. A deliberate independent verifier
  is not duplication.
- **Reachability** of anything proposed as shared — which callers must reach it, and
  whether Rust visibility and crate dependencies allow them to.
- **Adoption scope** — whether the plan names real consumers, or only asserts that the
  capability is reusable.

Exactly the four verdicts the constitution defines (Principle XIII-C):

- `APPROVE` — the delta conforms; continue to task generation.
- `APPROVE_WITH_ADR` — conforms, but records a durable decision: write the ADR
  under `docs/architecture/decisions/` first, then continue.
- `REVISE_ARCHITECTURE` — the requirements stand and the delta must change. Name
  the binding constraint (for example: put the conversion in the backend that needs
  it rather than in the solver-neutral core). Re-run step 2; the specification is untouched.
- `REJECT` — do not proceed on this architecture.

**Neither approving verdict may be recorded while the plan's reuse assessment is
unfilled.** Principle V requires the reuse question to be ANSWERED rather than assumed, and
the plan template's **Reuse and adoption assessment** is where the answer goes. The test is
mechanical, so that the refusal is reproducible rather than a matter of opinion: for each
applicable bullet in that section whose label is italicised — `- *Label*: …`, and equally
`- *Label* (XIII-E): …`, where a principle reference sits outside the asterisks — the text
after the **last** colon on the label line must be non-empty and must not begin with `[`,
which is how the template marks a placeholder. Match the label, then the colon that ends
it; a rule anchored on `*:` alone silently skips the three bullets the template writes with
a trailing principle reference, one of which is the Principle II behavioural-integration
evidence. The XIII-G sub-bullets are applicable only when the feature introduces or
materially changes a shared capability. Name every bullet that fails, and withhold `APPROVE` and `APPROVE_WITH_ADR`
until they are answered.

This adds no gate. It is a condition on a verdict this gate already issues, examining what
the three checks above already require, in the place the plan template already provides. The
mandatory gates remain exactly five: E, 1, 2A, 2 and 3. Keep the answer proportionate to the
change — a small change needs a few lines, and an exhaustive repository census is never
required.

Only `APPROVE` and `APPROVE_WITH_ADR` permit `speckit-tasks` to run — a task
list generated without an approving verdict is not a valid implementation
prerequisite (Principle VI phase order, Principle XIII-C). Record the verdict,
its rationale, and the UTC date in both `human-loop.md` and the plan's
Architecture Impact section.

On an approving verdict, continue:

3. `speckit-tasks`.
4. `speckit-analyze` — classify findings as blocking / should-fix / acceptable /
   deferred.
5. Write `specs/<feature>/implementation-review.md` (no source edits):

```markdown
# Implementation Review

## Summary
## Embedded Core Commands Completed
- constitution / specify / clarify / checklist / plan / architecture review / tasks / analyze:
## Architecture Verdict (Gate 2A)
- Verdict (APPROVE | APPROVE_WITH_ADR | REVISE_ARCHITECTURE | REJECT):
- ADR required / written:
- Date (UTC):
## Cross-Artifact Analysis Summary
## Proposed Implementation Scope
- Tasks proposed:
- First independently testable slice:
- Files likely to change:
- Files that should NOT change:
## Tests and Validation Expected (name the narrowest relevant test first)
## Blocking Issues
## Acceptable Risks
## Gate 2 Approval (compliance review)
- Approved: no
- Proposed scope (all | slice):
- Approved tasks/scope:
- Gate 2 review file and verdict:
- Date (UTC):
```

`present_files` the review, then **always** run Gate 2.

### Gate 2 — implementation approval (compliance review)

The agent proposes the implementation scope (all tasks, or a named independently
testable slice) in `implementation-review.md`; the Gate 2 reviewer checks the
spec, plan, tasks, analysis and that scope. Only a PASS authorises implementation,
and only of the scope it passed; on FAIL the task list or plan is repaired
(plan changes go back through Gate 2A) and a new reviewer reviews. Record the
approved scope in both `human-loop.md` and `implementation-review.md`.

## Bundle 3: approved implementation

Preconditions: `implementation-review.md` exists; `gate-reviews/` holds a Gate 2
PASS for the current spec, plan and tasks; the approved scope is "all" or a named
slice.

1. Mirror the approved `tasks.md` entries into the task list.
2. Invoke `speckit-implement`, constrained to the approved scope. Do not edit
   source directly and do not bypass `speckit-implement`.
3. Follow task order/dependencies from `tasks.md`. Stay inside the approved files;
   if a change outside scope is genuinely required to keep tests meaningful, record
   it as a deviation rather than silently expanding scope.
4. Apply constitution coding standards as you go (Principle VII, including VII-G).
5. Run the narrowest relevant test with Cargo after the change.
6. Write the constitutional **implementation receipt** at its mandated path with the
   required sections (Prompt, Final response, Diff summary, Tests, Unresolved
   issues). The `Final response` must be the actual final user-facing completion
   text, not a paraphrase. Point `human-loop.md` at the receipt.

## Bundle 4: verification and closeout

1. Inspect `git diff --stat` and `git diff`.
2. Compare the work against `tasks.md` and `implementation-review.md`.
3. **Architecture reconciliation** (Principle XIII-D). Re-run the conformance
   check and compare the OBSERVED architecture change against the delta approved
   at Gate 2A, reporting four categories explicitly:
   - approved and implemented;
   - approved but not implemented;
   - implemented but not approved;
   - existing violations worsened.
   Any implemented-but-not-approved component or dependency edge is drift and
   MUST be reported, never silently accepted. Record the result in
   `human-loop.md`.

   Within the changed scope, also reconcile (Principle XIII-D):
   - actual consumer adoption against the adoption planned at Gate 2A;
   - duplicated responsibilities that remain;
   - the behavioural integration evidence Principle II requires;
   - architecture prose, source references and contracts the change invalidated;
   - remaining work, routed to a named owner or a recorded decision, not left implicit.

   Do not conflate three separate facts: the model is VALID (its YAML parses and meets
   its schema), the generated artifacts are FRESH (built from the model as it stands),
   and the source CONFORMS (the tree matches the model). A valid model, a repaired
   source reference or a regenerated diagram establishes one of the first two and says
   nothing about the third.
4. Run or report the in-gate checks the constitution's Development Workflow And Quality
   Gates lists (narrowest tests first), and the benchmarks with their recorded
   configuration (Principle XIV). Preserve exact raw solver status codes.
5. Optionally re-run `speckit-analyze` post-implementation (does not replace the
   pre-implementation analyze).
6. Update `human-loop.md`: diff summary, tests, unresolved issues, deviations and
   next recommended choice. Confirm the constitutional receipt is complete.
7. **Knowledge promotion** (Governance, "Project Knowledge Lives In The
   Repository"). Ask explicitly whether anything learned in this feature
   generalises beyond it, and route each item to its durable home: a rule about
   how work is conducted → the constitution via `/speckit-constitution`;
   operational how-to → a skill under `.specify/skills/`; an observed fact about
   the code, toolchain, or environment → `docs/`. A lesson left only in this feature's directory binds
   this feature and nothing else — no later feature is required to read it — so
   "recorded in `lessons.md`" does NOT discharge the obligation for anything
   general. Record the promotion decision per item in `human-loop.md`, including
   a deliberate decision that an item is feature-specific and needs no promotion.
   Nothing learned may be left in an agent-private memory store.
8. **Gate 3** compliance review, given the diff against the feature's base and the
   validation evidence.

### Gate 3 — closeout decision (compliance review)

The agent chooses one outcome and passes Gate 3 only on a PASS. Outcomes:
- **Accept and close the feature.** This outcome, and only this one, leads to the
  branch integration the constitution requires under "Feature Branches Begin And
  End At The Integration Branch". Integration and publication are human decisions
  (constitution VI-A): steps 3 to 5 run only on the maintainer's instruction, and
  until then the feature is recorded as "accepted, awaiting integration":
  1. Confirm the feature's work is committed on its branch; nothing belonging to the
     feature may remain uncommitted.
  2. Verify the integration introduces no file exceeding the hosting limit of
     every remote it will be pushed to (`branches.max_file_bytes_remote` in `project.yml`).
  3. On the maintainer's instruction to integrate, bring the integration
     branch up to date with the feature branch — fast-forward where the history allows,
     a merge commit otherwise. **Never force-push it; never rewrite its history.**
  4. Where remotes exist, publish the integration branch to each, ask each host with
     `git ls-remote`, and record in `human-loop.md` the tip each one shows. A failed push
     is recorded as failed, and Bundle 0 of the next feature retries it.
  5. Switch the working branch to the integration branch. If the feature ran in its own
     worktree, remove that worktree with `git worktree remove` instead.
  6. The feature branch is **kept**, not deleted: it is the record of the work.
  The other outcomes below integrate nothing, because the feature is not closed.
- Continue with the next approved slice.
- Fix failing tests.
- Reconcile implementation drift by updating spec/plan/tasks.
- Revert selected changes.
- Open an exploration for the next candidate feature (Bundle E): begin
  `specs/<YYYYMMDD-HHMMSS>-<slug>/exploration.md` from the template (mature
  `<###-slug>` name assigned only at a later "go" decision), recording the
  rationale before any decision to specify. **Suggest
  this proactively at close-out** when the finished feature surfaced follow-up
  questions (deferred gaps, unexplained traces, "future work" notes). If the
  evidence for the next feature is already decisive, a direct follow-up
  specification remains acceptable — note in its spec's **Exploration** field
  why no exploration was needed.

## Gate discipline

**Binding under Principle VI-A.** Where this extension is installed, Gates E, 1,
2A, 2 and 3 are MANDATORY for every agent. A gate MUST NOT be skipped silently: a
deferred or unrun gate is recorded in the feature's `human-loop.md` with its
reason and surfaced at the next gate.

Each gate is passed by a compliance review:

1. Commit the feature's work to its branch, so the review names a revision.
2. Spawn a new fresh-context subagent (Agent tool, not a fork; one that has not
   produced or reviewed these artefacts). Its prompt gives only: the gate; the
   paths of the constitution, the Tier 1 companions and the feature's artefacts,
   with their revision; at Gate 3 also the diff against the feature's base and
   the paths of the validation evidence. Do not pass your own conclusions,
   summaries or a proposed verdict. Ask it to check every principle, cite the
   principle and evidence (file and line, test name, command output) for each
   finding, and return PASS or FAIL (at Gate 2A, one of the four XIII-C verdicts).
3. Write its review, unedited, to
   `specs/<feature>/gate-reviews/gate-<id>-<UTC-timestamp>.md` with the header
   fields the constitution lists (gate, date, reviewer, artefacts and revision,
   findings, verdict).
4. On FAIL, repair the cited findings and repeat from step 1 with a new
   reviewer. Advance only on PASS. A reviewer cannot waive a principle.
5. Record the verdict, the review file and the consequence in `human-loop.md`.

An agent that cannot spawn a reviewer stops at the gate and reports exactly what
the reviewer must be given.

**Checklists at a gate — binding under Principle VI-B.** Before recording any gate
as passed, every item of every checklist under `specs/<feature>/checklists/` MUST
be either resolved (ticked, with the closing artifact identifiable — ticking
asserts verification) or explicitly routed (left open with its destination named:
a later gate, a task ID, or a recorded decision to accept it open with
rationale; accepting open is an agent decision the
reviewer checks). An unmarked item is neither, and a checklist carrying unmarked items
has not passed. Record the routing per item in `human-loop.md` with the item
identifier, its question, and its destination. Do not tick items to clear a gate,
and do not assume unmarked items were considered — a zeroed checklist is as
likely to be unexamined as it is to be stale bookkeeping, and the two are
distinguished only by checking.

## Stop conditions

Stop rather than auto-continuing if: a core skill is missing; the active feature
is undeterminable; the constitution is missing and the user has not supplied
principles; clarify/checklist/analyze report blocking issues no permitted agent
decision resolves; source code would be modified before a Gate 2 PASS; the
approved scope cannot be enforced; a decision the constitution reserves for a
human is needed; or tests fail when the mode was not test repair.

## Concise progress report (after each bundle)

```text
Completed:
- Core skills run:
- Files created/updated:
- Source code modified: yes/no
- Receipt path (if implemented):
- Next gate (compliance review):
```
