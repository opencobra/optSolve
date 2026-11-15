//! `Backend` implementation for Gurobi (task T045): checks, bulk load, single parameter
//! application, optimisation with the optional callback, extraction of every available output,
//! and independent verification (constitution I, III, VII-G, XVI).

#![allow(non_snake_case)] // Notation Convention (XII-C): A is a matrix.

use core::ffi::{CStr, c_double, c_int};
use std::time::Instant;

use super::env::{self, Env};
use super::load::{self, Loaded};
use super::{callback, ffi, params, status};
use crate::backend::{Backend, Capabilities, ProblemClass, ProblemRef};
use crate::error::{Error, Result};
use crate::options::{Options, ParamValue, check};
use crate::result::{
    Basis, BasisStatus, Certificate, Counts, Solution, SolutionBuffers, SolutionReturned,
    SolutionView, SolverIdentity, Status, Timings,
};
use crate::verify::{Tolerances, verify};

impl Env {
    fn identity(&self) -> SolverIdentity {
        let (a, b, c) = self.version();
        SolverIdentity {
            name: "gurobi".to_string(),
            version: format!("{a}.{b}.{c}"),
        }
    }
}

impl Backend for Env {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            backend: "gurobi",
            problem_classes: vec![ProblemClass::Lp],
            options: params::MAPPINGS.to_vec(),
            certificates: true,
            basis: true,
            solver: self.identity(),
        }
    }

    fn solve(&self, problem: ProblemRef<'_, '_>, options: &mut Options<'_>) -> Result<Solution> {
        run(self, problem, options, None)
    }

    fn solve_into(
        &self,
        problem: ProblemRef<'_, '_>,
        options: &mut Options<'_>,
        out: SolutionBuffers<'_>,
    ) -> Result<Solution> {
        let (m, n) = dims(problem);
        let bad = |what: &'static str, len: usize, want: usize| Error::Invalid {
            what,
            detail: format!("buffer length {len}, expected {want}"),
        };
        if out.x.len() != n {
            return Err(bad("out.x", out.x.len(), n));
        }
        if let Some(y) = &out.y {
            if y.len() != m {
                return Err(bad("out.y", y.len(), m));
            }
        }
        if let Some(z) = &out.z {
            if z.len() != n {
                return Err(bad("out.z", z.len(), n));
            }
        }
        if let Some(r) = &out.row_activity {
            if r.len() != m {
                return Err(bad("out.row_activity", r.len(), m));
            }
        }
        for (what, buf) in [
            ("out.z_l", out.z_l.as_deref()),
            ("out.z_u", out.z_u.as_deref()),
        ] {
            if let Some(b) = buf {
                if b.len() != n {
                    return Err(bad(what, b.len(), n));
                }
            }
        }
        run(self, problem, options, Some(out))
    }
}

fn dims(problem: ProblemRef<'_, '_>) -> (usize, usize) {
    match problem {
        ProblemRef::Usize(p) => (p.m(), p.n()),
        ProblemRef::I32(p) => (p.m(), p.n()),
    }
}

fn run(
    env: &Env,
    problem: ProblemRef<'_, '_>,
    options: &mut Options<'_>,
    out: Option<SolutionBuffers<'_>>,
) -> Result<Solution> {
    let caps = env.capabilities();
    check::check_problem(problem, &caps)?;
    check::check(options, &caps)?;

    let t_load = Instant::now();
    let loaded = load::load(env, problem)?;
    let model = loaded.model.as_ptr();
    // SAFETY: `model` is a live model owned by `loaded`; GRBgetenv returns the model's own
    // environment, valid while the model lives (Gurobi C reference, GRBgetenv).
    let model_env = unsafe { ffi::GRBgetenv(model) };
    let effective_parameters = params::apply(model_env, options)?;
    let tolerances = params::effective_tolerances(model_env)?;
    let load_seconds = t_load.elapsed().as_secs_f64();

    let progress = options.progress.take();
    let mut state = callback::CallbackState::new(options.cancel, progress);
    callback::install(model, &mut state)?;
    let t_solve = Instant::now();
    // SAFETY: `model` is live; the callback state (if installed) outlives the call and is not
    // moved until `uninstall` below.
    let rc = unsafe { ffi::GRBoptimize(model) };
    let solve_seconds = t_solve.elapsed().as_secs_f64();
    callback::uninstall(model)?;
    let panic = callback::take_panic(&mut state);
    options.progress = state.into_progress();
    if let Some(payload) = panic {
        drop(loaded);
        std::panic::resume_unwind(payload);
    }
    if rc != 0 {
        return Err(env::error(model_env, rc));
    }

    let t_extract = Instant::now();
    let mut solution = extract(env, &loaded, problem, options, out)?;
    solution.effective_parameters = effective_parameters;
    let extraction_seconds = t_extract.elapsed().as_secs_f64();

    let t_verify = Instant::now();
    if options.effective_verification() {
        solution.verification = Some(run_verify(problem, &solution, &tolerances));
    }
    solution.timings = Timings {
        load: load_seconds,
        solve: solve_seconds,
        extraction: extraction_seconds,
        verification: t_verify.elapsed().as_secs_f64(),
    };
    Ok(solution)
}

