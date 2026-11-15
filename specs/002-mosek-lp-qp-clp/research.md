# Research: MOSEK Backend For LP, QP And CLP

Phase 0 of the plan. **H:n** is line n of the installed MOSEK 11.2.0 header
`mosek.h`; **M§x** is section x of the MOSEK 11.2 C API manual (local `capi.pdf`; online
https://docs.mosek.com/latest/capi/). **VK** paths are in the variational kinetics repository
(commit f164a3ac) under `src/matlab/`. Facts marked **probe** are confirmed by a licensed probe
task before code relies on them. Closes routed checklist items CHK008, CHK009, CHK012, CHK015,
CHK017, CHK021.

## R1. Bindings: own declarations (CHK021)

- **Decision**: optSolver declares the MOSEK functions it uses (about 45) and enums (about 15)
  in `src/mosek/ffi.rs`, checked against `mosek.h` by a header-conformance test as for Gurobi,
  linked at build time against `libmosek64` (feature `mosek`).
- **Rationale**: the official crate `mosek` 11.2.3 does pass borrowed slices to the bulk calls
  (`put_a_col_slice`, `put_afe_f_entry_list`, `put_q_obj`, solution getters into `&mut [f64]`),
  meeting constitution IV, but fails VII: `put_a_col_slice` does not check `ptrb.len()` against
  `last − first` (out-of-bounds read reachable from safe code); `put_afe_f_entry_list`
  silently truncates to the shortest slice; `try_into().unwrap()` and `CString::new().unwrap()`
  panic on user input; errors are `String`s without the raw code (`Env::new`/`Task::new` return
  `Option`); `Task` has no lifetime tie to `Env`; callbacks lack `catch_unwind`; its build script
  can download MOSEK at build time and declares no `links` key or `rerun-if-env-changed`;
  `Cargo.toml` says BSD-4-Clause while the LICENSE text is BSD-3-Clause.
- **Alternatives**: wrapping the crate (all gaps above become accepted risks).

## R2. Environment, task, licence, threads

- `MSK_makeenv` H:6270, `MSK_makeemptytask` H:6265, `MSK_deletetask` H:3024,
  `MSK_deleteenv` H:6117. The licence is checked out at the first optimisation (M§10.6), so
  `Env::new` calls `MSK_checkoutlicense(env, MSK_FEATURE_PTS)` (H:6086) to report a missing
  licence at creation, as the spec's edge case requires.
- Tasks may move between threads but not be shared (M§10.3): one task per solve, created and
  deleted inside the solve. The environment: concurrent use is implied but not stated (M§10.6);
  `Env` is neither `Send` nor `Sync`, as for Gurobi (constitution VII-B).
- Solver output: silent unless a stream is linked (M§7.4.1). With the solver log requested,
  a stream function forwards lines to the `log` facade (constitution III); `MSK_IPAR_LOG` is
  also set to 0 by default.

## R3. LP loading (zero copy except bound keys and infinite values)

- `MSK_putmaxnumvar/con/anz` presize; `MSK_appendvars`, `MSK_appendcons`;
  `MSK_putacolslice64(task, first, last, const int64 *ptrb, const int64 *ptre, const int32
  *asub, const double *aval)` H:4962 with ptrb = col_ptrs[..n], ptre = col_ptrs[1..]
  reinterpreted as `i64` (64-bit target, values ≤ nnz); `i32` row indices passed directly;
  `usize` row indices converted per column block through the existing fixed-size
  `IndexBuffer`, one `putacolslice64` per block.
- Objective `MSK_putcslice` H:5331, constant `MSK_putcfix` H:5238, sense `MSK_putobjsense`
  H:5467 (MINIMIZE 0, MAXIMIZE 1).
- Bounds: `MSK_putvarboundslice` H:5697 and `MSK_putconboundslice` H:5282 need bound-key arrays
  (`MSK_BK_LO` 0, `UP` 1, `FX` 2, `FR` 3, `RA` 4; H:52-56): a different encoding, built O(m + n)
  (allowed by IV). For sense rows, b is passed as both lower and upper bound. IEEE ±∞ or NaN in
  bound data is an error (`MSK_RES_ERR_NAN_IN_BLC`…, M§15.6); whether a value on a side the key
  marks infinite is ignored is undocumented (**probe**). Default: copy l, u (or b_l, b_u) with ±∞
  replaced by ±`MSK_INFINITY` (1e30, H:30) only when an infinite value is present (O(m + n)).
- SC-004 budget: bound keys 4 B per row and column plus at most 8 B per row and column for
  infinity copies, within 32 B per row and column.

## R4. QP loading

- `MSK_putqobj(task, int32 numqonz, const int32 *qosubi, const int32 *qosubj, const double
  *qoval)` H:5501: ½ xᵀQx convention, lower triangle only, triplets, replaces all terms
  (M§12.4). From the lower-triangle CSC: qosubj (column of each entry) must be built (4 B per
  nonzero of Q), qosubi converted when row indices are `usize`; qoval borrowed.
- **Complexity Tracking**: O(nnz(Q)) index arrays are unavoidable with MOSEK's triplet-only QP
  call (no slice form; one call replaces all terms, so no chunking). For the corpus QP
  (Q = 10⁻⁴ I, n = 2942) this is 2 × 4 × 2942 B.
- Non-convex Q: MOSEK checks convexity (`MSK_RES_ERR_OBJ_Q_NOT_PSD` 1295) and the backend
  returns that as an error (data-model CHK020).

## R5. Conic loading

- Rows of Fx + g: `MSK_appendafes` H:2745; F from our CSC through
  `MSK_putafefentrylist(task, numentr, const int64 *afeidx, const int32 *varidx, const double
  *val)` H:5030, called per column block through a fixed-size buffer holding afeidx (row
  indices widened to i64) and varidx (column of each entry); values borrowed. No per-nonzero
  allocation beyond the fixed buffer.
- g: `MSK_putafegslice` H:5070, borrowed.
- Domains: one per distinct (type, dimension, exponents): quadratic H:2847, rotated quadratic
  H:2871, primal exponential H:2819 (x₀ ≥ x₁ exp(x₂/x₁)), dual exponential H:2791
  (x₀ ≥ −x₂ e⁻¹ exp(x₁/x₂)), primal and dual power H:2830/2802 (exponents normalised by MOSEK,
  leading `nleft` coordinates), nonnegative H:2865, zero H:2877, svec PSD H:2904 (lower triangle
  column by column, off-diagonals × √2; self-dual). These match data-model.md's definitions.
- Cones: one `MSK_appendaccsseq(task, numaccs, const int64 *domidxs, numafeidx, afeidxfirst,
  NULL)` H:2736 with F's rows already ordered cone by cone.
- Conic duals: `MSK_getaccdotys` H:3228 (length `MSK_getaccntot`). MOSEK's dual stationarity
  is Aᵀy + s_l − s_u + Fᵀẏ = c with ẏ ∈ D* for minimisation and −ẏ ∈ D* for maximisation, and
  the dual objective contains −gᵀẏ (M§12.2.1, §12.2.3). With data-model.md's convention
  (z = c + Qx − Aᵀy − Fᵀs, σs ∈ K*): **s = ẏ, y = MOSEK y, z = s_l − s_u** for both senses.

## R6. Solving, solutions, statuses (CHK012)

- `MSK_optimizetrm` H:4831 (return value = error; trmcode = termination).
- Solution type: LP with basis identification (default) defines both interior (ITR) and basic
  (BAS) solutions: read BAS when `MSK_solutiondef(BAS)`, else ITR; QP and conic: ITR only
  (M§7.2.2). Which type carries an LP infeasibility certificate when both exist: **probe**.
- Getters: `MSK_getxx`, `MSK_gety`, `MSK_getslx`, `MSK_getsux`, `MSK_getskx`/`getskc` (basis),
  `MSK_getprimalobj`, `MSK_getdualobj`, `MSK_getaccdotys`.
- **Translation (total; raw codes kept: response/termination code, problem status, solution
  status, solution type)**:

| Condition (in order) | Status | Limit reason |
|---|---|---|
| Return value is an error | `Error::Solver` (code, `MSK_getlasterror` text) | — |
| solsta OPTIMAL | Optimal | — |
| solsta PRIM_INFEAS_CER | Infeasible | — |
| solsta DUAL_INFEAS_CER and prosta DUAL_INFEAS | Unbounded | — |
| solsta DUAL_INFEAS_CER otherwise | InfeasibleOrUnbounded | — |
| trm TRM_USER_CALLBACK | Interrupted | — |
| trm TRM_MAX_TIME, TRM_SERVER_MAX_TIME | StoppedAtLimit | Time |
| trm TRM_MAX_ITERATIONS | StoppedAtLimit | Iterations |
| trm TRM_MIO_NUM_BRANCHES | StoppedAtLimit | Nodes |
| trm TRM_MIO_NUM_RELAXS, TRM_NUM_MAX_NUM_INT_SOLUTIONS | StoppedAtLimit | Solutions |
| trm TRM_OBJECTIVE_RANGE | StoppedAtLimit | ObjectiveCutoff |
| trm TRM_SERVER_MAX_MEMORY | StoppedAtLimit | Memory |
| trm TRM_STALL, TRM_NUMERICAL_PROBLEM, TRM_MAX_NUM_SETBACKS, TRM_INTERNAL, TRM_INTERNAL_STOP, TRM_LOST_RACE | NumericalDifficulty | — |
| prosta PRIM_INFEAS | Infeasible | — |
| prosta DUAL_INFEAS | Unbounded | — |
| prosta PRIM_AND_DUAL_INFEAS, PRIM_INFEAS_OR_UNBOUNDED | InfeasibleOrUnbounded | — |
| anything else (including MSK_RES_OK with UNKNOWN statuses) | Unknown | — |

  Solution returned: OPTIMAL → Optimal; PRIM_FEAS, PRIM_AND_DUAL_FEAS → Feasible; else None.
  Totality is tested over every MSK_SOL_STA_* (H:1731-1740), MSK_PRO_STA_* (H:1099-1107) and
  MSK_RES_TRM_* (H:1652-1666) value, with `MSK_getresponseclass` (H:6200) as the fallback for
  any other response code; no licence needed.

## R7. Certificates

- Primal infeasibility (PRIM_INFEAS_CER): the dual solution is the certificate; its objective
  (l^c)ᵀs_l^c − (u^c)ᵀs_u^c + (l^x)ᵀs_l^x − (u^x)ᵀs_u^x − gᵀẏ is positive for minimisation and
  negative for maximisation (M§12.1.2, §12.2.2). Mapped to feature 001's sense-independent
  certificate convention (d_F > 0): y_F = σ·y, z_F = σ·(s_l − s_u), s_F = σ·ẏ.
- Dual infeasibility (DUAL_INFEAS_CER): the primal xx is the ray (M§12.1.3).
- MOSEK returns certificates without a parameter, so the `certificates` option sets nothing.

## R8. Callbacks

- `MSK_putcallbackfunc` H:5232; signature H:2650-2656 with `douinf`, `intinf`, `lintinf`.
  Progress codes INTPNT 90, CONIC 34, UPDATE_PRIMAL/DUAL/SIMPLEX 104/100/106; data
  `MSK_DINF_INTPNT_PRIMAL_OBJ` 18, `INTPNT_DUAL_OBJ` 13, `OPTIMIZER_TIME` 50, `SIM_OBJ` 64,
  `MSK_IINF_INTPNT_ITER` 17, `SIM_PRIMAL_ITER` 128, `SIM_DUAL_ITER` 121.
- A non-zero return terminates with `MSK_RES_TRM_USER_CALLBACK` (100007); there is no other
  termination function, so a cancellation flag requires the callback. The callback must not
  call MOSEK functions (M§7.7).
- Thread of invocation and non-overlap: not stated by the manual; established by the
  contract test (spec FR-005) on both backends.

## R9. Parameters (one applying function, constitution VII-G)

| Shared field | MOSEK parameter(s) | Note |
|---|---|---|
| time limit | `MSK_DPAR_OPTIMIZER_MAX_TIME` | |
| iteration limit (simplex iterations) | `MSK_IPAR_SIM_MAX_ITERATIONS` | |
| threads | `MSK_IPAR_NUM_THREADS` | |
| primal feasibility tolerance | `MSK_DPAR_INTPNT_TOL_PFEAS`, `INTPNT_CO_TOL_PFEAS`, `INTPNT_QO_TOL_PFEAS` | all three set; each applies to its class |
| dual feasibility tolerance | `…_TOL_DFEAS` likewise | |
| method | `MSK_IPAR_OPTIMIZER` | Auto FREE, PrimalSimplex PRIMAL_SIMPLEX, DualSimplex DUAL_SIMPLEX, Barrier INTPNT (CONIC for conic problems); Concurrent unsupported (error) |
| presolve | `MSK_IPAR_PRESOLVE_USE` | Auto FREE, Off OFF; Conservative and Aggressive unsupported (error) |
| random seed | — | unsupported for continuous problems (error) |
| solver log | `MSK_IPAR_LOG` and the log stream | default off (0, no stream) |
| certificates | — | always returned |

Effective tolerances for verification are read back for the problem class solved (LP basic
solution: `MSK_DPAR_BASIS_TOL_X`/`S`; LP interior, QP, conic: the matching `PFEAS`/`DFEAS`).
Raw parameters use `MSK_whichparam` (H:5985) for the type and the name-based setters.
Feature 001's test that sets the `random_seed` vendor parameter through the raw passthrough is
generalised to use any mapped integer parameter, since MOSEK maps no seed.

## R10. Cross-solver agreement tolerance (CHK015)

Objective values agree when |p_G − p_M| ≤ 10 × max(τ_G, τ_M) × (1 + max(|p_G|, |p_M|)), with
τ the effective dual feasibility (optimality) tolerance of each solver (Gurobi `OptimalityTol`,
MOSEK interior or basis tolerance actually used), read from each result's effective
parameters.

## R11. Variational kinetics CLP test problems (CHK008, CHK009)

- `optimizeVKmodel` assembles the conic problem (A `optimizeVKmodel.m:1248`, bl/bu `:1255-1278`,
  c `:1322`, xl/xu `:1458-1459`, accs `:1594-1624`, F `:2010`, g `:2038`) and passes it to
  `solveCobraCLP` (`:2483`). On the stored route (`solveStrategy` absent = sequential) the
  solver rescales rows and rebuilds c before its first MOSEK call, so that literal inner problem
  is not reproducible without editing variational kinetics code.
- **Operational definition**: the CLP test problem is **the problem `optimizeVKmodel` passes to
  `solveCobraCLP`**, captured by running `optimizeVKmodel(model, param)` with the stored
  `param` plus `solveStrategy = 'single'` and `printLevel = 0`, taking `sol.prob`; its expected
  status is the variational kinetics single route's (`mosekopt` once, `parseMskResult`), with
  `saveProb = 1` keeping the MOSEK parameters used. Bit-identical across runs; no seeds.
  Confirmed by the maintainer at Gate 2A (2026-11-15); iDopaNeuroC_problem_4 excluded (stalls).
- Observed (MOSEK 11.2.0): 12 instances (iCoreED 1–5; iDopaNeuroC 1–5, 2 and 3 full-rank);
  cones only primal exponential (dimension 3) and rotated quadratic; all linear rows equalities;
  c = 0 on 8 instances (feasibility problems; the gap check is then trivial); expected status
  OPTIMAL for 11, and `UNKNOWN & MSK_RES_TRM_STALL` for iDopaNeuroC_problem_4 (deterministic over
  two runs). Sizes: iCoreED n ≤ 2084, k ≤ 2992; iDopaNeuroC n ≤ 38,763, k ≤ 56,159; MOSEK time
  0.03–0.15 s (iCoreED) and 0.4–2.6 s (iDopaNeuroC).
- Fixture format: **CBF** (Conic Benchmark Format, text; supports affine conic rows, exponential,
  power, quadratic, rotated quadratic and PSD cones), written by MOSEK from MATLAB and read back
  for a bit-for-bit round trip; a small CBF reader joins the shared test module. Precision of
  MOSEK's CBF writer: **probe** (the round trip decides; if it fails, fall back to a binary
  format of our own).
