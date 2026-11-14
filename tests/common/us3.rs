//! User Story 3 per-backend tests (tasks T030–T035; analysis findings C1 and C2), written once
//! against `optsolver::Backend` and registered for every declared backend through
//! [`super::Registry`] (constitution II, XVI). Nothing here names a solver: where a test needs
//! a vendor parameter name it asks the backend's capabilities for the parameter a shared
//! options field maps to.
//!
//! Assumptions about solver behaviour these tests encode are stated on each test.

#![allow(non_snake_case)] // Notation Convention (XII-C): A, Q are matrices.

use super::fixtures::{
    CorpusFixture, Json, Provenance, corpus_instances, fixtures_dir, load_corpus_fixture,
    sha256_hex,
};
use super::mps::{MpsProblem, read_mps_file};
use super::objectives::seeded_objective;
use super::{BackendTest, Failed, ensure, fail};
use optsolver::{
    Backend, Certificate, CscRef, Error, LimitReason, Method, Options, Overall, ParamValue,
    Problem, ProblemClass, Progress, RowBounds, Sense, Solution, SolutionBuffers, SolutionReturned,
    Status,
};
use std::sync::atomic::{AtomicBool, Ordering};

const INF: f64 = f64::INFINITY;

/// The largest corpus instance (spec SC-004): used where a solve must last long enough.
pub const LARGEST_INSTANCE: &str = "iDopaNeuroC_problem_1";

/// Trial name (without the backend suffix) of the helper the silence test runs in a child
/// process.
pub const QUIET_CHILD: &str = "us3::t033_quiet_child";

// ---------------------------------------------------------------------------------------------
// Helpers

/// Relative closeness used for objective values: |a − b| ≤ 1e-9 · max(1, |b|).
pub fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * b.abs().max(1.0)
}

/// Options for deterministic corpus solves (SC-003): dual simplex, one thread, random seed 0.
pub fn deterministic_options() -> Options<'static> {
    let mut o = Options::default();
    o.method = Some(Method::DualSimplex);
    o.threads = Some(1);
    o.random_seed = Some(0);
    o
}

/// The i-th corpus instance declared in `project.yml`.
pub fn instance(i: usize) -> Result<String, Failed> {
    let all = corpus_instances().map_err(fail)?;
    all.get(i).cloned().ok_or_else(|| {
        fail(format!(
            "corpus declares {} instances, no index {i}",
            all.len()
        ))
    })
}

/// A corpus fixture and the seed at position `s` of its provenance record.
pub fn corpus_with_seed(name: &str, s: usize) -> Result<(CorpusFixture, u64), Failed> {
    let f = load_corpus_fixture(name).map_err(fail)?;
    let seeds = f.provenance.seeds().map_err(fail)?;
    let seed = *seeds
        .get(s)
        .ok_or_else(|| fail(format!("{name}: provenance records {seeds:?}, no seed {s}")))?;
    Ok((f, seed))
}

/// Short description of a solution for failure messages.
pub fn describe(sol: &Solution) -> String {
    let failing: Vec<String> = sol
        .verification
        .iter()
        .flat_map(|r| r.measures.iter())
        .filter(|q| q.verdict != optsolver::Verdict::Pass)
        .map(|q| {
            format!(
                "{} {:?} (rel {:e}, tol {:e})",
                q.name, q.verdict, q.relative, q.tolerance
            )
        })
        .collect();
    format!(
        "status {:?}, limit {:?}, returned {:?}, objective {:?}, |x| {}, overall {:?}, \
         non-passing measures {failing:?}, raw status {:?}",
        sol.status,
        sol.limit_reason,
        sol.returned,
        sol.objective,
        sol.x.len(),
        sol.verification.as_ref().map(|r| r.overall),
        sol.raw_status,
    )
}

/// The solution's verification report says `overall`.
pub fn expect_overall(sol: &Solution, overall: Overall, what: &str) -> Result<(), Failed> {
    let got = sol.verification.as_ref().map(|r| r.overall);
    ensure(
        got == Some(overall),
        format!(
            "{what}: verification {got:?}, expected {overall:?}; {}",
            describe(sol)
        ),
    )
}

/// Status Optimal, an optimal solution of length n returned, and verification Verified.
pub fn expect_optimal_verified(sol: &Solution, n: usize, what: &str) -> Result<(), Failed> {
    ensure(
        sol.status == Status::Optimal && sol.returned == SolutionReturned::Optimal,
        format!("{what}: expected Optimal; {}", describe(sol)),
    )?;
    ensure(
        sol.x.len() == n,
        format!("{what}: |x| = {}, n = {n}", sol.x.len()),
    )?;
    expect_overall(sol, Overall::Verified, what)
}

/// The objective value equals `expected` (relative 1e-9).
pub fn expect_objective(sol: &Solution, expected: f64, what: &str) -> Result<(), Failed> {
    ensure(
        sol.objective.is_some_and(|v| close(v, expected)),
        format!("{what}: objective {:?}, expected {expected}", sol.objective),
    )
}

