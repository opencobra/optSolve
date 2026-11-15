# Test corpus

This document describes every test fixture: what problem it holds, where it came from, how it
was extracted, which objective a test uses with it, and what a solver is expected to report. It
is descriptive (Tier 3). The corpus itself, the fixture directory and the source repository are
declared once, in `.specify/memory/project.yml` under `test_corpus`; this document does not
restate those values and defers to that file where they differ. Every fixture has a provenance
record next to it (`<fixture>.provenance.json`), which is the authority for its hashes, source
commit and expected outcome.

## Notation

Householder notation, as in the constitution's notation convention. A is the m × n constraint
matrix, b the row right-hand side (each row with a sense: =, ≤ or ≥), c the objective vector,
l and u the variable bounds, x the primal variables. Conic problems add F (k × n), g (length k)
and the cone K, a product of cones, with the constraint F x + g ∈ K. nnz is the number of stored
nonzeros.

## Summary

| Fixture | Class | m | n | nnz(A) | k | Tier | Expected status |
|---|---|---|---|---|---|---|---|
| `iDopaNeuroC_problem_LP` | LP | 1344 | 2942 | 13748 | — | licensed | Optimal (each seed) |
| `iDopaNeuroC_problem_LP_minNorm` | QP (derived) | 1344 | 2942 | 13748 | — | licensed | Optimal |
| `iCoreED_problem_1` | CLP | 78 | 2084 | 1128 | 2992 | licensed | Optimal |
| `iCoreED_problem_2` | CLP | 63 | 1287 | 497 | 1806 | licensed | Optimal |
| `iCoreED_problem_3` | CLP | 75 | 1299 | 715 | 1818 | licensed | Optimal |
| `iCoreED_problem_4` | CLP | 78 | 2084 | 1128 | 2992 | licensed | Optimal |
| `iCoreED_problem_5` | CLP | 78 | 2084 | 1128 | 2992 | licensed | Optimal |
| `iDopaNeuroC_problem_1` | CLP | 1242 | 38742 | 17188 | 56138 | heavy | Optimal |
| `iDopaNeuroC_problem_2` | CLP | 1138 | 25819 | 11576 | 36807 | heavy | Optimal |
| `iDopaNeuroC_problem_2_fullRankFR` | CLP | 1138 | 25813 | 11660 | 36799 | heavy | Optimal |
| `iDopaNeuroC_problem_3` | CLP | 1242 | 25923 | 17161 | 36911 | heavy | Optimal |
| `iDopaNeuroC_problem_3_fullRankFR` | CLP | 1241 | 25916 | 16703 | 36902 | heavy | Optimal |
| `iDopaNeuroC_problem_5` | CLP | 1263 | 38763 | 23721 | 56159 | heavy | Optimal reported; verification Failed (known finding) |
| `infeasible` | LP (small) | 1 | 2 | 2 | — | licensed | Infeasible |
| `unbounded` | LP (small) | 1 | 2 | 2 | — | licensed | Unbounded |
| `ranged` | LP (small) | 2 | 2 | 4 | — | licensed | Optimal, objective 1 |
| `empty_column` | LP (small) | 1 | 2 | 1 | — | licensed | Optimal, objective −3 |
| `toy_lp_max` | LP (toy) | 2 | 2 | 4 | — | licensed | Optimal, objective 5 |
| `toy_lp_mixed` | LP (toy) | 3 | 3 | 7 | — | licensed | Optimal, objective 5 |
| `toy_qp_diag` | QP (toy) | 1 | 2 | 2 | — | licensed | Optimal, objective 1 |
| `toy_qp_offdiag` | QP (toy) | 1 | 2 | 2 | — | licensed | Optimal, objective 3 |
| `toy_qp_max` | QP (toy) | 1 | 2 | 2 | — | licensed | Optimal, objective -1.75 |
| `toy_clp_quad` | CLP (toy) | 2 | 3 | 2 | 3 | licensed | Optimal, objective 5 |
| `toy_clp_rquad` | CLP (toy) | 2 | 3 | 2 | 3 | licensed | Optimal, objective 2 |
| `toy_clp_pexp` | CLP (toy) | 2 | 3 | 2 | 3 | licensed | Optimal, objective 2.71828 |
| `toy_clp_dexp` | CLP (toy) | 2 | 3 | 2 | 3 | licensed | Optimal, objective 0.367879 |
| `toy_clp_ppow` | CLP (toy) | 2 | 3 | 2 | 3 | licensed | Optimal, objective 1 |
| `toy_clp_dpow` | CLP (toy) | 2 | 3 | 2 | 3 | licensed | Optimal, objective 4 |
| `toy_clp_rplus` | CLP (toy) | 1 | 2 | 1 | 2 | licensed | Optimal, objective 2 |
| `toy_clp_rzero` | CLP (toy) | 0 | 2 | 0 | 2 | licensed | Optimal, objective 3 |
| `toy_clp_psd2` | CLP (toy) | 1 | 3 | 1 | 3 | licensed | Optimal, objective 2 |
| `toy_clp_psd3` | CLP (toy) | 3 | 6 | 3 | 6 | licensed | Optimal, objective 2 |
| `toy_clp_infeasible` | CLP (toy) | 2 | 3 | 2 | 3 | licensed | Infeasible |

