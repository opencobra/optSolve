# Specification Quality Checklist: A C Interface Over The Solver-Independent API

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-11-15
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] CHK001 No implementation details (languages, frameworks, APIs) — the feature *is* a C
  interface and a build artefact, so the C language, compile/link flags and the Cargo feature are
  the requirement, not an implementation choice; how the header is produced, the type layouts and
  function names are left to the plan.
- [x] CHK002 Focused on user value and business needs (the COBRA gateway and any C caller)
- [x] CHK003 Written for non-technical stakeholders — as far as a library-interface feature allows;
  every concept is named descriptively (XII-A)
- [x] CHK004 All mandatory sections completed

## Requirement Completeness

- [x] CHK005 No [NEEDS CLARIFICATION] markers remain — the two in the memory-limit requirement
  (FR-011) and the thread question were decided on 2026-11-15 under constitution 3.0.0 and
  recorded with dated vendor evidence in the spec's Clarifications section (closing artifact:
  spec.md, Clarifications, the thread-rule requirement FR-008 and the memory-limit requirement
  FR-011).
- [x] CHK006 Requirements are testable and unambiguous — re-verified 2026-11-15 after the third
  Gate 1 review (the agreement criterion SC-001 now refers to the comparison tolerance in words; the
  memory-limit criterion SC-005 fixes one thread, a limit far below need, the instance and N = 3);
  after the second (the comparison rule fixes a deterministic configuration and states its
  tolerance in words with rationale); after the first: the output-array rule (wrong length, quantity not returned) and the C-versus-Rust
  comparison rule are now decided in the Clarifications; each functional requirement is checked by
  the header-agreement requirement (FR-013), the C-level test requirement (FR-014), the
  narrowest-check requirement (FR-022) or a success criterion.
- [x] CHK007 Success criteria are measurable (the heap, wall-time and environment-reuse criteria,
  SC-002 to SC-004, give instance, solver versions, N and statistic)
- [x] CHK008 Success criteria are technology-agnostic — except where the feature's subject is a
  technology (C, link flags); accepted for this feature
- [x] CHK009 All acceptance scenarios are defined
- [x] CHK010 Edge cases are identified
- [x] CHK011 Scope is clearly bounded (MILP, callbacks, persistent models, lazy vendor loading out)
- [x] CHK012 Dependencies and assumptions identified

## Feature Readiness

- [x] CHK013 All functional requirements have clear acceptance criteria — re-verified 2026-11-15
  after the second and third Gate 1 reviews (deterministic comparison configuration; the
  memory-limit criterion made reliably testable):
  the comparison rule (Clarifications) defines vector agreement; the mixed-integer extension point
  (FR-020) now has an observable check (a caller structure of an earlier, shorter size is
  accepted).
- [x] CHK014 User scenarios cover primary flows
- [ ] CHK015 Feature meets measurable outcomes defined in Success Criteria — accepted open for the
  wall-time criterion (SC-003) only, by the agent at Gate 3 (constitution VI-B), rationale and
  evidence in human-loop.md ("Wall-time criterion (SC-003) — reading and checklist decision"):
  one of six comparisons at 3e7aeb0 exceeded the 1 ms limit by 0.012 ms, and the measurement's
  resolution at N = 5 is about the limit itself. Met, with evidence in the implementation receipts
  and human-loop.md: agreement (SC-001), heap (SC-002), environment reuse (SC-004), memory limit
  (SC-005), errors (SC-006), link demonstration (SC-007).
- [x] CHK016 No implementation details leak into specification (see the first content-quality item,
  CHK001)

## Notes

- The item on meeting the success criteria (CHK015) is accepted open for the wall-time criterion
  only (routing recorded in human-loop.md).
