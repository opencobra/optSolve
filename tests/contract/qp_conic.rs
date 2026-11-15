//! Feature 002 contract tests without a solver: validation of Q and conic data (T003) and
//! the QP and conic verification measures (T004), on hand-built problems with closed-form
//! optima (data-model.md of feature 002).

use super::{TOL, check_measure, expect_invalid, measure};
use crate::common::fake_backend::FakeBackend;
use crate::common::{Failed, Trial, ensure, fail};
use optsolver::backend::ProblemClass;
use optsolver::verify::names;
use optsolver::{
    Backend, Certificate, Cone, ConeKind, ConicRef, CscRef, Error, Options, Overall, Problem,
    RowBounds, Sense, SolutionReturned, SolutionView, Status, Verdict, VerificationReport, verify,
};

const INF: f64 = f64::INFINITY;
const SQRT2: f64 = std::f64::consts::SQRT_2;

type TestFn = fn() -> Result<(), Failed>;

pub fn trials() -> Vec<Trial> {
    let tests: Vec<(&str, TestFn)> = vec![
        // T003: validation.
        (
            "t003_cone_dimensions_sum_to_k",
            t003_cone_dimensions_sum_to_k,
        ),
        ("t003_cone_minimum_dimensions", t003_cone_minimum_dimensions),
        ("t003_power_cone_exponents", t003_power_cone_exponents),
        ("t003_psd_dimension", t003_psd_dimension),
        ("t003_F_rows_and_g", t003_f_rows_and_g),
        ("t003_with_conic_columns", t003_with_conic_columns),
        ("t003_with_Q_validation", t003_with_q_validation),
        ("t003_check_problem_classes", t003_check_problem_classes),
        // T004: verification.
        ("t004_qp_diagonal_verified", t004_qp_diagonal_verified),
        ("t004_qp_off_diagonal_lower_triangle", t004_qp_off_diagonal),
        ("t004_qp_maximise_verified", t004_qp_maximise),
        ("t004_qp_without_x_not_evaluated", t004_qp_without_x),
        ("t004_cone_violations_per_type", t004_cone_violations),
        ("t004_dual_cones", t004_dual_cones),
        ("t004_conic_minimise_verified", t004_conic_minimise),
        ("t004_conic_maximise_verified", t004_conic_maximise),
        (
            "t004_conic_wrong_dual_sign_fails",
            t004_conic_wrong_dual_sign,
        ),
        ("t004_conic_primal_violation", t004_conic_primal_violation),
        ("t004_conic_complementarity", t004_conic_complementarity),
        ("t004_conic_gap_with_g", t004_conic_gap_with_g),
        ("t004_conic_farkas_certificate", t004_conic_farkas),
        ("t004_conic_unbounded_ray", t004_conic_ray),
        ("t004_conic_missing_s", t004_conic_missing_s),
        ("t004_guard_all_zero_conic_fails", t004_guard_all_zero_conic),
        // Implementation decisions of 2026-11-15 (research R19).
        (
            "r19_split_multipliers_exact_gap",
            r19_split_multipliers_exact_gap,
        ),
        (
            "r19_split_multipliers_checked",
            r19_split_multipliers_checked,
        ),
        (
            "r19_interior_cs_informational",
            r19_interior_cs_informational,
        ),
        ("r19_zero_rows_need_no_y", r19_zero_rows_need_no_y),
        // Feature 003 T003: per-cone capabilities.
        ("t003_cone_kinds_and_names", t003_cone_kinds_and_names),
        (
            "t003_check_problem_per_cone_kind",
            t003_check_problem_per_cone_kind,
        ),
    ];
    tests
        .into_iter()
        .map(|(name, f)| Trial::test(name, f))
        .collect()
}

fn view<'a>(
    status: Status,
    x: Option<&'a [f64]>,
    y: Option<&'a [f64]>,
    z: Option<&'a [f64]>,
    s: Option<&'a [f64]>,
) -> SolutionView<'a> {
    SolutionView {
        status,
        returned: if x.is_some() {
            SolutionReturned::Optimal
        } else {
            SolutionReturned::None
        },
        x,
        y,
        z,
        z_l: None,
        z_u: None,
        s,
        basic: true,
        certificate: None,
    }
}

fn expect_verified(rep: &VerificationReport) -> Result<(), Failed> {
    ensure(
        rep.overall == Overall::Verified,
        format!("expected Verified: {rep:?}"),
    )
}

// --- T003 -----------------------------------------------------------------------------------

/// F = I (k × k) for k ≤ 6.
const EYE_CP: [usize; 7] = [0, 1, 2, 3, 4, 5, 6];
const EYE_RI: [usize; 6] = [0, 1, 2, 3, 4, 5];
const EYE_VA: [f64; 6] = [1.0; 6];
const ZERO6: [f64; 6] = [0.0; 6];

fn eye(k: usize) -> CscRef<'static, usize> {
    CscRef::new(k, k, &EYE_CP[..=k], &EYE_RI[..k], &EYE_VA[..k]).expect("identity")
}