- Cone types not present in the variational kinetics problems (quadratic, dual exponential,
  power, nonnegative, zero, PSD) are covered by small hand-built problems with known optima.

## R12. QP test problem (CHK017)

The driver's QP (`driver_optimizeVKmodel_VK1to3m.m:872-877` → COBRA `optimizeCbModel`
`:439-445, :782-783, :816, :822`): single stage, c = 0, minimise ½ · 10⁻⁴ · Σ_j v_j² over the LP's
feasible set (all n variables penalised). Rebuilt in the test module from
iDopaNeuroC_problem_LP with Q = 10⁻⁴ I; reference objective 2.12873 × 10⁻⁶ (MOSEK via variational
kinetics parameters) and 2.12847 × 10⁻⁶ (Gurobi via COBRA defaults), recorded in a provenance
record for the derived fixture.

## R13. Corpus naming and document

- The LP fixture is renamed iDopaNeuroC_problem_LP: `project.yml` lists fixture names; the
  extraction script maps the source instance iDopaNeuroC_problem_1 to that name and records
  `source_instance` and `renamed_from` in provenance.
- `docs/test-corpus.md` (Tier 3, descriptive): every fixture, its class, dimensions, source,
  extraction, objective rule and expected outcome; why variational kinetics problems 2–5 are
  not LP test problems (feature 001 research R18).

## R14. Heavy tier

Heavy trials (iDopaNeuroC CLP problems) are registered by the shared harness and reported as
ignored with the reason "heavy tier: set OPTSOLVER_HEAVY_TESTS=1" unless that environment
variable is set; the in-gate checks set it.

## R15. Rust 1.85

The loader's unit test uses `u32::cast_unsigned` (stable from 1.87); replaced by an explicit
conversion. The benchmark's failure on 1.85 is diagnosed in its task. The in-gate checks add
`cargo +1.85.0 test --all-targets` (both tiers).