"Licensed" tests run wherever the solver and its licence are present and are skipped visibly
otherwise. "Heavy" tests also need `OPTSOLVER_HEAVY_TESTS=1`. Note that the CLP fixture
`iDopaNeuroC_problem_1` and the LP fixture `iDopaNeuroC_problem_LP` are different problems: the
first is the conic problem variational kinetics builds from source instance 1, the second the
flux-balance LP held in that same source instance.

## LP: `iDopaNeuroC_problem_LP`

- **Problem.** The flux-balance linear problem of the iDopaNeuroC dopaminergic neuron model:
  A x = b with the row senses as stored (A is the stoichiometric matrix S), l ≤ x ≤ u.
  m = 1344, n = 2942, nnz(A) = 13748. The stored objective sense is maximise.
- **Source.** Variational kinetics source instance iDopaNeuroC_problem_1 (its `model`
  structure), from the frozen problem corpus of the variational kinetics repository. The fixture
  was renamed from that instance name; the provenance record keeps `source_instance` and
  `renamed_from`. Citation: Brunk et al., Nature Biotechnology, 2018, doi:10.1038/nbt.4072.
- **Extraction.** `tools/corpus/extract_idopaneuroc_fixtures.m` builds a Gurobi model field by
  field from the COBRA structure, writes MPS with Gurobi's writer, reads it back and requires
  A, b, row senses, l, u and c to be bit-for-bit identical. The provenance record holds SHA-256
  hashes of each array in compressed sparse column layout, and a test parses the MPS file and
  compares them, so the fixture is shown to hold the source arrays exactly.
- **Objective rule.** The stored c is zero. Tests replace it by seeded random objectives, seeds
  1, 2 and 3 (recorded in the provenance record), generated by SplitMix64 in
  `tests/common/objectives.rs`: each entry is uniform on [−1, 1), computed with integer
  arithmetic so the vector is identical on every platform.
- **Expected status.** Optimal for each seed, with the solution verified independently of the
  solver.

### Why variational kinetics problems 2 to 5 are not LP test problems