fn t003_cone_dimensions_sum_to_k() -> Result<(), Failed> {
    let cones = [Cone::Quadratic(2)];
    expect_invalid(
        ConicRef::new(eye(3), &ZERO6[..3], &cones),
        "cones",
        "sum to 2",
    )?;
    let cones = [Cone::Quadratic(2), Cone::Nonnegative(2)];
    expect_invalid(
        ConicRef::new(eye(3), &ZERO6[..3], &cones),
        "cones",
        "sum to 4",
    )?;
    let cones = [Cone::Quadratic(2), Cone::Nonnegative(1)];
    ConicRef::new(eye(3), &ZERO6[..3], &cones).map_err(fail)?;
    // k = 0: no cones, empty F and g.
    let f0 = CscRef::<usize>::new(0, 0, &[0], &[], &[]).map_err(fail)?;
    ConicRef::new(f0, &[], &[]).map_err(fail).map(|_| ())
}

fn t003_cone_minimum_dimensions() -> Result<(), Failed> {
    let f = CscRef::<usize>::new(0, 0, &[0], &[], &[]).map_err(fail)?;
    for (cone, detail) in [
        (Cone::Quadratic(0), "dimension 0"),
        (Cone::Nonnegative(0), "dimension 0"),
        (Cone::Zero(0), "dimension 0"),
        (Cone::RotatedQuadratic(1), "minimum 2"),
        (Cone::PositiveSemidefinite { order: 0 }, "order 0"),
    ] {
        expect_invalid(ConicRef::new(f, &[], &[cone]), "cones", detail)?;
    }
    for (cone, k) in [
        (Cone::Quadratic(1), 1),
        (Cone::RotatedQuadratic(2), 2),
        (Cone::PrimalExponential, 3),
        (Cone::DualExponential, 3),
    ] {
        ensure(cone.dim() == k, format!("{cone:?} has dimension {k}"))?;
        ConicRef::new(eye(k), &ZERO6[..k], std::slice::from_ref(&cone)).map_err(fail)?;
    }
    Ok(())
}

fn t003_power_cone_exponents() -> Result<(), Failed> {
    let g = &ZERO6[..3];
    for (alpha, detail) in [
        (vec![], "without exponents"),
        (vec![1.0, 0.0], "not positive"),
        (vec![1.0, -1.0], "not positive"),
        (vec![f64::NAN, 1.0], "not positive"),
        (vec![1.0, 1.0, 1.0], "must be fewer"),
    ] {
        for cone in [
            Cone::PrimalPower {
                dim: 3,
                alpha: alpha.clone(),
            },
            Cone::DualPower {
                dim: 3,
                alpha: alpha.clone(),
            },
        ] {
            expect_invalid(ConicRef::new(eye(3), g, &[cone]), "cones", detail)?;
        }
    }
    let ok = [Cone::PrimalPower {
        dim: 3,
        alpha: vec![2.0, 1.0],
    }];
    ConicRef::new(eye(3), g, &ok).map_err(fail).map(|_| ())
}

fn t003_psd_dimension() -> Result<(), Failed> {
    let cone = Cone::PositiveSemidefinite { order: 3 };
    ensure(cone.dim() == 6, "order 3 has dimension 6")?;
    ConicRef::new(eye(6), &ZERO6, std::slice::from_ref(&cone)).map_err(fail)?;
    expect_invalid(
        ConicRef::new(eye(5), &ZERO6[..5], &[cone]),
        "cones",
        "sum to 6",
    )
}

fn t003_f_rows_and_g() -> Result<(), Failed> {
    // A row index of F at or beyond k is rejected by the CSC validation.
    expect_invalid(
        CscRef::<usize>::new(2, 1, &[0, 1], &[2], &[1.0]),
        "A.row_indices",
        "out of range",
    )?;
    let cones = [Cone::Nonnegative(3)];
    expect_invalid(ConicRef::new(eye(3), &ZERO6[..2], &cones), "g", "length")?;
    expect_invalid(
        ConicRef::new(eye(3), &[0.0, INF, 0.0], &cones),
        "g",
        "non-finite",
    )
}

fn lp_free3() -> Problem<'static, usize> {
    // minimise x0 subject to x1 = 3, x2 = 4, x free.
    const CP: [usize; 4] = [0, 0, 1, 2];
    const RI: [usize; 2] = [0, 1];
    const VA: [f64; 2] = [1.0, 1.0];
    let a = CscRef::new(2, 3, &CP, &RI, &VA).expect("A");
    Problem::new(
        a,
        &[1.0, 0.0, 0.0],
        Sense::Minimise,
        RowBounds::SenseRhs {
            sense: b"==",
            b: &[3.0, 4.0],
        },
        &[-INF, -INF, -INF],
        &[INF, INF, INF],
    )
    .expect("problem")
}

fn t003_with_conic_columns() -> Result<(), Failed> {
    let cones = [Cone::Nonnegative(2)];
    let conic = ConicRef::new(eye(2), &ZERO6[..2], &cones).map_err(fail)?;
    expect_invalid(lp_free3().with_conic(conic), "F", "columns")
}

fn t003_with_q_validation() -> Result<(), Failed> {
    let p = lp_free3();
    // Wrong size.
    let q = CscRef::<usize>::new(2, 2, &[0, 1, 2], &[0, 1], &[1.0, 1.0]).map_err(fail)?;
    expect_invalid(p.with_Q(q), "Q", "expected 3x3")?;
    // Upper-triangle entry (0, 1).
    let q = CscRef::<usize>::new(3, 3, &[0, 1, 3, 4], &[0, 0, 1, 2], &[1.0, 0.5, 1.0, 1.0])
        .map_err(fail)?;
    expect_invalid(p.with_Q(q), "Q", "above the diagonal")?;
    // Non-finite values are rejected when the matrix is built.
    match CscRef::<usize>::new(3, 3, &[0, 1, 2, 3], &[0, 1, 2], &[1.0, f64::NAN, 1.0]) {
        Err(Error::Invalid { .. }) => {}
        other => return Err(fail(format!("NaN in Q accepted: {other:?}"))),
    }
    // Lower triangle accepted.
    let q = CscRef::<usize>::new(3, 3, &[0, 2, 3, 4], &[0, 1, 1, 2], &[1.0, 0.5, 1.0, 1.0])
        .map_err(fail)?;
    p.with_Q(q).map_err(fail).map(|_| ())
}

