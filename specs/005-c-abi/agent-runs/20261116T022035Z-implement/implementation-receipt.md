# Implementation Receipt — feature 005, tasks T001–T022

## Prompt

The task given to the feature agent (relayed by the COBRA Toolbox session on the maintainer's
instruction "optSolver can also use independent subagents at each gate"): take feature 005-c-abi
through all its gates with fresh-context compliance-review subagents, through plan, tasks, analyze,
implementation and validation; Gate 2 review 7 (`gate-reviews/gate-2-20261116T013252Z.md`)
authorised implementation of tasks T001–T022.

## Final response

Implementation of feature 005 (tasks T001–T022) is complete and validated on the reference
machine; the Gate 3 compliance review is next. The C interface is `src/capi/` with header
`src/capi/optsolver.h` (interface 1.0, 57 exported functions), built as `liboptsolver.a` and
`liboptsolver.so` with `--features capi,gurobi,mosek`. The Rust API gains `Options::memory_limit_gb`
(Gurobi `SoftMemLimit`; MOSEK unsupported), `Send` for both `Env` types with Gurobi environment
creation and freeing serialised, and the Gurobi `solve_into` conic-dual length check. All eight
feature combinations pass fmt, clippy with warnings denied and the test suite; the 1.85.0 build
passes. C-versus-Rust agreement: 136 LP/QP runs per backend, all bit-identical; conic, certificate,
reuse, thread, memory-limit, capability, invalid-input, panic and leak trials pass. Heap: the C
path adds 0 B (owned) on iDopaNeuroC_problem_LP for both backends (N = 3); wall time: within
max(1 %, 1 ms) in 6 of 6 alternating runs (N = 5 each), with the caveats below. Gate 3 review 1
failed with ten findings; all are repaired (commit c9cf95b) and the evidence re-run at f9a30b2.

## Diff summary

- Cargo.toml: crate types lib, staticlib, cdylib; feature `capi`; test `c_interface`.
- src/lib.rs: `#[doc(hidden)] pub mod capi` behind `capi`.
- src/capi/ (new): mod.rs (constants, codes, per-thread errors, guard/contain, silent wrapping
  panic hook, names, test support), problem.rs (C problem/options structures and translation,
  borrowing in place, dimensions first), env.rs (handles, capabilities), result.rs (solve entry
  points, result accessors, safe in-process helper for the benchmark), optsolver.h (header).
- src/options.rs: `memory_limit_gb`. src/gurobi/params.rs: `SoftMemLimit` mapping and its unit
  test through the lifecycle lock. src/gurobi/env.rs: `Lifecycle` token, `lifecycle`, `free`,
  `unsafe impl Send`, docs and history. src/mosek/env.rs: `unsafe impl Send`, docs and history.
  src/gurobi/solve.rs: conic-dual buffer length check.
- tests/c_interface.rs, tests/c/ (driver.c, errors.c, layout.c, printer.c, minimal.c,
  lsan-vendor.supp), tests/common/ccompiler.rs (extracted; header-conformance tests migrated),
  tests/common/cdata.rs, tests/common/compare.rs (agreement rule and its self-tests),
  tests/contract/f005.rs, tests/standards.rs (FFI modules, guard check, lifecycle check, header
  drift check, each with a negative test).
- benches/load_solve.rs: C-path suite; run directory resolved through project.yml and
  .specify/feature.json.
- docs/llm-interface.md: C interface section, memory limit, Threading. docs/architecture:
  components.yaml, boundaries.yaml reconciled.
- specs/005-c-abi: tasks ticked; plan file list and stale text corrected; architecture-delta.yaml
  made valid YAML (two values quoted; content unchanged).

## Tests