Feature 001 investigated every iDopaNeuroC instance as a flux-balance LP
(`specs/001-core-gurobi-lp/research.md`, R18). Read that way, problems 2 and 3 and their
full-rank variants are infeasible (Gurobi's conflict set is a single metabolite row that forces
a flux below its lower bound of 0), while problems 1, 4 and 5 are feasible. The cause is the
variational kinetics driver: it runs its single flux-balance LP before experiments 2 and 3 set
b from a generated kinetic steady state, and the stored instances are the inputs to
`optimizeVKmodel`, which does not impose the stored l and u as net-flux bounds. The stored
arrays of problems 2 to 5 are therefore not flux-balance LPs anyone solves; they are inputs to a
conic problem. Problems 4 and 5 are variational kinetics problems too. The maintainer decided
that the LP corpus is problem 1 only. The other instances enter the corpus as CLP fixtures,
below.

## QP: `iDopaNeuroC_problem_LP_minNorm`

- **Problem.** Minimise ½ · 10⁻⁴ · ‖x‖² (Q = 10⁻⁴ I, c = 0) over the feasible set of
  `iDopaNeuroC_problem_LP`: the same A, b, row senses, l and u, every one of the n variables
  penalised. This is the minimum-norm step the variational kinetics driver takes through COBRA
  `optimizeCbModel` with `minNorm` (`specs/002-mosek-lp-qp-clp/research.md`, R12).
- **Source and extraction.** Derived, not extracted: tests rebuild it from the LP fixture. Its
  provenance record is `tests/fixtures/qp/iDopaNeuroC_problem_LP_minNorm.provenance.json`.
- **Objective rule.** Fixed by the definition above; no seed.
- **Expected status.** Optimal. Reference objectives: 2.12873 × 10⁻⁶ (MOSEK, with variational
  kinetics parameters) and 2.12847 × 10⁻⁶ (Gurobi, with COBRA defaults).

## CLP: the variational kinetics conic problems

### Definition

The CLP test problem of an instance is **the problem `optimizeVKmodel` passes to
`solveCobraCLP`** (`specs/002-mosek-lp-qp-clp/research.md`, R11):

  minimise c'x subject to A x = b, l ≤ x ≤ u, F x + g ∈ K.

It is captured by loading the instance's stored `model` and `param`, setting
`solveStrategy = 'single'`, `printLevel = 0` and `saveProb = 1`, running
`optimizeVKmodel(model, param)` and taking `sol.prob`. On the stored route (sequential) the
solver rescales rows and rebuilds c before its first MOSEK call, so that inner problem cannot be
reproduced without editing variational kinetics code; the single route passes the assembled
problem to MOSEK once. The extraction checks that `sol.prob` is bit-for-bit the problem MOSEK
received (the copy `saveProb` writes), that the cone dimensions sum to k, and that every cone is
of an expected type. The problem is identical across runs, so no seed is needed.

In every instance all rows of A are equalities, and K is a product of primal exponential cones
(dimension 3) and eight rotated quadratic cones. The expected status and objective are those of
the variational kinetics single route (`mosekopt` once, then `parseMskResult`), measured with
MOSEK 11.2.0. Where c = 0 the problem is a feasibility problem, the optimal objective is 0 and
the duality-gap check is trivially met.

- **Extraction script.** `tools/corpus/extract_vk_clp_fixtures.m` (file format below).
- **Source.** The frozen variational kinetics problem corpus, one source instance per fixture,
  with the same name.
- **Objective rule.** As captured (c from `sol.prob`); no seed.

### Licensed tier: iCoreED

| Fixture | nnz(F) | Exponential cones | Rotated quadratic cones | Expected objective |
|---|---|---|---|---|
| `iCoreED_problem_1` | 1506 | 762 | 8 | 0 (c = 0) |
| `iCoreED_problem_2` | 789 | 446 | 8 | 0 (c = 0) |
| `iCoreED_problem_3` | 1008 | 446 | 8 | 0 (c = 0) |
| `iCoreED_problem_4` | 1907 | 762 | 8 | 4.20848 × 10⁻⁷ |
| `iCoreED_problem_5` | 1634 | 762 | 8 | 38.2671 |

### Heavy tier: iDopaNeuroC

