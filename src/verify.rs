//! Solver-independent verification of feasibility and optimality (constitution XVI-D;
//! data-model.md, Verification). Implemented by task T027.

#![forbid(unsafe_code)]

use crate::conic::ConicRef;
use crate::options::defaults::VERIFICATION_FACTOR;
use crate::problem::{CscRef, Problem, RowIndex};
use crate::result::{Certificate, SolutionReturned, SolutionView, Status};

/// Tolerances a solution is judged against (the effective solver tolerances; the factor in
/// [`crate::options::defaults::VERIFICATION_FACTOR`] is applied by `verify`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerances {
    /// Primal feasibility tolerance.
    pub primal: f64,
    /// Dual feasibility tolerance.
    pub dual: f64,
    /// Integrality tolerance (MILP).
    pub integrality: f64,
}

/// Verdict on one measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Relative value ≤ factor × tolerance.
    Pass,
    /// Relative value > factor × tolerance.
    Fail,
    /// The inputs the measure needs were not returned.
    NotEvaluated,
}

/// One verification measure.
#[derive(Debug, Clone, PartialEq)]
pub struct Measure {
    /// Name, for example `"row-bound violation"`.
    pub name: &'static str,
    /// Absolute value.
    pub absolute: f64,
    /// Relative value.
    pub relative: f64,
    /// Effective tolerance it was judged against (before the factor).
    pub tolerance: f64,
    /// Verdict.
    pub verdict: Verdict,
    /// Whether the status requires this measure.
    pub required: bool,
}

/// Overall verdict of a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overall {
    /// Every evaluated measure passes and no required measure is NotEvaluated.
    Verified,
    /// Some measure fails.
    Failed,
    /// No failure, but some required measure was not evaluated.
    PartiallyVerified,
}

/// Verification report.
#[derive(Debug, Clone, PartialEq)]
pub struct VerificationReport {
    /// Every measure.
    pub measures: Vec<Measure>,
    /// Overall verdict.
    pub overall: Overall,
}

/// Measure names, as they appear in [`Measure::name`].
pub mod names {
    /// max_i max(b_l,i − (Ax)_i, (Ax)_i − b_u,i, 0).
    pub const ROW_BOUND: &str = "row-bound violation";
    /// max_j max(l_j − x_j, x_j − u_j, 0).
    pub const VARIABLE_BOUND: &str = "variable-bound violation";
    /// ‖z − (c − Aᵀy)‖∞.
    pub const REDUCED_COST: &str = "reduced-cost consistency";
    /// Largest wrong-signed part of σy_i or σz_j.
    pub const DUAL_SIGN: &str = "dual sign violation";
    /// Largest |dual| × distance from the bound its sign selects.
    pub const COMPLEMENTARY_SLACKNESS: &str = "complementary slackness";
    /// |p − d|, relative to max(1, min(|p|, |d|)).
    pub const GAP: &str = "primal-dual gap";
    /// Farkas certificate of infeasibility.
    pub const FARKAS: &str = "infeasibility certificate";
    /// Unbounded ray.
    pub const RAY: &str = "unbounded ray";
    /// Largest violation of Fx + g ∈ K over the cone blocks.
    pub const PRIMAL_CONE: &str = "primal cone violation";
    /// Largest violation of σs ∈ K* over the cone blocks.
    pub const DUAL_CONE: &str = "dual cone violation";
    /// max over cones K of |s_Kᵀ(Fx + g)_K|.
    pub const CONIC_COMPLEMENTARITY: &str = "conic complementarity";
}

