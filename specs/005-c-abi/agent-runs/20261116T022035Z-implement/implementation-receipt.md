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
max(1 %, 1 ms) in 6 of 6 alternating runs (N = 5 each), with the caveats below.

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

In-gate run `results/005-c-abi/runs/20261116T021452Z-gate/summary.txt` (generated output, not
tracked): fmt ok; clippy `-D warnings` ok and `cargo test` ok for `""`, `gurobi`, `mosek`,
`gurobi,mosek`, `capi`, `capi,gurobi`, `capi,mosek`, `capi,gurobi,mosek`; 1.85.0 build ok. With
`capi,gurobi,mosek`: unit 56, c_interface 35, contract 82, corpus 199 (18 heavy-tier ignored, as
before), header conformance 2 + 2, standards 11, doc tests 6 — all pass. With `capi` only:
c_interface 15 pass, 20 licensed ignored with the reason.
Benchmarks (`…-capi-bench`, three bundles): heap (SC-002) difference 0 B owned and negative on the
buffer path, both backends, N = 3, deterministic; allocation counts differ by one (positive
control). Wall time (SC-003), Rust and C alternating in one process, N = 5 each: differences
+6.4e-5, −4.1e-4, +1.0e-3 s (Gurobi) and −5.4e-3, −5.8e-5, +6.7e-4 s (MOSEK); all within
max(1 %, 1 ms), one at 0.9993 ms; spreads up to 15 ms. Earlier runs with Rust and C in separate
processes, after fixing a benchmark defect (the C path minimised a maximisation instance), passed
4 of 6 — the difference lay in the solvers' own solve phase, not in the C layer's work; the
alternating method replaced them.
Link demonstration (SC-007): `results/005-c-abi/runs/20261116T020949Z-link/link.log` — static and
shared builds with the documented flags; both backends Optimal, objective 5, Verified.
Build cost of the extra crate types (incremental debug, N = 2): 1.8–3.6 s versus 1.1–1.4 s rlib only.

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
- Pre-existing: Gurobi unit tests in src/gurobi/params.rs panic without a licence instead of being
  skipped visibly (constitution II gap, not worsened).
- The Notation Convention lacks symbols for the power-cone exponents (`alpha`) and the unbounded
  ray (`r`) the Rust API uses; a human-reserved amendment.
- Constitution 3.0.0 and this feature are committed on the feature branch only; committing 3.0.0
  to `devel` and integrating the feature are the maintainer's decisions.

## Other information

Leak checks used AddressSanitizer's leak checker with a recorded suppression list for vendor
frames (tests/c/lsan-vendor.supp); both positive controls reported their deliberate leaks.