/// A hand-built fixture from `tests/fixtures/small/`, with its provenance record; the MPS
/// file's SHA-256 is checked against the record.
pub struct SmallFixture {
    pub name: &'static str,
    pub problem: MpsProblem,
    pub provenance: Provenance,
}

impl SmallFixture {
    /// Loads `tests/fixtures/small/<name>.mps` and its provenance record.
    pub fn load(name: &'static str) -> Result<Self, Failed> {
        let dir = fixtures_dir().map_err(fail)?.join("small");
        let provenance =
            Provenance::load(&dir.join(format!("{name}.provenance.json"))).map_err(fail)?;
        let path = dir.join(format!("{name}.mps"));
        let bytes = std::fs::read(&path).map_err(|e| fail(format!("{}: {e}", path.display())))?;
        let recorded = provenance.str("mps_sha256").map_err(fail)?;
        ensure(
            sha256_hex(&bytes) == recorded,
            format!(
                "{}: SHA-256 differs from its provenance record",
                path.display()
            ),
        )?;
        let problem = read_mps_file(&path).map_err(fail)?;
        ensure(
            problem.m == provenance.usize("m").map_err(fail)?
                && problem.n == provenance.usize("n").map_err(fail)?,
            format!("{name}: dimensions differ from its provenance record"),
        )?;
        Ok(Self {
            name,
            problem,
            provenance,
        })
    }

    /// The `expected_status` field of the provenance record.
    pub fn expected_status(&self) -> Result<Status, Failed> {
        match self.provenance.str("expected_status").map_err(fail)? {
            "Optimal" => Ok(Status::Optimal),
            "Infeasible" => Ok(Status::Infeasible),
            "Unbounded" => Ok(Status::Unbounded),
            other => Err(fail(format!(
                "{}: unknown expected_status {other}",
                self.name
            ))),
        }
    }

    /// The `expected_objective` field of the provenance record (`None` when null).
    pub fn expected_objective(&self) -> Result<Option<f64>, Failed> {
        match self.provenance.fields.get("expected_objective") {
            Some(Json::Number(v)) => Ok(Some(*v)),
            Some(Json::Null) => Ok(None),
            other => Err(fail(format!(
                "{}: expected_objective is {other:?}",
                self.name
            ))),
        }
    }
}

/// The ranged-row fixture as a problem with `RowBounds::Ranges` (from `row_ranges()`).
fn ranged_problem<'a>(
    fx: &'a MpsProblem,
    b_l: &'a [f64],
    b_u: &'a [f64],
) -> Result<Problem<'a, usize>, Failed> {
    Problem::new(
        fx.A().map_err(fail)?,
        &fx.c,
        fx.objective_sense,
        RowBounds::Ranges { b_l, b_u },
        &fx.l,
        &fx.u,
    )
    .and_then(|p| p.with_gamma(fx.gamma))
    .map_err(fail)
}

// Problems built directly in Rust (T031). Each comment derives the status the data implies.

/// Zero rows (m = 0): minimise x1 − x2 with 0 ≤ x ≤ 1. Bounded and feasible, so Optimal with
/// x = (0, 1) and objective −1.
pub fn zero_rows_problem() -> Result<Problem<'static, usize>, Failed> {
    let a = CscRef::new(0, 2, &[0, 0, 0], &[], &[]).map_err(fail)?;
    let rows = RowBounds::SenseRhs { sense: &[], b: &[] };
    Problem::new(
        a,
        &[1.0, -1.0],
        Sense::Minimise,
        rows,
        &[0.0, 0.0],
        &[1.0, 1.0],
    )
    .map_err(fail)
}
pub const ZERO_ROWS_OBJECTIVE: f64 = -1.0;

/// Zero columns (n = 0), one row −1 ≤ (Ax)_1 ≤ 1 where Ax = 0, objective constant γ = 2.5.
/// The only point (the empty x) satisfies the row, so Optimal with objective γ = 2.5.
pub fn zero_columns_problem() -> Result<Problem<'static, usize>, Failed> {
    let a = CscRef::new(1, 0, &[0], &[], &[]).map_err(fail)?;
    let rows = RowBounds::Ranges {
        b_l: &[-1.0],
        b_u: &[1.0],
    };
    Problem::new(a, &[], Sense::Minimise, rows, &[], &[])
        .and_then(|p| p.with_gamma(2.5))
        .map_err(fail)
}
pub const ZERO_COLUMNS_OBJECTIVE: f64 = 2.5;