/// Verifies a solution against its problem, from the problem and the solution alone
/// (data-model.md, Verification). Ax and Aᵀy are computed here; nothing the solver reports
/// about residuals or row activity is used.
///
/// Which measures appear:
///
/// - row-bound and variable-bound violations (and, with conic constraints, the primal cone
///   violation) when x is returned, or when the status is Optimal or a solution is reported
///   returned (then they are required);
/// - reduced-cost consistency (z = c + Qx − Aᵀy − Fᵀs), dual sign violation, complementary
///   slackness and primal-dual gap (p = ½xᵀQx + cᵀx + γ; d includes −½xᵀQx and −gᵀs) when
///   the status is Optimal, and with conic constraints the dual cone violation (σs ∈ K*) and
///   conic complementarity (required; NotEvaluated when x, y, z or s is absent). For other
///   statuses they are not part of what the status claims and are not listed;
/// - the infeasibility certificate when a Farkas certificate is returned, or when the status is
///   Infeasible (then required); the unbounded ray likewise with Unbounded.
///
/// An array of the wrong length makes the measures needing it Fail with infinite values; a
/// NaN anywhere in a measure makes it infinite, so it fails. The Farkas certificate is checked
/// with σ = +1 whatever the objective sense, since with c = 0 the sense plays no part
/// (Aᵀy_F + z_F + Fᵀs_F = 0, s_F ∈ K*); its relative value is +∞ (Fail) unless d_F > 0. The
/// ray must satisfy Fr ∈ K and Qr = 0, and its relative value is +∞ unless σcᵀr < 0.
pub fn verify<I: RowIndex>(
    problem: &Problem<'_, I>,
    solution: &SolutionView<'_>,
    tolerances: &Tolerances,
) -> VerificationReport {
    let n = problem.n();
    let optimal = solution.status == Status::Optimal;
    let primal_required = optimal || solution.returned != SolutionReturned::None;
    let mut measures = Vec::new();

    let x = Input::of(solution.x, n);
    let ax = match x {
        Input::Ok(x) => Some(times(problem, x)),
        _ => None,
    };

    let conic = problem.conic().filter(|c| c.k() > 0);
    if primal_required || !matches!(x, Input::Missing) {
        let tol = tolerances.primal;
        let (row, var) = match (x, &ax) {
            (Input::Ok(x), Some(ax)) => {
                let (a, s) = row_violation(problem, ax);
                let (b, t) = variable_violation(problem, x);
                (
                    judged(names::ROW_BOUND, a, s, tol, primal_required),
                    judged(names::VARIABLE_BOUND, b, t, tol, primal_required),
                )
            }
            (Input::Missing, _) => (
                not_evaluated(names::ROW_BOUND, tol, primal_required),
                not_evaluated(names::VARIABLE_BOUND, tol, primal_required),
            ),
            _ => (
                malformed(names::ROW_BOUND, tol, primal_required),
                malformed(names::VARIABLE_BOUND, tol, primal_required),
            ),
        };
        measures.push(row);
        measures.push(var);
        if let Some(conic) = conic {
            measures.push(match x {
                Input::Ok(x) => {
                    let fx = csc_times(conic.F(), x, conic.k());
                    let (abs, scale) = primal_cone_violation(conic, &fx);
                    judged(
                        names::PRIMAL_CONE,
                        abs,
                        abs / (1.0 + scale),
                        tol,
                        primal_required,
                    )
                }
                Input::Missing => not_evaluated(names::PRIMAL_CONE, tol, primal_required),
                Input::WrongLength => malformed(names::PRIMAL_CONE, tol, primal_required),
            });
        }
    }

    if optimal {
        measures.extend(dual_measures(
            problem,
            x,
            ax.as_deref(),
            solution,
            tolerances,
        ));
    }

    let farkas = matches!(solution.certificate, Some(Certificate::Farkas { .. }));
    let ray = matches!(solution.certificate, Some(Certificate::UnboundedRay { .. }));
    let infeasible = solution.status == Status::Infeasible;
    let unbounded = solution.status == Status::Unbounded;
    match solution.certificate {
        Some(Certificate::Farkas { y, z, s }) => {
            measures.push(farkas_measure(
                problem,
                y,
                z,
                s,
                tolerances.primal,
                infeasible,
            ));
        }
        Some(Certificate::UnboundedRay { r }) => {
            measures.push(ray_measure(problem, r, tolerances.primal, unbounded));
        }
        None => {}
    }
    if infeasible && !farkas {
        measures.push(not_evaluated(names::FARKAS, tolerances.primal, true));
    }
    if unbounded && !ray {
        measures.push(not_evaluated(names::RAY, tolerances.primal, true));
    }

    // With status Optimal every measure the status claims is required; a measure reported
    // but not required there (per-item complementary slackness of an interior-point
    // solution) is informational and does not decide the verdict.
    let decides = |q: &Measure| q.required || !optimal;
    let overall = if measures
        .iter()
        .any(|q| q.verdict == Verdict::Fail && decides(q))
    {
        Overall::Failed
    } else if measures
        .iter()
        .any(|q| q.required && q.verdict == Verdict::NotEvaluated)
    {
        Overall::PartiallyVerified
    } else {
        Overall::Verified
    };
    VerificationReport { measures, overall }
}

