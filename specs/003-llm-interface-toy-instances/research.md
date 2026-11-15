# Research: LLM Interface Documentation And Self-Contained Cross-Solver Test Instances

Phase 0 of the plan. **G:n** is line n of the installed Gurobi 13.0.3 header
`$GUROBI_HOME/include/gurobi_c.h`; Gurobi documentation is
https://docs.gurobi.com/projects/optimizer/en/current/ (the "current" tree already mentions
14.0.0; where it and a probe could differ, the 13.0.3 probe decides). MOSEK facts are from
MOSEK 11.2.0 tool output. Probes (C against `libgurobi130`, MATLAB with both solvers) were run
2026-11-15 by two read-only research agents; sources and output in the session scratchpad,
summarised here. Closes routed checklist items CHK004, CHK005, CHK006, CHK008, CHK010, CHK015,
CHK019, CHK020, CHK022.

## R1. Gurobi QP loading (FR-014, FR-016; CHK022)

- **Decision**: after the model is loaded as today, one `GRBaddqpterms(model, int numqnz,
  int *qrow, int *qcol, double *qval)` (G:1083) with our lower-triangle Q mapped as: diagonal
  (i, i) → 0.5·Q_ii; off-diagonal (i > j) → 1.0·Q_ij, emitted once. Index arrays (row and column
  per entry) and the value array are built (12 B per nonzero of Q; Complexity Tracking): the
  diagonal needs a scaled value, and Gurobi takes no slice form. nnz(Q) < 2³¹ checked.
- **Rationale**: Gurobi's objective is xᵀQx + cᵀx with terms added as given; "adding a term in
  row i and column j is equivalent to adding a term in row j and column i"
  (reference/c/model.html#c.GRBaddqpterms). Probe: terms (0,0,1), (1,1,1), (1,0,1) at x = (1,1)
  give 3 = ½xᵀ[[2,1],[1,2]]x. `GRBXloadmodel` (G:967) has no Q arguments.
- **Non-convex Q**: the default `NonConvex` = −1 (G:1378) silently enables spatial branching.
  optSolver sets `NonConvex` = 0 whenever Q is present (in `gurobi::params::apply`, VII-G), so a
  non-convex Q is an error: probe returned 10020 `GRB_ERROR_Q_NOT_PSD` (G:82), "Objective Q not
  PSD (negative diagonal entry)" — matching MOSEK's behaviour (feature 002 R4). The caller can
  override through the raw route.
- **Alternatives**: 0.5 on off-diagonal terms (wrong objective); `NonConvex` −1 (hides
  non-convexity; changes problem class silently).

## R2. Gurobi cones (FR-014; CHK004)

- **Decision**: supported cone types on Gurobi: quadratic, rotated quadratic, nonnegative, zero.
  - Nonnegative and zero blocks: linear rows Fx ≥ −g and Fx = −g.
  - Quadratic and rotated quadratic blocks: auxiliary columns u (one per row of the block;
    bounds u₁ ≥ 0 for quadratic, u₁, u₂ ≥ 0 for rotated, free otherwise), linear rows
    Fx − u = −g, and one `GRBaddqconstr` (G:1077) per cone with diagonal terms:
    quadratic Σ_{i≥2} u_i² − u₁² ≤ 0; rotated Σ_{i≥3} u_i² − 2u₁u₂ ≤ 0 (one (1, 2) term, value
    −2). Gurobi recognises both as convex second-order cones (concepts/modeling/constraints.html).
  - Loading: the conic rows are stacked under A, so each column of the loaded matrix holds its A
    entries followed by its F entries; columns are loaded through the existing blocked path with a
    fixed-size buffer that holds the converted row indices **and** the block's values (F's values
    follow A's in each column, so the block is assembled, not borrowed: O(block) memory, 12 B per
    buffered entry, Complexity Tracking). Auxiliary columns are appended with `GRBXaddvars` with
    their −1 coefficients. LP and QP problems keep the zero-copy i32 single-call path.
  - Any other cone type: `Error::Unsupported` before any solver call (R8).
- **Rationale**: there is no bulk multi-quadratic-constraint call (one call per cone, as many
  calls as cones); probes: quadratic x = (5, 3, 4) (4 barrier iterations), rotated x = (2, 1, 2).
- **Alternatives**: `GRBaddgenconstrNorm` (G:1032; general constraint without duals);
  `GRBaddcone` (G:1081; exported but undocumented, so unsupported); Fx + g inside the quadratic
  constraint directly (dense, harder dual mapping).

## R3. Gurobi duals for QP and cones (FR-014; CHK004)

