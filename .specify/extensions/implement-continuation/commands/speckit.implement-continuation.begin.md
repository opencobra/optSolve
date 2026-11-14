---
description: "Mark an authorised implementation run as in flight"
---

# Mark An Implementation Run In Flight

Record that an implementation run has been authorised and has not yet been
declared finished. This command is invoked as a `before_implement` hook.

It makes no approval decision. It writes one state flag, so that a trigger outside Spec
Kit can later ask "is a run in flight?" without knowing anything about tasks,
scope or gates, and records a task-count baseline in feature state for the later
progress check. Every question about *whether to continue* belongs to
`speckit.implement-continuation.continue`.

## Procedure

1. Run `.specify/scripts/bash/check-prerequisites.sh --json --require-tasks
   --include-tasks` from the repository root to resolve `FEATURE_DIR`.

2. Write `.specify/.implement-in-flight` containing, one per line:

   - `feature_dir=` the resolved `FEATURE_DIR`
   - `authorised_at=` the current UTC timestamp, ISO 8601
   - `iterations=0`

   If the file already exists and names the same feature, leave it alone and
   leave `iterations` as it stands — a resumed run is the same run, and resetting
   the count would defeat the iteration cap. If it names a **different** feature,
   overwrite it and note the replacement, because the earlier run is no longer
   the one in progress.

3. Before task execution, record the approved task IDs and their unchecked count
   in the feature's `human-loop.md` as this implementation cycle's baseline.
   Resolve the scope from its "Approved Implementation Scope" section; if it
   cannot be resolved, stop and report rather than assuming all tasks are approved.
   This task baseline belongs in feature state, not in the marker.

4. Say nothing further. Do not report the marker to the human as though it were
   an event; it is bookkeeping, and the implementation run is what matters.

## Who removes it

Only `speckit.implement-continuation.continue`, on every path where it decides
implementation is over. This command never removes the marker, and neither does
any trigger.

A marker left behind by an interrupted run is harmless: it causes continuation to
be *attempted*, not performed, and the continuation command applies the full
predicate before doing anything. It may be deleted by hand at any time.

## What this does not do

It does not authorise implementation. Principle VI's requirement of an explicit
implementation invocation is untouched — this command only records that such an
invocation happened, and runs as a hook of that invocation rather than in place
of one. It does not approve or modify any task; the feature's existing approval
determines the baseline.
