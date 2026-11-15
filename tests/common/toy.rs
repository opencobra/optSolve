//! Feature 003: self-contained toy instances (task T014) and their solve rule, shared with the
//! cross-solver comparison (task T017). Each toy instance is a small MPS (LP, QP) or JTASK
//! (conic) file in `tests/fixtures/small/` with a provenance record holding its known outcome
//! (data-model.md of feature 003). Nothing here needs the variational kinetics corpus.

#![allow(non_snake_case)] // Notation Convention (XII-C): A, F are matrices.

use super::fixtures::{Json, Provenance, sha256_hex, toy_path, toy_provenance_path};
use super::jtask::{JtaskProblem, read_jtask_file};
use super::mps::{MpsProblem, read_mps_file};
use super::us3::describe;
use super::{Failed, ensure, fail};
use optsolver::{
    Backend, ConicRef, CscRef, Options, Overall, Problem, ProblemClass, RowBounds, Sense, Solution,
    Status,
};

/// A toy instance read from its file, with its provenance record.
pub enum ToyData {
    /// LP or QP from MPS.
    Mps(MpsProblem),
    /// Conic programme from JTASK.
    Jtask(JtaskProblem),
}

/// A loaded toy instance.
pub struct Toy {
    pub name: String,
    pub data: ToyData,
    pub provenance: Provenance,
}

/// The recorded outcome of a toy instance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Expected {
    pub status: Status,
    /// Optimal objective (minimise form as recorded), when Optimal.
    pub objective: Option<f64>,
}

impl Toy {
    /// Reads `tests/fixtures/small/<name>.{mps,jtask}` and its provenance record, checking the
    /// file's SHA-256 and dimensions against the record.
    pub fn load(name: &str) -> Result<Self, Failed> {
        let path = toy_path(name).map_err(fail)?;
        let provenance =
            Provenance::load(&toy_provenance_path(name).map_err(fail)?).map_err(fail)?;
        let bytes = std::fs::read(&path).map_err(|e| fail(format!("{}: {e}", path.display())))?;
        let recorded = provenance
            .str("file_sha256")
            .or_else(|_| provenance.str("mps_sha256"))
            .map_err(fail)?;
        ensure(
            sha256_hex(&bytes) == recorded,
            format!("{name}: file SHA-256 differs from its provenance record"),
        )?;
        let data = if path.extension().is_some_and(|e| e == "jtask") {
            ToyData::Jtask(read_jtask_file(&path).map_err(fail)?)
        } else {
            ToyData::Mps(read_mps_file(&path).map_err(fail)?)
        };
        let (m, n) = match &data {
            ToyData::Mps(p) => (p.m, p.n),
            ToyData::Jtask(p) => (p.m, p.n),
        };
        ensure(
            provenance.usize("m").map_err(fail)? == m && provenance.usize("n").map_err(fail)? == n,
            format!("{name}: dimensions differ from the provenance record"),
        )?;
        Ok(Self {
            name: name.to_string(),
            data,
            provenance,
        })
    }

    /// The recorded outcome.
    pub fn expected(&self) -> Result<Expected, Failed> {
        let status = match self.provenance.str("expected_status").map_err(fail)? {
            "Optimal" => Status::Optimal,
            "Infeasible" => Status::Infeasible,
            "Unbounded" => Status::Unbounded,
            other => return Err(fail(format!("{}: expected_status {other}", self.name))),
        };
        let objective = match self.provenance.fields.get("expected_objective") {
            Some(Json::Number(v)) => Some(*v),
            _ => None,
        };
        Ok(Expected { status, objective })
    }

    /// Whether the conic form is solvable in maximise form too (conic instances only).
    pub fn is_conic(&self) -> bool {
        matches!(self.data, ToyData::Jtask(_))
    }

    /// Whether `b` supports this instance's class and cone kinds; otherwise the field it would
    /// report as unsupported.
    pub fn unsupported_field(&self, b: &dyn Backend) -> Option<&'static str> {
        let caps = b.capabilities();
        let has = |c| caps.problem_classes.contains(&c);
        match &self.data {
            ToyData::Mps(p) if !p.q_col_ptrs.is_empty() && !has(ProblemClass::Qp) => Some("Q"),
            ToyData::Mps(_) if !has(ProblemClass::Lp) => Some("LP"),
            ToyData::Mps(_) => None,
            ToyData::Jtask(p) if p.k > 0 => {
                if !has(ProblemClass::Conic) {
                    return Some("conic");
                }
                p.cones
                    .iter()
                    .map(|c| c.kind())
                    .find(|k| !caps.cones.contains(k))
                    .map(|k| k.name())
            }
            ToyData::Jtask(_) => (!has(ProblemClass::Lp)).then_some("LP"),
        }
    }

    /// Solves the instance on `b` (in maximise form when `maximise`: c negated, sense flipped,
    /// conic instances only), requesting certificates when the recorded outcome is not Optimal.
    pub fn solve(
        &self,
        b: &dyn Backend,
        maximise: bool,
    ) -> Result<optsolver::error::Result<Solution>, Failed> {
        let mut o = Options::default();
        o.certificates = self.expected()?.status != Status::Optimal;
        Ok(match &self.data {
            ToyData::Mps(p) if p.has_ranges() => {
                // RANGES rows need the ranges form of the row bounds.
                let (b_l, b_u) = p.row_ranges();
                let A = p.A().map_err(fail)?;
                let rows = RowBounds::Ranges {
                    b_l: &b_l,
                    b_u: &b_u,
                };
                let mut problem = Problem::new(A, &p.c, p.objective_sense, rows, &p.l, &p.u)
                    .and_then(|q| q.with_gamma(p.gamma))
                    .map_err(fail)?;
                if let Some(q) = p.Q().map_err(fail)? {
                    problem = problem.with_Q(q).map_err(fail)?;
                }
                b.solve((&problem).into(), &mut o)
            }
            ToyData::Mps(p) => {
                let problem = p.problem(&p.c).map_err(fail)?;
                b.solve((&problem).into(), &mut o)
            }
            ToyData::Jtask(p) => {
                let c: Vec<f64> = if maximise {
                    p.c.iter().map(|v| -v).collect()
                } else {
                    p.c.clone()
                };
                let sense = match (p.sense, maximise) {
                    (s, false) => s,
                    (Sense::Minimise, true) => Sense::Maximise,
                    (Sense::Maximise, true) => Sense::Minimise,
                };
                let problem = jtask_problem(p, &c, sense)?;
                b.solve((&problem).into(), &mut o)
            }
        })
    }
}

