# Implementation Receipt — feature 005, repairs after Gate 3 review 4

## Prompt

Within the scope Gate 2 review 7 approved (tasks T001–T022), repair the two findings of Gate 3
review 4 (`gate-reviews/gate-3-20261116T034201Z.md`), re-run the validation at a committed
revision, and request a new fresh-context Gate 3 review.

## Final response

Gate 3 review 4 is repaired: the five NULL top-level checks (`optsolver_solve` and
`optsolver_solve_into` with NULL `out`, `optsolver_capabilities` with NULL `out`,
`optsolver_env_capabilities` with NULL `env` and with NULL `out`) now have failing-input tests in
tests/c/errors.c; the header, contract and interface document state the NULL rule for top-level
arguments and that only array pointers are checked for alignment; output buffers are filled with
NaN only after every buffer passes its own checks; and the regression record carries its spreads.
The previous receipt's claim that every untested check had a test was too broad — these five were
missing. The validation re-run at 3e7aeb0 passed everything except one of six wall-time
comparisons (MOSEK, +1.012 ms against a 1 ms limit); the success-criteria checklist item is
accepted open for that criterion with its rationale. Gate 3 review 5 is next.

## Diff summary

Commit 3e7aeb0: tests/c/errors.c (five NULL cases); src/capi/result.rs (NaN fill after all buffer
checks); src/capi/optsolver.h, docs/llm-interface.md, specs/005-c-abi/contracts/c-interface.md
(rules). This run's records: human-loop.md, checklists/requirements.md, this receipt.

## Tests

`results/005-c-abi/runs/20261116T034733Z-gate/summary.txt` at 3e7aeb0, clean tree: fmt, clippy `-D warnings`, `cargo test` ok for all eight
feature combinations; five parallel C-interface repeats ok; 1.85.0 build ok; link demonstration
ok; default benchmark ok; C-path benchmark: heap 0 B difference in all three bundles; wall time
within the limit in five of six comparisons, one MOSEK comparison at +1.012 ms (FAIL).

## Unresolved issues

The wall-time criterion (SC-003) is not resolved by the N = 5 measurement (accepted open, routed
to the maintainer). Otherwise unchanged from the receipt of the review-1 and review-2 repairs.
