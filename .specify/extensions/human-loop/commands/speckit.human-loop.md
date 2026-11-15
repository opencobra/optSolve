---
description: "Run bundled core Spec Kit phases with gates passed by independent compliance reviews before and after implementation (Claude-tailored)."
argument-hint: "<feature request | continue | prepare | implement approved | closeout> [--scope all|slice:<phase>|tests-only]"
---

# Spec Kit Human Loop Orchestrator (Claude)

## User input

```text
$ARGUMENTS
```

You MUST consider the user input before proceeding.

## Behaviour

Follow the agent-neutral workflow definition at
`.specify/extensions/human-loop/workflow.md`. That file is the authoritative
definition of this workflow; the per-agent skill files under `.claude/skills/`
and `.agents/skills/` are thin pointers to it. In summary it:

1. Detects the project, active feature, and requested mode (Bundle 0), reading and
   obeying `.specify/memory/constitution.md`.
2. Bundles requirements preparation — `speckit.constitution` (check), `speckit.specify`,
   `speckit.clarify`, `speckit.checklist` — then runs Gate 1.
3. Bundles implementation preparation, split by an architecture gate —
   `speckit.plan`, then `architecture-delta.yaml` and the Gate 2A review
   (APPROVE | APPROVE_WITH_ADR | REVISE_ARCHITECTURE | REJECT); only an
   approving verdict permits `speckit.tasks`, `speckit.analyze`, plus
   `implementation-review.md` — then always runs Gate 2. Tasks MUST NOT be
   generated before the architecture delta is approved (Principle XIII-C).
4. Implements only the scope the Gate 2 compliance review passed, through
   `speckit.implement`; writes the constitutional implementation receipt.
5. Verifies (git diff, narrowest relevant Cargo tests and benchmarks) and
   closes out at Gate 3.

## Embedding requirement

Invoke the installed core Spec Kit skills via the Skill tool
(`speckit-constitution`, `speckit-specify`, `speckit-clarify`, `speckit-checklist`,
`speckit-plan`, `speckit-tasks`, `speckit-analyze`, `speckit-implement`). Do not
hand-write replacements. If a required core command is unavailable, stop and report
which one is missing.

## Safety rule

Do not modify source code until the Gate 2 compliance review has recorded a PASS.
Run each gate as a fresh-context compliance review (Agent tool); use `AskUserQuestion`
only for decisions the constitution reserves for a human; track bundles and tasks with
the task list.
