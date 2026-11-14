# Benchmark Results: Feature 001 (task T049)

Run bundle (not tracked): `results/001-core-gurobi-lp/runs/20261114T211851Z-bench/`
(`records.jsonl`, `summary.md`). Command: `cargo bench --features gurobi --bench load_solve`.

**Scope of every figure**: instance iDopaNeuroC_problem_1 (m = 1344, n = 2942, nnz = 13,748),
objective from seed 1, Gurobi 13.0.3, options DualSimplex, 1 thread, seed 0 (effective
parameters OutputFlag=0; Threads=1; Method=1; Seed=0), bench (release) profile, Intel Core
i9-10980XE, optSolver commit b4628b3 plus the uncommitted feature-001 work. Timings: median
(min–max) over N = 5 solves, each with a fresh environment, in one process per index type;
memory: N = 5 dedicated processes per index type.

## Positive control (constitution XIV-D)

Every memory run reported the expected load path before any comparison: `usize` row indices →
blocked `GRBXaddvars` load (1 block, conversion-buffer high-water 13,748 entries); `i32` row
indices → one `GRBXloadmodel` call (no conversion buffer).

## SC-004: extra peak memory during load

| Row indices | optSolver Rust-heap peak during load | Limit 32 B × (m + n) + 1 MiB | Result |
|---|---|---|---|
| `usize` | 78,536 B (all 5 runs identical) | 1,185,728 B | **Pass** |
| `i32` | 16,384 B (all 5 runs identical) | 1,185,728 B | **Pass** |

Cross-check (research R8), from the timing process: peak resident memory rose by about 5.5–6.0
MiB over five full solves, and Gurobi reported `MaxMemUsed` of about 3.1–3.4 MB. A first run
failed SC-004 for `usize` (3,145,736 B) because the conversion buffers were allocated at full
capacity; they are now reserved only as far as the largest block needs.

## SC-005: optSolver's added time relative to the solve

Re-measured on 2026-11-14 after slice 2 (tasks T055–T057), run bundle
`results/001-core-gurobi-lp/runs/20261114T220238Z-bench/`, same scope as above. The loader now
times Gurobi's own load calls (`GRBXloadmodel`, `GRBXaddvars`, `GRBupdatemodel`) separately, and
the benchmark times validation (`CscRef::new` + `Problem::new`). SC-005 is judged on
optSolver's parts only: (validation + optSolver's own load work + extraction + verification) ÷
solve.

| Row indices | Ratio, median (min–max), N = 5 | Limit | Result |
|---|---|---|---|
| `usize` | 0.019 (0.015–0.031) | 0.05 | **Pass** |
| `i32` | 0.019 (0.013–0.026) | 0.05 | **Pass** |

| Component (median, min–max) | `usize` | `i32` |
|---|---|---|
| Validation | 0.081 ms (0.077–0.103) | 0.097 ms (0.094–0.116) |
| Load, optSolver's own work | 0.031 ms (0.030–0.042) | 0.019 ms (0.018–0.023) |
| Load, inside Gurobi calls | 0.97 ms (0.95–1.02) | 1.55 ms (1.52–1.76) |
| Extraction | 0.13 ms (0.12–0.34) | 0.16 ms (0.11–0.35) |
| Verification | 0.16 ms (0.14–0.39) | 0.21 ms (0.15–0.48) |
| Solve | 22.0 ms (19.5–31.5) | 26.5 ms (19.7–37.2) |

Unexplained, recorded: Gurobi's share measured in load-only calls (0.97 and 1.55 ms) is larger
than the whole "load" phase timed inside full solves (0.79 and 1.17 ms median). The two come
from different calls (load-only on one long-lived environment versus loads inside solves, each
with a fresh environment); SC-005 does not depend on that phase. The first measurement
(`20261114T211851Z-bench`) counted Gurobi's model build as optSolver's time and gave 0.057 and
0.061; it is superseded.

## Other phases (median, min–max)

| Phase | `usize` | `i32` |
|---|---|---|
| Environment creation | 1.67 ms (1.48–3.59) | 1.60 ms (1.51–3.97) |
| Load (incl. Gurobi model build) | 0.95 ms (0.80–2.04) | 0.73 ms (0.72–2.04) |
| Solve | 21.7 ms (19.6–42.5) | 23.4 ms (20.9–30.1) |
| Extraction | 0.13 ms (0.12–0.25) | 0.14 ms (0.12–0.37) |
| Verification | 0.15 ms (0.13–0.29) | 0.17 ms (0.16–0.47) |

Cold first solve: 42.5 ms (`usize` process) and 23.4 ms (`i32` process); with N = 5 the cold
and warm figures overlap and no cold-start effect is claimed.