/// A free row: minimise x1 + x2 subject to x1 + x2 ≥ 1 and −∞ ≤ x1 − x2 ≤ +∞ (free), with
/// 0 ≤ x ≤ 10. Feasible and bounded, so Optimal with objective 1 (x not unique; the free row's
/// dual is 0).
pub fn free_row_problem() -> Result<Problem<'static, usize>, Failed> {
    let a = CscRef::new(2, 2, &[0, 2, 4], &[0, 1, 0, 1], &[1.0, 1.0, 1.0, -1.0]).map_err(fail)?;
    let rows = RowBounds::Ranges {
        b_l: &[1.0, -INF],
        b_u: &[INF, INF],
    };
    Problem::new(
        a,
        &[1.0, 1.0],
        Sense::Minimise,
        rows,
        &[0.0, 0.0],
        &[10.0, 10.0],
    )
    .map_err(fail)
}
pub const FREE_ROW_OBJECTIVE: f64 = 1.0;

/// 2 × 2 identity, for the quadratic objective of the C2 test.
const Q_COL_PTRS: [usize; 3] = [0, 1, 2];
const Q_ROW_INDICES: [usize; 2] = [0, 1];
const Q_VALUES: [f64; 2] = [1.0, 1.0];

/// Solver-independent self-test (runs without any solver): every problem the per-backend
/// tests build, from the small fixtures and in Rust, passes validation, and every small
/// fixture's provenance gives an expected status.
pub fn problems_build() -> Result<(), Failed> {
    for name in ["infeasible", "unbounded", "ranged", "empty_column"] {
        let fx = SmallFixture::load(name)?;
        fx.expected_status()?;
        fx.expected_objective()?;
        if name == "ranged" {
            ensure(fx.problem.has_ranges(), "ranged fixture has RANGES")?;
            let (b_l, b_u) = fx.problem.row_ranges();
            ranged_problem(&fx.problem, &b_l, &b_u)?;
        } else {
            fx.problem.problem(&fx.problem.c).map_err(fail)?;
        }
    }
    zero_rows_problem()?;
    zero_columns_problem()?;
    let p = free_row_problem()?;
    p.with_var_types(b"CI").map_err(fail)?;
    let q = CscRef::new(2, 2, &Q_COL_PTRS, &Q_ROW_INDICES, &Q_VALUES).map_err(fail)?;
    p.with_Q(q).map_err(fail)?;
    let all = corpus_instances().map_err(fail)?;
    ensure(
        all.iter().any(|i| i == LARGEST_INSTANCE),
        format!("{LARGEST_INSTANCE} is not in the corpus"),
    )
}

// ---------------------------------------------------------------------------------------------
// T030: corpus instances × seeds (SC-003; spec US3 scenario 1)

/// Instance `I` of the corpus with the seed at position `S` of its provenance record, solved
/// with [`deterministic_options`]: Optimal, optimal x of length n returned, Verified.
pub fn corpus_seeded<const I: usize, const S: usize>(b: &dyn Backend) -> Result<(), Failed> {
    let name = instance(I)?;
    let (f, seed) = corpus_with_seed(&name, S)?;
    let c = seeded_objective(seed, f.problem.n);
    let p = f.problem.problem(&c).map_err(fail)?;
    let mut o = deterministic_options();
    let sol = b.solve((&p).into(), &mut o).map_err(fail)?;
    expect_optimal_verified(&sol, f.problem.n, &format!("{name} seed {seed}"))
}

/// Instance `I` with its first seeded objective and `Options::default()`: Optimal, Verified.
pub fn corpus_default<const I: usize>(b: &dyn Backend) -> Result<(), Failed> {
    let name = instance(I)?;
    let (f, seed) = corpus_with_seed(&name, 0)?;
    let c = seeded_objective(seed, f.problem.n);
    let p = f.problem.problem(&c).map_err(fail)?;
    let sol = b
        .solve((&p).into(), &mut Options::default())
        .map_err(fail)?;
    expect_optimal_verified(
        &sol,
        f.problem.n,
        &format!("{name} seed {seed}, default options"),
    )
}

/// Slots for corpus instances in the per-instance test tables (the corpus declares 7).
pub const MAX_CORPUS_INSTANCES: usize = 8;
/// Seeds per instance (SC-003: three).
pub const SEEDS_PER_INSTANCE: usize = 3;

macro_rules! seeded_row {
    ($i:literal) => {
        [
            corpus_seeded::<$i, 0> as BackendTest,
            corpus_seeded::<$i, 1> as BackendTest,
            corpus_seeded::<$i, 2> as BackendTest,
        ]
    };
}

/// `corpus_seeded::<I, S>` as function pointers, indexed `[I][S]`.
pub fn seeded_tests() -> [[BackendTest; SEEDS_PER_INSTANCE]; MAX_CORPUS_INSTANCES] {
    [
        seeded_row!(0),
        seeded_row!(1),
        seeded_row!(2),
        seeded_row!(3),
        seeded_row!(4),
        seeded_row!(5),
        seeded_row!(6),
        seeded_row!(7),
    ]
}

