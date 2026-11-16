# Implementation Receipt — feature 005, repairs after Gate 3 review 3

## Prompt

Within the scope Gate 2 review 7 approved (tasks T001–T022), repair the three findings of Gate 3
review 3 (`gate-reviews/gate-3-20261116T032912Z.md`), re-run the validation at a committed
revision, and request a new fresh-context Gate 3 review.

## Final response

Gate 3 review 3 is repaired. The C-layer checks that had no failing-input tests now have them —
option flags outside their values (through the C driver, with no environment created), NULL raw
parameter names and string values, misaligned input and output pointers, output buffers NULL with
a nonzero length or too large, and a too-small `buffers.struct_size` (with no environment
created) — and the contract, header and interface document list the misalignment and buffer
checks. The benchmark times each path's environment creation and reports every spread. The
validation is re-run at da7bf81, and the regression comparison against feature 003 and the
corrected wall-time figures are recorded in human-loop.md. Gate 3 review 4 is next.

## Diff summary

Commit da7bf81: src/capi/problem.rs (tests: option flags, NULL raw strings, misaligned input);
src/capi/result.rs (tests: output-array checks, buffers.struct_size before any environment);
tests/c_interface.rs (four C-driver invalid-option cases); benches/load_solve.rs (per-path
environment timing, spreads); src/capi/optsolver.h, docs/llm-interface.md,
specs/005-c-abi/contracts/c-interface.md (checks listed); specs/005-c-abi/tasks.md (T006 text).
This run's records: human-loop.md, this receipt.

## Tests

`results/005-c-abi/runs/20261116T033246Z-gate/summary.txt` at da7bf81, clean tree: fmt, clippy `-D warnings` and `cargo test` ok for all
eight feature combinations (unit tests 61 with all features; C-interface suite 35); five parallel
C-interface repeats ok; 1.85.0 build ok; three C-path benchmark bundles and the default benchmark
ok; link demonstration re-run, all four runs Optimal and Verified. Figures in human-loop.md.

## Unresolved issues

Unchanged from the receipt of the review-1 and review-2 repairs.