Revised after Gate 3 review 1 (FAIL; repairs in commit c9cf95b). In-gate run at commit f9a30b2
with a clean tree, `results/005-c-abi/runs/20261116T023921Z-gate/summary.txt` (generated output,
untracked): fmt ok; clippy `-D warnings` ok and `cargo test` ok for `""`, `gurobi`, `mosek`,
`gurobi,mosek`, `capi`, `capi,gurobi`, `capi,mosek`, `capi,gurobi,mosek`; the C-interface suite
re-run 5 times in parallel with fresh C builds, all ok (the relink race Gate 3 found is fixed);
`cargo +1.85.0 build --features capi,gurobi,mosek` ok (command and toolchain in msrv.log).
C-versus-Rust agreement (SC-001), per backend: 136 LP/QP runs (owned and buffer paths, both row
forms, both index widths, static and shared), 4 certificate runs, and 10 (Gurobi) / 22 (MOSEK)
conic runs — all bit-identical.
Heap (SC-002), three bundles `capi-bench-1..3.log` in the gate directory, iDopaNeuroC_problem_LP
(m = 1344, n = 2942, nnz = 13748), N = 3 per path, deterministic: the C path reading every
accessor has the same peak as the Rust path (137 822 B Gurobi, 136 587 B MOSEK; difference 0 B);
the buffer path's peak is lower; the C path makes about 40 more allocations (the lazily converted
names), which is also the positive control that it ran.
Wall time (SC-003), Rust and C alternating in one process, N = 5 warm calls each, three bundles:
median differences +0.29, −0.27, −0.14 ms (Gurobi) and −0.79, +0.22, −0.28 ms (MOSEK), all within
max(1 %, 1 ms). Single calls spread up to 40 ms (outliers on a shared machine), so these medians
show no C-layer cost above about 1 ms; they do not resolve differences far below it.
Regression check against feature 003's release-profile bundle (main checkout,
results/003-llm-interface-toy-instances/runs/20261115T160715Z-bench) with the default suite at
f9a30b2 (results/005-c-abi/runs/20261116T025309Z-bench): iDopaNeuroC_problem_LP heap peak during
load 78 536 B both runs; overhead ratio 0.019 / 0.015 (Gurobi / MOSEK) against 0.018 / 0.018;
solve medians 22.6 / 30.8 ms against 29.4 / 29.2 ms (N = 5 each) — no regression.
Link demonstration (SC-007): `results/005-c-abi/runs/20261116T020949Z-link/link.log`.
Build cost of the extra crate types (incremental debug, N = 2): 1.8–3.6 s against 1.1–1.4 s for
the rlib only (`results/005-c-abi/runs/20261116T021452Z-gate/build-time.txt`).
Withdrawn: an earlier statement in this receipt that separate-process wall-time runs "passed 4 of
6" had no retained run bundles (deleted when a benchmark defect was fixed); it is withdrawn
(recorded in human-loop.md).

## Unresolved issues

- The `runmat mex` build of the COBRA gateway is not demonstrated here (the RunMat binary does not
  start on this machine: missing libhdf5.so.103); routed to the COBRA feature (its T032–T034,
  SC-008). Consumer adoption (XIII-G) is not complete until then.
- Gurobi environment creation and freeing on any thread under a lock rests on an inference from
  the vendor article (accepted risk, research R3); the concurrency trial is evidence for its run
  only. MOSEK concurrent creation and freeing is unverified.
- Spec User Story 3 still says "toy-sized" where the memory-limit criterion uses
  iDopaNeuroC_problem_LP (wording; a spec change needs a Gate 1 re-review).
- FR-021's list of Rust-API changes omits the Gurobi conic-dual length check (a repaired defect:
  a wrong-length `s` used to panic).
- The T013 trial checks the recorded effective-parameter list for `SoftMemLimit` rather than
  reading the value back from Gurobi; the behavioural memory-limit trials are the stronger evidence.
- Pre-existing: Gurobi unit tests in src/gurobi/params.rs panic without a licence instead of being
  skipped visibly (constitution II gap, not worsened).
- The Notation Convention lacks symbols for the power-cone exponents (`alpha`) and the unbounded
  ray (`r`) the Rust API uses; a human-reserved amendment.
- Constitution 3.0.0 and this feature are committed on the feature branch only; committing 3.0.0
  to `devel` and integrating the feature are the maintainer's decisions.

## Other information

Leak checks used AddressSanitizer's leak checker with a recorded suppression list for vendor
frames (tests/c/lsan-vendor.supp); both positive controls reported their deliberate leaks.