- **Decision**: `QCPDual` = 1 (G:1393) when cones are present (set in `params::apply`). Then:
  y = `Pi` of the original rows; z = `RC` of the original columns (split into z_l, z_u by sign,
  as for LP); conic duals s = −`Pi` of the rows Fx − u = −g (quadratic and rotated blocks) and
  s = +`Pi` of the rows Fx ≥ −g, Fx = −g (nonnegative and zero blocks). `QCPi` (G:430) is not
  used.
- **Derivation**: Gurobi's Lagrangian cᵀx + ½xᵀQx − yᵀ(Ax − b) − wᵀ(Fx − u + g) − λq(u)
  gives x-stationarity c + Qx − Aᵀy − Fᵀw − z = 0; with the contract z = c + Qx − Aᵀy − Fᵀs this
  is s = w for rows written Fx ⋛ −g. For the cone rows written u − Fx = g (the probe's form)
  s = −w; the implementation writes them Fx − u = −g so that s = +Pi uniformly, and the sign is
  asserted by the hand-built cone tests (feature 002 T034) on Gurobi. Mapping through QCPi fails
  at the cone apex (∇q = 0; the bound multiplier of u₁ ≥ 0 is then in RC_u), while Pi of the
  defining rows includes it.
- **Evidence (probe, minimise)**: quadratic: Pi(A rows) = (0.6, 0.8) = y; −Pi(u − Fx = g rows) =
  (1, −0.6, −0.8) = s (feature 002 contract test value). Rotated: s = (1, 2, −2), y = (−2, 2),
  checked by hand against KKT. Maximise: every Pi flips, s = −Pi still holds and σs ∈ K*.
- **Docs**: `Pi` sign rule (attributes/constraintlinear.html#Pi); `Pi`, `RC` "only available for
  convex, continuous models"; for QCP models `Pi` needs `QCPDual` = 1.

## R4. Gurobi certificates for QP and cones (CHK005)

- **Decision**: none. Infeasible and unbounded QP and conic problems on Gurobi are reported with
  their status and no certificate; with certificates requested the verification report then has
  the certificate measure NotEvaluated (required), i.e. PartiallyVerified — documented in the
  interface document. `DualReductions` = 0 is kept (with it the infeasible SOCP reports status 3,
  without it 4).
- **Rationale**: `InfUnbdInfo` "Only affects linear programming (LP) models"
  (reference/parameters.html#InfUnbdInfo); `UnbdRay` "for unbounded linear models only"
  (attributes/variable.html#UnbdRay). Probe: `FarkasDual`, `FarkasProof`, `UnbdRay` (G:526–528)
  return 10005 (data not available) for an infeasible SOCP and QP.
- **Alternative**: build a certificate ourselves (e.g. from a phase-one problem) — out of scope.

## R5. Gurobi barrier, tolerances, callbacks, statuses (CHK022)

- QP and SOCP are solved by barrier (default method); crossover is not available for QP and QCP
  (parameters.html#Crossover), so their solutions are interior (no basis; per-item complementary
  slackness informational, feature 002 R19).
- **Effective tolerances for verification**: LP: `FeasibilityTol`, `OptimalityTol` (unchanged);
  QP: `FeasibilityTol` (primal), max(`OptimalityTol`, `BarConvTol`) (dual); conic:
  `FeasibilityTol`, max(`OptimalityTol`, `BarQCPConvTol`) (defaults 1e-6, 1e-6, 1e-8, 1e-6;
  G:1208, 1213, 1236, 1240).
- `GRB_CB_BARRIER` fires for QP (12 calls) and SOCP (5 calls): progress and the
  callback-thread test apply.
- **Statuses**: an unbounded convex QP returns `GRB_SUBOPTIMAL` (13) under barrier, `GRB_UNBOUNDED`
  under simplex; the existing translation maps 13 to NumericalDifficulty (never Optimal), which
  stands. Errors to expect: 10020 `Q_NOT_PSD`, 10021 `QCP_EQUALITY_CONSTRAINT` (never write cones
  as equalities), 10024 `NOT_SUPPORTED`; all carried as `Error::Solver` with Gurobi's message.

## R6. QP in MPS (FR-006; CHK010)

- **Decision**: toy QPs are written by Gurobi (`gurobi_write`), which emits a **QUADOBJ** section
  with one triangle (upper), entries Q_ij with objective ½xᵀQx. The test MPS reader accepts
  QUADOBJ, QSECTION <objective row> and QMATRIX: QUADOBJ/QSECTION are one-triangle (either
  triangle, mirrored to the lower triangle, duplicates summed as both tools do); QMATRIX is full
  (lower triangle taken, not doubled).
- **Evidence**: on Q = [[2,1],[1,2]], x₁ + x₂ = 2, x ≥ 0: Gurobi writes `QUADOBJ / x1 x1 2 /
  x1 x2 1 / x2 x2 2`; MOSEK writes `QSECTION OBJ` with the lower triangle; each tool reads the
  other's file (and three hand-written variants) to objective 3. Trap: QUADOBJ listing both (1,2)
  and (2,1) gives 4 in both tools (summed) — the reader sums too, matching them.
- Other MPS facts: a second N row (the only way to write a free row) is dropped by both tools (the
  reader keeps rejecting it; free rows stay covered by an in-code test); a column declared only in BOUNDS is not portable (MOSEK error 1106), so every toy
  column appears in COLUMNS (with an explicit 0 objective entry if empty).

## R7. Conic toy instances in JTASK, including PSD (FR-006, FR-007; CHK020)

- **Decision**: one JTASK file per cone type, written by MOSEK: `["quad", d]`, `["rquad", d]`,
  `["pexp"]`, `["dexp"]`, `["ppow", d, [w…]]`, `["dpow", d, [w…]]`, `["rplus", d]`,
  `["rzero", d]`, **`["svecpsd", d]`** (d = r(r+1)/2; √2 off-diagonal scaling carried in F, as in
  our svec layout); ACC without `b` (zero). The JTASK reader adds `svecpsd` (reject non-triangular
  d) and normalises power weights (MOSEK keeps them unnormalised).
- **Evidence**: PSD toy (min X₁₁ + X₂₂, X₂₁ = 1, optimum 2) written as JTASK, read back and solved
  to 2; PTF and native task also round-trip; CBF changes the structure (PSDCON/HCOORD); MPS is
  refused (MOSEK 4012). Power and dual exponential toys round-trip to their known optima (−1, −2,
  e⁻¹).
- **Caveat**: MOSEK's JSON format page (C API 11.2.8, json-format.html) lists no domain type
  names; `svecpsd` (and the others) are evidenced by MOSEK 11.2.0 writer output, matching the PTF
  keyword `SVECPSD`. The feature 002 reader header's claim that the names are "confirmed in the
  specification" is corrected to "observed in writer output".
- **Maximise forms**: one file per cone (minimise); the maximise form is derived in the test (c
  negated, sense flipped), as the feature 002 derived QP is, so each file stays a single problem.

## R8. Per-cone capabilities (FR-014)

- **Decision**: `Cone::kind()` returns a parameter-free `ConeKind` (quadratic, rotated quadratic,
  primal/dual exponential, primal/dual power, nonnegative, zero, positive semidefinite);
  `Capabilities` gains `cones: Vec<ConeKind>`; `check_problem` rejects a conic problem with a cone
  kind not listed, with `Error::Unsupported { field: <cone kind name>, backend }`. MOSEK lists all
  kinds; Gurobi lists quadratic, rotated quadratic, nonnegative, zero.
- **Alternative**: a Gurobi-internal check after load (rejected: FR-014 requires rejection before
  any solver call, and the shared check keeps the rule in one place).

## R9. Toy instance construction and provenance (FR-006, FR-009; CHK006, CHK019)

- **Decision**: one MATLAB script, `tools/corpus/make_toy_instances.m`, holds each toy's
  definition (matrices, known solution, derivation text), writes the file with the solver tool
  that owns the format (Gurobi for MPS, MOSEK for JTASK), reads it back, solves it with **both**
  tools where both support the class (MOSEK only otherwise), and writes the provenance record. It
  stops if any tool's status or objective disagrees with the analytic value (the derivation is the
  arbiter). Existing small LP fixtures are kept and gain the independent-tool checks.
- **Provenance fields**: instance, class, format, file_sha256, construction (prose), sense,
  dimensions (m, n, nnz, k, nnz(F), cones), expected_status, expected_objective, expected_x (or
  null when not unique), derivation (prose), independent_checks [{tool, version, status,
  objective}], extracting_script, optsolver_commit, optsolver_worktree_clean, licence
  ("MIT OR Apache-2.0, project-authored").
- **Reader versus tool (CHK019)**: the Rust test reads the file and solves; a disagreement with the
  recorded tool results fails the test; resolution is by the analytic derivation, recorded in
  research of the feature that fixes it.

## R10. The interface document as one source (FR-001–FR-005; CHK008)

- **Decision**: `docs/llm-interface.md` is the crate-level documentation:
  `#![doc = include_str!("../docs/llm-interface.md")]` in `src/lib.rs`. Its Rust code blocks are
  documentation tests: compiled in every tier; an example that solves selects its backend with
  `#[cfg(feature = …)]` inside the block and, when the environment cannot be created (no
  licence), prints the error and returns — so `cargo test --doc` compiles every example and
  runs each solve where the backend is available (SC-001).
- **llms.txt**: `docs/llms.txt` (llmstxt.org form: title, one-paragraph summary, link list)
  pointing to `docs/llm-interface.md`, `docs/test-corpus.md` and the README.
- **Drift test (CHK008)**: a test scans the solver-independent modules (`src/lib.rs`,
  `problem.rs`, `conic.rs`, `options.rs`, `result.rs`, `verify.rs`, `backend.rs`, `error.rs`) for
  public items — `pub fn`, `pub struct`, `pub enum` and their variants, `pub trait`, `pub const`,
  `pub type`, public struct fields — excluding `pub(crate)`, items after `#[doc(hidden)]` and
  `mod tests`; plus `gurobi::Env`, `mosek::Env`. Each identifier must appear in the document in
  backticks. A test of the test proves it fails on an unmentioned identifier.
- **Alternatives**: rustdoc JSON (nightly only); two copies kept in sync by a test (rejected by
  the single-source clarification).

## R11. Cross-solver comparison (FR-011–FR-013, FR-015; CHK015)

- **Decision**: pair trials (feature 002 harness, `Registry::pair_trials`) over: every toy
  instance; the LP corpus (iDopaNeuroC_problem_LP, seeds 1–3, existing T019 generalised); the
  derived QP (iDopaNeuroC_problem_LP_minNorm). The CLP corpus contains exponential cones, so it is
  not comparable with Gurobi: reported as not comparable (ignored with reason), never passed;
  heavy-tier instances likewise (not run in the comparison). Rule: equal status; for Optimal,
  objectives within feature 002 R10 with R19 normalisation, both Verified; for Infeasible and
  Unbounded, equal status only (certificates verified per backend where available, R4).
- **Report**: each pair trial appends a line (instance, pair, statuses, objectives, absolute and
  relative difference, tolerance, verdict) to `results/003-llm-interface-toy-instances/runs/
  <UTC>-compare/report.tsv` created once per test-binary run (Principle IX: generated output home);
  the path is printed.

## R12. SC-002 LLM trial (Gate 3)

Three independent agents (Task tool), each given only `docs/llm-interface.md` and one task
statement (LP from `toy_lp_*`, QP from `toy_qp_*`, conic from `toy_clp_quad`), with their code
compiled in a scratch crate depending on optsolver by path and run against the toy instance;
three attempts per task; model, prompts, code, compiler output and results recorded under
`results/003-llm-interface-toy-instances/runs/<UTC>-llm-trial/`.

## R13. Findings during implementation (T012–T013, 2026-11-15)

- **One-line `OBJSENSE MAX`**: Gurobi writes `OBJSENSE MAX` on one line; MOSEK 11.2 cannot read
  it (error 1100 at the next line, in free, relaxed and CPLEX read modes). The generator
  therefore rewrites that line of `toy_lp_max.mps` and `toy_qp_max.mps` to the two-line form
  `OBJSENSE` / `    MAX` (as `unbounded.mps` already uses) and requires both tools to read the
  file back equal to the definition; every other byte is Gurobi's. Recorded in each record's
  `construction`. The test reader accepts both forms.
- **Ranged rows in Gurobi**: Gurobi reads an MPS RANGES row as an equality row with an extra
  column (`MPS_Rgr1`) bounded by the range; folded back, it is the same problem as MOSEK's and
  the test reader's reading (two representations, one problem).
- Every toy instance has a unique optimal x; each solver's x is within 1e-4 relative of it
  (largest deviation 7.7e-10). The generator's agreement check was shown to fail on a wrong
  expected objective (5.001).
- **Gurobi conic loading route (T019)**: instead of stacking F under A in the column loader
  (plan wording), the conic rows are appended after the linear model: `GRBXaddconstrs` (rows
  with sense and right-hand side −g, no coefficients), F's coefficients by `GRBXchgcoeffs` in
  blocks through a fixed buffer of at most `BLOCK_CAPACITY` triplets, auxiliary columns by
  `GRBXaddvars`, then one `GRBaddqconstr` per cone. Same memory bound (fixed buffer, O(k) for
  rows and auxiliaries), and the A load path (zero-copy i32 single call) is untouched.
  `src/gurobi/load_qc.rs`.
- **Error messages lost after the optimise call (bug, features 001–002)**: both backends read the
  vendor error message after removing the callback; Gurobi's successful `GRBsetcallbackfunc`
  clears the last error, so a failing `GRBoptimize` (for example 10020, non-convex Q) returned
  its code with an empty message. Both backends now read the error immediately after the
  optimise call (constitution III), and `f003::t018_nonconvex_q_is_an_error` asserts a non-empty
  message on every backend.
