//! Feature 003 (task T017; research R11): cross-solver comparison. For every pair of declared
//! backends and every comparable instance — each toy instance, the LP corpus fixture with each
//! of its seeds, and the derived corpus QP — both backends solve the instance and the outcomes
//! are compared: equal status; when Optimal, objectives within feature 002 R10 (R19
//! normalisation) and both reports Verified. A pair with an unavailable member, and an instance
//! one member does not support, give ignored trials with the reason (never passed). Each
//! comparison appends a line to the run's report.

use super::toy::{Toy, gap_tolerance, objectives_agree};
use super::us3::describe;
use super::{Availability, DeclaredBackend, Failed, Registry, Trial, ensure, fail};
use optsolver::{Backend, Overall, ProblemClass, Solution, Status};
use std::io::Write;
use std::sync::{Mutex, OnceLock};

/// One instance to compare.
#[derive(Debug, Clone)]
pub enum Item {
    /// A toy instance by name.
    Toy(String),
    /// The LP corpus fixture with the seed at this position of its provenance record.
    LpSeed(String, usize),
    /// The derived corpus QP.
    DerivedQp,
}

impl Item {
    fn label(&self) -> String {
        match self {
            Item::Toy(n) => format!("toy:{n}"),
            Item::LpSeed(n, s) => format!("lp:{n}:seed{}", s + 1),
            Item::DerivedQp => format!("qp:{}", super::f002::DERIVED_QP),
        }
    }

    /// The field `b` would report as unsupported for this item, if any.
    fn unsupported(&self, b: &dyn Backend) -> Result<Option<&'static str>, String> {
        let caps = b.capabilities();
        Ok(match self {
            Item::Toy(n) => Toy::load(n)
                .map_err(|e| format!("{e:?}"))?
                .unsupported_field(b),
            Item::LpSeed(..) => (!caps.problem_classes.contains(&ProblemClass::Lp)).then_some("LP"),
            Item::DerivedQp => (!caps.problem_classes.contains(&ProblemClass::Qp)).then_some("Q"),
        })
    }

    fn solve(&self, b: &dyn Backend) -> Result<Solution, Failed> {
        match self {
            Item::Toy(n) => Toy::load(n)?.solve(b, false)?.map_err(fail),
            Item::LpSeed(n, s) => {
                let (f, seed) = super::us3::corpus_with_seed(n, *s)?;
                let c = super::objectives::seeded_objective(seed, f.problem.n);
                let p = f.problem.problem(&c).map_err(fail)?;
                b.solve((&p).into(), &mut optsolver::Options::default())
                    .map_err(fail)
            }
            Item::DerivedQp => super::f002::solve_derived_qp(b)?.2.map_err(fail),
        }
    }
}

/// What a comparison needs from one solve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outcome {
    pub status: Status,
    pub objective: Option<f64>,
    pub tolerance: Option<f64>,
    pub overall: Option<Overall>,
}

impl Outcome {
    fn of(sol: &Solution) -> Self {
        Self {
            status: sol.status,
            objective: sol.objective,
            tolerance: gap_tolerance(sol),
            overall: sol.verification.as_ref().map(|r| r.overall),
        }
    }
}

/// The comparison rule; returns (absolute difference, relative difference, tolerance) for the
/// report, or the disagreement.
pub fn judge(a: &Outcome, b: &Outcome) -> Result<(f64, f64, f64), String> {
    if a.status != b.status {
        return Err(format!(
            "statuses differ: {:?} and {:?}",
            a.status, b.status
        ));
    }
    if a.status != Status::Optimal {
        return Ok((f64::NAN, f64::NAN, f64::NAN));
    }
    let (Some(pa), Some(pb)) = (a.objective, b.objective) else {
        return Err("an Optimal outcome without an objective".into());
    };
    let tau = match (a.tolerance, b.tolerance) {
        (Some(x), Some(y)) => x.max(y),
        _ => return Err("an outcome without a verified gap tolerance".into()),
    };
    if a.overall != Some(Overall::Verified) || b.overall != Some(Overall::Verified) {
        return Err(format!(
            "verification {:?} and {:?}, both must be Verified",
            a.overall, b.overall
        ));
    }
    let abs = (pa - pb).abs();
    let rel = abs / 1f64.max(pa.abs().min(pb.abs()));
    if !objectives_agree(pa, pb, tau) {
        return Err(format!(
            "objectives {pa:e} and {pb:e} differ by {abs:e} (relative {rel:e}) > 10 · {tau:e}"
        ));
    }
    Ok((abs, rel, tau))
}

