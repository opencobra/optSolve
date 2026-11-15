# Data Model: MOSEK Backend For LP, QP And CLP

Extends `specs/001-core-gurobi-lp/data-model.md` (unchanged parts are not repeated). Closes the
routed checklist items CHK001–CHK005, CHK010, CHK011 and CHK020. MOSEK-specific conventions
(how MOSEK's own duals map onto these) are in research.md. Symbols marked **new** are proposed
for the Notation Convention (constitution XII-C) through `/speckit-constitution` before tasks.

## Symbols

| Symbol | Object | Status |
|---|---|---|
| Q | quadratic objective matrix, n × n, symmetric positive semidefinite | existing |
| F | affine conic constraint matrix, k × n, sparse | **new** |
| g | affine conic constraint offset, length k | **new** |
| K | product of cones K_1 × … × K_p, dimensions summing to k | **new** |
| s | conic dual vector, length k, in the dual cone K* | **new** |
| k | number of affine conic rows | **new** |

## Problem (additions)

Optimise σ-sense of ½ xᵀQx + cᵀx + γ subject to the linear rows, l ≤ x ≤ u, and Fx + g ∈ K.

**Quadratic term (CHK001)**: Q is given by its lower triangle (row ≥ column entries only) in
compressed sparse column form, n × n; the objective uses ½ xᵀQx with Q the full symmetric
matrix the lower triangle defines. Validation: dimensions, finite values, lower-triangle
entries only, sorted rows within a column. Positive semidefiniteness is **not** checked by
optSolver (cost); a non-convex Q is passed to the solver, and a solver that rejects it returns
an error carrying its code and message (CHK020). For maximisation the solver requires −Q to be
positive semidefinite; the same rule applies.

**Affine conic constraints**: F (k × n, CSC, finite), g (length k, finite), and an ordered cone
list; each entry is a cone type and a dimension; the dimensions sum to k; cone i applies to the
consecutive block of rows of Fx + g that it covers. k = 0 means no conic constraints.

## Cones (CHK002–CHK004)

For a block u ∈ ℝ^d:

| Type | Definition of the cone | Dual cone |
|---|---|---|
| Quadratic (d ≥ 1) | u₁ ≥ ‖u₂:d‖₂ | itself |
| Rotated quadratic (d ≥ 2) | 2u₁u₂ ≥ ‖u₃:d‖₂², u₁, u₂ ≥ 0 | itself |
| Primal exponential (d = 3) | closure of {u₁ ≥ u₂ exp(u₃/u₂), u₂ > 0} | dual exponential |
| Dual exponential (d = 3) | closure of {u₁ ≥ −u₃ exp(u₂/u₃ − 1), u₃ < 0} | primal exponential |
| Primal power (d ≥ 2) with exponents α₁…α_ℓ > 0, ℓ < d, normalised to sum 1 | ∏_{i≤ℓ} u_i^{α_i} ≥ ‖u_{ℓ+1:d}‖₂, u_{1:ℓ} ≥ 0 | dual power with the same α |
| Dual power | ∏_{i≤ℓ} (u_i/α_i)^{α_i} ≥ ‖u_{ℓ+1:d}‖₂, u_{1:ℓ} ≥ 0 | primal power |
| Nonnegative | u ≥ 0 | itself |
| Zero | u = 0 | all of ℝ^d |
| Positive semidefinite, order r, d = r(r+1)/2 | svec⁻¹(u) ⪰ 0 | itself |

Exponential-cone coordinate order and the dual-exponential form follow MOSEK's definitions
(research.md confirms them against the manual). Exponents are supplied unnormalised and
normalised by optSolver (or the solver) to sum to 1.

**Positive semidefinite layout (CHK003)**: svec(X) lists the lower triangle of the symmetric
r × r matrix X column by column (X₁₁, X₂₁, …, X_r1, X₂₂, X₃₂, …), with every off-diagonal entry
multiplied by √2, so that svec(X)ᵀsvec(Y) = trace(XY).

## Result (additions)

- s: conic duals, length k, in K* (sign convention: research.md maps the solver's duals onto
  the stationarity condition below).
- For QP and CLP, y and z keep their LP meaning; the basis is absent for interior-point
  solutions.
- z_l, z_u (implementation decision 2026-11-15, research R19): separate bound multipliers,
  length n, σz_l ≥ 0, σz_u ≥ 0, z = z_l − z_u; the dual objective's bound terms are
  lᵀz_l − uᵀz_u. MOSEK fills them from s_l, s_u; Gurobi splits its reduced cost by sign. The
  solution view also says whether the solution is basic (a basis is returned).

## Verification (CHK005)

Stationarity, extending feature 001's convention (z = c − Aᵀy, with sign conditions on y and
z through σ = +1 for minimise and −1 for maximise):

  z = c + Qx − Aᵀy − Fᵀs, with σs ∈ K*.

