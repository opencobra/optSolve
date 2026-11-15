# Benchmark Results: Derived Corpus QP On Both Backends (task T021)

**Scope**: instances iDopaNeuroC_problem_LP (LP, seed 1) and iDopaNeuroC_problem_LP_minNorm (QP:
the same feasible set, c = 0, Q = 10⁻⁴·I, 2942 terms; feature 002 research R12); m = 1344,
n = 2942, nnz(A) = 13,748; N = 5 per figure; every memory figure from a dedicated process.
Options: one thread; LP dual simplex (Gurobi seed 0); QP each solver's default (barrier). Gurobi
13.0.3, MOSEK 11.2.0, Intel Core i9-10980XE, release (bench) profile. Run bundle:
`results/003-llm-interface-toy-instances/runs/20261115T160715Z-bench/`. Commit 89776f4 plus
uncommitted feature 003 work (recorded in every record).

## Heap peak during load (SC-004 rule plus the documented Q triplets)

Limit: 32 B·(m + n) + 1 MiB + 12 B·(Q terms): 1,185,728 B (LP), 1,221,032 B (QP).

| Solver | Instance | Row indices | Load path (positive control) | Q terms | Heap peak, median (min–max) | Verdict |
|---|---|---|---|---|---|---|
| Gurobi | LP | usize | blocked, 1 block | 0 | 78,536 B (78,536–78,536) | PASS |
| Gurobi | LP | i32 | single call | 0 | 16,384 B (16,384–16,384) | PASS |
| Gurobi | QP | usize | blocked, 1 block | 2942 | 78,536 B (78,536–78,536) | PASS |
| Gurobi | QP | i32 | single call | 2942 | 47,072 B (47,072–47,072) | PASS |
| MOSEK | LP | usize | blocked, 1 block | 0 | 78,536 B (78,536–78,536) | PASS |
| MOSEK | LP | i32 | single call | 0 | 11,768 B (11,768–11,768) | PASS |
| MOSEK | QP | usize | blocked, 1 block | 2942 | 78,536 B (78,536–78,536) | PASS |
| MOSEK | QP | i32 | single call | 2942 | 23,536 B (23,536–23,536) | PASS |

The QP triplets are the only addition: MOSEK's 23,536 B is exactly 8 B per term (two i32 index
arrays); Gurobi's i32 QP peak (47,072 B) is within its 12 B per term plus the LP path's 16,384 B
(the arrays are not all live at once). On the usize path the converted A block dominates.

## optSolver's own time / solve time (feature 001 SC-005 rule, limit 0.05)

| Solver | Instance | usize median (min–max) | i32 median (min–max) |
|---|---|---|---|
| Gurobi | LP | 0.018 (0.014–0.023) | 0.018 (0.014–0.021) |
| Gurobi | QP | 0.009 (0.007–0.010) | 0.009 (0.006–0.010) |
| MOSEK | LP | 0.018 (0.017–0.019) | 0.014 (0.011–0.015) |
| MOSEK | QP | 0.007 (0.006–0.007) | 0.010 (0.006–0.010) |

All PASS. Phase timings, VmHWM rises and effective parameters per scenario are in the bundle's
`summary.md`.

## Not measured

Gurobi conic loading (the toy instances are too small to measure; no corpus conic instance has
only Gurobi-supported cones).