/// `corpus_default::<I>` as function pointers, indexed `[I]`.
pub fn default_tests() -> [BackendTest; MAX_CORPUS_INSTANCES] {
    [
        corpus_default::<0>,
        corpus_default::<1>,
        corpus_default::<2>,
        corpus_default::<3>,
        corpus_default::<4>,
        corpus_default::<5>,
        corpus_default::<6>,
        corpus_default::<7>,
    ]
}

// ---------------------------------------------------------------------------------------------
// T031: small fixtures and edge-case problems (spec US3 scenario 2; edge cases)

/// Solves a small infeasible or unbounded fixture with certificates requested. A backend
/// that does not declare certificates must reject the request with `Unsupported`.
fn certificate_case(b: &dyn Backend, name: &'static str) -> Result<(), Failed> {
    let fx = SmallFixture::load(name)?;
    let expected = fx.expected_status()?;
    let p = fx.problem.problem(&fx.problem.c).map_err(fail)?;
    let mut o = Options::default();
    o.certificates = true;
    let caps = b.capabilities();
    if !caps.certificates {
        let r = b.solve((&p).into(), &mut o);
        return ensure(
            matches!(
                r,
                Err(Error::Unsupported {
                    field: "certificates",
                    ..
                })
            ),
            format!("{name}: certificates not declared, expected Unsupported, got {r:?}"),
        );
    }
    let sol = b.solve((&p).into(), &mut o).map_err(fail)?;
    ensure(
        sol.status == expected,
        format!("{name}: expected {expected:?}; {}", describe(&sol)),
    )?;
    let right_kind = match expected {
        Status::Infeasible => matches!(sol.certificate, Some(Certificate::Farkas { .. })),
        Status::Unbounded => matches!(sol.certificate, Some(Certificate::UnboundedRay { .. })),
        _ => false,
    };
    ensure(
        right_kind,
        format!(
            "{name}: certificate {:?} does not match {expected:?}",
            sol.certificate
        ),
    )?;
    // Verified means the certificate measure is evaluated and passes.
    expect_overall(&sol, Overall::Verified, name)
}

/// Infeasible fixture with certificates: Infeasible, Farkas certificate, Verified.
/// Assumes the backend resolves "infeasible or unbounded" into a definite status when
/// certificates are requested.
pub fn t031_infeasible_certificate(b: &dyn Backend) -> Result<(), Failed> {
    certificate_case(b, "infeasible")
}

/// Unbounded fixture with certificates: Unbounded, ray, Verified (same assumption).
pub fn t031_unbounded_certificate(b: &dyn Backend) -> Result<(), Failed> {
    certificate_case(b, "unbounded")
}

/// Ranged rows passed as `RowBounds::Ranges`: Optimal with the provenance objective and x of
/// length n (no auxiliary columns leak into the result), Verified.
pub fn t031_ranged_rows(b: &dyn Backend) -> Result<(), Failed> {
    let fx = SmallFixture::load("ranged")?;
    let (b_l, b_u) = fx.problem.row_ranges();
    let p = ranged_problem(&fx.problem, &b_l, &b_u)?;
    let sol = b
        .solve((&p).into(), &mut Options::default())
        .map_err(fail)?;
    ensure(
        sol.status == fx.expected_status()?,
        format!("ranged: {}", describe(&sol)),
    )?;
    expect_optimal_verified(&sol, fx.problem.n, "ranged")?;
    let expected = fx
        .expected_objective()?
        .ok_or_else(|| fail("ranged: no expected objective"))?;
    expect_objective(&sol, expected, "ranged")
}

/// A column with no constraint entries: Optimal with the provenance objective, Verified.
pub fn t031_empty_column(b: &dyn Backend) -> Result<(), Failed> {
    let fx = SmallFixture::load("empty_column")?;
    let p = fx.problem.problem(&fx.problem.c).map_err(fail)?;
    let sol = b
        .solve((&p).into(), &mut Options::default())
        .map_err(fail)?;
    ensure(
        sol.status == fx.expected_status()?,
        format!("empty_column: {}", describe(&sol)),
    )?;
    expect_optimal_verified(&sol, fx.problem.n, "empty_column")?;
    let expected = fx
        .expected_objective()?
        .ok_or_else(|| fail("empty_column: no expected objective"))?;
    expect_objective(&sol, expected, "empty_column")
}

/// Optimal with the expected objective and x of length n, and verification not Failed. With
/// m = 0 or n = 0 some arrays are empty, and `Solution::view` treats an empty array as not
/// returned, so the dual measures can only be NotEvaluated: PartiallyVerified is accepted.
fn degenerate_case(
    b: &dyn Backend,
    p: &Problem<'_, usize>,
    objective: f64,
    what: &str,
) -> Result<(), Failed> {
    let sol = b.solve(p.into(), &mut Options::default()).map_err(fail)?;
    ensure(
        sol.status == Status::Optimal && sol.returned == SolutionReturned::Optimal,
        format!("{what}: expected Optimal; {}", describe(&sol)),
    )?;
    ensure(
        sol.x.len() == p.n(),
        format!("{what}: |x| = {}, n = {}", sol.x.len(), p.n()),
    )?;
    expect_objective(&sol, objective, what)?;
    let overall = sol.verification.as_ref().map(|r| r.overall);
    ensure(
        matches!(
            overall,
            Some(Overall::Verified | Overall::PartiallyVerified)
        ),
        format!("{what}: verification {overall:?}; {}", describe(&sol)),
    )
}