/// An optional solution array checked against its expected length.
#[derive(Clone, Copy)]
enum Input<'a> {
    Missing,
    WrongLength,
    Ok(&'a [f64]),
}

impl<'a> Input<'a> {
    fn of(v: Option<&'a [f64]>, len: usize) -> Self {
        match v {
            // An array of length 0 (m = 0, n = 0 or k = 0) is never missing: `Solution::view`
            // reports every empty array as absent.
            None if len == 0 => Input::Ok(&[]),
            None => Input::Missing,
            Some(v) if v.len() == len => Input::Ok(v),
            Some(_) => Input::WrongLength,
        }
    }
}

fn judged(
    name: &'static str,
    absolute: f64,
    relative: f64,
    tolerance: f64,
    required: bool,
) -> Measure {
    // NaN compares false, so it fails.
    let verdict = if relative <= VERIFICATION_FACTOR * tolerance {
        Verdict::Pass
    } else {
        Verdict::Fail
    };
    Measure {
        name,
        absolute,
        relative,
        tolerance,
        verdict,
        required,
    }
}

fn not_evaluated(name: &'static str, tolerance: f64, required: bool) -> Measure {
    Measure {
        name,
        absolute: f64::NAN,
        relative: f64::NAN,
        tolerance,
        verdict: Verdict::NotEvaluated,
        required,
    }
}

fn malformed(name: &'static str, tolerance: f64, required: bool) -> Measure {
    Measure {
        name,
        absolute: f64::INFINITY,
        relative: f64::INFINITY,
        tolerance,
        verdict: Verdict::Fail,
        required,
    }
}

/// Running maximum in which NaN counts as +∞ (so it can never hide a failure).
fn upd(acc: &mut f64, v: f64) {
    if v.is_nan() {
        *acc = f64::INFINITY;
    } else if v > *acc {
        *acc = v;
    }
}

fn norm_inf(v: &[f64]) -> f64 {
    let mut acc = 0.0;
    for &e in v {
        upd(&mut acc, e.abs());
    }
    acc
}

/// A v (length m), computed from the problem's own A.
fn times<I: RowIndex>(problem: &Problem<'_, I>, v: &[f64]) -> Vec<f64> {
    csc_times(problem.A(), v, problem.m())
}

/// M v (length `rows`) for a CSC matrix M.
fn csc_times<I: RowIndex>(a: &CscRef<'_, I>, v: &[f64], rows: usize) -> Vec<f64> {
    let mut out = vec![0.0; rows];
    for (j, &vj) in v.iter().enumerate().take(a.n()) {
        let (idx, vals) = a.column(j);
        for (&i, &aij) in idx.iter().zip(vals) {
            if let Some(o) = i.index().and_then(|i| out.get_mut(i)) {
                *o += aij * vj;
            }
        }
    }
    out
}

/// Σ_i M_ij w_i and Σ_i |M_ij w_i| for column j of a CSC matrix M.
fn csc_column_dot_and_magnitude<I: RowIndex>(a: &CscRef<'_, I>, j: usize, w: &[f64]) -> (f64, f64) {
    let (idx, vals) = a.column(j);
    idx.iter()
        .zip(vals)
        .fold((0.0, 0.0), |(dot, mag), (&i, &aij)| {
            let t = i.index().and_then(|i| w.get(i)).map_or(0.0, |&wi| aij * wi);
            (dot + t, mag + t.abs())
        })
}

/// (Aᵀw)_j, computed from the problem's own A.
fn column_dot<I: RowIndex>(problem: &Problem<'_, I>, j: usize, w: &[f64]) -> f64 {
    csc_column_dot_and_magnitude(problem.A(), j, w).0
}

/// Σ_i A_ij w_i and Σ_i |A_ij w_i| for column j.
fn column_dot_and_magnitude<I: RowIndex>(
    problem: &Problem<'_, I>,
    j: usize,
    w: &[f64],
) -> (f64, f64) {
    csc_column_dot_and_magnitude(problem.A(), j, w)
}

/// Qx for Q given by its lower triangle (the full symmetric product).
fn sym_times(q: &CscRef<'_, usize>, x: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; q.n()];
    for j in 0..q.n().min(x.len()) {
        let (idx, vals) = q.column(j);
        for (&i, &qij) in idx.iter().zip(vals) {
            if i < out.len() && i < x.len() {
                out[i] += qij * x[j];
                if i != j {
                    out[j] += qij * x[i];
                }
            }
        }
    }
    out
}