/// The run's report file, created on first use under the feature's run bundle.
fn report() -> &'static Mutex<Option<(std::path::PathBuf, std::fs::File)>> {
    static REPORT: OnceLock<Mutex<Option<(std::path::PathBuf, std::fs::File)>>> = OnceLock::new();
    REPORT.get_or_init(|| {
        let stamp = std::process::Command::new("date")
            .args(["-u", "+%Y%m%dT%H%M%SZ"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown-time".into());
        let dir = super::fixtures::repo_root()
            .join("results/003-llm-interface-toy-instances/runs")
            .join(format!("{stamp}-compare"));
        let file = std::fs::create_dir_all(&dir).ok().and_then(|_| {
            let path = dir.join("report.tsv");
            let mut f = std::fs::File::create(&path).ok()?;
            writeln!(
                f,
                "instance\tbackend_a\tbackend_b\tstatus_a\tstatus_b\tobjective_a\tobjective_b\tabs_diff\trel_diff\ttolerance\tverdict"
            )
            .ok()?;
            Some((path, f))
        });
        Mutex::new(file)
    })
}

fn write_report(line: &str) {
    if let Ok(mut guard) = report().lock() {
        if let Some((_, f)) = guard.as_mut() {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// Path of this run's report, if one was created.
pub fn report_path() -> Option<std::path::PathBuf> {
    report().lock().ok()?.as_ref().map(|(p, _)| p.clone())
}

fn compare(item: &Item, a: &dyn Backend, b: &dyn Backend) -> Result<(), Failed> {
    let (na, nb) = (a.capabilities().backend, b.capabilities().backend);
    let sa = item.solve(a)?;
    let sb = item.solve(b)?;
    let (oa, ob) = (Outcome::of(&sa), Outcome::of(&sb));
    let verdict = judge(&oa, &ob);
    let num = |v: Option<f64>| v.map_or("".into(), |x| format!("{x:e}"));
    let (abs, rel, tol) = verdict.clone().unwrap_or((f64::NAN, f64::NAN, f64::NAN));
    write_report(&format!(
        "{}\t{na}\t{nb}\t{:?}\t{:?}\t{}\t{}\t{abs:e}\t{rel:e}\t{tol:e}\t{}",
        item.label(),
        oa.status,
        ob.status,
        num(oa.objective),
        num(ob.objective),
        match &verdict {
            Ok(_) => "agree".to_string(),
            Err(e) => format!("disagree: {e}"),
        }
    ));
    verdict.map(|_| ()).map_err(|e| {
        fail(format!(
            "{}: {na} and {nb} disagree: {e}\n  {na}: {}\n  {nb}: {}",
            item.label(),
            describe(&sa),
            describe(&sb)
        ))
    })
}

/// Every item to compare (research R11).
pub fn items() -> Result<Vec<Item>, String> {
    let mut v: Vec<Item> = super::fixtures::toy_instances()?
        .into_iter()
        .map(Item::Toy)
        .collect();
    for lp in super::fixtures::lp_instances()? {
        for s in 0..3 {
            v.push(Item::LpSeed(lp.clone(), s));
        }
    }
    v.push(Item::DerivedQp);
    Ok(v)
}

/// Comparison trials for every pair of declared backends in `registry` (FR-011, FR-012).
pub fn trials(registry: &Registry, items: &[Item]) -> Vec<Trial> {
    let mut out = Vec::new();
    let reason = |b: &DeclaredBackend, a: &Availability| match (a, b.make) {
        (Availability::Available, Some(_)) => None,
        (Availability::Unavailable(r), _) => {
            Some(format!("{}: unavailable: {}", b.name, r.replace('\n', " ")))
        }
        _ => Some(format!("{}: not compiled", b.name)),
    };
    for (i, (b1, a1)) in registry.entries.iter().enumerate() {
        for (b2, a2) in &registry.entries[i + 1..] {
            let pair = format!("{}+{}", b1.name, b2.name);
            let missing: Vec<String> = reason(b1, a1).into_iter().chain(reason(b2, a2)).collect();
            let (Some(m1), Some(m2)) = (b1.make, b2.make) else {
                out.extend(items.iter().map(|it| {
                    ignored(format!(
                        "compare::{}::{pair} [{}]",
                        it.label(),
                        missing.join("; ")
                    ))
                }));
                continue;
            };
            if !missing.is_empty() {
                out.extend(items.iter().map(|it| {
                    ignored(format!(
                        "compare::{}::{pair} [{}]",
                        it.label(),
                        missing.join("; ")
                    ))
                }));
                continue;
            }
            // Both available: read capabilities once to decide comparability.
            let (x, y) = match (m1(), m2()) {
                (Ok(x), Ok(y)) => (x, y),
                (r1, r2) => {
                    let why = format!("{:?} {:?}", r1.err(), r2.err());
                    out.extend(
                        items
                            .iter()
                            .map(|it| ignored(format!("compare::{}::{pair} [{why}]", it.label()))),
                    );
                    continue;
                }
            };
            for item in items {
                let label = format!("compare::{}::{pair}", item.label());
                let not = match (item.unsupported(&*x), item.unsupported(&*y)) {
                    (Ok(None), Ok(None)) => None,
                    (Ok(Some(f)), _) => Some(format!("not comparable: {} lacks {f}", b1.name)),
                    (_, Ok(Some(f))) => Some(format!("not comparable: {} lacks {f}", b2.name)),
                    (Err(e), _) | (_, Err(e)) => {
                        out.push(Trial::test(label, move || Err(fail(&e))));
                        continue;
                    }
                };
                match not {
                    Some(why) => out.push(ignored(format!("{label} [{why}]"))),
                    None => {
                        let item = item.clone();
                        out.push(Trial::test(label, move || {
                            let a = m1().map_err(Failed::from)?;
                            let b = m2().map_err(Failed::from)?;
                            compare(&item, &*a, &*b)
                        }));
                    }
                }
            }
        }
    }
    out
}

fn ignored(name: String) -> Trial {
    Trial::test(name, || Ok(())).with_ignored_flag(true)
}

/// Guard self-tests (constitution XV-C): the rule rejects disagreements and unverified
/// outcomes, and a registry with a missing backend yields only ignored comparison trials.
pub fn self_tests() -> Vec<Trial> {
    vec![
        Trial::test("compare::self_test::judge", || {
            let ok = Outcome {
                status: Status::Optimal,
                objective: Some(5.0),
                tolerance: Some(1e-8),
                overall: Some(Overall::Verified),
            };
            ensure(judge(&ok, &ok).is_ok(), "identical outcomes agree")?;
            let far = Outcome {
                objective: Some(5.001),
                ..ok
            };
            ensure(judge(&ok, &far).is_err(), "objectives 5 and 5.001 disagree")?;
            let infeasible = Outcome {
                status: Status::Infeasible,
                ..ok
            };
            ensure(
                judge(&ok, &infeasible).is_err(),
                "different statuses disagree",
            )?;
            let failed = Outcome {
                overall: Some(Overall::Failed),
                ..ok
            };
            ensure(judge(&ok, &failed).is_err(), "a Failed report never agrees")?;
            ensure(
                judge(&infeasible, &infeasible).is_ok(),
                "equal non-optimal statuses agree",
            )
        }),
        Trial::test("compare::self_test::missing_backend_ignored", || {
            let registry = Registry::from_declared(vec![
                DeclaredBackend {
                    name: "a",
                    make: Some(|| Ok(Box::new(super::fake_backend::FakeBackend::lp_only()))),
                },
                DeclaredBackend {
                    name: "absent",
                    make: None,
                },
            ]);
            let trials = trials(&registry, &[Item::Toy("toy_lp_max".into())]);
            ensure(
                trials.len() == 1
                    && trials[0].has_ignored_flag()
                    && trials[0].name().contains("absent: not compiled"),
                format!(
                    "{:?}",
                    trials
                        .iter()
                        .map(|t| t.name().to_string())
                        .collect::<Vec<_>>()
                ),
            )
        }),
    ]
}

// ---------------------------------------------------------------------------------------------
// Feature 005 (task T003): agreement between the C interface and the Rust API on the same
// backend, instance and options — the rule of specs/005-c-abi/spec.md, Clarifications.

/// What one path returned, in a form both paths can be reduced to.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Observed {
    pub status: i32,
    pub limit_reason: i32,
    pub returned: i32,
    pub verification: i32,
    /// Names of the measures whose verdict is Fail.
    pub failed_measures: Vec<String>,
    pub objective: Option<f64>,
    pub bound: Option<f64>,
    /// Arrays by name (`x`, `y`, `z`, `z_l`, `z_u`, `s`, `row_activity`, certificate arrays);
    /// `None` when absent.
    pub arrays: Vec<(String, Option<Vec<f64>>)>,
    /// Effective tolerance of the row-bound measure (primal quantities).
    pub primal_tolerance: Option<f64>,
    /// Effective tolerance of the reduced-cost consistency measure (dual quantities).
    pub dual_tolerance: Option<f64>,
}

/// Arrays judged against the primal tolerance; every other array against the dual one.
const PRIMAL_ARRAYS: [&str; 3] = ["x", "row_activity", "cert_ray"];

/// The fallback tolerance (Gurobi 13.0's default `FeasibilityTol` and `OptimalityTol`).
pub const FALLBACK_TOLERANCE: f64 = 1e-6;

fn within(a: f64, b: f64, tol: f64) -> bool {
    if a == b || (a.is_nan() && b.is_nan()) {
        return true;
    }
    (a - b).abs() <= 10.0 * tol * 1f64.max(b.abs())
}

/// Judges the C result `c` against the Rust result `r`: `Ok(true)` when everything agrees
/// bit for bit, `Ok(false)` when it agrees within the bound but not bit for bit, `Err` naming
/// the first disagreement. An "everything failed" pair (both without status) is
/// uninterpretable, never agreement.
pub fn agree_c_rust(c: &Observed, r: &Observed) -> Result<bool, String> {
    if c.status == 0 || r.status == 0 {
        return Err("uninterpretable: a path returned no status".into());
    }
    let same = |what: &str, a: &dyn std::fmt::Debug, b: &dyn std::fmt::Debug| {
        let (a, b) = (format!("{a:?}"), format!("{b:?}"));
        if a == b {
            Ok(())
        } else {
            Err(format!("{what}: C {a}, Rust {b}"))
        }
    };
    same("status", &c.status, &r.status)?;
    same("limit reason", &c.limit_reason, &r.limit_reason)?;
    same("solution returned", &c.returned, &r.returned)?;
    same("verification", &c.verification, &r.verification)?;
    same("failed measures", &c.failed_measures, &r.failed_measures)?;
    let p = r.primal_tolerance.unwrap_or(FALLBACK_TOLERANCE);
    let d = r.dual_tolerance.unwrap_or(FALLBACK_TOLERANCE);
    let mut exact = true;
    for (what, a, b, tol) in [
        ("objective", c.objective, r.objective, p),
        ("bound", c.bound, r.bound, d),
    ] {
        match (a, b) {
            (None, None) => {}
            (Some(a), Some(b)) => {
                if !within(a, b, tol) {
                    return Err(format!("{what}: C {a:e}, Rust {b:e} (tolerance {tol:e})"));
                }
                exact &= a.to_bits() == b.to_bits();
            }
            _ => {
                return Err(format!(
                    "{what}: present in one path only (C {a:?}, Rust {b:?})"
                ));
            }
        }
    }
    for (name, rv) in &r.arrays {
        let cv = c
            .arrays
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| format!("array {name}: missing from the C result"))?;
        match (&cv, rv) {
            (None, None) => {}
            (Some(a), Some(b)) => {
                if a.len() != b.len() {
                    return Err(format!(
                        "array {name}: length C {}, Rust {}",
                        a.len(),
                        b.len()
                    ));
                }
                let tol = if PRIMAL_ARRAYS.contains(&name.as_str()) {
                    p
                } else {
                    d
                };
                for (i, (x, y)) in a.iter().zip(b).enumerate() {
                    if !within(*x, *y, tol) {
                        return Err(format!(
                            "array {name}[{i}]: C {x:e}, Rust {y:e} (tolerance {tol:e})"
                        ));
                    }
                    exact &= x.to_bits() == y.to_bits();
                }
            }
            _ => {
                return Err(format!(
                    "array {name}: present in one path only (C {}, Rust {})",
                    cv.is_some(),
                    rv.is_some()
                ));
            }
        }
    }
    Ok(exact)
}

/// Observed form of a Rust solution (arrays absent when empty, as the C interface reports).
#[cfg(feature = "capi")]
pub fn observe_rust(s: &Solution) -> Observed {
    let arr = |v: &[f64]| (!v.is_empty()).then(|| v.to_vec());
    let tol = |name: &str| {
        s.verification.as_ref().and_then(|r| {
            r.measures
                .iter()
                .find(|m| m.name == name)
                .map(|m| m.tolerance)
        })
    };
    let (cy, cz, cs, cr) = match &s.certificate {
        Some(optsolver::Certificate::Farkas { y, z, s, .. }) => (arr(y), arr(z), arr(s), None),
        Some(optsolver::Certificate::UnboundedRay { r }) => (None, None, None, arr(r)),
        _ => (None, None, None, None),
    };
    Observed {
        status: optsolver::capi::status_code(s.status),
        limit_reason: optsolver::capi::limit_code(s.limit_reason),
        returned: optsolver::capi::returned_code(s.returned),
        verification: optsolver::capi::overall_code(s.verification.as_ref().map(|r| r.overall)),
        failed_measures: s
            .verification
            .as_ref()
            .map(|r| {
                r.measures
                    .iter()
                    .filter(|m| m.verdict == optsolver::Verdict::Fail)
                    .map(|m| m.name.to_string())
                    .collect()
            })
            .unwrap_or_default(),
        objective: s.objective,
        bound: s.bound,
        arrays: vec![
            ("x".into(), arr(&s.x)),
            ("y".into(), arr(&s.y)),
            ("z".into(), arr(&s.z)),
            ("z_l".into(), arr(&s.z_l)),
            ("z_u".into(), arr(&s.z_u)),
            ("s".into(), arr(&s.s)),
            ("row_activity".into(), arr(&s.row_activity)),
            ("cert_y".into(), cy),
            ("cert_z".into(), cz),
            ("cert_s".into(), cs),
            ("cert_ray".into(), cr),
        ],
        primal_tolerance: tol(optsolver::verify::names::ROW_BOUND),
        dual_tolerance: tol(optsolver::verify::names::REDUCED_COST),
    }
}

/// Licence-free tests of [`agree_c_rust`] against the failures it guards (XV-C).
pub fn agreement_self_tests() -> Vec<Trial> {
    fn base() -> Observed {
        Observed {
            status: 1,
            returned: 2,
            verification: 1,
            objective: Some(2.0),
            arrays: vec![("x".into(), Some(vec![1.0, 2.0])), ("y".into(), None)],
            primal_tolerance: Some(1e-6),
            dual_tolerance: Some(1e-6),
            ..Observed::default()
        }
    }
    let t = |name: &'static str, f: fn() -> Result<(), Failed>| Trial::test(name, f);
    vec![
        t("t003::agreement_identical_is_exact", || {
            ensure(
                agree_c_rust(&base(), &base()) == Ok(true),
                "identical results",
            )
        }),
        t("t003::agreement_within_bound_accepted_not_exact", || {
            let mut c = base();
            c.arrays[0].1 = Some(vec![1.0 + 5e-6, 2.0]);
            ensure(
                agree_c_rust(&c, &base()) == Ok(false),
                "within 10 tolerances",
            )
        }),
        t("t003::agreement_beyond_bound_rejected", || {
            let mut c = base();
            c.arrays[0].1 = Some(vec![1.0 + 2e-5, 2.0]);
            ensure(agree_c_rust(&c, &base()).is_err(), "beyond 10 tolerances")
        }),
        t(
            "t003::agreement_status_limit_names_presence_rejected",
            || {
                for change in 0..5 {
                    let mut c = base();
                    match change {
                        0 => c.status = 2,
                        1 => c.limit_reason = 8,
                        2 => c.failed_measures = vec!["primal-dual gap".into()],
                        3 => c.arrays[1].1 = Some(vec![0.0]),
                        _ => c.objective = None,
                    }
                    ensure(
                        agree_c_rust(&c, &base()).is_err(),
                        format!("change {change} accepted"),
                    )?;
                }
                Ok(())
            },
        ),
        t(
            "t003::agreement_everything_failed_is_uninterpretable",
            || {
                let none = Observed::default();
                ensure(
                    agree_c_rust(&none, &none).is_err_and(|e| e.contains("uninterpretable")),
                    "everything failed must not be agreement",
                )
            },
        ),
    ]
}