So for minimisation s ∈ K*, and for maximisation −s ∈ K* (the same flip feature 001 applies
to y and z). research.md R5 maps MOSEK's duals onto this directly.

Measures (each absolute and relative; relative scalings as listed; judged as in feature 001
against the effective solver tolerances × 10):

| Measure | Absolute | Relative scaling | Tolerance |
|---|---|---|---|
| Reduced-cost consistency (now with Qx and Fᵀs) | per column \|z_j − (c_j + (Qx)_j − (Aᵀy)_j − (Fᵀs)_j)\| | per column ÷ (1 + \|c_j\| + \|(Qx)_j\| + Σ\|A_ij y_i\| + Σ\|F_ij s_i\|), then max | dual |
| Primal cone violation | max over cone blocks of the block's violation of u = (Fx + g)_block (table below) | ÷ (1 + ‖g‖∞ + ‖Fx‖∞) | primal |
| Dual cone violation | same for σs against K* | ÷ (1 + ‖s‖∞) | dual |
| Conic complementarity (per cone, 2026-11-15) | max over cones K of \|s_Kᵀ(Fx + g)_K\| | ÷ (1 + \|p\|) | dual |
| Primal-dual gap | \|p − d\| with p = ½xᵀQx + cᵀx + γ and d = γ − ½xᵀQx + row terms as in feature 001 + bound terms (lᵀz_l − uᵀz_u when z_l, z_u are given, else as in feature 001) − gᵀs | ÷ max(1, min(\|p\|, \|d\|)) (MOSEK's normalisation, 2026-11-15) | dual |

Per-item complementary slackness (feature 001) is required only for basic solutions; for
interior-point solutions it is reported, not required, and does not decide the verdict
(2026-11-15, research R19). With z_l, z_u given it uses each multiplier against its own
bound, and the reduced-cost and dual-sign measures also check z = z_l − z_u, σz_l, σz_u ≥ 0
and no multiplier on an infinite bound.

Block violations: quadratic max(0, ‖u₂:‖ − u₁); rotated quadratic max(0, ‖u₃:‖² − 2u₁u₂, −u₁,
−u₂) scaled by 1/(1 + ‖u‖∞); primal exponential max(0, u₂exp(u₃/u₂) − u₁, −u₂) with the closure
cases u₂ = 0 handled exactly (u₁ ≥ 0, u₃ ≤ 0); dual exponential symmetrically; power cones
max(0, ‖u_tail‖ − ∏ ...) with negative leading coordinates counted as violations; nonnegative
max(0, −min u); zero ‖u‖∞; positive semidefinite max(0, −λ_min(svec⁻¹(u))). Linear, bound,
dual-sign and gap measures of feature 001 are unchanged for the linear part.

Certificates for conic problems extend feature 001's Farkas certificate with s_F (σ-free
convention, d_F > 0): Aᵀy_F + z_F + Fᵀs_F = 0, s_F ∈ K*, and d_F includes −gᵀs_F.

## Contract behaviour tests (CHK010, CHK011)

**Certificate sign (CHK010)**: the problem "minimise x₁ + x₂ subject to x₁ + x₂ ≥ 3,
0 ≤ x ≤ 1" (tests/fixtures/small/infeasible.mps) has, up to positive scaling, the unique Farkas
certificate y_F = (1) with z_F = (−1, −1): the row's lower bound 3 is the violated side, so the
convention (d_F > 0, data-model.md of feature 001) requires y_F,1 > 0 and z_F,j < 0 (upper
bounds active). The test asserts these signs, not only that verification passes.

**Callback thread (CHK011)**: the progress closure records the current thread's identifier and
increments a shared in-flight counter on entry and decrements it on exit; the test asserts that
every recorded identifier equals the identifier of the thread that called `solve`, that the
maximum observed in-flight count is 1, and that at least one call occurred, with threads = 4
and an algorithm that reports progress (barrier).