/// Fx + g: largest block violation of membership in K, and the scale ‖g‖∞ + ‖Fx‖∞.
fn primal_cone_violation(conic: &ConicRef<'_>, fx: &[f64]) -> (f64, f64) {
    let g = conic.g();
    let u: Vec<f64> = fx.iter().zip(g).map(|(a, b)| a + b).collect();
    let mut abs = 0.0;
    for (start, cone) in conic.blocks() {
        upd(&mut abs, cone.violation(&u[start..start + cone.dim()]));
    }
    (abs, norm_inf(g) + norm_inf(fx))
}

/// Largest block violation of σs ∈ K*.
fn dual_cone_violation(conic: &ConicRef<'_>, s: &[f64], sigma: f64) -> f64 {
    let mut abs = 0.0;
    let mut block = Vec::new();
    for (start, cone) in conic.blocks() {
        block.clear();
        block.extend(s[start..start + cone.dim()].iter().map(|v| sigma * v));
        upd(&mut abs, cone.dual_violation(&block));
    }
    abs
}

fn finite_max_abs(acc: &mut f64, v: f64) {
    if v.is_finite() && v.abs() > *acc {
        *acc = v.abs();
    }
}

fn row_violation<I: RowIndex>(problem: &Problem<'_, I>, ax: &[f64]) -> (f64, f64) {
    let rows = problem.rows();
    let (mut abs, mut scale) = (0.0, 0.0);
    for (i, &axi) in ax.iter().enumerate() {
        let (lo, hi) = rows.bounds(i);
        upd(&mut abs, lo - axi);
        upd(&mut abs, axi - hi);
        finite_max_abs(&mut scale, lo);
        finite_max_abs(&mut scale, hi);
    }
    (abs, abs / (1.0 + scale))
}

fn variable_violation<I: RowIndex>(problem: &Problem<'_, I>, x: &[f64]) -> (f64, f64) {
    let (mut abs, mut scale) = (0.0, 0.0);
    for ((&xj, &lj), &uj) in x.iter().zip(problem.l()).zip(problem.u()) {
        upd(&mut abs, lj - xj);
        upd(&mut abs, xj - uj);
        finite_max_abs(&mut scale, lj);
        finite_max_abs(&mut scale, uj);
    }
    (abs, abs / (1.0 + scale))
}

/// Amount by which σv has the wrong sign for a dual of a quantity bounded by [lo, hi]:
/// σv ≥ 0 where hi = +∞, σv ≤ 0 where lo = −∞, v = 0 where both are infinite.
fn sign_violation(sigma: f64, v: f64, lo: f64, hi: f64) -> f64 {
    let s = sigma * v;
    match (lo == f64::NEG_INFINITY, hi == f64::INFINITY) {
        (true, true) => v.abs(),
        (false, true) => (-s).max(0.0),
        (true, false) => s.max(0.0),
        (false, false) => {
            if v.is_nan() {
                f64::NAN
            } else {
                0.0
            }
        }
    }
}

/// The bound a nonzero dual selects (lo if σv > 0, else hi); `None` for a zero dual or when
/// it selects an infinite bound (that is a sign violation, counted there).
fn selected(sigma: f64, v: f64, lo: f64, hi: f64) -> Option<f64> {
    if v == 0.0 || v.is_nan() {
        return None;
    }
    let bound = if sigma * v > 0.0 { lo } else { hi };
    bound.is_finite().then_some(bound)
}