fn t003_check_problem_classes() -> Result<(), Failed> {
    let backend = FakeBackend::lp_only();
    let cones = [Cone::Quadratic(3)];
    let conic = ConicRef::new(eye(3), &ZERO6[..3], &cones).map_err(fail)?;
    let p = lp_free3().with_conic(conic).map_err(fail)?;
    match backend.solve((&p).into(), &mut Options::default()) {
        Err(Error::Unsupported {
            field: "conic",
            backend: "fake",
        }) => {}
        other => return Err(fail(format!("expected Unsupported conic: {other:?}"))),
    }
    let q = CscRef::<usize>::new(3, 3, &[0, 1, 2, 3], &[0, 1, 2], &[1.0; 3]).map_err(fail)?;
    let pq = lp_free3().with_Q(q).map_err(fail)?;
    match backend.solve((&pq).into(), &mut Options::default()) {
        Err(Error::Unsupported { field: "Q", .. }) => {}
        other => return Err(fail(format!("expected Unsupported Q: {other:?}"))),
    }
    // A backend declaring Conic accepts it.
    let mut conic_backend = FakeBackend::lp_only();
    conic_backend.caps.problem_classes.push(ProblemClass::Conic);
    conic_backend.caps.cones.push(ConeKind::Quadratic);
    let sol = conic_backend
        .solve((&p).into(), &mut Options::default())
        .map_err(fail)?;
    ensure(sol.status == Status::NotSolved, "conic accepted")
}

// --- T004: QP -------------------------------------------------------------------------------

/// minimise ½ xᵀQx subject to x1 + x2 = 2, x ≥ 0, with Q given by its lower triangle.
fn qp(q: CscRef<'static, usize>, c: &'static [f64], sense: Sense) -> Problem<'static, usize> {
    const CP: [usize; 3] = [0, 1, 2];
    const RI: [usize; 2] = [0, 0];
    const VA: [f64; 2] = [1.0, 1.0];
    let a = CscRef::new(1, 2, &CP, &RI, &VA).expect("A");
    let l: &'static [f64] = if sense == Sense::Minimise {
        &[0.0, 0.0]
    } else {
        &[-INF, -INF]
    };
    Problem::new(
        a,
        c,
        sense,
        RowBounds::SenseRhs {
            sense: b"=",
            b: &[2.0],
        },
        l,
        &[INF, INF],
    )
    .expect("problem")
    .with_Q(q)
    .expect("Q")
}

fn t004_qp_diagonal_verified() -> Result<(), Failed> {
    // Q = I, c = 0: x = (1, 1), Qx = (1, 1), y = 1, z = 0, p = d = 1.
    let q = CscRef::<usize>::new(2, 2, &[0, 1, 2], &[0, 1], &[1.0, 1.0]).map_err(fail)?;
    let p = qp(q, &[0.0, 0.0], Sense::Minimise);
    let x = [1.0, 1.0];
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&x),
            Some(&[1.0]),
            Some(&[0.0, 0.0]),
            None,
        ),
        &TOL,
    );
    expect_verified(&rep)?;
    check_measure(&rep, names::GAP, 0.0, 0.0, Verdict::Pass)?;
    // Without the Qx term the LP rule would give z = -y = -1: the duals of the LP would fail.
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&x),
            Some(&[0.0]),
            Some(&[0.0, 0.0]),
            None,
        ),
        &TOL,
    );
    let q = measure(&rep, names::REDUCED_COST)?;
    ensure(
        q.verdict == Verdict::Fail && (q.absolute - 1.0).abs() < 1e-15,
        format!("y = 0 must fail reduced costs with residual 1: {q:?}"),
    )
}

fn t004_qp_off_diagonal() -> Result<(), Failed> {
    // Q = [[2, 1], [1, 2]] given as lower triangle (0,0)=2, (1,0)=1, (1,1)=2: x = (1, 1),
    // Qx = (3, 3), y = 3, z = 0, p = ½ xᵀQx = 3, d = -3 + 3·2 = 3.
    let q = CscRef::<usize>::new(2, 2, &[0, 2, 3], &[0, 1, 1], &[2.0, 1.0, 2.0]).map_err(fail)?;
    let p = qp(q, &[0.0, 0.0], Sense::Minimise);
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[1.0, 1.0]),
            Some(&[3.0]),
            Some(&[0.0, 0.0]),
            None,
        ),
        &TOL,
    );
    expect_verified(&rep)?;
    // y = 2 (the value if the off-diagonal entry were counted once) fails.
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[1.0, 1.0]),
            Some(&[2.0]),
            Some(&[0.0, 0.0]),
            None,
        ),
        &TOL,
    );
    ensure(rep.overall == Overall::Failed, format!("y = 2: {rep:?}"))
}

