//! Feature 002 per-backend tests (tasks T014, T015, T019, T029, T033, T034), written once
//! against `optsolver::Backend` and registered for every declared backend (constitution II).
//! Nothing here names a solver.

#![allow(non_snake_case)] // Notation Convention (XII-C): A, Q, F are matrices.

use super::us3::{
    LARGEST_INSTANCE, SmallFixture, corpus_with_seed, describe, expect_overall, long_solve,
};
use super::{Failed, ensure, fail};
use optsolver::{Backend, Certificate, Method, Options, Overall, Progress, Status};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::ThreadId;

// ---------------------------------------------------------------------------------------------
// User Story 2: contract behaviour verified on every backend

/// T014 (spec FR-004; data-model.md CHK010): "minimise x₁ + x₂ subject to x₁ + x₂ ≥ 3,
/// 0 ≤ x ≤ 1" has, up to positive scaling, the unique Farkas certificate y_F = (1),
/// z_F = (−1, −1) under the sense-independent convention d_F > 0. The test asserts these
/// signs, not only that verification passes.
pub fn t014_certificate_sign(b: &dyn Backend) -> Result<(), Failed> {
    let fx = SmallFixture::load("infeasible")?;
    let p = fx.problem.problem(&fx.problem.c).map_err(fail)?;
    let mut o = Options::default();
    o.certificates = true;
    let sol = b.solve((&p).into(), &mut o).map_err(fail)?;
    ensure(
        sol.status == Status::Infeasible,
        format!("expected Infeasible; {}", describe(&sol)),
    )?;
    let Some(Certificate::Farkas { y, z, s }) = &sol.certificate else {
        return Err(fail(format!(
            "expected a Farkas certificate, got {:?}",
            sol.certificate
        )));
    };
    ensure(
        y.len() == 1 && z.len() == 2 && s.is_empty(),
        format!(
            "certificate lengths y {}, z {}, s {}",
            y.len(),
            z.len(),
            s.len()
        ),
    )?;
    ensure(y[0] > 0.0, format!("y_F,1 = {} must be positive", y[0]))?;
    ensure(
        z[0] < 0.0 && z[1] < 0.0,
        format!("z_F = {z:?} must be negative (upper bounds active)"),
    )?;
    // Up to scaling, z_F = −y_F (Aᵀy_F + z_F = 0 with A = [1 1]).
    ensure(
        (z[0] + y[0]).abs() <= 1e-9 * y[0] && (z[1] + y[0]).abs() <= 1e-9 * y[0],
        format!("z_F = {z:?} is not −y_F = {:?}", -y[0]),
    )?;
    expect_overall(&sol, Overall::Verified, "certificate")
}

/// T015 (spec FR-005; data-model.md CHK011): with four solver threads and the barrier
/// algorithm on the largest LP, every progress call runs on the thread that called `solve`
/// and no two calls overlap (an in-flight counter never exceeds one); at least one call
/// occurs.
pub fn t015_callback_thread(b: &dyn Backend) -> Result<(), Failed> {
    let (f, c) = long_solve()?;
    let p = f.problem.problem(&c).map_err(fail)?;
    let caller = std::thread::current().id();
    let ids: Mutex<Vec<ThreadId>> = Mutex::new(Vec::new());
    let in_flight = AtomicUsize::new(0);
    let max_in_flight = AtomicUsize::new(0);
    let mut on_progress = |_: &Progress| {
        let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        max_in_flight.fetch_max(now, Ordering::SeqCst);
        ids.lock()
            .expect("unpoisoned")
            .push(std::thread::current().id());
        // Widen the window in which an overlapping call would be seen.
        std::thread::sleep(std::time::Duration::from_micros(200));
        in_flight.fetch_sub(1, Ordering::SeqCst);
    };
    let mut o = Options::default();
    o.threads = Some(4);
    o.method = Some(Method::Barrier);
    o.progress = Some(&mut on_progress);
    let sol = b.solve((&p).into(), &mut o).map_err(fail)?;
    drop(o);
    let ids = ids.into_inner().expect("unpoisoned");
    ensure(
        !ids.is_empty(),
        format!("no progress call on {LARGEST_INSTANCE}; {}", describe(&sol)),
    )?;
    ensure(
        ids.iter().all(|&id| id == caller),
        format!(
            "{} of {} progress calls ran on another thread",
            ids.iter().filter(|&&id| id != caller).count(),
            ids.len()
        ),
    )?;
    let max = max_in_flight.load(Ordering::SeqCst);
    ensure(max == 1, format!("{max} progress calls overlapped"))?;
    ensure(
        sol.status == Status::Optimal,
        format!("expected Optimal; {}", describe(&sol)),
    )
}

