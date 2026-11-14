//! Solver-independent verification of feasibility and optimality (constitution XVI-D;
//! data-model.md, Verification). Implemented by task T027.

#![forbid(unsafe_code)]

use crate::options::defaults::VERIFICATION_FACTOR;
use crate::problem::{Problem, RowIndex};
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
    /// |p − d|.
    pub const GAP: &str = "primal-dual gap";
    /// Farkas certificate of infeasibility.
    pub const FARKAS: &str = "infeasibility certificate";
    /// Unbounded ray.
    pub const RAY: &str = "unbounded ray";
}

/// Verifies a solution against its problem, from the problem and the solution alone
/// (data-model.md, Verification). Ax and Aᵀy are computed here; nothing the solver reports
/// about residuals or row activity is used.
///
/// Which measures appear:
///
/// - row-bound and variable-bound violations when x is returned, or when the status is
///   Optimal or a solution is reported returned (then they are required);
/// - reduced-cost consistency, dual sign violation, complementary slackness and primal-dual
///   gap when the status is Optimal (required; NotEvaluated when x, y or z is absent). For
///   other statuses they are not part of what the status claims and are not listed;
/// - the infeasibility certificate when a Farkas certificate is returned, or when the status is
///   Infeasible (then required); the unbounded ray likewise with Unbounded.
///
/// An array of the wrong length makes the measures needing it Fail with infinite values; a
/// NaN anywhere in a measure makes it infinite, so it fails. The Farkas certificate is checked
/// with σ = +1 whatever the objective sense, since with c = 0 the sense plays no part; its
/// relative value is +∞ (Fail) unless d_F > 0. The ray's relative value is +∞ unless
/// σcᵀr < 0.
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
        Some(Certificate::Farkas { y, z }) => {
            measures.push(farkas_measure(problem, y, z, tolerances.primal, infeasible));
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

    let overall = if measures.iter().any(|q| q.verdict == Verdict::Fail) {
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
    let a = problem.A();
    let mut out = vec![0.0; problem.m()];
    for (j, &vj) in v.iter().enumerate().take(a.n()) {
        let (rows, vals) = a.column(j);
        for (&i, &aij) in rows.iter().zip(vals) {
            if let Some(o) = i.index().and_then(|i| out.get_mut(i)) {
                *o += aij * vj;
            }
        }
    }
    out
}

/// (Aᵀw)_j, computed from the problem's own A.
fn column_dot<I: RowIndex>(problem: &Problem<'_, I>, j: usize, w: &[f64]) -> f64 {
    let (rows, vals) = problem.A().column(j);
    rows.iter()
        .zip(vals)
        .map(|(&i, &aij)| i.index().and_then(|i| w.get(i)).map_or(0.0, |&wi| aij * wi))
        .sum()
}

/// Σ_i A_ij w_i and Σ_i |A_ij w_i| for column j.
fn column_dot_and_magnitude<I: RowIndex>(
    problem: &Problem<'_, I>,
    j: usize,
    w: &[f64],
) -> (f64, f64) {
    let (rows, vals) = problem.A().column(j);
    rows.iter()
        .zip(vals)
        .fold((0.0, 0.0), |(dot, mag), (&i, &aij)| {
            let t = i.index().and_then(|i| w.get(i)).map_or(0.0, |&wi| aij * wi);
            (dot + t, mag + t.abs())
        })
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
    let y = Input::of(solution.y, m);
    let z = Input::of(solution.z, n);
    let all = [
        names::REDUCED_COST,
        names::DUAL_SIGN,
        names::COMPLEMENTARY_SLACKNESS,
        names::GAP,
    ];

    let (y, z) = match (y, z) {
        (Input::Ok(y), Input::Ok(z)) => (y, z),
        (Input::WrongLength, _) | (_, Input::WrongLength) => {
            return all.iter().map(|&name| malformed(name, tol, true)).collect();
        }
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

    // Reduced-cost consistency and dual sign (need y and z only).
    // Reduced-cost consistency is scaled per column by the magnitude of the terms actually
    // summed, 1 + |c_j| + Σ_i |A_ij y_i| (data-model.md; maintainer decision 2026-11-14): with
    // large duals, floating-point cancellation alone leaves residuals far above 1 + ‖c‖∞.
    let mut rc = 0.0;
    let mut rc_rel = 0.0;
    let mut sign = 0.0;
    for j in 0..n {
        let (dot, magnitude) = column_dot_and_magnitude(problem, j, y);
        let residual = (z[j] - (c[j] - dot)).abs();
        upd(&mut rc, residual);
        upd(&mut rc_rel, residual / (1.0 + c[j].abs() + magnitude));
        upd(&mut sign, sign_violation(sigma, z[j], l[j], u[j]));
    }
    for (i, &yi) in y.iter().enumerate() {
        let (lo, hi) = rows.bounds(i);
        upd(&mut sign, sign_violation(sigma, yi, lo, hi));
    }
    let mut out = vec![
        judged(names::REDUCED_COST, rc, rc_rel, tol, true),
        judged(names::DUAL_SIGN, sign, sign / c_scale, tol, true),
    ];

    // Complementary slackness and gap (need x as well).
    let (x, ax) = match (x, ax) {
        (Input::Ok(x), Some(ax)) => (x, ax),
        (Input::Missing, _) => {
            out.push(not_evaluated(names::COMPLEMENTARY_SLACKNESS, tol, true));
            out.push(not_evaluated(names::GAP, tol, true));
            return out;
        }
        _ => {
            out.push(malformed(names::COMPLEMENTARY_SLACKNESS, tol, true));
            out.push(malformed(names::GAP, tol, true));
            return out;
        }
    };
    let gamma = problem.gamma();
    let p = c.iter().zip(x).map(|(&cj, &xj)| cj * xj).sum::<f64>() + gamma;
    let mut d = gamma;
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
        if let Some(bound) = selected(sigma, z[j], l[j], u[j]) {
            d += z[j] * bound;
            upd(&mut cs, z[j].abs() * (x[j] - bound).abs());
        } else if z[j].is_nan() {
            d = f64::NAN;
            cs = f64::INFINITY;
        }
    }
    let gap = (p - d).abs();
    let gap = if gap.is_nan() { f64::INFINITY } else { gap };
    out.push(judged(
        names::COMPLEMENTARY_SLACKNESS,
        cs,
        cs / (1.0 + p.abs()),
        tol,
        true,
    ));
    out.push(judged(
        names::GAP,
        gap,
        gap / (1.0 + p.abs() + d.abs()),
        tol,
        true,
    ));
    out
}

fn farkas_measure<I: RowIndex>(
    problem: &Problem<'_, I>,
    y: &[f64],
    z: &[f64],
    tol: f64,
    required: bool,
) -> Measure {
    let (m, n) = (problem.m(), problem.n());
    if y.len() != m || z.len() != n {
        return malformed(names::FARKAS, tol, required);
    }
    // σ = +1: the certificate concerns the feasible set only (c = 0).
    let rows = problem.rows();
    let (l, u) = (problem.l(), problem.u());
    let mut abs = 0.0;
    let mut d = 0.0;
    for j in 0..n {
        upd(&mut abs, (column_dot(problem, j, y) + z[j]).abs());
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
    let relative = if d > 0.0 {
        abs / (1.0 + norm_inf(y))
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
    let ctr: f64 = problem.c().iter().zip(r).map(|(&cj, &rj)| cj * rj).sum();
    let improving = problem.sense().sigma() * ctr < 0.0;
    let relative = if improving {
        abs / (1.0 + norm_inf(r))
    } else {
        f64::INFINITY
    };
    judged(names::RAY, abs, relative, tol, required)
}