fn t004_qp_maximise() -> Result<(), Failed> {
    // maximise -½‖x‖² subject to x1 + x2 = 2: Q = -I, x = (1, 1), y = -1, z = 0, p = d = -1.
    let q = CscRef::<usize>::new(2, 2, &[0, 1, 2], &[0, 1], &[-1.0, -1.0]).map_err(fail)?;
    let p = qp(q, &[0.0, 0.0], Sense::Maximise);
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[1.0, 1.0]),
            Some(&[-1.0]),
            Some(&[0.0, 0.0]),
            None,
        ),
        &TOL,
    );
    expect_verified(&rep)?;
    check_measure(&rep, names::GAP, 0.0, 0.0, Verdict::Pass)
}

fn t004_qp_without_x() -> Result<(), Failed> {
    let q = CscRef::<usize>::new(2, 2, &[0, 1, 2], &[0, 1], &[1.0, 1.0]).map_err(fail)?;
    let p = qp(q, &[0.0, 0.0], Sense::Minimise);
    let mut v = view(Status::Optimal, None, Some(&[1.0]), Some(&[0.0, 0.0]), None);
    v.returned = SolutionReturned::None;
    let rep = verify(&p, &v, &TOL);
    let q = measure(&rep, names::REDUCED_COST)?;
    ensure(
        q.verdict == Verdict::NotEvaluated && rep.overall == Overall::PartiallyVerified,
        format!("Qx needs x: {rep:?}"),
    )
}

// --- T004: cones ----------------------------------------------------------------------------

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * (1.0 + b.abs())
}

fn expect_violation(cone: &Cone, u: &[f64], v: f64) -> Result<(), Failed> {
    let got = cone.violation(u);
    ensure(
        near(got, v),
        format!("{cone:?} at {u:?}: violation {got}, expected {v}"),
    )
}

fn t004_cone_violations() -> Result<(), Failed> {
    let e = std::f64::consts::E;
    let q = Cone::Quadratic(3);
    expect_violation(&q, &[2.0, 1.0, 1.0], 0.0)?; // inside
    expect_violation(&q, &[5.0, 3.0, 4.0], 0.0)?; // boundary
    expect_violation(&q, &[1.0, 1.0, 1.0], SQRT2 - 1.0)?; // outside
    let r = Cone::RotatedQuadratic(3);
    expect_violation(&r, &[1.0, 1.0, 1.0], 0.0)?;
    expect_violation(&r, &[0.5, 1.0, 1.0], 0.0)?;
    expect_violation(&r, &[0.25, 1.0, 1.0], 0.5 / 2.0)?;
    expect_violation(&r, &[-1.0, -1.0, 0.0], 1.0)?;
    let x = Cone::PrimalExponential;
    expect_violation(&x, &[3.0, 1.0, 1.0], 0.0)?;
    expect_violation(&x, &[e, 1.0, 1.0], 0.0)?;
    expect_violation(&x, &[2.0, 1.0, 1.0], e - 2.0)?;
    expect_violation(&x, &[1.0, 0.0, -1.0], 0.0)?; // closure u₂ = 0, u₃ ≤ 0
    expect_violation(&x, &[1.0, 0.0, 1.0], 1.0)?;
    expect_violation(&x, &[1.0, -1.0, 0.0], 1.0)?;
    let d = Cone::DualExponential;
    expect_violation(&d, &[1.0, 0.0, -1.0], 0.0)?;
    expect_violation(&d, &[0.1, 0.0, -1.0], (-1.0f64).exp() - 0.1)?;
    expect_violation(&d, &[1.0, 1.0, 0.0], 0.0)?; // closure u₃ = 0, u₂ ≥ 0
    expect_violation(&d, &[1.0, -1.0, 0.0], 1.0)?;
    let pp = Cone::PrimalPower {
        dim: 3,
        alpha: vec![1.0, 1.0],
    };
    expect_violation(&pp, &[4.0, 1.0, 1.0], 0.0)?;
    expect_violation(&pp, &[1.0, 1.0, 1.0], 0.0)?;
    expect_violation(&pp, &[1.0, 1.0, 2.0], 1.0)?;
    expect_violation(&pp, &[-1.0, 1.0, 0.0], 1.0)?;
    // Unnormalised exponents (3, 1) are normalised to (¾, ¼): 16^¾ · 1^¼ = 8.
    let pp31 = Cone::PrimalPower {
        dim: 3,
        alpha: vec![3.0, 1.0],
    };
    expect_violation(&pp31, &[16.0, 1.0, 8.0], 0.0)?;
    expect_violation(&pp31, &[16.0, 1.0, 9.0], 1.0)?;
    let dp = Cone::DualPower {
        dim: 3,
        alpha: vec![1.0, 1.0],
    };
    expect_violation(&dp, &[1.0, 1.0, 2.0], 0.0)?;
    expect_violation(&dp, &[1.0, 1.0, 3.0], 1.0)?;
    let n = Cone::Nonnegative(2);
    expect_violation(&n, &[1.0, 0.0], 0.0)?;
    expect_violation(&n, &[1.0, -2.0], 2.0)?;
    let z = Cone::Zero(2);
    expect_violation(&z, &[0.0, 0.0], 0.0)?;
    expect_violation(&z, &[0.0, 0.5], 0.5)?;
    let s2 = Cone::PositiveSemidefinite { order: 2 };
    // [[1, 2], [2, 5]]: eigenvalues 3 ± √8 > 0.
    expect_violation(&s2, &[1.0, 2.0 * SQRT2, 5.0], 0.0)?;
    // [[1, 2], [2, 1]]: eigenvalues 3 and -1.
    expect_violation(&s2, &[1.0, 2.0 * SQRT2, 1.0], 1.0)?;
    // Order 3: diag(1, 1, 1) with X₃₁ = 2 gives eigenvalues 1, 3, -1.
    let s3 = Cone::PositiveSemidefinite { order: 3 };
    expect_violation(&s3, &[1.0, 0.0, 2.0 * SQRT2, 1.0, 0.0, 1.0], 1.0)?;
    expect_violation(&s3, &[1.0, 0.0, 0.0, 1.0, 0.0, 1.0], 0.0)?;
    // NaN and wrong lengths never pass.
    ensure(q.violation(&[f64::NAN, 0.0, 0.0]).is_infinite(), "NaN")?;
    ensure(q.violation(&[1.0, 0.0]).is_infinite(), "wrong length")
}