fn dual_measures<I: RowIndex>(
    problem: &Problem<'_, I>,
    x: Input<'_>,
    ax: Option<&[f64]>,
    solution: &SolutionView<'_>,
    tolerances: &Tolerances,
) -> Vec<Measure> {
    let (m, n) = (problem.m(), problem.n());
    let tol = tolerances.dual;
    let conic = problem.conic().filter(|c| c.k() > 0);
    let y = Input::of(solution.y, m);
    let z = Input::of(solution.z, n);
    let s_in = conic.map(|c| Input::of(solution.s, c.k()));
    let mut all = vec![
        names::REDUCED_COST,
        names::DUAL_SIGN,
        names::COMPLEMENTARY_SLACKNESS,
        names::GAP,
    ];
    if conic.is_some() {
        all.push(names::DUAL_CONE);
        all.push(names::CONIC_COMPLEMENTARITY);
    }
    // With Q, every dual measure needs x (through Qx).
    let x_needed = problem.Q().is_some();
    // Separate bound multipliers: both or neither (both absent: the dual objective uses z).
    let split_in = match (solution.z_l, solution.z_u) {
        (None, None) => None,
        (zl, zu) => Some((Input::of(zl, n), Input::of(zu, n))),
    };

    let wrong = matches!(y, Input::WrongLength)
        || matches!(z, Input::WrongLength)
        || matches!(s_in, Some(Input::WrongLength))
        || (x_needed && matches!(x, Input::WrongLength))
        || matches!(
            split_in,
            Some(
                (Input::WrongLength | Input::Missing, _) | (_, Input::WrongLength | Input::Missing)
            )
        );
    if wrong {
        return all.iter().map(|&name| malformed(name, tol, true)).collect();
    }
    let s: &[f64] = match s_in {
        None => &[],
        Some(Input::Ok(s)) => s,
        Some(_) => {
            return all
                .iter()
                .map(|&name| not_evaluated(name, tol, true))
                .collect();
        }
    };
    let (y, z) = match (y, z) {
        (Input::Ok(y), Input::Ok(z)) if !x_needed || matches!(x, Input::Ok(_)) => (y, z),
        _ => {
            return all
                .iter()
                .map(|&name| not_evaluated(name, tol, true))
                .collect();
        }
    };

    let sigma = problem.sense().sigma();
    let c = problem.c();
    let rows = problem.rows();
    let (l, u) = (problem.l(), problem.u());
    let c_scale = 1.0 + norm_inf(c);
    let split = match split_in {
        Some((Input::Ok(zl), Input::Ok(zu))) => Some((zl, zu)),
        _ => None,
    };
    let qx = match (problem.Q(), x) {
        (Some(q), Input::Ok(x)) => Some(sym_times(q, x)),
        _ => None,
    };

    // Reduced-cost consistency and dual sign (need y, z, and s with conic constraints; x
    // with Q). Reduced-cost consistency is scaled per column by the magnitude of the terms
    // actually summed, 1 + |c_j| + |(Qx)_j| + Σ_i |A_ij y_i| + Σ_i |F_ij s_i| (data-model.md;
    // maintainer decision 2026-11-14): with large duals, floating-point cancellation alone
    // leaves residuals far above 1 + ‖c‖∞.
    let mut rc = 0.0;
    let mut rc_rel = 0.0;
    let mut sign = 0.0;
    for j in 0..n {
        let (dot, magnitude) = column_dot_and_magnitude(problem, j, y);
        let (fdot, fmag) = match conic {
            Some(cn) => csc_column_dot_and_magnitude(cn.F(), j, s),
            None => (0.0, 0.0),
        };
        let qxj = qx.as_ref().map_or(0.0, |q| q[j]);
        let residual = (z[j] - (c[j] + qxj - dot - fdot)).abs();
        upd(&mut rc, residual);
        upd(
            &mut rc_rel,
            residual / (1.0 + c[j].abs() + qxj.abs() + magnitude + fmag),
        );
        upd(&mut sign, sign_violation(sigma, z[j], l[j], u[j]));
        if let Some((zl, zu)) = split {
            // z = z_l − z_u, σz_l ≥ 0, σz_u ≥ 0, and no multiplier on an infinite bound.
            let mismatch = (z[j] - (zl[j] - zu[j])).abs();
            upd(&mut rc, mismatch);
            upd(&mut rc_rel, mismatch / (1.0 + z[j].abs()));
            upd(&mut sign, (-sigma * zl[j]).max(0.0));
            upd(&mut sign, (-sigma * zu[j]).max(0.0));
            if l[j] == f64::NEG_INFINITY {
                upd(&mut sign, zl[j].abs());
            }
            if u[j] == f64::INFINITY {
                upd(&mut sign, zu[j].abs());
            }
        }
    }
    for (i, &yi) in y.iter().enumerate() {
        let (lo, hi) = rows.bounds(i);
        upd(&mut sign, sign_violation(sigma, yi, lo, hi));
    }
    let mut out = vec![
        judged(names::REDUCED_COST, rc, rc_rel, tol, true),
        judged(names::DUAL_SIGN, sign, sign / c_scale, tol, true),
    ];
    let mut tail = Vec::new();
    if let Some(cn) = conic {
        let v = dual_cone_violation(cn, s, sigma);
        tail.push(judged(
            names::DUAL_CONE,
            v,
            v / (1.0 + norm_inf(s)),
            tol,
            true,
        ));
    }

    // Complementary slackness and gap (need x as well).
    let (x, ax) = match (x, ax) {
        (Input::Ok(x), Some(ax)) => (x, ax),
        (Input::Missing, _) => {
            out.push(not_evaluated(
                names::COMPLEMENTARY_SLACKNESS,
                tol,
                solution.basic,
            ));
            out.push(not_evaluated(names::GAP, tol, true));
            out.extend(tail);
            if conic.is_some() {
                out.push(not_evaluated(names::CONIC_COMPLEMENTARITY, tol, true));
            }
            return out;
        }
        _ => {
            out.push(malformed(
                names::COMPLEMENTARY_SLACKNESS,
                tol,
                solution.basic,
            ));
            out.push(malformed(names::GAP, tol, true));
            out.extend(tail);
            if conic.is_some() {
                out.push(malformed(names::CONIC_COMPLEMENTARITY, tol, true));
            }
            return out;
        }
    };
    let gamma = problem.gamma();
    let half_xqx = qx.as_ref().map_or(0.0, |q| {
        0.5 * q.iter().zip(x).map(|(a, b)| a * b).sum::<f64>()
    });
    let p = c.iter().zip(x).map(|(&cj, &xj)| cj * xj).sum::<f64>() + gamma + half_xqx;
    let mut d = gamma - half_xqx;
    let mut cs = 0.0;
    for (i, (&yi, &axi)) in y.iter().zip(ax).enumerate() {
        let (lo, hi) = rows.bounds(i);
        if let Some(bound) = selected(sigma, yi, lo, hi) {
            d += yi * bound;
            upd(&mut cs, yi.abs() * (axi - bound).abs());
        } else if yi.is_nan() {
            d = f64::NAN;
            cs = f64::INFINITY;
        }
    }
    for j in 0..n {
        if let Some((zl, zu)) = split {
            // The dual objective's bound terms lᵀz_l − uᵀz_u, exactly (MOSEK's standard form,
            // where every bound has its own multiplier); infinite bounds carry no term (a
            // multiplier on one is a sign violation).
            if l[j].is_finite() {
                d += l[j] * zl[j];
                upd(&mut cs, zl[j].abs() * (x[j] - l[j]).abs());
            }
            if u[j].is_finite() {
                d -= u[j] * zu[j];
                upd(&mut cs, zu[j].abs() * (u[j] - x[j]).abs());
            }
            if zl[j].is_nan() || zu[j].is_nan() {
                d = f64::NAN;
                cs = f64::INFINITY;
            }
        } else if let Some(bound) = selected(sigma, z[j], l[j], u[j]) {
            d += z[j] * bound;
            upd(&mut cs, z[j].abs() * (x[j] - bound).abs());
        } else if z[j].is_nan() {
            d = f64::NAN;
            cs = f64::INFINITY;
        }
    }
    let mut conic_cs = None;
    if let Some(cn) = conic {
        let g = cn.g();
        d -= g.iter().zip(s).map(|(a, b)| a * b).sum::<f64>();
        let fx = csc_times(cn.F(), x, cn.k());
        // Per cone (maintainer decision 2026-11-15): the largest |s_Kᵀ(Fx + g)_K| over the
        // cones K, not the sum over all cones, in which many small interior-point products
        // accumulate.
        let mut v = 0.0;
        for (start, cone) in cn.blocks() {
            let r = start..start + cone.dim();
            let dot: f64 = s[r.clone()]
                .iter()
                .zip(fx[r.clone()].iter().zip(&g[r]))
                .map(|(si, (f, gi))| si * (f + gi))
                .sum();
            upd(&mut v, dot.abs());
        }
        conic_cs = Some(v);
    }
    let gap = (p - d).abs();
    let gap = if gap.is_nan() { f64::INFINITY } else { gap };
    // Per-item complementary slackness is exact for basic solutions and required there; for
    // interior-point solutions (no basis) it is reported, not required: MOSEK and Gurobi
    // barrier terminate on the total complementarity and the gap, both covered by the exact
    // gap measure (maintainer decision 2026-11-15).
    out.push(judged(
        names::COMPLEMENTARY_SLACKNESS,
        cs,
        cs / (1.0 + p.abs()),
        tol,
        solution.basic,
    ));
    // MOSEK's relative-gap normalisation, max(1, min(|p|, |d|)) (MOSEK 11.2 C API manual
    // §13.3; maintainer decision 2026-11-15).
    out.push(judged(
        names::GAP,
        gap,
        gap / 1f64.max(p.abs().min(d.abs())),
        tol,
        true,
    ));
    out.extend(tail);
    if let Some(v) = conic_cs {
        out.push(judged(
            names::CONIC_COMPLEMENTARITY,
            v,
            v / (1.0 + p.abs()),
            tol,
            true,
        ));
    }
    out
}