/// m = 0: Optimal, objective −1 (see [`zero_rows_problem`]).
pub fn t031_zero_rows(b: &dyn Backend) -> Result<(), Failed> {
    degenerate_case(b, &zero_rows_problem()?, ZERO_ROWS_OBJECTIVE, "zero rows")
}

/// n = 0: Optimal, objective γ = 2.5 (see [`zero_columns_problem`]).
pub fn t031_zero_columns(b: &dyn Backend) -> Result<(), Failed> {
    degenerate_case(
        b,
        &zero_columns_problem()?,
        ZERO_COLUMNS_OBJECTIVE,
        "zero columns",
    )
}

/// A free row (−∞, +∞): Optimal, objective 1, Verified (see [`free_row_problem`]).
pub fn t031_free_row(b: &dyn Backend) -> Result<(), Failed> {
    let p = free_row_problem()?;
    let sol = b
        .solve((&p).into(), &mut Options::default())
        .map_err(fail)?;
    expect_optimal_verified(&sol, p.n(), "free row")?;
    expect_objective(&sol, FREE_ROW_OBJECTIVE, "free row")
}

// ---------------------------------------------------------------------------------------------
// T032: time limit (spec US3 scenario 3)

/// Time limit 0 on the largest corpus instance: StoppedAtLimit with reason Time, and the
/// returned field consistent with x (None: x empty; Feasible: x of length n). Assumes the
/// solver cannot finish this instance (1344 × 2942) within a zero time limit.
pub fn t032_time_limit_zero(b: &dyn Backend) -> Result<(), Failed> {
    let (f, seed) = corpus_with_seed(LARGEST_INSTANCE, 0)?;
    let c = seeded_objective(seed, f.problem.n);
    let p = f.problem.problem(&c).map_err(fail)?;
    let mut o = Options::default();
    o.time_limit = Some(0.0);
    let sol = b.solve((&p).into(), &mut o).map_err(fail)?;
    ensure(
        sol.status == Status::StoppedAtLimit && sol.limit_reason == Some(LimitReason::Time),
        format!("expected StoppedAtLimit(Time); {}", describe(&sol)),
    )?;
    match sol.returned {
        SolutionReturned::None => ensure(sol.x.is_empty(), "no solution returned but x is set"),
        SolutionReturned::Feasible => ensure(
            sol.x.len() == f.problem.n,
            format!("feasible solution with |x| = {}", sol.x.len()),
        ),
        SolutionReturned::Optimal => Err(fail("a stopped solve cannot return a proven optimum")),
    }
}

// ---------------------------------------------------------------------------------------------
// T033: raw parameters and a silent default solve (spec US3 scenario 5; constitution III)

/// The vendor parameter a shared options field maps to, from the backend's capabilities.
fn vendor_parameter(b: &dyn Backend, field: &str) -> Result<&'static str, Failed> {
    let caps = b.capabilities();
    caps.options
        .iter()
        .find(|o| o.field == field)
        .and_then(|o| o.vendor_parameters.first().copied())
        .ok_or_else(|| fail(format!("{}: no vendor parameter for {field}", caps.backend)))
}

/// A raw parameter (the vendor parameter behind `random_seed`, value 7) appears unchanged in
/// `effective_parameters`.
pub fn t033_raw_parameter_effective(b: &dyn Backend) -> Result<(), Failed> {
    let name = vendor_parameter(b, "random_seed")?;
    let fx = SmallFixture::load("empty_column")?;
    let p = fx.problem.problem(&fx.problem.c).map_err(fail)?;
    let mut o = Options::default();
    o.raw.push((name.to_string(), ParamValue::Int(7)));
    let sol = b.solve((&p).into(), &mut o).map_err(fail)?;
    ensure(
        sol.status == Status::Optimal,
        format!("raw parameter solve: {}", describe(&sol)),
    )?;
    ensure(
        sol.effective_parameters
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case(name) && *v == ParamValue::Int(7)),
        format!(
            "({name}, Int(7)) missing from {:?}",
            sol.effective_parameters
        ),
    )
}

/// The helper run in a child process by [`t033_default_solve_is_silent`]: one corpus solve
/// with `Options::default()`. In the normal run it is an ordinary solve test.
pub fn t033_quiet_child(b: &dyn Backend) -> Result<(), Failed> {
    let (f, seed) = corpus_with_seed("iDopaNeuroC_problem_1", 0)?;
    let c = seeded_objective(seed, f.problem.n);
    let p = f.problem.problem(&c).map_err(fail)?;
    let sol = b
        .solve((&p).into(), &mut Options::default())
        .map_err(fail)?;
    ensure(
        sol.status == Status::Optimal,
        format!("quiet solve: {}", describe(&sol)),
    )
}