fn t004_dual_cones() -> Result<(), Failed> {
    // Self-dual cones.
    let q = Cone::Quadratic(3);
    ensure(
        q.dual_violation(&[1.0, 1.0, 1.0]) == q.violation(&[1.0, 1.0, 1.0]),
        "quadratic self-dual",
    )?;
    // The dual of the primal exponential cone is the dual exponential cone, and back.
    let u = [0.1, 0.0, -1.0];
    ensure(
        Cone::PrimalExponential.dual_violation(&u) == Cone::DualExponential.violation(&u),
        "exp dual",
    )?;
    ensure(
        Cone::DualExponential.dual_violation(&u) == Cone::PrimalExponential.violation(&u),
        "dual exp dual",
    )?;
    // Power cones.
    let dp = Cone::DualPower {
        dim: 3,
        alpha: vec![1.0, 1.0],
    };
    let pp = Cone::PrimalPower {
        dim: 3,
        alpha: vec![1.0, 1.0],
    };
    ensure(
        pp.dual_violation(&[1.0, 1.0, 2.0]) == 0.0 && dp.dual_violation(&[1.0, 1.0, 2.0]) == 1.0,
        "power duals",
    )?;
    // The dual of {0} is everything.
    ensure(
        Cone::Zero(2).dual_violation(&[-5.0, 7.0]) == 0.0,
        "zero cone dual is free",
    )
}

/// minimise σ-sense of c0·x0 subject to x1 = 3, x2 = 4, x free, (x0, x1, x2) + g ∈ Q³.
fn conic_problem(c: &'static [f64], sense: Sense, g: &'static [f64]) -> Problem<'static, usize> {
    const CP: [usize; 4] = [0, 0, 1, 2];
    const RI: [usize; 2] = [0, 1];
    const VA: [f64; 2] = [1.0, 1.0];
    static CONES: [Cone; 1] = [Cone::Quadratic(3)];
    let a = CscRef::new(2, 3, &CP, &RI, &VA).expect("A");
    let conic = ConicRef::new(eye(3), g, &CONES).expect("conic");
    Problem::new(
        a,
        c,
        sense,
        RowBounds::SenseRhs {
            sense: b"==",
            b: &[3.0, 4.0],
        },
        &[-INF, -INF, -INF],
        &[INF, INF, INF],
    )
    .expect("problem")
    .with_conic(conic)
    .expect("with_conic")
}

const Z3: [f64; 3] = [0.0, 0.0, 0.0];

fn t004_conic_minimise() -> Result<(), Failed> {
    // x = (5, 3, 4); s = c - Aᵀy with y = (0.6, 0.8): s = (1, -0.6, -0.8) ∈ Q*; p = d = 5.
    let p = conic_problem(&[1.0, 0.0, 0.0], Sense::Minimise, &Z3);
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[5.0, 3.0, 4.0]),
            Some(&[0.6, 0.8]),
            Some(&Z3),
            Some(&[1.0, -0.6, -0.8]),
        ),
        &TOL,
    );
    expect_verified(&rep)?;
    for name in [
        names::PRIMAL_CONE,
        names::DUAL_CONE,
        names::CONIC_COMPLEMENTARITY,
        names::GAP,
        names::REDUCED_COST,
    ] {
        let q = measure(&rep, name)?;
        ensure(q.absolute < 1e-12, format!("{name}: {q:?}"))?;
    }
    Ok(())
}

fn t004_conic_maximise() -> Result<(), Failed> {
    // maximise -x0: y = (-0.6, -0.8), s = (-1, 0.6, 0.8), σs = (1, -0.6, -0.8) ∈ Q; p = d = -5.
    let p = conic_problem(&[-1.0, 0.0, 0.0], Sense::Maximise, &Z3);
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[5.0, 3.0, 4.0]),
            Some(&[-0.6, -0.8]),
            Some(&Z3),
            Some(&[-1.0, 0.6, 0.8]),
        ),
        &TOL,
    );
    expect_verified(&rep)
}

fn t004_conic_wrong_dual_sign() -> Result<(), Failed> {
    // The minimisation duals on the maximisation problem: σs = (-1, 0.6, 0.8) ∉ Q.
    let p = conic_problem(&[-1.0, 0.0, 0.0], Sense::Maximise, &Z3);
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[5.0, 3.0, 4.0]),
            Some(&[0.6, 0.8]),
            Some(&Z3),
            Some(&[1.0, -0.6, -0.8]),
        ),
        &TOL,
    );
    let q = measure(&rep, names::DUAL_CONE)?;
    ensure(
        q.verdict == Verdict::Fail && rep.overall == Overall::Failed,
        format!("wrong dual cone sign must fail: {rep:?}"),
    )
}