fn farkas_measure<I: RowIndex>(
    problem: &Problem<'_, I>,
    y: &[f64],
    z: &[f64],
    s: &[f64],
    tol: f64,
    required: bool,
) -> Measure {
    let (m, n, k) = (problem.m(), problem.n(), problem.k());
    if y.len() != m || z.len() != n || s.len() != k {
        return malformed(names::FARKAS, tol, required);
    }
    let conic = problem.conic().filter(|c| c.k() > 0);
    // σ = +1: the certificate concerns the feasible set only (c = 0).
    let rows = problem.rows();
    let (l, u) = (problem.l(), problem.u());
    let mut abs = 0.0;
    let mut d = 0.0;
    for j in 0..n {
        let fdot = conic.map_or(0.0, |cn| csc_column_dot_and_magnitude(cn.F(), j, s).0);
        upd(&mut abs, (column_dot(problem, j, y) + z[j] + fdot).abs());
        upd(&mut abs, sign_violation(1.0, z[j], l[j], u[j]));
        if let Some(bound) = selected(1.0, z[j], l[j], u[j]) {
            d += z[j] * bound;
        }
    }
    for (i, &yi) in y.iter().enumerate() {
        let (lo, hi) = rows.bounds(i);
        upd(&mut abs, sign_violation(1.0, yi, lo, hi));
        if let Some(bound) = selected(1.0, yi, lo, hi) {
            d += yi * bound;
        }
    }
    let mut scale = norm_inf(y);
    if let Some(cn) = conic {
        upd(&mut abs, dual_cone_violation(cn, s, 1.0));
        d -= cn.g().iter().zip(s).map(|(a, b)| a * b).sum::<f64>();
        scale = scale.max(norm_inf(s));
    }
    let relative = if d > 0.0 {
        abs / (1.0 + scale)
    } else {
        f64::INFINITY
    };
    judged(names::FARKAS, abs, relative, tol, required)
}