/// Default options write nothing to stdout or stderr: this test binary is re-run as a child
/// process restricted to [`QUIET_CHILD`] for this backend, and its output must hold nothing
/// beyond libtest-mimic's own lines. The child also creates the backend (as every trial does),
/// so environment creation must be silent too (no licence banner; constitution III).
pub fn t033_default_solve_is_silent(b: &dyn Backend) -> Result<(), Failed> {
    let label = format!("{QUIET_CHILD}::{}", b.capabilities().backend);
    let exe = std::env::current_exe().map_err(fail)?;
    let out = std::process::Command::new(exe)
        .args([
            label.as_str(),
            "--exact",
            "--test-threads",
            "1",
            "--color",
            "never",
        ])
        .output()
        .map_err(fail)?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    ensure(
        out.status.success(),
        format!("child run failed ({}):\n{stdout}\n{stderr}", out.status),
    )?;
    ensure(
        stderr.is_empty(),
        format!("solve wrote to stderr:\n{stderr}"),
    )?;
    let ok_line = format!("test {label} ... ok");
    let unexpected: Vec<&str> = stdout
        .lines()
        .filter(|l| {
            !(l.is_empty()
                || *l == "running 1 test"
                || *l == ok_line
                || l.starts_with("test result: ok. 1 passed; 0 failed;"))
        })
        .collect();
    ensure(
        stdout.contains("test result: ok. 1 passed;"),
        format!("child did not run {label} exactly once:\n{stdout}"),
    )?;
    ensure(
        unexpected.is_empty(),
        format!("solve wrote to stdout: {unexpected:?}"),
    )
}

// ---------------------------------------------------------------------------------------------
// T034: cancellation and progress (spec US3 scenario 7)

/// Options that make the largest instance take long enough for progress callbacks: barrier,
/// one thread, first seeded objective. Assumes the backend reports progress at least once
/// during this solve (barrier iterations) before it completes.
fn long_solve() -> Result<(CorpusFixture, Vec<f64>), Failed> {
    let (f, seed) = corpus_with_seed(LARGEST_INSTANCE, 0)?;
    let c = seeded_objective(seed, f.problem.n);
    Ok((f, c))
}

fn long_options<'cb>() -> Options<'cb> {
    let mut o = Options::default();
    o.method = Some(Method::Barrier);
    o.threads = Some(1);
    o
}

/// The progress callback sets the cancellation flag on its first call: at least one progress
/// record is delivered, the status is Interrupted, the returned field is not Optimal, and the
/// records are well-formed (elapsed finite and non-negative, objectives finite when present).
pub fn t034_cancel_from_progress(b: &dyn Backend) -> Result<(), Failed> {
    let (f, c) = long_solve()?;
    let p = f.problem.problem(&c).map_err(fail)?;
    let cancel = AtomicBool::new(false);
    let mut records: Vec<Progress> = Vec::new();
    let mut on_progress = |r: &Progress| {
        records.push(*r);
        cancel.store(true, Ordering::SeqCst);
    };
    let mut o = long_options();
    o.cancel = Some(&cancel);
    o.progress = Some(&mut on_progress);
    let sol = b.solve((&p).into(), &mut o).map_err(fail)?;
    drop(o);
    ensure(
        !records.is_empty(),
        format!("no progress call before the end; {}", describe(&sol)),
    )?;
    ensure(
        sol.status == Status::Interrupted,
        format!(
            "{} progress calls, flag set after the first; {}",
            records.len(),
            describe(&sol)
        ),
    )?;
    ensure(
        sol.returned != SolutionReturned::Optimal,
        "an interrupted solve returned Optimal",
    )?;
    for r in &records {
        ensure(
            r.elapsed.is_finite() && r.elapsed >= 0.0,
            format!("bad elapsed in {r:?}"),
        )?;
        ensure(
            r.primal_objective.is_none_or(|v| !v.is_nan())
                && r.dual_objective.is_none_or(|v| !v.is_nan())
                && r.iterations.is_none_or(|v| v.is_finite() && v >= 0.0),
            format!("malformed progress record {r:?}"),
        )?;
    }
    Ok(())
}