fn run_verify(
    problem: ProblemRef<'_, '_>,
    solution: &Solution,
    tolerances: &Tolerances,
) -> crate::verify::VerificationReport {
    let view: SolutionView<'_> = solution.view();
    match problem {
        ProblemRef::Usize(p) => verify(p, &view, tolerances),
        ProblemRef::I32(p) => verify(p, &view, tolerances),
    }
}

fn int_attr(model: *mut ffi::GRBmodel, name: &CStr) -> Option<c_int> {
    let mut v: c_int = 0;
    // SAFETY: `model` is live, `name` is NUL-terminated, `v` is a valid out-pointer.
    let rc = unsafe { ffi::GRBgetintattr(model, name.as_ptr(), &mut v) };
    (rc == 0).then_some(v)
}

fn dbl_attr(model: *mut ffi::GRBmodel, name: &CStr) -> Option<f64> {
    let mut v: c_double = 0.0;
    // SAFETY: as in `int_attr`.
    let rc = unsafe { ffi::GRBgetdblattr(model, name.as_ptr(), &mut v) };
    (rc == 0).then_some(v)
}

/// Reads `len` entries of a double array attribute starting at `first` into `dst`;
/// `false` when the attribute is not available (for example no basis).
fn dbl_array(model: *mut ffi::GRBmodel, name: &CStr, first: usize, dst: &mut [f64]) -> bool {
    if dst.is_empty() {
        return true;
    }
    let (Ok(first), Ok(len)) = (c_int::try_from(first), c_int::try_from(dst.len())) else {
        return false;
    };
    // SAFETY: `model` is live; `dst` has exactly `len` writable entries; Gurobi writes at most
    // `len` values (Gurobi C reference, GRBgetdblattrarray).
    let rc = unsafe { ffi::GRBgetdblattrarray(model, name.as_ptr(), first, len, dst.as_mut_ptr()) };
    rc == 0
}

fn int_array(model: *mut ffi::GRBmodel, name: &CStr, first: usize, dst: &mut [c_int]) -> bool {
    if dst.is_empty() {
        return true;
    }
    let (Ok(first), Ok(len)) = (c_int::try_from(first), c_int::try_from(dst.len())) else {
        return false;
    };
    // SAFETY: as in `dbl_array`, for an integer array attribute.
    let rc = unsafe { ffi::GRBgetintattrarray(model, name.as_ptr(), first, len, dst.as_mut_ptr()) };
    rc == 0
}

/// Row bounds (b_l, b_u) of row i, for basis classification.
fn row_bounds(problem: ProblemRef<'_, '_>, i: usize) -> (f64, f64) {
    match problem {
        ProblemRef::Usize(p) => p.rows().bounds(i),
        ProblemRef::I32(p) => p.rows().bounds(i),
    }
}

fn var_bounds(problem: ProblemRef<'_, '_>, j: usize) -> (f64, f64) {
    match problem {
        ProblemRef::Usize(p) => (p.l()[j], p.u()[j]),
        ProblemRef::I32(p) => (p.l()[j], p.u()[j]),
    }
}

fn basis_status(code: c_int, lo: f64, up: f64) -> BasisStatus {
    match code {
        0 => BasisStatus::Basic,
        -1 if lo == up => BasisStatus::Fixed,
        -1 => BasisStatus::AtLower,
        -2 => BasisStatus::AtUpper,
        _ => BasisStatus::Superbasic,
    }
}