fn t004_conic_primal_violation() -> Result<(), Failed> {
    // x0 = 4 < ‖(3, 4)‖ = 5: violation 1, scaled by 1 + ‖g‖∞ + ‖Fx‖∞ = 5.
    let p = conic_problem(&[1.0, 0.0, 0.0], Sense::Minimise, &Z3);
    let rep = verify(
        &p,
        &view(Status::NotSolved, Some(&[4.0, 3.0, 4.0]), None, None, None),
        &TOL,
    );
    check_measure(&rep, names::PRIMAL_CONE, 1.0, 0.2, Verdict::Fail)
}

fn t004_conic_complementarity() -> Result<(), Failed> {
    // A feasible but not optimal x0 = 6 with the optimal duals: sᵀ(Fx + g) = 1.
    let p = conic_problem(&[1.0, 0.0, 0.0], Sense::Minimise, &Z3);
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[6.0, 3.0, 4.0]),
            Some(&[0.6, 0.8]),
            Some(&Z3),
            Some(&[1.0, -0.6, -0.8]),
        ),
        &TOL,
    );
    check_measure(
        &rep,
        names::CONIC_COMPLEMENTARITY,
        1.0,
        1.0 / 7.0,
        Verdict::Fail,
    )
}

fn t004_conic_gap_with_g() -> Result<(), Failed> {
    // g = (1, 0, 0): (x0 + 1, 3, 4) ∈ Q gives x0 = 4, p = 4; d = y·b − gᵀs = 5 − 1 = 4.
    let p = conic_problem(&[1.0, 0.0, 0.0], Sense::Minimise, &[1.0, 0.0, 0.0]);
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[4.0, 3.0, 4.0]),
            Some(&[0.6, 0.8]),
            Some(&Z3),
            Some(&[1.0, -0.6, -0.8]),
        ),
        &TOL,
    );
    expect_verified(&rep)?;
    check_measure(&rep, names::GAP, 0.0, 0.0, Verdict::Pass)
}

fn t004_conic_farkas() -> Result<(), Failed> {
    // x1 = 3, x2 = 4, x0 ≤ 4 and (x0, x1, x2) ∈ Q³ is infeasible. Certificate: s = (1, -0.6,
    // -0.8) ∈ Q*, z = (-1, 0, 0) (upper bound of x0), y = (0.6, 0.8): Aᵀy + z + Fᵀs = 0 and
    // d_F = 0.6·3 + 0.8·4 − 1·4 = 1 > 0.
    const CP: [usize; 4] = [0, 0, 1, 2];
    const RI: [usize; 2] = [0, 1];
    const VA: [f64; 2] = [1.0, 1.0];
    static CONES: [Cone; 1] = [Cone::Quadratic(3)];
    let a = CscRef::new(2, 3, &CP, &RI, &VA).map_err(fail)?;
    let conic = ConicRef::new(eye(3), &Z3, &CONES).map_err(fail)?;
    let p = Problem::new(
        a,
        &[0.0, 0.0, 0.0],
        Sense::Minimise,
        RowBounds::SenseRhs {
            sense: b"==",
            b: &[3.0, 4.0],
        },
        &[-INF, -INF, -INF],
        &[4.0, INF, INF],
    )
    .map_err(fail)?
    .with_conic(conic)
    .map_err(fail)?;
    let good = Certificate::Farkas {
        y: vec![0.6, 0.8],
        z: vec![-1.0, 0.0, 0.0],
        s: vec![1.0, -0.6, -0.8],
    };
    let mut v = view(Status::Infeasible, None, None, None, None);
    v.certificate = Some(&good);
    let rep = verify(&p, &v, &TOL);
    check_measure(&rep, names::FARKAS, 0.0, 0.0, Verdict::Pass)?;
    expect_verified(&rep)?;
    // The negated certificate has zero residual but wrong signs: s ∉ Q*, d_F < 0.
    let bad = Certificate::Farkas {
        y: vec![-0.6, -0.8],
        z: vec![1.0, 0.0, 0.0],
        s: vec![-1.0, 0.6, 0.8],
    };
    v.certificate = Some(&bad);
    let rep = verify(&p, &v, &TOL);
    let q = measure(&rep, names::FARKAS)?;
    ensure(
        q.verdict == Verdict::Fail && q.relative.is_infinite(),
        format!("negated conic certificate: {q:?}"),
    )?;
    // Missing s is malformed.
    let short = Certificate::Farkas {
        y: vec![0.6, 0.8],
        z: vec![-1.0, 0.0, 0.0],
        s: vec![],
    };
    v.certificate = Some(&short);
    let rep = verify(&p, &v, &TOL);
    ensure(
        measure(&rep, names::FARKAS)?.verdict == Verdict::Fail,
        "s of the wrong length fails",
    )
}