/// Payload of the panic raised by the progress callback.
#[derive(Debug, PartialEq)]
struct ProgressPanic(&'static str);

/// A panic in the progress callback propagates out of `solve` as a panic with the original
/// payload, the process does not abort, and the same backend solves again afterwards. Same
/// assumption as [`t034_cancel_from_progress`] (at least one progress call).
pub fn t034_progress_panic_propagates(b: &dyn Backend) -> Result<(), Failed> {
    let (f, c) = long_solve()?;
    let p = f.problem.problem(&c).map_err(fail)?;
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut on_progress = |_: &Progress| {
            std::panic::panic_any(ProgressPanic("us3 progress callback panic"));
        };
        let mut o = long_options();
        o.progress = Some(&mut on_progress);
        b.solve((&p).into(), &mut o).map(|s| describe(&s))
    }));
    match caught {
        Ok(r) => return Err(fail(format!("solve returned instead of panicking: {r:?}"))),
        Err(payload) => ensure(
            payload.downcast_ref::<ProgressPanic>()
                == Some(&ProgressPanic("us3 progress callback panic")),
            "the panic payload is not the callback's",
        )?,
    }
    let fx = SmallFixture::load("empty_column")?;
    let q = fx.problem.problem(&fx.problem.c).map_err(fail)?;
    let sol = b
        .solve((&q).into(), &mut Options::default())
        .map_err(fail)?;
    expect_optimal_verified(&sol, fx.problem.n, "solve after the callback panic")
}

// ---------------------------------------------------------------------------------------------
// T035: one backend instance, several problems (spec US3 scenario 6)

/// The same backend instance solves three different problems in sequence, then the first
/// again; each result is its own (dimensions, objective, verification). The licence-check
/// count of scenario 6 is not observable through `&dyn Backend`; one instance means one
/// environment, which is what the registry provides.
pub fn t035_one_backend_three_problems(b: &dyn Backend) -> Result<(), Failed> {
    let ranged = SmallFixture::load("ranged")?;
    let (b_l, b_u) = ranged.problem.row_ranges();
    let p1 = ranged_problem(&ranged.problem, &b_l, &b_u)?;
    let obj1 = ranged
        .expected_objective()?
        .ok_or_else(|| fail("ranged objective"))?;

    let (f, seed) = corpus_with_seed("iDopaNeuroC_problem_1", 1)?;
    let c = seeded_objective(seed, f.problem.n);
    let p2 = f.problem.problem(&c).map_err(fail)?;

    let empty = SmallFixture::load("empty_column")?;
    let p3 = empty.problem.problem(&empty.problem.c).map_err(fail)?;
    let obj3 = empty
        .expected_objective()?
        .ok_or_else(|| fail("empty_column objective"))?;

    let s1 = b
        .solve((&p1).into(), &mut Options::default())
        .map_err(fail)?;
    expect_optimal_verified(&s1, p1.n(), "first: ranged")?;
    expect_objective(&s1, obj1, "first: ranged")?;

    let s2 = b
        .solve((&p2).into(), &mut deterministic_options())
        .map_err(fail)?;
    expect_optimal_verified(&s2, p2.n(), "second: corpus")?;
    ensure(
        s2.y.len() == p2.m(),
        format!("second: |y| = {}, m = {}", s2.y.len(), p2.m()),
    )?;

    let s3 = b
        .solve((&p3).into(), &mut Options::default())
        .map_err(fail)?;
    expect_optimal_verified(&s3, p3.n(), "third: empty column")?;
    expect_objective(&s3, obj3, "third: empty column")?;

    let s4 = b
        .solve((&p1).into(), &mut Options::default())
        .map_err(fail)?;
    expect_optimal_verified(&s4, p1.n(), "fourth: ranged again")?;
    expect_objective(&s4, obj1, "fourth: ranged again")
}

// ---------------------------------------------------------------------------------------------
// C1: caller buffers (solve_into)

fn same(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| (x - y).abs() <= 1e-9 * (1.0 + y.abs()))
}

/// `solve_into` with buffers for x, y, z and row activity gives the values `solve` gives
/// (deterministic options, so the two solves follow the same path; compared to 1e-9 relative),
/// and its returned `Solution` has those arrays empty. With only x supplied, x is filled.
pub fn c1_solve_into_matches_solve(b: &dyn Backend) -> Result<(), Failed> {
    let (f, seed) = corpus_with_seed("iDopaNeuroC_problem_1", 2)?;
    let c = seeded_objective(seed, f.problem.n);
    let p = f.problem.problem(&c).map_err(fail)?;
    let (m, n) = (p.m(), p.n());
    let reference = b
        .solve((&p).into(), &mut deterministic_options())
        .map_err(fail)?;
    expect_optimal_verified(&reference, n, "solve")?;

    let (mut x, mut y, mut z, mut ra) = (
        vec![f64::NAN; n],
        vec![f64::NAN; m],
        vec![f64::NAN; n],
        vec![f64::NAN; m],
    );
    let out = SolutionBuffers {
        x: &mut x,
        y: Some(&mut y),
        z: Some(&mut z),
        row_activity: Some(&mut ra),
    };
    let sol = b
        .solve_into((&p).into(), &mut deterministic_options(), out)
        .map_err(fail)?;
    ensure(
        sol.status == Status::Optimal && sol.returned == SolutionReturned::Optimal,
        format!("solve_into: {}", describe(&sol)),
    )?;
    ensure(
        sol.x.is_empty() && sol.y.is_empty() && sol.z.is_empty() && sol.row_activity.is_empty(),
        "solve_into returned owned arrays as well as filling the buffers",
    )?;
    ensure(
        same(&x, &reference.x),
        "x differs between solve and solve_into",
    )?;
    ensure(
        reference.y.is_empty() || same(&y, &reference.y),
        "y differs",
    )?;
    ensure(
        reference.z.is_empty() || same(&z, &reference.z),
        "z differs",
    )?;
    ensure(
        reference.row_activity.is_empty() || same(&ra, &reference.row_activity),
        "row activity differs",
    )?;
    let (Some(o1), Some(o2)) = (reference.objective, sol.objective) else {
        return Err(fail("objective missing"));
    };
    ensure(close(o2, o1), format!("objective {o2} vs {o1}"))?;

    let mut x_only = vec![f64::NAN; n];
    let out = SolutionBuffers {
        x: &mut x_only,
        y: None,
        z: None,
        row_activity: None,
    };
    let sol = b
        .solve_into((&p).into(), &mut deterministic_options(), out)
        .map_err(fail)?;
    ensure(
        sol.status == Status::Optimal,
        format!("x-only solve_into: {}", describe(&sol)),
    )?;
    ensure(same(&x_only, &reference.x), "x-only solve_into: x differs")
}