/// The JTASK problem with objective `c` and `sense` (the maximise form uses its own c).
fn jtask_problem<'a>(
    p: &'a JtaskProblem,
    c: &'a [f64],
    sense: Sense,
) -> Result<Problem<'a, usize>, Failed> {
    let A = CscRef::new(p.m, p.n, &p.A_col_ptrs, &p.A_row_indices, &p.A_values).map_err(fail)?;
    let rows = RowBounds::Ranges {
        b_l: &p.b_l,
        b_u: &p.b_u,
    };
    let problem = Problem::new(A, c, sense, rows, &p.l, &p.u)
        .and_then(|q| q.with_gamma(if sense == p.sense { p.gamma } else { -p.gamma }))
        .map_err(fail)?;
    if p.k == 0 {
        return Ok(problem);
    }
    let F = CscRef::new(p.k, p.n, &p.F_col_ptrs, &p.F_row_indices, &p.F_values).map_err(fail)?;
    problem
        .with_conic(ConicRef::new(F, &p.g, &p.cones).map_err(fail)?)
        .map_err(fail)
}

/// The effective dual tolerance the solution was verified against (its gap measure's).
pub fn gap_tolerance(sol: &Solution) -> Option<f64> {
    sol.verification.as_ref().and_then(|r| {
        r.measures
            .iter()
            .find(|q| q.name == optsolver::verify::names::GAP)
            .map(|q| q.tolerance)
    })
}

/// Agreement of two objective values (feature 002 research R10 with R19's normalisation):
/// |a − b| ≤ 10 · τ · max(1, min(|a|, |b|)).
pub fn objectives_agree(a: f64, b: f64, tau: f64) -> bool {
    (a - b).abs() <= 10.0 * tau * 1f64.max(a.abs().min(b.abs()))
}

/// The outcome rule of a toy instance on one backend (T014): its recorded status; when
/// Optimal, its recorded objective (sign-flipped in maximise form) within the agreement rule
/// and a Verified report; when Infeasible or Unbounded, Verified with a certificate, or
/// PartiallyVerified without one (a backend that documents none, feature 003 research R4).
pub fn check_outcome(toy: &Toy, sol: &Solution, maximise: bool) -> Result<(), Failed> {
    let what = format!(
        "{}{}",
        toy.name,
        if maximise { " (maximise form)" } else { "" }
    );
    let expected = toy.expected()?;
    ensure(
        sol.status == expected.status,
        format!("{what}: expected {:?}; {}", expected.status, describe(sol)),
    )?;
    let overall = sol.verification.as_ref().map(|r| r.overall);
    match expected.status {
        Status::Optimal => {
            let want = expected
                .objective
                .ok_or_else(|| fail(format!("{what}: no expected objective")))?;
            let want = if maximise { -want } else { want };
            let got = sol
                .objective
                .ok_or_else(|| fail(format!("{what}: no objective")))?;
            let tau = gap_tolerance(sol).ok_or_else(|| fail(format!("{what}: no gap measure")))?;
            ensure(
                objectives_agree(got, want, tau),
                format!("{what}: objective {got:e}, expected {want:e} (τ {tau:e})"),
            )?;
            ensure(
                overall == Some(Overall::Verified),
                format!("{what}: {}", describe(sol)),
            )
        }
        _ => ensure(
            (sol.certificate.is_some() && overall == Some(Overall::Verified))
                || (sol.certificate.is_none() && overall == Some(Overall::PartiallyVerified)),
            format!(
                "{what}: certificate {:?}; {}",
                sol.certificate,
                describe(sol)
            ),
        ),
    }
}

/// T014: one toy instance on one backend (both forms for conic instances).
pub fn toy_trial(b: &dyn Backend, name: &str) -> Result<(), Failed> {
    let toy = Toy::load(name)?;
    let forms: &[bool] = if toy.is_conic() {
        &[false, true]
    } else {
        &[false]
    };
    for &maximise in forms {
        let r = toy.solve(b, maximise)?;
        match toy.unsupported_field(b) {
            Some(field) => match r {
                Err(optsolver::Error::Unsupported { field: f, .. }) if f == field => {}
                other => {
                    return Err(fail(format!(
                        "{name}: expected Unsupported {field}, got {:?}",
                        other.map(|s| describe(&s))
                    )));
                }
            },
            None => check_outcome(&toy, &r.map_err(fail)?, maximise)?,
        }
    }
    Ok(())
}