| Fixture | nnz(F) | Exponential cones | Rotated quadratic cones | Expected objective |
|---|---|---|---|---|
| `iDopaNeuroC_problem_1` | 31416 | 14456 | 8 | 0 (c = 0) |
| `iDopaNeuroC_problem_2` | 17255 | 9288 | 8 | 0 (c = 0) |
| `iDopaNeuroC_problem_2_fullRankFR` | 17253 | 9286 | 8 | 0 (c = 0) |
| `iDopaNeuroC_problem_3` | 20975 | 9288 | 8 | 0 (c = 0) |
| `iDopaNeuroC_problem_3_fullRankFR` | 20970 | 9286 | 8 | 0 (c = 0) |
| `iDopaNeuroC_problem_5` | 31715 | 14456 | 8 | 381.405 |

**Known finding, `iDopaNeuroC_problem_5`** (maintainer decision at feature 002 Gate 3,
2026-11-15; declared in `project.yml` as `clp_expected_verification_failures`): MOSEK reports
Optimal, but the independent verifier finds a row-bound violation of about 2e-7 relative,
twenty times MOSEK's own feasibility tolerance (1e-8, ×10 in the verdict). The violation is
unchanged with tighter MOSEK tolerances, and MOSEK run with the variational kinetics
parameters reproduces that run's objective (381.40464759) exactly with the same violation, so
the reference solution itself carries it (feature 002 research R19). The heavy-tier test
asserts that the verifier reports exactly this failure.

The `_fullRankFR` variants are repaired versions of source instances 2 and 3 in the variational
kinetics corpus: the model is edited (zero rows removed, identical rows lumped) until the matrix
[F_S, R_S] of forward and reverse stoichiometric coefficients has full row rank (here F_S and
R_S denote the positive parts of −S and S; they are not the conic F).

### Why iDopaNeuroC_problem_4 is excluded

On iDopaNeuroC_problem_4 MOSEK 11.2.0 stops with status `UNKNOWN` and response
`MSK_RES_TRM_STALL`: it stalls before reaching its tolerances, deterministically (the same
result in two runs). With no definite expected outcome the instance cannot serve as a test of
correct behaviour, so it is excluded by design.

### Files and format

Each CLP fixture is `tests/fixtures/clp/<fixture>.jtask` with its provenance record
`<fixture>.provenance.json`. JTASK is MOSEK's JSON task format, written by MOSEK's own writer from
MATLAB. It keeps the problem as MOSEK received it: variable bounds with bound keys, the linear
rows of A with their bounds, c, F and g as sparse triplets, one domain per cone and the affine
conic constraints mapping rows of F x + g to those domains. The extraction reads every file back
with MOSEK and requires A, b, c, l, u, F, g, the cones and the objective sense to be bit-for-bit
identical to what was written. The text-based Conic Benchmark Format was tried first and
rejected: it has no variable bounds and no separate linear block, so MOSEK writes bounds as extra
conic rows and A inside the affine rows, and the arrays read back are not the arrays written
(maintainer decision 2026-11-15; the provenance records say so). The extraction also re-solves
each problem once with the saved MOSEK command and parameters; the provenance record keeps that
time, the parameters and the expected status and objective.

| Fixture | JTASK file size (bytes) |
|---|---|
| `iCoreED_problem_1` | 131528 |
| `iCoreED_problem_2` | 78994 |
| `iCoreED_problem_3` | 87194 |
| `iCoreED_problem_4` | 137269 |
| `iCoreED_problem_5` | 147184 |
| `iDopaNeuroC_problem_1` | 2412460 |
| `iDopaNeuroC_problem_2` | 1595508 |
| `iDopaNeuroC_problem_2_fullRankFR` | 1590225 |
| `iDopaNeuroC_problem_3` | 1814712 |
| `iDopaNeuroC_problem_3_fullRankFR` | 1802557 |
| `iDopaNeuroC_problem_5` | 2882423 |

## Small hand-built fixtures

