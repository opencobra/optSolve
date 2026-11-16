# Implementation Receipt — feature 005, repairs after Gate 3 reviews 1 and 2

## Prompt

Within the scope Gate 2 review 7 approved (tasks T001–T022), repair the findings of Gate 3
review 1 (`gate-reviews/gate-3-20261116T023304Z.md`, ten findings) and Gate 3 review 2
(`gate-reviews/gate-3-20261116T030058Z.md`, four findings), re-run the validation at a committed
revision, and request a new fresh-context Gate 3 review.

## Final response

Gate 3 reviews 1 and 2 are repaired. Code: C test programs are built once per process (the relink
race is gone); the zero-copy positive control covers every borrowed array; the benchmark helper
checks slice lengths and reads every result accessor; conic and certificate trials report
bit-identity; history prose is at the end of its files; the Gurobi library name comes from the
declared version, in the test harness and in the leak-checker suppression list; benchmark bundles
record dimensions, configuration and a commit or working-tree hash, and write their temporary index
inside the target directory. Records: the first receipt is restored verbatim and this receipt
covers the repairs; the withdrawn wall-time claim and the withdrawn attribution of the failed
separate-process runs are recorded, with the reason the method changed; the closeout
reconciliation lists the remaining duplication, the invalidated wording and the unowned contract
tests. The validation is re-run at the committed revision named in human-loop.md; Gate 3 review 3
is next.

## Diff summary

- Commit c9cf95b (review 1): tests/common/cdata.rs (cached builds, unique paths per flag set,
  Gurobi library name from `OPTSOLVER_GUROBI_DECLARED_VERSION`); src/capi/problem.rs (second
  pointer-identity test: row sense, b, Q, F, g); src/capi/result.rs (length checks and
  `read_every_accessor` in `solve_lp_through_c`); tests/c_interface.rs (bit-identity counts for
  t010 certificates and t015); src/gurobi/solve.rs and tests/common/ccompiler.rs (history at the
  end); benches/load_solve.rs (dimensions, effective configuration, working-tree hash);
  docs/llm-interface.md (version-free library names); src/capi/optsolver.h (not-usable
  capabilities documented); docs/architecture/repo_folder_organisation.md (C test inputs).
- This run (review 2): tests/c/lsan-vendor.supp (version-free patterns); benches/load_solve.rs
  (temporary index under `CARGO_TARGET_TMPDIR`); the first receipt restored from commit 11647b1;
  human-loop.md, checklists/requirements.md, this receipt.

## Tests

- After the suppression-list change: `cargo test --features capi,gurobi,mosek --test c_interface
  -- leak` with fresh C builds — unlicensed and both licensed leak trials pass, each positive
  control reporting its deliberate leak.
- clippy `-D warnings` with all features: clean.
- The full in-gate run (eight feature combinations, five parallel C-interface repeats, 1.85.0
  build, three C-path benchmark bundles, the default benchmark) is re-run at the committed revision
  and its directory named in human-loop.md.
- Evidence of the review-1 repairs: `results/005-c-abi/runs/20261116T023921Z-gate/summary.txt`
  (commit f9a30b2, clean tree).

## Unresolved issues

As in the first receipt, except the items repaired above: the consumer's `runmat mex` build
(routed to COBRA), the Gurobi lifecycle-lock inference (accepted risk), MOSEK concurrent creation
(unverified), the User Story 3 wording and FR-021's list (spec revision needs a Gate 1
re-review), the pre-existing Gurobi unit tests that panic without a licence, the Notation
inventory gap (`alpha`, `r`), and integration into `devel` (the maintainer's decision).