fn ray_measure<I: RowIndex>(
    problem: &Problem<'_, I>,
    r: &[f64],
    tol: f64,
    required: bool,
) -> Measure {
    if r.len() != problem.n() {
        return malformed(names::RAY, tol, required);
    }
    let rows = problem.rows();
    let mut abs = 0.0;
    for (i, ari) in times(problem, r).into_iter().enumerate() {
        let (lo, hi) = rows.bounds(i);
        if lo.is_finite() {
            upd(&mut abs, -ari);
        }
        if hi.is_finite() {
            upd(&mut abs, ari);
        }
        if ari.is_nan() {
            upd(&mut abs, f64::NAN);
        }
    }
    for ((&rj, &lj), &uj) in r.iter().zip(problem.l()).zip(problem.u()) {
        if lj.is_finite() {
            upd(&mut abs, -rj);
        }
        if uj.is_finite() {
            upd(&mut abs, rj);
        }
        if rj.is_nan() {
            upd(&mut abs, f64::NAN);
        }
    }
    if let Some(cn) = problem.conic().filter(|c| c.k() > 0) {
        let fr = csc_times(cn.F(), r, cn.k());
        for (start, cone) in cn.blocks() {
            upd(&mut abs, cone.violation(&fr[start..start + cone.dim()]));
        }
    }
    if let Some(q) = problem.Q() {
        upd(&mut abs, norm_inf(&sym_times(q, r)));
    }
    let ctr: f64 = problem.c().iter().zip(r).map(|(&cj, &rj)| cj * rj).sum();
    let improving = problem.sense().sigma() * ctr < 0.0;
    let relative = if improving {
        abs / (1.0 + norm_inf(r))
    } else {
        f64::INFINITY
    };
    judged(names::RAY, abs, relative, tol, required)
}