fn extract(
    env: &Env,
    loaded: &Loaded<'_>,
    problem: ProblemRef<'_, '_>,
    options: &Options<'_>,
    out: Option<SolutionBuffers<'_>>,
) -> Result<Solution> {
    let model = loaded.model.as_ptr();
    let (m, n) = (loaded.m, loaded.n);
    let code = int_attr(model, c"Status").unwrap_or(ffi::GRB_LOADED);
    let (status, limit_reason) = status::translate(code);
    let sol_count = int_attr(model, c"SolCount").unwrap_or(0);
    let returned = status::solution_returned(status, sol_count);

    let mut s = Solution::empty(status, env.identity());
    s.limit_reason = limit_reason;
    s.returned = returned;
    s.raw_status = vec![
        ("Status".to_string(), i64::from(code)),
        ("SolCount".to_string(), i64::from(sol_count)),
    ];

    let arrays = options.effective_array_outputs();
    let mut out = out;

    if returned != SolutionReturned::None {
        // Primal x (auxiliary range columns excluded: they follow the caller's n columns).
        let mut x_owned = Vec::new();
        let x: &mut [f64] = match out.as_mut() {
            Some(b) => b.x,
            None => {
                x_owned = vec![0.0; n];
                &mut x_owned
            }
        };
        if !dbl_array(model, c"X", 0, x) {
            return Err(Error::Solver {
                code: -1,
                message: "solution X not available".into(),
            });
        }
        s.x = x_owned;
        s.objective = dbl_attr(model, c"ObjVal");

        if arrays {
            // Row activity Ax from the solver: rhs - slack, plus the auxiliary column value for
            // ranged rows (whose Gurobi row reads Ax - s = 0).
            let mut rhs = vec![0.0; m];
            let mut slack = vec![0.0; m];
            if dbl_array(model, c"RHS", 0, &mut rhs) && dbl_array(model, c"Slack", 0, &mut slack) {
                let mut act: Vec<f64> = rhs.iter().zip(&slack).map(|(r, sl)| r - sl).collect();
                if loaded.aux_columns > 0 {
                    let mut aux = vec![0.0; loaded.aux_columns];
                    if dbl_array(model, c"X", n, &mut aux) {
                        for (k, &i) in loaded.ranged_rows.iter().enumerate() {
                            act[i] += aux[k];
                        }
                    }
                }
                match out.as_mut() {
                    Some(b) => {
                        if let Some(dst) = b.row_activity.as_deref_mut() {
                            dst.copy_from_slice(&act);
                        }
                    }
                    None => s.row_activity = act,
                }
            }
        }
    }

    if arrays && status == Status::Optimal {
        let mut y = vec![0.0; m];
        let mut z = vec![0.0; n];
        let have_y = dbl_array(model, c"Pi", 0, &mut y);
        let have_z = dbl_array(model, c"RC", 0, &mut z);
        if have_y && have_z {
            // Gurobi reports one reduced cost per variable; its sign gives the bound it
            // belongs to: z_l = the part with σz ≥ 0, z_u = the part with σz ≤ 0 (z = z_l − z_u).
            let sigma = match problem {
                ProblemRef::Usize(p) => p.sense().sigma(),
                ProblemRef::I32(p) => p.sense().sigma(),
            };
            let z_l: Vec<f64> = z
                .iter()
                .map(|&v| if sigma * v > 0.0 { v } else { 0.0 })
                .collect();
            let z_u: Vec<f64> = z
                .iter()
                .map(|&v| if sigma * v < 0.0 { -v } else { 0.0 })
                .collect();
            match out.as_mut() {
                Some(b) => {
                    if let Some(dst) = b.y.as_deref_mut() {
                        dst.copy_from_slice(&y);
                    }
                    if let Some(dst) = b.z.as_deref_mut() {
                        dst.copy_from_slice(&z);
                    }
                    if let Some(dst) = b.z_l.as_deref_mut() {
                        dst.copy_from_slice(&z_l);
                    }
                    if let Some(dst) = b.z_u.as_deref_mut() {
                        dst.copy_from_slice(&z_u);
                    }
                }
                None => {
                    s.y = y;
                    s.z = z;
                    s.z_l = z_l;
                    s.z_u = z_u;
                }
            }
        }
        let mut vb = vec![0 as c_int; n + loaded.aux_columns];
        let mut cb = vec![0 as c_int; m];
        if int_array(model, c"VBasis", 0, &mut vb) && int_array(model, c"CBasis", 0, &mut cb) {
            let variables = (0..n)
                .map(|j| {
                    let (lo, up) = var_bounds(problem, j);
                    basis_status(vb[j], lo, up)
                })
                .collect();
            let mut rows: Vec<BasisStatus> = (0..m)
                .map(|i| {
                    let (lo, up) = row_bounds(problem, i);
                    match cb[i] {
                        0 => BasisStatus::Basic,
                        _ if lo == up => BasisStatus::Fixed,
                        _ if up.is_finite() && !lo.is_finite() => BasisStatus::AtUpper,
                        _ => BasisStatus::AtLower,
                    }
                })
                .collect();
            for (k, &i) in loaded.ranged_rows.iter().enumerate() {
                let (lo, up) = row_bounds(problem, i);
                rows[i] = basis_status(vb[n + k], lo, up);
            }
            s.basis = Some(Basis { variables, rows });
        }
    }

    if options.certificates {
        if status == Status::Infeasible {
            let mut y_f = vec![0.0; m];
            if dbl_array(model, c"FarkasDual", 0, &mut y_f) {
                // Sign: the data model's convention needs d_F > 0, so y_F = −FarkasDual. This
                // is established as behaviour by the per-backend contract test
                // `f002::t014_certificate_sign` (y_F,1 > 0, z_F < 0 on
                // tests/fixtures/small/infeasible.mps), and every returned certificate is
                // checked by the verifier.
                for v in &mut y_f {
                    *v = -*v;
                }
                // Bound multipliers z_F = -A^T y_F (data-model.md: A^T y_F + z_F = 0).
                let z_f = minus_at_y(problem, &y_f);
                s.certificate = Some(Certificate::Farkas {
                    y: y_f,
                    z: z_f,
                    s: Vec::new(),
                });
            }
        } else if status == Status::Unbounded {
            let mut r = vec![0.0; n];
            if dbl_array(model, c"UnbdRay", 0, &mut r) {
                s.certificate = Some(Certificate::UnboundedRay { r });
            }
        }
    }

    s.bound = dbl_attr(model, c"ObjBound");
    s.counts = Counts {
        simplex_iterations: dbl_attr(model, c"IterCount"),
        barrier_iterations: int_attr(model, c"BarIterCount").map(i64::from),
    };
    for name in [c"Runtime", c"Work", c"MaxMemUsed", c"MemUsed"] {
        if let Some(v) = dbl_attr(model, name) {
            s.attributes
                .push((name.to_string_lossy().into_owned(), ParamValue::Double(v)));
        }
    }
    Ok(s)
}