// ---------------------------------------------------------------------------------------------
// User Story 3: cross-solver agreement (T019, spec SC-001)

/// Effective dual (optimality) tolerance a result was verified against: the tolerance of its
/// primal-dual gap measure (research R10).
fn dual_tolerance(sol: &optsolver::Solution) -> Result<f64, Failed> {
    sol.verification
        .as_ref()
        .and_then(|r| {
            r.measures
                .iter()
                .find(|q| q.name == optsolver::verify::names::GAP)
        })
        .map(|q| q.tolerance)
        .ok_or_else(|| fail(format!("no gap measure; {}", describe(sol))))
}

/// T019: on iDopaNeuroC_problem_LP with the objectives of seeds 1, 2 and 3, both backends
/// reach Optimal with a Verified report and objectives agreeing within
/// 10 · max(τ₁, τ₂) · (1 + max(|p₁|, |p₂|)), τ the effective dual tolerance (research R10).
pub fn t019_lp_agreement(a: &dyn Backend, b: &dyn Backend) -> Result<(), Failed> {
    for s in 0..3 {
        let (f, seed) = corpus_with_seed(LARGEST_INSTANCE, s)?;
        let c = super::objectives::seeded_objective(seed, f.problem.n);
        let p = f.problem.problem(&c).map_err(fail)?;
        let mut results = Vec::new();
        for backend in [a, b] {
            let sol = backend
                .solve((&p).into(), &mut Options::default())
                .map_err(fail)?;
            let name = backend.capabilities().backend;
            super::us3::expect_optimal_verified(&sol, f.problem.n, &format!("{name} seed {seed}"))?;
            let objective = sol
                .objective
                .ok_or_else(|| fail(format!("{name} seed {seed}: no objective")))?;
            results.push((name, objective, dual_tolerance(&sol)?));
        }
        let ((n1, p1, t1), (n2, p2, t2)) = (results[0], results[1]);
        let allowed = 10.0 * t1.max(t2) * (1.0 + p1.abs().max(p2.abs()));
        ensure(
            (p1 - p2).abs() <= allowed,
            format!(
                "seed {seed}: {n1} {p1:e} and {n2} {p2:e} differ by {:e} > {allowed:e}",
                (p1 - p2).abs()
            ),
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// User Story 4: QP (T029)

/// Unsupported error expected from a backend without `class`.
fn expect_unsupported(
    r: optsolver::error::Result<optsolver::Solution>,
    field: &str,
) -> Result<(), Failed> {
    match r {
        Err(optsolver::Error::Unsupported { field: f, .. }) if f == field => Ok(()),
        other => Err(fail(format!(
            "expected Unsupported {field}, got {:?}",
            other.map(|s| describe(&s))
        ))),
    }
}

fn supports(b: &dyn Backend, class: optsolver::ProblemClass) -> bool {
    b.capabilities().problem_classes.contains(&class)
}

/// T029: the derived QP iDopaNeuroC_problem_LP_minNorm (research R12): minimise
/// ½ · 10⁻⁴ · ‖x‖² over the LP's feasible set. Optimal, Verified, objective within
/// 10 · τ · (1 + |p_ref|) of one of the recorded reference objectives. A backend without QP
/// reports Unsupported.
pub fn t029_corpus_qp(b: &dyn Backend) -> Result<(), Failed> {
    use super::fixtures::Json;
    let name = DERIVED_QP;
    let (prov, n, r) = solve_derived_qp(b)?;
    if !supports(b, optsolver::ProblemClass::Qp) {
        return expect_unsupported(r, "Q");
    }
    let sol = r.map_err(fail)?;
    super::us3::expect_optimal_verified(&sol, n, name)?;
    let obj = sol.objective.ok_or_else(|| fail("no objective"))?;
    let tau = dual_tolerance(&sol)?;
    let refs: Vec<f64> = match prov.fields.get("reference_objectives") {
        Some(Json::Object(m)) => m
            .values()
            .filter_map(|v| match v {
                Json::Number(x) => Some(*x),
                _ => None,
            })
            .collect(),
        other => return Err(fail(format!("reference_objectives is {other:?}"))),
    };
    ensure(
        refs.iter()
            .any(|r| (obj - r).abs() <= 10.0 * tau * (1.0 + r.abs())),
        format!("objective {obj:e} not within 10·{tau:e}·(1+|p_ref|) of {refs:?}"),
    )
}

/// Name of the derived QP fixture (feature 002 research R12).
pub const DERIVED_QP: &str = "iDopaNeuroC_problem_LP_minNorm";

/// Builds the derived QP (minimise ½·q·‖x‖² over the LP fixture's feasible set, research R12
/// of feature 002) and solves it on `b` with default options; returns its provenance record,
/// n and the solve result (shared with the feature 003 comparison).
pub fn solve_derived_qp(
    b: &dyn Backend,
) -> Result<
    (
        super::fixtures::Provenance,
        usize,
        optsolver::error::Result<optsolver::Solution>,
    ),
    Failed,
> {
    use super::fixtures::{Json, Provenance, qp_provenance_path};
    let prov = Provenance::load(&qp_provenance_path(DERIVED_QP).map_err(fail)?).map_err(fail)?;
    let base = prov.str("derived_from").map_err(fail)?.to_string();
    let f = super::fixtures::load_corpus_fixture(&base).map_err(fail)?;
    ensure(
        prov.str("derived_from_mps_sha256").map_err(fail)?
            == f.provenance.str("mps_sha256").map_err(fail)?,
        "the QP's base LP fixture changed since its provenance record",
    )?;
    let q_diag = match prov.fields.get("q_diagonal") {
        Some(Json::Number(v)) => *v,
        other => return Err(fail(format!("q_diagonal is {other:?}"))),
    };
    let n = f.problem.n;
    let c = vec![0.0; n];
    let q_cp: Vec<usize> = (0..=n).collect();
    let q_ri: Vec<usize> = (0..n).collect();
    let q_va = vec![q_diag; n];
    let q = optsolver::CscRef::new(n, n, &q_cp, &q_ri, &q_va).map_err(fail)?;
    let mp = &f.problem;
    let p = optsolver::Problem::new(
        mp.A().map_err(fail)?,
        &c,
        optsolver::Sense::Minimise,
        optsolver::RowBounds::SenseRhs {
            sense: &mp.sense,
            b: &mp.b,
        },
        &mp.l,
        &mp.u,
    )
    .and_then(|p| p.with_Q(q))
    .map_err(fail)?;
    let r = b.solve((&p).into(), &mut Options::default());
    Ok((prov, n, r))
}

/// minimise or maximise ½ xᵀQx + cᵀx subject to x₁ + x₂ = 2, x ≥ 0 (Q by lower triangle).
#[allow(clippy::too_many_arguments)] // One closed-form case, spelled out at the call.
fn small_qp(
    b: &dyn Backend,
    q_cp: &[usize],
    q_ri: &[usize],
    q_va: &[f64],
    c: &[f64],
    sense: optsolver::Sense,
    x_star: [f64; 2],
    p_star: f64,
    what: &str,
) -> Result<(), Failed> {
    let a = optsolver::CscRef::new(1, 2, &[0, 1, 2], &[0usize, 0], &[1.0, 1.0]).map_err(fail)?;
    let q = optsolver::CscRef::new(2, 2, q_cp, q_ri, q_va).map_err(fail)?;
    let p = optsolver::Problem::new(
        a,
        c,
        sense,
        optsolver::RowBounds::SenseRhs {
            sense: b"=",
            b: &[2.0],
        },
        &[0.0, 0.0],
        &[f64::INFINITY, f64::INFINITY],
    )
    .and_then(|p| p.with_Q(q))
    .map_err(fail)?;
    if !supports(b, optsolver::ProblemClass::Qp) {
        return expect_unsupported(b.solve((&p).into(), &mut Options::default()), "Q");
    }
    let sol = b
        .solve((&p).into(), &mut Options::default())
        .map_err(fail)?;
    super::us3::expect_optimal_verified(&sol, 2, what)?;
    let obj = sol.objective.ok_or_else(|| fail("no objective"))?;
    ensure(
        (obj - p_star).abs() <= 1e-6 * (1.0 + p_star.abs())
            && sol.x.iter().zip(x_star).all(|(a, b)| (a - b).abs() <= 1e-5),
        format!(
            "{what}: x {:?}, objective {obj}; expected {x_star:?}, {p_star}",
            sol.x
        ),
    )
}

/// T029: two hand-built QPs with closed-form optima (Q = [[2,1],[1,2]] with an off-diagonal
/// entry: x = (1, 1), p = 3; and maximise −½‖x‖² − x₁: x = (½, 3/2), p = −7/4).
pub fn t029_small_qps(b: &dyn Backend) -> Result<(), Failed> {
    small_qp(
        b,
        &[0, 2, 3],
        &[0, 1, 1],
        &[2.0, 1.0, 2.0],
        &[0.0, 0.0],
        optsolver::Sense::Minimise,
        [1.0, 1.0],
        3.0,
        "off-diagonal QP",
    )?;
    small_qp(
        b,
        &[0, 1, 2],
        &[0, 1],
        &[-1.0, -1.0],
        &[-1.0, 0.0],
        optsolver::Sense::Maximise,
        [0.5, 1.5],
        -1.75,
        "maximise QP",
    )
}

// ---------------------------------------------------------------------------------------------
// User Story 5: hand-built conic problems (T034)

/// One hand-built conic problem: n variables, F = diag(f), g, one cone covering all n rows,
/// linear rows fixing x_i = v for each (i, v) in `fixed`, objective c, free bounds except
/// `upper`.
struct ConicCase {
    name: &'static str,
    cone: optsolver::Cone,
    f: Vec<f64>,
    g: Vec<f64>,
    fixed: Vec<(usize, f64)>,
    c: Vec<f64>,
    upper: Vec<(usize, f64)>,
    optimum: f64,
}

fn conic_cases() -> Vec<ConicCase> {
    use optsolver::Cone;
    let e = std::f64::consts::E;
    let r2 = std::f64::consts::SQRT_2;
    let case = |name, cone, f: Vec<f64>, fixed, c, optimum| ConicCase {
        name,
        cone,
        g: vec![0.0; f.len()],
        f,
        fixed,
        c,
        upper: vec![],
        optimum,
    };
    vec![
        // x0 ≥ ‖(3, 4)‖ = 5.
        case(
            "quadratic",
            Cone::Quadratic(3),
            vec![1.0; 3],
            vec![(1, 3.0), (2, 4.0)],
            vec![1.0, 0.0, 0.0],
            5.0,
        ),
        // 2 x0 · 1 ≥ 2² gives x0 ≥ 2.
        case(
            "rotated quadratic",
            Cone::RotatedQuadratic(3),
            vec![1.0; 3],
            vec![(1, 1.0), (2, 2.0)],
            vec![1.0, 0.0, 0.0],
            2.0,
        ),
        // x0 ≥ 1 · exp(1/1) = e.
        case(
            "primal exponential",
            Cone::PrimalExponential,
            vec![1.0; 3],
            vec![(1, 1.0), (2, 1.0)],
            vec![1.0, 0.0, 0.0],
            e,
        ),
        // x0 ≥ −(−1) · exp(0/(−1) − 1) = 1/e.
        case(
            "dual exponential",
            Cone::DualExponential,
            vec![1.0; 3],
            vec![(1, 0.0), (2, -1.0)],
            vec![1.0, 0.0, 0.0],
            1.0 / e,
        ),
        // √(x0 · 4) ≥ 2 gives x0 ≥ 1.
        case(
            "primal power",
            Cone::PrimalPower {
                dim: 3,
                alpha: vec![1.0, 1.0],
            },
            vec![1.0; 3],
            vec![(1, 4.0), (2, 2.0)],
            vec![1.0, 0.0, 0.0],
            1.0,
        ),
        // (x0/½)^½ (1/½)^½ = 2√x0 ≥ 4 gives x0 ≥ 4.
        case(
            "dual power",
            Cone::DualPower {
                dim: 3,
                alpha: vec![1.0, 1.0],
            },
            vec![1.0; 3],
            vec![(1, 1.0), (2, 4.0)],
            vec![1.0, 0.0, 0.0],
            4.0,
        ),
        // x ≥ 0 with x0 − x1 fixed through x1 = 0... : x1 = 2, minimise x0 + x1 with x0 ≥ 0.
        case(
            "nonnegative",
            Cone::Nonnegative(2),
            vec![1.0; 2],
            vec![(1, 2.0)],
            vec![1.0, 1.0],
            2.0,
        ),
        // svec [[x0, x1], [x1, x2]] = (x0, √2 x1, x2) ⪰ 0 with x1 = 1: x0 x2 ≥ 1, min x0 + x2 = 2.
        case(
            "semidefinite order 2",
            Cone::PositiveSemidefinite { order: 2 },
            vec![1.0, r2, 1.0],
            vec![(1, 1.0)],
            vec![1.0, 0.0, 1.0],
            2.0,
        ),
        // svec of [[x0, x1, x2], [x1, x3, x4], [x2, x4, x5]] with x1 = 1, x2 = x4 = 0:
        // min x0 + x3 + x5 = 2 (x0 = x3 = 1, x5 = 0).
        case(
            "semidefinite order 3",
            Cone::PositiveSemidefinite { order: 3 },
            vec![1.0, r2, r2, 1.0, r2, 1.0],
            vec![(1, 1.0), (2, 0.0), (4, 0.0)],
            vec![1.0, 0.0, 0.0, 1.0, 0.0, 1.0],
            2.0,
        ),
        // (x0 − 1, x1 − 2) = 0: min x0 + x1 = 3.
        ConicCase {
            name: "zero",
            cone: Cone::Zero(2),
            f: vec![1.0; 2],
            g: vec![-1.0, -2.0],
            fixed: vec![],
            c: vec![1.0, 1.0],
            upper: vec![],
            optimum: 3.0,
        },
    ]
}

/// Builds and solves `case` with objective sense `sense` (c negated for maximise); returns
/// the solution.
fn solve_conic_case(
    b: &dyn Backend,
    case: &ConicCase,
    sense: optsolver::Sense,
    certificates: bool,
) -> Result<optsolver::error::Result<optsolver::Solution>, Failed> {
    let n = case.f.len();
    let m = case.fixed.len();
    // A: row r has a single 1 in column fixed[r].0.
    let mut by_col: Vec<Option<usize>> = vec![None; n];
    for (r, &(j, _)) in case.fixed.iter().enumerate() {
        by_col[j] = Some(r);
    }
    let mut cp = vec![0usize];
    let mut ri = Vec::new();
    for row in &by_col {
        if let Some(r) = *row {
            ri.push(r);
        }
        cp.push(ri.len());
    }
    let va = vec![1.0; ri.len()];
    let bvals: Vec<f64> = case.fixed.iter().map(|&(_, v)| v).collect();
    let rsense = vec![b'='; m];
    let a = optsolver::CscRef::new(m, n, &cp, &ri, &va).map_err(fail)?;
    let f_cp: Vec<usize> = (0..=n).collect();
    let f_ri: Vec<usize> = (0..n).collect();
    let fm = optsolver::CscRef::new(n, n, &f_cp, &f_ri, &case.f).map_err(fail)?;
    let cones = [case.cone.clone()];
    let conic = optsolver::ConicRef::new(fm, &case.g, &cones).map_err(fail)?;
    let c: Vec<f64> = match sense {
        optsolver::Sense::Minimise => case.c.clone(),
        optsolver::Sense::Maximise => case.c.iter().map(|v| -v).collect(),
    };
    let l = vec![f64::NEG_INFINITY; n];
    let mut u = vec![f64::INFINITY; n];
    for &(j, v) in &case.upper {
        u[j] = v;
    }
    let p = optsolver::Problem::new(
        a,
        &c,
        sense,
        optsolver::RowBounds::SenseRhs {
            sense: &rsense,
            b: &bvals,
        },
        &l,
        &u,
    )
    .and_then(|p| p.with_conic(conic))
    .map_err(fail)?;
    let mut o = Options::default();
    o.certificates = certificates;
    Ok(b.solve((&p).into(), &mut o))
}

/// T034: every cone type, minimise and maximise: Optimal, objective ±p*, Verified (which
/// checks the conic duals' sign convention against data-model.md). Unsupported without the
/// conic class.
pub fn t034_conic_cases(b: &dyn Backend) -> Result<(), Failed> {
    let caps = b.capabilities();
    let conic = caps
        .problem_classes
        .contains(&optsolver::ProblemClass::Conic);
    for case in conic_cases() {
        let kind = case.cone.kind();
        for sense in [optsolver::Sense::Minimise, optsolver::Sense::Maximise] {
            let r = solve_conic_case(b, &case, sense, false)?;
            if !conic {
                expect_unsupported(r, "conic")?;
                continue;
            }
            // Per-kind capability (feature 003): an undeclared kind is rejected by name.
            if !caps.cones.contains(&kind) {
                expect_unsupported(r, kind.name())?;
                continue;
            }
            let sol = r.map_err(fail)?;
            let what = format!("{} {sense:?}", case.name);
            super::us3::expect_optimal_verified(&sol, case.f.len(), &what)?;
            let expected = if sense == optsolver::Sense::Minimise {
                case.optimum
            } else {
                -case.optimum
            };
            let obj = sol
                .objective
                .ok_or_else(|| fail(format!("{what}: no objective")))?;
            ensure(
                (obj - expected).abs() <= 1e-6 * (1.0 + expected.abs()),
                format!("{what}: objective {obj}, expected {expected}"),
            )?;
            ensure(
                sol.s.len() == case.f.len(),
                format!("{what}: |s| = {}", sol.s.len()),
            )?;
        }
    }
    Ok(())
}

/// T034: x0 ≤ 4 with (x0, 3, 4) in the quadratic cone is infeasible; with certificates the
/// result is Infeasible, and a backend that returns a certificate returns a Farkas certificate
/// including s that verifies. A backend that documents none for conic problems (Gurobi,
/// feature 003 research R4) still reports Infeasible; its report is then PartiallyVerified.
pub fn t034_conic_infeasible(b: &dyn Backend) -> Result<(), Failed> {
    let case = ConicCase {
        name: "infeasible quadratic",
        cone: optsolver::Cone::Quadratic(3),
        f: vec![1.0; 3],
        g: vec![0.0; 3],
        fixed: vec![(1, 3.0), (2, 4.0)],
        c: vec![1.0, 0.0, 0.0],
        upper: vec![(0, 4.0)],
        optimum: f64::NAN,
    };
    for sense in [optsolver::Sense::Minimise, optsolver::Sense::Maximise] {
        let r = solve_conic_case(b, &case, sense, true)?;
        if !supports(b, optsolver::ProblemClass::Conic) {
            return expect_unsupported(r, "conic");
        }
        let sol = r.map_err(fail)?;
        ensure(
            sol.status == Status::Infeasible,
            format!("{sense:?}: expected Infeasible; {}", describe(&sol)),
        )?;
        match &sol.certificate {
            Some(Certificate::Farkas { s, .. }) if s.len() == 3 => expect_overall(
                &sol,
                Overall::Verified,
                &format!("{sense:?} conic certificate"),
            )?,
            None => expect_overall(
                &sol,
                Overall::PartiallyVerified,
                &format!("{sense:?} conic infeasibility without a certificate"),
            )?,
            other => return Err(fail(format!("{sense:?}: certificate {other:?}"))),
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// User Story 5: variational kinetics CLP fixtures (T033)

/// The field `b` reports as unsupported for a conic problem with `cones`, if any: `"conic"`
/// without the class, else the name of the first undeclared cone kind (feature 003).
fn conic_unsupported(b: &dyn Backend, cones: &[optsolver::Cone]) -> Option<&'static str> {
    let caps = b.capabilities();
    if !caps
        .problem_classes
        .contains(&optsolver::ProblemClass::Conic)
    {
        return Some("conic");
    }
    cones
        .iter()
        .map(|c| c.kind())
        .find(|k| !caps.cones.contains(k))
        .map(|k| k.name())
}

/// Options for CLP fixture solves: solver defaults plus one thread, for determinism (plan,
/// Solver configuration audit).
fn clp_options() -> Options<'static> {
    let mut o = Options::default();
    o.threads = Some(1);
    o
}

/// T033: the CLP fixture `instance` reaches its expected status (provenance `expected_stat`;
/// COBRA's 1 = optimal) and, when optimal, verifies, with the objective within
/// 10 · max(τ, τ_ref) · max(1, min(|p|, |p_ref|)) of the variational kinetics objective. A backend without the conic
/// class reports Unsupported.
pub fn t033_clp_fixture(b: &dyn Backend, instance: &str) -> Result<(), Failed> {
    use super::fixtures::{Json, Provenance, clp_provenance_path};
    let prov = Provenance::load(&clp_provenance_path(instance).map_err(fail)?).map_err(fail)?;
    let jt = super::jtask::read_jtask_file(&super::jtask::jtask_path(instance).map_err(fail)?)
        .map_err(fail)?;
    let p = jt.problem().map_err(fail)?;
    if let Some(field) = conic_unsupported(b, &jt.cones) {
        return expect_unsupported(b.solve((&p).into(), &mut clp_options()), field);
    }
    let stat = match prov.fields.get("expected_stat") {
        Some(Json::Number(v)) => *v,
        other => return Err(fail(format!("{instance}: expected_stat is {other:?}"))),
    };
    ensure(
        stat == 1.0,
        format!("{instance}: only optimal fixtures are declared, got stat {stat}"),
    )?;
    let sol = b.solve((&p).into(), &mut clp_options()).map_err(fail)?;
    if super::fixtures::clp_expected_verification_failures()
        .map_err(fail)?
        .iter()
        .any(|n| n == instance)
    {
        // Declared known finding (research R19): the solver reports Optimal, and the
        // verifier must catch the row-bound violation, and only that.
        let failing: Vec<&str> = sol
            .verification
            .iter()
            .flat_map(|r| r.measures.iter())
            .filter(|q| q.verdict == optsolver::Verdict::Fail && q.required)
            .map(|q| q.name)
            .collect();
        return ensure(
            sol.status == Status::Optimal
                && failing == [optsolver::verify::names::ROW_BOUND]
                && sol.verification.as_ref().map(|r| r.overall) == Some(Overall::Failed),
            format!(
                "{instance}: expected Optimal with only the row-bound violation failing; {}",
                describe(&sol)
            ),
        );
    }
    super::us3::expect_optimal_verified(&sol, jt.n, instance)?;
    ensure(
        sol.s.len() == jt.k,
        format!("{instance}: |s| = {}, k = {}", sol.s.len(), jt.k),
    )?;
    let reference = match prov.fields.get("expected_objective") {
        Some(Json::Number(v)) => *v,
        other => return Err(fail(format!("{instance}: expected_objective is {other:?}"))),
    };
    let obj = sol.objective.ok_or_else(|| fail("no objective"))?;
    // Research R10 with the larger of the two runs' tolerances: ours, and the relative-gap
    // tolerance of the variational kinetics run recorded in provenance (its solve was looser),
    // normalised as MOSEK does, max(1, min(|p|, |p_ref|)).
    let tau = dual_tolerance(&sol)?;
    let tau_ref = match prov.fields.get("mosek_parameters") {
        Some(Json::Object(m)) => match m.get("MSK_DPAR_INTPNT_CO_TOL_REL_GAP") {
            Some(Json::Number(v)) => *v,
            _ => tau,
        },
        _ => tau,
    };
    let allowed = 10.0 * tau.max(tau_ref) * 1f64.max(obj.abs().min(reference.abs()));
    ensure(
        (obj - reference).abs() <= allowed,
        format!(
            "{instance}: objective {obj:e}, variational kinetics {reference:e}, allowed {allowed:e}"
        ),
    )
}

/// T033: `solve_into` with caller buffers for x, y, z, s and row activity on the first
/// licensed CLP fixture gives exactly the arrays `solve` returns (one thread).
pub fn t033_clp_solve_into(b: &dyn Backend) -> Result<(), Failed> {
    let instance = super::fixtures::clp_instances()
        .map_err(fail)?
        .into_iter()
        .next()
        .ok_or_else(|| fail("no CLP instance declared"))?;
    let jt = super::jtask::read_jtask_file(&super::jtask::jtask_path(&instance).map_err(fail)?)
        .map_err(fail)?;
    let p = jt.problem().map_err(fail)?;
    if let Some(field) = conic_unsupported(b, &jt.cones) {
        return expect_unsupported(b.solve((&p).into(), &mut clp_options()), field);
    }
    let owned = b.solve((&p).into(), &mut clp_options()).map_err(fail)?;
    let (mut x, mut y, mut z, mut s, mut ra) = (
        vec![f64::NAN; jt.n],
        vec![f64::NAN; jt.m],
        vec![f64::NAN; jt.n],
        vec![f64::NAN; jt.k],
        vec![f64::NAN; jt.m],
    );
    let out = optsolver::SolutionBuffers {
        x: &mut x,
        y: Some(&mut y),
        z: Some(&mut z),
        row_activity: Some(&mut ra),
        s: Some(&mut s),
        z_l: None,
        z_u: None,
    };
    let into = b
        .solve_into((&p).into(), &mut clp_options(), out)
        .map_err(fail)?;
    ensure(
        into.status == owned.status && into.x.is_empty() && into.s.is_empty(),
        format!("solve_into: {}", describe(&into)),
    )?;
    for (what, a, b) in [
        ("x", &x, &owned.x),
        ("y", &y, &owned.y),
        ("z", &z, &owned.z),
        ("s", &s, &owned.s),
        ("row activity", &ra, &owned.row_activity),
    ] {
        ensure(
            a.iter()
                .map(|v| v.to_bits())
                .eq(b.iter().map(|v| v.to_bits())),
            format!("{instance}: {what} from solve_into differs from solve"),
        )?;
    }
    Ok(())
}

/// Feature 003 T018: a non-convex Q (minimise −x₁² over 0 ≤ x ≤ 1, one row x₁ + x₂ ≤ 2) is an
/// `Error::Solver` from every backend with the QP class (Gurobi 10020 with `NonConvex` = 0,
/// MOSEK 1295), never a silent change of problem class.
pub fn t018_nonconvex_q_is_an_error(b: &dyn Backend) -> Result<(), Failed> {
    let a = optsolver::CscRef::new(1, 2, &[0, 1, 2], &[0usize, 0], &[1.0, 1.0]).map_err(fail)?;
    let q = optsolver::CscRef::new(2, 2, &[0, 1, 1], &[0usize], &[-1.0]).map_err(fail)?;
    let p = optsolver::Problem::new(
        a,
        &[0.0, 0.0],
        optsolver::Sense::Minimise,
        optsolver::RowBounds::SenseRhs {
            sense: b"<",
            b: &[2.0],
        },
        &[0.0, 0.0],
        &[1.0, 1.0],
    )
    .and_then(|p| p.with_Q(q))
    .map_err(fail)?;
    let r = b.solve((&p).into(), &mut Options::default());
    if !supports(b, optsolver::ProblemClass::Qp) {
        return expect_unsupported(r, "Q");
    }
    match r {
        Err(optsolver::Error::Solver { code, message }) => ensure(
            code != 0 && !message.is_empty(),
            format!("vendor code and message: {code} {message:?}"),
        ),
        other => Err(fail(format!(
            "non-convex Q: expected Error::Solver, got {:?}",
            other.map(|s| describe(&s))
        ))),
    }
}
