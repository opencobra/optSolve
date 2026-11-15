---
name: "speckit-human-loop"
description: "Orchestrate GitHub Spec Kit's core commands as bundled phases, each gate passed by an independent fresh-context compliance review. Use when the user wants semi-automated spec-driven development: requirements preparation, implementation preparation, an architecture-delta gate, an approval gate before any source-code change, scoped implementation, then verification and closeout. Embeds and invokes the installed core Spec Kit skills (constitution, specify, clarify, checklist, plan, tasks, analyze, implement) rather than replacing them."
argument-hint: "<explore <question> | feature request | continue | prepare | implement approved | closeout> [--scope all|slice:<phase>|tests-only]"
compatibility: "Requires a Spec Kit project initialized with .specify/ and the core speckit skills installed. Honors the project constitution at .specify/memory/constitution.md, including its implementation gate and implementation-receipt ledger."
metadata:
  author: "local"
  source: "pointer to .specify/extensions/human-loop/workflow.md"
user-invocable: true
disable-model-invocation: false
---

# Human Loop Orchestrator — pointer

The authoritative, agent-neutral definition of this workflow lives at:

**`.specify/extensions/human-loop/workflow.md`**

Read that file now and follow it exactly. It defines Bundles 0–4, Gate E,
Gate 1, **Gate 2A (architecture delta review)**, Gate 2, and Gate 3.

User input: $ARGUMENTS

This file is a thin pointer and MUST NOT restate, summarise, or extend the
workflow. Behaviour is changed by editing the neutral definition, never by
growing this file — see the constitution's "Agent Instruction Files Delegate To
Spec Kit" (Governance) and Principle XIII-B. A rule that lives only here does
not survive a switch to another agent, which is precisely what XIII-B forbids.

Claude-native affordances the neutral definition names generically: run each gate's
compliance review with the Agent tool (a new fresh-context subagent), use `AskUserQuestion`
only for decisions the constitution reserves for a human, track bundles with the task list,
and verify with Cargo.