/// Buffers of the wrong length are an error (x of length n − 1; y of length m + 1).
pub fn c1_solve_into_wrong_lengths(b: &dyn Backend) -> Result<(), Failed> {
    let fx = SmallFixture::load("ranged")?;
    let (b_l, b_u) = fx.problem.row_ranges();
    let p = ranged_problem(&fx.problem, &b_l, &b_u)?;
    let (m, n) = (p.m(), p.n());

    let mut x = vec![0.0; n - 1];
    let out = SolutionBuffers {
        x: &mut x,
        y: None,
        z: None,
        row_activity: None,
    };
    let r = b.solve_into((&p).into(), &mut Options::default(), out);
    ensure(
        r.is_err(),
        format!("x of length n - 1 accepted: {:?}", r.map(|s| describe(&s))),
    )?;

    let (mut x, mut y) = (vec![0.0; n], vec![0.0; m + 1]);
    let out = SolutionBuffers {
        x: &mut x,
        y: Some(&mut y),
        z: None,
        row_activity: None,
    };
    let r = b.solve_into((&p).into(), &mut Options::default(), out);
    ensure(
        r.is_err(),
        format!("y of length m + 1 accepted: {:?}", r.map(|s| describe(&s))),
    )
}

// ---------------------------------------------------------------------------------------------
// C2: problem classes outside the backend's capabilities (FR-013; spec edge case)

/// Expects `Unsupported { field, backend }` from both `solve` and `solve_into`.
fn expect_unsupported(b: &dyn Backend, p: &Problem<'_, usize>, field: &str) -> Result<(), Failed> {
    let backend = b.capabilities().backend;
    let is_expected = |r: &optsolver::error::Result<Solution>| matches!(r, Err(Error::Unsupported { field: f, backend: k }) if *f == field && *k == backend);
    let r = b.solve(p.into(), &mut Options::default());
    ensure(
        is_expected(&r),
        format!(
            "solve: expected Unsupported({field}), got {:?}",
            r.map(|s| describe(&s))
        ),
    )?;
    let mut x = vec![0.0; p.n()];
    let out = SolutionBuffers {
        x: &mut x,
        y: None,
        z: None,
        row_activity: None,
    };
    let r = b.solve_into(p.into(), &mut Options::default(), out);
    ensure(
        is_expected(&r),
        format!(
            "solve_into: expected Unsupported({field}), got {:?}",
            r.map(|s| describe(&s))
        ),
    )
}

/// Variable types (MILP) are rejected with `Unsupported("var_types")` when the backend does
/// not declare MILP. A backend declaring MILP passes vacuously here (MILP tests are separate).
pub fn c2_milp_rejected(b: &dyn Backend) -> Result<(), Failed> {
    if b.capabilities()
        .problem_classes
        .contains(&ProblemClass::Milp)
    {
        return Ok(());
    }
    let p = free_row_problem()?.with_var_types(b"CI").map_err(fail)?;
    expect_unsupported(b, &p, "var_types")
}

/// A quadratic objective (QP) is rejected with `Unsupported("Q")` when the backend does not
/// declare QP. A backend declaring QP passes vacuously here.
pub fn c2_qp_rejected(b: &dyn Backend) -> Result<(), Failed> {
    if b.capabilities().problem_classes.contains(&ProblemClass::Qp) {
        return Ok(());
    }
    let q = CscRef::new(2, 2, &Q_COL_PTRS, &Q_ROW_INDICES, &Q_VALUES).map_err(fail)?;
    let p = free_row_problem()?.with_Q(q).map_err(fail)?;
    expect_unsupported(b, &p, "Q")
}