Hand-built for feature 001 (task T031), in `tests/fixtures/small`, part of optSolver and under
its licence. Each has a provenance record with its purpose and expected outcome.

| Fixture | Problem | Expected |
|---|---|---|
| `infeasible` | x1 + x2 ≥ 3 with x1, x2 ≤ 1; minimise | Infeasible |
| `unbounded` | maximise x1 + x2 with x1 = x2 and no upper bounds | Unbounded |
| `ranged` | 1 ≤ x1 + x2 ≤ 3 (MPS RANGES on an L row) and x1 − x2 ≥ −1; minimise x1 + 2 x2 | Optimal, x = (1, 0), objective 1 |
| `empty_column` | minimise x1 − x2 with x1 = 2, 0 ≤ x2 ≤ 5; column x2 has no constraint entries | Optimal, objective −3 |

## Toy instances (feature 003)

Small self-contained problems for every problem class and cone kind, declared in
`project.yml` (`test_corpus.toy_instances`, which also lists the four small fixtures above), in
`tests/fixtures/small`: LP and QP as MPS (written by Gurobi; the two maximise files use the
two-line `OBJSENSE` / `MAX` form, which MOSEK also reads), conic programmes as MOSEK JTASK (one
per cone kind, including the semidefinite cone in svec layout). Each was defined, written, read
back and solved by `tools/corpus/make_toy_instances.m`, which stops unless every independent
solver check equals the closed-form optimum recorded, with its derivation, in the provenance
record. Conic instances are also solved in maximise form by the tests (c negated, sense flipped,
objective negated). Their tests need neither the variational kinetics repository nor the large
fixtures, and the cross-solver comparison uses them (Gurobi supports the LP, QP, quadratic,
rotated quadratic, nonnegative and zero instances).

| Instance | Format | Structure | Expected | Independent checks |
|---|---|---|---|---|
| `toy_lp_max` | MPS | linear | Optimal, objective 5 | MOSEK and Gurobi |
| `toy_lp_mixed` | MPS | linear | Optimal, objective 5 | MOSEK and Gurobi |
| `toy_qp_diag` | MPS | quadratic objective | Optimal, objective 1 | MOSEK and Gurobi |
| `toy_qp_offdiag` | MPS | quadratic objective | Optimal, objective 3 | MOSEK and Gurobi |
| `toy_qp_max` | MPS | quadratic objective | Optimal, objective -1.75 | MOSEK and Gurobi |
| `toy_clp_quad` | JTASK | quadratic (3) | Optimal, objective 5 | MOSEK and Gurobi |
| `toy_clp_rquad` | JTASK | rotated quadratic (3) | Optimal, objective 2 | MOSEK and Gurobi |
| `toy_clp_pexp` | JTASK | primal exponential (3) | Optimal, objective 2.71828 | MOSEK |
| `toy_clp_dexp` | JTASK | dual exponential (3) | Optimal, objective 0.367879 | MOSEK |
| `toy_clp_ppow` | JTASK | primal power (3) | Optimal, objective 1 | MOSEK |
| `toy_clp_dpow` | JTASK | dual power (3) | Optimal, objective 4 | MOSEK |
| `toy_clp_rplus` | JTASK | nonnegative (2) | Optimal, objective 2 | MOSEK and Gurobi |
| `toy_clp_rzero` | JTASK | zero (2) | Optimal, objective 3 | MOSEK and Gurobi |
| `toy_clp_psd2` | JTASK | positive semidefinite (3) | Optimal, objective 2 | MOSEK |
| `toy_clp_psd3` | JTASK | positive semidefinite (6) | Optimal, objective 2 | MOSEK |
| `toy_clp_infeasible` | JTASK | quadratic (3) | Infeasible | MOSEK and Gurobi |

## Licence

The licence of the variational kinetics models has not been verified (maintainer decision
2026-11-14, extended to the CLP fixtures at Gate 2A on 2026-11-15); each provenance record says
so.