fn t004_conic_ray() -> Result<(), Failed> {
    // minimise -x0 subject to (x0, 3, 4) ∈ Q (rows fix x1, x2): r = (1, 0, 0) is a ray with
    // Fr ∈ Q, Ar = 0 and cᵀr = -1 < 0. r = (-1, 0, 0) leaves the cone.
    let p = conic_problem(&[-1.0, 0.0, 0.0], Sense::Minimise, &Z3);
    let ray = Certificate::UnboundedRay {
        r: vec![1.0, 0.0, 0.0],
    };
    let mut v = view(Status::Unbounded, None, None, None, None);
    v.certificate = Some(&ray);
    let rep = verify(&p, &v, &TOL);
    check_measure(&rep, names::RAY, 0.0, 0.0, Verdict::Pass)?;
    let p = conic_problem(&[1.0, 0.0, 0.0], Sense::Minimise, &Z3);
    let ray = Certificate::UnboundedRay {
        r: vec![-1.0, 0.0, 0.0],
    };
    v.certificate = Some(&ray);
    let rep = verify(&p, &v, &TOL);
    let q = measure(&rep, names::RAY)?;
    ensure(
        q.absolute == 1.0 && q.verdict == Verdict::Fail,
        format!("ray leaving the cone: {q:?}"),
    )
}

fn t004_conic_missing_s() -> Result<(), Failed> {
    let p = conic_problem(&[1.0, 0.0, 0.0], Sense::Minimise, &Z3);
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[5.0, 3.0, 4.0]),
            Some(&[0.6, 0.8]),
            Some(&Z3),
            None,
        ),
        &TOL,
    );
    ensure(
        measure(&rep, names::DUAL_CONE)?.verdict == Verdict::NotEvaluated
            && rep.overall == Overall::PartiallyVerified,
        format!("s missing: {rep:?}"),
    )?;
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[5.0, 3.0, 4.0]),
            Some(&[0.6, 0.8]),
            Some(&Z3),
            Some(&[1.0, 0.0]),
        ),
        &TOL,
    );
    ensure(
        rep.overall == Overall::Failed,
        "s of the wrong length fails",
    )
}

fn t004_guard_all_zero_conic() -> Result<(), Failed> {
    // XV-C guard self-test: everything zero, claimed optimal, must never verify.
    let p = conic_problem(&[1.0, 0.0, 0.0], Sense::Minimise, &Z3);
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&Z3),
            Some(&[0.0, 0.0]),
            Some(&Z3),
            Some(&Z3),
        ),
        &TOL,
    );
    ensure(
        rep.overall == Overall::Failed,
        format!("all-zero conic solution must fail: {rep:?}"),
    )
}

// --- Research R19: split bound multipliers, per-item CS for interior solutions, m = 0 -----

/// minimise x subject to no rows, 0 ≤ x ≤ 2 (n = 1, m = 0).
fn box1(c: &'static [f64]) -> Problem<'static, usize> {
    let a = CscRef::<usize>::new(0, 1, &[0, 0], &[], &[]).expect("A");
    Problem::new(
        a,
        c,
        Sense::Minimise,
        RowBounds::SenseRhs { sense: b"", b: &[] },
        &[0.0],
        &[2.0],
    )
    .expect("problem")
}

fn split_view<'a>(
    x: &'a [f64],
    z: &'a [f64],
    zl: &'a [f64],
    zu: &'a [f64],
    basic: bool,
) -> SolutionView<'a> {
    let mut v = view(Status::Optimal, Some(x), None, Some(z), None);
    v.z_l = Some(zl);
    v.z_u = Some(zu);
    v.basic = basic;
    v
}

fn r19_split_multipliers_exact_gap() -> Result<(), Failed> {
    // c = 1, x = 1e-9 (interior point near l = 0), z_l = 1 + 1e-9, z_u = 1e-9: z = 1.
    // With z alone the dual objective is l·z = 0; with the split it is l·z_l − u·z_u = −2e-9,
    // and p = 1e-9: the exact gap is 3e-9, judged relative to max(1, min(|p|, |d|)) = 1.
    let p = box1(&[1.0]);
    let rep = verify(
        &p,
        &split_view(&[1e-9], &[1.0], &[1.0 + 1e-9], &[1e-9], false),
        &TOL,
    );
    let q = measure(&rep, names::GAP)?;
    ensure(
        (q.absolute - 3e-9).abs() < 1e-15 && q.verdict == Verdict::Pass,
        format!("exact gap with split multipliers: {q:?}"),
    )?;
    ensure(rep.overall == Overall::Verified, format!("{rep:?}"))
}

fn r19_split_multipliers_checked() -> Result<(), Failed> {
    let p = box1(&[1.0]);
    // z_l − z_u ≠ z fails reduced-cost consistency.
    let rep = verify(&p, &split_view(&[0.0], &[1.0], &[0.5], &[0.0], true), &TOL);
    ensure(
        measure(&rep, names::REDUCED_COST)?.verdict == Verdict::Fail,
        "z_l − z_u must equal z",
    )?;
    // A negative multiplier fails the dual sign measure.
    let rep = verify(&p, &split_view(&[0.0], &[1.0], &[1.5], &[0.5], true), &TOL);
    ensure(
        measure(&rep, names::DUAL_SIGN)?.verdict == Verdict::Pass,
        "z_u = 0.5 ≥ 0 is sign-feasible",
    )?;
    let rep = verify(&p, &split_view(&[0.0], &[1.0], &[2.0], &[1.0], true), &TOL);
    ensure(
        measure(&rep, names::DUAL_SIGN)?.verdict == Verdict::Pass,
        "both multipliers non-negative",
    )?;
    let rep = verify(&p, &split_view(&[0.0], &[1.0], &[0.0], &[-1.0], true), &TOL);
    ensure(
        measure(&rep, names::DUAL_SIGN)?.verdict == Verdict::Fail,
        "z_u = −1 violates σz_u ≥ 0",
    )?;
    // Only one of the two given: malformed.
    let mut v = split_view(&[0.0], &[1.0], &[1.0], &[0.0], true);
    v.z_u = None;
    let rep = verify(&p, &v, &TOL);
    ensure(
        rep.overall == Overall::Failed,
        "z_l without z_u is malformed",
    )
}

