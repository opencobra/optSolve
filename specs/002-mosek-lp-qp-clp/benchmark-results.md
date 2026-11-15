# Benchmark Results: MOSEK Backend (task T036)

**Scope of every figure**: instance iDopaNeuroC_problem_LP (m = 1344, n = 2942, nnz = 13,748),
seeded objective (seed 1), deterministic options (dual simplex, one thread; Gurobi also
seed 0), N = 5 per measurement, each memory figure from a process dedicated to one scenario
(constitution XIV-C). Machine: Intel Core i9-10980XE; release (bench) profile; MOSEK 11.2.0,
Gurobi 13.0.3. Run bundle:
`results/002-mosek-lp-qp-clp/runs/20261115T103501Z-bench/` (`records.jsonl`, `summary.md`).
Commit 2b3be46 plus uncommitted feature 002 work (recorded as such in every record).

## Spec SC-004 (MOSEK as for Gurobi): optSolver's heap peak during load

Limit 32 B × (m + n) + 1 MiB = 1,185,728 B.

| Solver | Row indices | Load path (positive control) | Heap peak, median (min–max), N = 5 | Verdict |
|---|---|---|---|---|
| MOSEK | usize | blocked, 1 block, buffer high-water 13,748 | 78,536 B (78,536–78,536) | PASS |
| MOSEK | i32 | single call, no buffer | 11,768 B (11,768–11,768) | PASS |
| Gurobi | usize | blocked, 1 block, buffer high-water 13,748 | 78,536 B (78,536–78,536) | PASS |
| Gurobi | i32 | single call, no buffer | 16,384 B (16,384–16,384) | PASS |

The MOSEK i32 figure is the bound-key array of the variables (4 B × n = 11,768 B; the row
keys, 4 B × m, are allocated after it is freed); no bound value is copied, since the corpus
LP's bounds are all finite (copies are made only when an infinite value is present;
research R3, R18). The usize figure is the converted row-index block (4 B × nnz = 54,992 B) plus its
rebased column starts, as for Gurobi.

## Feature 001 SC-005 rerun on both solvers: optSolver's own time / solve time

(validation + optSolver's own load work + extraction + verification) / solve; limit 0.05.

| Solver | Row indices | Median (min–max), N = 5 | Verdict |
|---|---|---|---|
| MOSEK | usize | 0.013 (0.011–0.015) | PASS |
| MOSEK | i32 | 0.016 (0.014–0.022) | PASS |
| Gurobi | usize | 0.019 (0.015–0.024) | PASS |
| Gurobi | i32 | 0.017 (0.014–0.027) | PASS |

MOSEK phases (usize; medians): validation 1.08e-4 s, optSolver's own load work 1.82e-4 s,
inside MOSEK's load calls 7.87e-4 s, environment creation (with licence check-out)
1.21e-2 s, solve 6.16e-2 s, extraction 9.7e-5 s, verification 3.6e-4 s. VmHWM rise over the
time run: 29,192 KiB (whole process, includes MOSEK).

## Not measured here

- QP and CLP loading are not benchmarked (the task covers the LP scenarios); their extra
  allocations are bounded by design: Q triplet indices 8 B per nonzero of Q, F column indices
  in a fixed buffer of at most 2^18 entries, one domain index per cone (8 B per cone).
- The i32 path's own load work (2.6e-4 s) exceeds the usize path's (1.8e-4 s) on MOSEK;
  not investigated (both far within SC-005).
