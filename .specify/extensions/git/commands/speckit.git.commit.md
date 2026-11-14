---
description: "Auto-commit changes after a Spec Kit command completes"
---

# Auto-Commit Changes

Automatically stage and commit all changes after a Spec Kit command completes.

## Behavior

This command is invoked as a hook after (or before) core commands. It:

1. Determines the event name from the hook context (e.g., if invoked as an `after_specify` hook, the event is `after_specify`; if `before_plan`, the event is `before_plan`)
2. Checks `.specify/extensions/git/git-config.yml` for the `auto_commit` section
3. Looks up the specific event key to see if auto-commit is enabled
4. Falls back to `auto_commit.default` if no event-specific key exists
5. Uses the per-command `message` if configured, otherwise a default message
6. If enabled and there are uncommitted changes, runs `git add .` + `git commit`

## Commit contents (constitution: "Commits Carry In-Flight Manual Work")

A commit MUST NOT strand the maintainer's manual edits. Whether this command is
invoked as a hook or a commit is made by hand, the commit MUST carry the
in-flight manual changes in the working tree alongside the Spec Kit work, and the
message MUST note them.

Before committing:

- Inspect `git status --short` and `git diff --stat`. Do not commit blind.
- Include tracked modifications and deletions.
- Do NOT sweep untracked files. Name each one deliberately and check it against
  Principle IX first, so generated output, run bundles, executed notebooks, and
  large binaries are never added by accident.
- Never stage gitignored artifacts, credentials, secrets, or personal data.
- Surface — do not silently combine — an in-flight change that looks like a
  partial refactor, a debugging scaffold, or a file you cannot read.

In the message:

- Distinguish the Spec Kit work from the manual in-flight edits it carries.
- List the manual paths, one line each, saying what the edit appears to change.
- State verification status plainly. If you did not verify a manual edit, say so
  rather than implying it was reviewed or tested.

Carrying a human's manual edit into a commit is NOT an agent implementation: it
requires no implementation receipt, no task, and no implementation invocation
(Principle VI is unchanged). The message note is its audit record.

> **Known gap**: the `auto-commit.sh` / `auto-commit.ps1` scripts run a bare
> `git add .` with a fixed message. That satisfies the sweep but not the note,
> and applies none of the guards above. Fixing the scripts is a script change and
> therefore goes through the Spec Kit implementation gate, not an amendment.
> Until that lands, an agent committing on your behalf must apply this section
> manually.

## Execution

Determine the event name from the hook that triggered this command, then run the script:

- **Bash**: `.specify/extensions/git/scripts/bash/auto-commit.sh <event_name>`
- **PowerShell**: `.specify/extensions/git/scripts/powershell/auto-commit.ps1 <event_name>`

Replace `<event_name>` with the actual hook event (e.g., `after_specify`, `before_plan`, `after_implement`).

## Configuration

In `.specify/extensions/git/git-config.yml`:

```yaml
auto_commit:
  default: false          # Global toggle — set true to enable for all commands
  after_specify:
    enabled: true          # Override per-command
    message: "[Spec Kit] Add specification"
  after_plan:
    enabled: false
    message: "[Spec Kit] Add implementation plan"
```

## Graceful Degradation

- If Git is not available or the current directory is not a repository: skips with a warning
- If no config file exists: skips (disabled by default)
- If no changes to commit: skips with a message
