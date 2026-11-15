# Data Model: LLM Interface Documentation And Self-Contained Cross-Solver Test Instances

Extends feature 002's data model (unchanged parts not repeated). Closes routed checklist items
CHK006 and CHK019 (with research R9) and records the entities of spec §Key Entities.

## ConeKind (new, core)

The parameter-free kind of a cone: Quadratic, RotatedQuadratic, PrimalExponential,
DualExponential, PrimalPower, DualPower, Nonnegative, Zero, PositiveSemidefinite.
`Cone::kind()` maps each cone to its kind; each kind has a stable name used in errors and
reports ("quadratic", "rotated quadratic", …).

## Capabilities (changed)

Gains `cones: Vec<ConeKind>`: the cone kinds a backend accepts. Rule (`check_problem`): a
problem with conic data is accepted only if the backend lists `ProblemClass::Conic` and every
cone's kind; otherwise `Error::Unsupported { field: <kind name>, backend }` before any solver
call. MOSEK: all kinds. Gurobi: Quadratic, RotatedQuadratic, Nonnegative, Zero.

## Gurobi formulation of conic data (internal; research R2, R3)

| Block kind | Gurobi model | s from |
|---|---|---|
| Nonnegative | rows Fx ≥ −g | +Pi of those rows |
| Zero | rows Fx = −g | +Pi |
| Quadratic (d) | aux columns u (u₁ ≥ 0), rows Fx − u = −g, QC Σ_{i≥2}u_i² − u₁² ≤ 0 | +Pi of the Fx − u rows |
| Rotated quadratic (d) | aux u (u₁, u₂ ≥ 0), rows Fx − u = −g, QC Σ_{i≥3}u_i² − 2u₁u₂ ≤ 0 | +Pi |

Auxiliary columns are never returned: x has the caller's n entries; row activity the caller's m.
Certificates: none for QP and conic problems on Gurobi (research R4).

## Toy instance (test corpus)

A file in `tests/fixtures/small/`: `<name>.mps` (LP, QP) or `<name>.jtask` (conic), with
`<name>.provenance.json`. Declared in `project.yml` `test_corpus.toy_instances` (one list, all
classes). Provenance fields (research R9): instance, class (LP | QP | Conic), format (mps |
jtask), file_sha256, construction, sense, m, n, nnz, k, nnz_F, cones (kind and dimension per
cone, power exponents), expected_status, expected_objective (null unless Optimal), expected_x
(null when not unique), derivation, independent_checks (list of {tool, version, status,
objective}), extracting_script, optsolver_commit, optsolver_worktree_clean, licence.

Validation (tests): file hash equals file_sha256; the reader's dimensions equal the record's;
every independent check agrees with expected_status and expected_objective.

### Toy set (FR-007)

| Name | Class | Outcome | Purpose |
|---|---|---|---|
| `infeasible`, `unbounded`, `ranged`, `empty_column` | LP | existing | kept; gain independent checks |
| `toy_lp_max` | LP | Optimal | maximise |
| `toy_lp_mixed` | LP | Optimal | equality, ≤ and ≥ rows with a degenerate-free unique optimum |
| `toy_qp_diag` | QP | Optimal | diagonal Q |
| `toy_qp_offdiag` | QP | Optimal | off-diagonal Q |
| `toy_qp_max` | QP | Optimal | maximise a concave quadratic |
| `toy_clp_quad`, `toy_clp_rquad`, `toy_clp_pexp`, `toy_clp_dexp`, `toy_clp_ppow`, `toy_clp_dpow`, `toy_clp_rplus`, `toy_clp_rzero`, `toy_clp_psd2`, `toy_clp_psd3` | Conic | Optimal | one per cone kind (PSD order 2 and 3); maximise form derived in the test |
| `toy_clp_infeasible` | Conic | Infeasible | quadratic cone, infeasible |

Free rows are not portable in MPS (a second N row is dropped by both tools, research R6): they
stay covered by feature 001's in-code test `t031_free_row`. Each optimum is the closed form of feature 002's hand-built cases (T034) unless stated in the
derivation.

## Comparison record (FR-013)

One line per (instance, backend pair): instance, backend_a, backend_b, status_a, status_b,
objective_a, objective_b, abs_diff, rel_diff, tolerance, verdict ∈ {agree, disagree,
not comparable (<reason>)}. Written to the run bundle `results/<feature>/runs/<UTC>-compare/
report.tsv` (research R11).

## Interface document (FR-001–FR-005)

Sections (each a heading in `docs/llm-interface.md`): Add the dependency; The problem model
and notation; Building a problem without copying (CSC rules, i32 versus usize, validation
errors); Quadratic objective; Conic constraints (every cone, svec layout); Choosing a backend
and capabilities; Options; Solving; Statuses and limit reasons; Reading the solution (sign
conventions of y, z, z_l, z_u, s); Certificates; Verification; Caller buffers; Errors;
Threading; Complete examples (LP, QP, conic, unsupported class, non-optimal status, buffers).