fn r19_interior_cs_informational() -> Result<(), Failed> {
    // As in interior-point solutions: products of multipliers and slacks that are not each
    // small but cancel in the exact gap through a tolerated bound violation. n = 2, m = 0,
    // 0 ≤ x ≤ 2, c = (0, 200). x₁ = 1 with z_l = z_u = 1e-4 (products 1e-4 and 1e-4);
    // x₂ = −1e-6 (within the primal tolerance) with z_l = 200 (product −2e-4). The exact gap
    // is 1e-4 + 1e-4 − 2e-4 = 0; per-item complementary slackness is 2e-4.
    let a = CscRef::<usize>::new(0, 2, &[0, 0, 0], &[], &[]).map_err(fail)?;
    let p = Problem::new(
        a,
        &[0.0, 200.0],
        Sense::Minimise,
        RowBounds::SenseRhs { sense: b"", b: &[] },
        &[0.0, 0.0],
        &[2.0, 2.0],
    )
    .map_err(fail)?;
    let (x, z, zl, zu) = ([1.0, -1e-6], [0.0, 200.0], [1e-4, 200.0], [1e-4, 0.0]);
    let rep = verify(&p, &split_view(&x, &z, &zl, &zu, false), &TOL);
    let q = measure(&rep, names::COMPLEMENTARY_SLACKNESS)?;
    ensure(
        q.verdict == Verdict::Fail && !q.required,
        format!("reported, not required: {q:?}"),
    )?;
    ensure(
        measure(&rep, names::GAP)?.verdict == Verdict::Pass && rep.overall == Overall::Verified,
        format!("an informational failure does not decide the verdict: {rep:?}"),
    )?;
    // The same numbers for a basic solution: required, Failed.
    let rep = verify(&p, &split_view(&x, &z, &zl, &zu, true), &TOL);
    ensure(
        measure(&rep, names::COMPLEMENTARY_SLACKNESS)?.required && rep.overall == Overall::Failed,
        format!("basic: {rep:?}"),
    )
}

fn r19_zero_rows_need_no_y() -> Result<(), Failed> {
    // m = 0: y is empty and `Solution::view` reports it absent; that is not missing.
    let p = box1(&[1.0]);
    let rep = verify(
        &p,
        &view(Status::Optimal, Some(&[0.0]), None, Some(&[1.0]), None),
        &TOL,
    );
    ensure(rep.overall == Overall::Verified, format!("{rep:?}"))
}

// --- Feature 003 T003: cone kinds and per-kind capabilities ---------------------------------

fn t003_cone_kinds_and_names() -> Result<(), Failed> {
    let cases = [
        (Cone::Quadratic(3), ConeKind::Quadratic, "quadratic"),
        (
            Cone::RotatedQuadratic(3),
            ConeKind::RotatedQuadratic,
            "rotated quadratic",
        ),
        (
            Cone::PrimalExponential,
            ConeKind::PrimalExponential,
            "primal exponential",
        ),
        (
            Cone::DualExponential,
            ConeKind::DualExponential,
            "dual exponential",
        ),
        (
            Cone::PrimalPower {
                dim: 3,
                alpha: vec![1.0, 1.0],
            },
            ConeKind::PrimalPower,
            "primal power",
        ),
        (
            Cone::DualPower {
                dim: 3,
                alpha: vec![1.0, 1.0],
            },
            ConeKind::DualPower,
            "dual power",
        ),
        (Cone::Nonnegative(2), ConeKind::Nonnegative, "nonnegative"),
        (Cone::Zero(2), ConeKind::Zero, "zero"),
        (
            Cone::PositiveSemidefinite { order: 2 },
            ConeKind::PositiveSemidefinite,
            "positive semidefinite",
        ),
    ];
    for (cone, kind, name) in cases {
        ensure(cone.kind() == kind, format!("{cone:?} has kind {kind:?}"))?;
        ensure(kind.name() == name, format!("{kind:?} is named {name:?}"))?;
    }
    Ok(())
}

fn t003_check_problem_per_cone_kind() -> Result<(), Failed> {
    let cones = [Cone::Quadratic(3)];
    let conic = ConicRef::new(eye(3), &ZERO6[..3], &cones).map_err(fail)?;
    let p = lp_free3().with_conic(conic).map_err(fail)?;
    // Conic class declared, but not the quadratic kind: Unsupported naming the kind.
    let mut backend = FakeBackend::lp_only();
    backend.caps.problem_classes.push(ProblemClass::Conic);
    backend.caps.cones = vec![ConeKind::Nonnegative];
    match backend.solve((&p).into(), &mut Options::default()) {
        Err(Error::Unsupported {
            field: "quadratic",
            backend: "fake",
        }) => {}
        other => return Err(fail(format!("expected Unsupported quadratic: {other:?}"))),
    }
    // Listing the kind accepts it.
    backend.caps.cones.push(ConeKind::Quadratic);
    let sol = backend
        .solve((&p).into(), &mut Options::default())
        .map_err(fail)?;
    ensure(sol.status == Status::NotSolved, "quadratic accepted")
}