/// −Aᵀy, length n.
fn minus_at_y(problem: ProblemRef<'_, '_>, y: &[f64]) -> Vec<f64> {
    fn go<I: crate::problem::RowIndex>(A: &crate::problem::CscRef<'_, I>, y: &[f64]) -> Vec<f64> {
        (0..A.n())
            .map(|j| {
                let (rows, vals) = A.column(j);
                -rows
                    .iter()
                    .zip(vals)
                    .map(|(i, v)| v * y[i.index().unwrap_or(0)])
                    .sum::<f64>()
            })
            .collect()
    }
    match problem {
        ProblemRef::Usize(p) => go(p.A(), y),
        ProblemRef::I32(p) => go(p.A(), y),
    }
}

/// Load-only report used by the benchmark's positive control and memory measurement
/// (constitution XIV-C, XIV-D). Not part of the solver-independent contract.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct LoadReport {
    /// `"single-call"` or `"blocked"`.
    pub path: &'static str,
    /// Number of `GRBXaddvars` blocks (0 for a single call).
    pub blocks: usize,
    /// Largest number of converted row indices held at once.
    pub buffer_high_water: usize,
    /// Auxiliary columns added for ranged rows.
    pub aux_columns: usize,
    /// Seconds spent inside Gurobi's load calls.
    pub vendor_seconds: f64,
    /// Seconds of the whole load; the difference is optSolver's own loading work.
    pub total_seconds: f64,
}

impl Env {
    /// Loads `problem` into a fresh Gurobi model and frees it, reporting how it was loaded.
    /// Benchmark support only (positive control, load-phase memory); not a solve.
    #[doc(hidden)]
    pub fn load_report(&self, problem: ProblemRef<'_, '_>) -> Result<LoadReport> {
        let loaded = load::load(self, problem)?;
        let (path, blocks) = match loaded.path {
            load::LoadPath::SingleCall => ("single-call", 0),
            load::LoadPath::Blocked { blocks } => ("blocked", blocks),
        };
        Ok(LoadReport {
            path,
            blocks,
            buffer_high_water: loaded.buffer_high_water,
            aux_columns: loaded.aux_columns,
            vendor_seconds: loaded.vendor_seconds,
            total_seconds: loaded.total_seconds,
        })
    }
}
