//! `Backend` implementation for MOSEK (task T027): checks, bulk load, single parameter
//! application, optimisation with the optional callback, extraction (feature 002 research
//! R5–R7) and independent verification (constitution I, III, VII-G, XVI).

#![allow(non_snake_case)] // Notation Convention (XII-C): A is a matrix.

use std::time::Instant;

use super::env::{Env, Task};
use super::load::{self, Loaded};
use super::{callback, ffi, params, status};
use crate::backend::{Backend, Capabilities, ProblemClass, ProblemRef};
use crate::error::{Error, Result};
use crate::options::{Options, ParamValue, check};
use crate::problem::Sense;
use crate::result::{
    Basis, BasisStatus, Certificate, Counts, Solution, SolutionBuffers, SolutionReturned,
    SolutionView, SolverIdentity, Status, Timings,
};
use crate::verify::{Tolerances, verify};

impl Env {
    fn identity(&self) -> SolverIdentity {
        let (a, b, c) = self.version();
        SolverIdentity {
            name: "mosek".to_string(),
            version: format!("{a}.{b}.{c}"),
        }
    }
}

impl Backend for Env {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            backend: "mosek",
            problem_classes: vec![ProblemClass::Lp, ProblemClass::Qp, ProblemClass::Conic],
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
        let (m, n, k) = dims(problem);
        let bad = |what: &'static str, len: usize, want: usize| Error::Invalid {
            what,
            detail: format!("buffer length {len}, expected {want}"),
        };
        if out.x.len() != n {
            return Err(bad("out.x", out.x.len(), n));
        }
        for (what, buf, want) in [
            ("out.y", out.y.as_deref(), m),
            ("out.z", out.z.as_deref(), n),
            ("out.row_activity", out.row_activity.as_deref(), m),
            ("out.s", out.s.as_deref(), k),
            ("out.z_l", out.z_l.as_deref(), n),
            ("out.z_u", out.z_u.as_deref(), n),
        ] {
            if let Some(b) = buf {
                if b.len() != want {
                    return Err(bad(what, b.len(), want));
                }
            }
        }
        run(self, problem, options, Some(out))
    }
}

fn dims(problem: ProblemRef<'_, '_>) -> (usize, usize, usize) {
    match problem {
        ProblemRef::Usize(p) => (p.m(), p.n(), p.k()),
        ProblemRef::I32(p) => (p.m(), p.n(), p.k()),
    }
}

fn sense(problem: ProblemRef<'_, '_>) -> Sense {
    match problem {
        ProblemRef::Usize(p) => p.sense(),
        ProblemRef::I32(p) => p.sense(),
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
    let effective_parameters = params::apply(&loaded.task, options, loaded.class)?;
    let load_seconds = t_load.elapsed().as_secs_f64();

    let progress = options.progress.take();
    let mut state = callback::CallbackState::new(options.cancel, progress);
    callback::install(&loaded.task, &mut state)?;
    let t_solve = Instant::now();
    let mut trm = ffi::MSK_RES_OK;
    // SAFETY: the task is live; `trm` is a writable local; the callback state (if installed)
    // outlives the call and is not moved until `uninstall` below.
    let rc = unsafe { ffi::MSK_optimizetrm(loaded.task.raw(), &mut trm) };
    let solve_seconds = t_solve.elapsed().as_secs_f64();
    callback::uninstall(&loaded.task)?;
    let panic = state.take_panic();
    options.progress = state.into_progress();
    if let Some(payload) = panic {
        drop(loaded);
        std::panic::resume_unwind(payload);
    }
    // A response code that is an error (not OK, a warning or a termination code) is an
    // error; termination codes go to the status translation (research R6).
    let mut class = ffi::MSK_RESPONSE_UNK;
    // SAFETY: `class` is a writable local; MSK_getresponseclass only classifies the code.
    unsafe { ffi::MSK_getresponseclass(rc, &mut class) };
    if !matches!(
        class,
        ffi::MSK_RESPONSE_OK | ffi::MSK_RESPONSE_WRN | ffi::MSK_RESPONSE_TRM
    ) {
        return Err(super::env::task_error(loaded.task.raw(), rc));
    }
    let trm = if class == ffi::MSK_RESPONSE_TRM {
        rc
    } else {
        trm
    };

    let t_extract = Instant::now();
    let (mut solution, basic) = extract(env, &loaded, problem, options, out, rc, trm)?;
    solution.effective_parameters = effective_parameters;
    let tolerances = params::effective_tolerances(&loaded.task, loaded.class, basic)?;
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

fn defined(task: &Task<'_>, soltype: i32) -> bool {
    let mut d = 0;
    // SAFETY: live task, valid MSKsoltypee, writable local.
    let rc = unsafe { ffi::MSK_solutiondef(task.raw(), soltype, &mut d) };
    rc == ffi::MSK_RES_OK && d != 0
}

/// Reads one solution array of `soltype` into `dst` (length as MOSEK defines for `get`).
fn read(
    task: &Task<'_>,
    soltype: i32,
    dst: &mut [f64],
    get: unsafe extern "C" fn(ffi::MSKtask_t, i32, *mut f64) -> i32,
) -> Result<()> {
    if dst.is_empty() {
        return Ok(());
    }
    // SAFETY: live task; `get` is one of the MOSEK getters declared in ffi.rs, each writing
    // exactly the number of values the caller sized `dst` for (n, m or the conic row count).
    let rc = unsafe { get(task.raw(), soltype, dst.as_mut_ptr()) };
    task.check(rc)
}

fn read_keys(
    task: &Task<'_>,
    soltype: i32,
    dst: &mut [i32],
    get: unsafe extern "C" fn(ffi::MSKtask_t, i32, *mut i32) -> i32,
) -> bool {
    if dst.is_empty() {
        return true;
    }
    // SAFETY: as in `read`, for the status-key getters (n or m values).
    let rc = unsafe { get(task.raw(), soltype, dst.as_mut_ptr()) };
    rc == ffi::MSK_RES_OK
}

fn basis_status(key: i32) -> BasisStatus {
    match key {
        ffi::MSK_SK_BAS => BasisStatus::Basic,
        ffi::MSK_SK_LOW => BasisStatus::AtLower,
        ffi::MSK_SK_UPR => BasisStatus::AtUpper,
        ffi::MSK_SK_FIX => BasisStatus::Fixed,
        _ => BasisStatus::Superbasic,
    }
}

/// Writes `v` to the caller's buffer when one is given, else keeps it in `owned`.
fn deliver(v: Vec<f64>, buf: Option<&mut [f64]>, owned: &mut Vec<f64>) {
    match buf {
        Some(dst) => dst.copy_from_slice(&v),
        None => *owned = v,
    }
}

#[allow(clippy::too_many_arguments)]
fn extract(
    env: &Env,
    loaded: &Loaded<'_>,
    problem: ProblemRef<'_, '_>,
    options: &Options<'_>,
    out: Option<SolutionBuffers<'_>>,
    rc: i32,
    trm: i32,
) -> Result<(Solution, bool)> {
    let task = &loaded.task;
    let (m, n, k) = (loaded.m, loaded.n, loaded.k);
    // Solution type: the basic solution for an LP when defined, else the interior-point one
    // (research R6; probe in research R18).
    let soltype = [ffi::MSK_SOL_BAS, ffi::MSK_SOL_ITR]
        .into_iter()
        .filter(|&s| s == ffi::MSK_SOL_ITR || loaded.class == ProblemClass::Lp)
        .find(|&s| defined(task, s));
    let (mut prosta, mut solsta) = (ffi::MSK_PRO_STA_UNKNOWN, ffi::MSK_SOL_STA_UNKNOWN);
    if let Some(st) = soltype {
        // SAFETY: live task, defined solution type, writable locals.
        let r1 = unsafe { ffi::MSK_getprosta(task.raw(), st, &mut prosta) };
        // SAFETY: as above.
        let r2 = unsafe { ffi::MSK_getsolsta(task.raw(), st, &mut solsta) };
        task.check(r1)?;
        task.check(r2)?;
    }
    let (status, limit_reason) = status::translate(trm, prosta, solsta);
    let returned = status::solution_returned(status, solsta);

    let mut s = Solution::empty(status, env.identity());
    s.limit_reason = limit_reason;
    s.returned = returned;
    s.raw_status = vec![
        ("response".to_string(), i64::from(rc)),
        ("termination".to_string(), i64::from(trm)),
        ("problem_status".to_string(), i64::from(prosta)),
        ("solution_status".to_string(), i64::from(solsta)),
        ("solution_type".to_string(), soltype.map_or(-1, i64::from)),
    ];
    let Some(st) = soltype else {
        return Ok((s, false));
    };
    let basic = st == ffi::MSK_SOL_BAS;
    let arrays = options.effective_array_outputs();
    let mut out = out;

    if returned != SolutionReturned::None {
        let mut x = vec![0.0; n];
        read(task, st, &mut x, ffi::MSK_getxx)?;
        deliver(x, out.as_mut().map(|b| &mut *b.x), &mut s.x);
        let mut obj = 0.0;
        // SAFETY: live task, defined solution, writable local.
        let r = unsafe { ffi::MSK_getprimalobj(task.raw(), st, &mut obj) };
        if r == ffi::MSK_RES_OK {
            s.objective = Some(obj);
        }
        if arrays {
            let mut act = vec![0.0; m];
            read(task, st, &mut act, ffi::MSK_getxc)?;
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

    if arrays && status == Status::Optimal {
        let mut y = vec![0.0; m];
        read(task, st, &mut y, ffi::MSK_gety)?;
        let mut slx = vec![0.0; n];
        let mut sux = vec![0.0; n];
        read(task, st, &mut slx, ffi::MSK_getslx)?;
        read(task, st, &mut sux, ffi::MSK_getsux)?;
        let z: Vec<f64> = slx.iter().zip(&sux).map(|(a, b)| a - b).collect();
        let mut dots = vec![0.0; k];
        if k > 0 {
            read(task, st, &mut dots, ffi::MSK_getaccdotys)?;
        }
        // y, z = s_l − s_u (with z_l = s_l, z_u = s_u) and s = ẏ map MOSEK's duals onto the
        // contract for both senses (research R5).
        match out.as_mut() {
            Some(b) => {
                if let Some(dst) = b.y.as_deref_mut() {
                    dst.copy_from_slice(&y);
                }
                if let Some(dst) = b.z.as_deref_mut() {
                    dst.copy_from_slice(&z);
                }
                if let Some(dst) = b.s.as_deref_mut() {
                    dst.copy_from_slice(&dots);
                }
                if let Some(dst) = b.z_l.as_deref_mut() {
                    dst.copy_from_slice(&slx);
                }
                if let Some(dst) = b.z_u.as_deref_mut() {
                    dst.copy_from_slice(&sux);
                }
            }
            None => {
                s.y = y;
                s.z = z;
                s.s = dots;
                s.z_l = slx;
                s.z_u = sux;
            }
        }
        let mut dual = 0.0;
        // SAFETY: live task, defined solution, writable local.
        if unsafe { ffi::MSK_getdualobj(task.raw(), st, &mut dual) } == ffi::MSK_RES_OK {
            s.bound = Some(dual);
        }
        if basic {
            let mut skx = vec![0; n];
            let mut skc = vec![0; m];
            if read_keys(task, st, &mut skx, ffi::MSK_getskx)
                && read_keys(task, st, &mut skc, ffi::MSK_getskc)
            {
                s.basis = Some(Basis {
                    variables: skx.into_iter().map(basis_status).collect(),
                    rows: skc.into_iter().map(basis_status).collect(),
                });
            }
        }
    }

    if options.certificates {
        if solsta == ffi::MSK_SOL_STA_PRIM_INFEAS_CER {
            // Sense-independent convention d_F > 0 (research R7): y_F = σy,
            // z_F = σ(s_l − s_u), s_F = σẏ.
            let sigma = sense(problem).sigma();
            let mut y = vec![0.0; m];
            read(task, st, &mut y, ffi::MSK_gety)?;
            let mut z = reduced_costs(task, st, n)?;
            let mut dots = vec![0.0; k];
            if k > 0 {
                read(task, st, &mut dots, ffi::MSK_getaccdotys)?;
            }
            for v in y.iter_mut().chain(z.iter_mut()).chain(dots.iter_mut()) {
                *v *= sigma;
            }
            s.certificate = Some(Certificate::Farkas { y, z, s: dots });
        } else if solsta == ffi::MSK_SOL_STA_DUAL_INFEAS_CER {
            let mut r = vec![0.0; n];
            read(task, st, &mut r, ffi::MSK_getxx)?;
            s.certificate = Some(Certificate::UnboundedRay { r });
        }
    }

    let (mut sim_p, mut sim_d, mut ipm) = (0, 0, 0);
    // SAFETY: (three calls): live task, valid MSKiinfiteme values, writable locals.
    let ok = unsafe {
        ffi::MSK_getintinf(task.raw(), ffi::MSK_IINF_SIM_PRIMAL_ITER, &mut sim_p)
    } == ffi::MSK_RES_OK
        // SAFETY: as above.
        && unsafe { ffi::MSK_getintinf(task.raw(), ffi::MSK_IINF_SIM_DUAL_ITER, &mut sim_d) }
            == ffi::MSK_RES_OK
        // SAFETY: as above.
        && unsafe { ffi::MSK_getintinf(task.raw(), ffi::MSK_IINF_INTPNT_ITER, &mut ipm) }
            == ffi::MSK_RES_OK;
    if ok {
        s.counts = Counts {
            simplex_iterations: Some(f64::from(sim_p) + f64::from(sim_d)),
            barrier_iterations: Some(i64::from(ipm)),
        };
    }
    let mut t = 0.0;
    // SAFETY: live task, valid MSKdinfiteme value, writable local.
    if unsafe { ffi::MSK_getdouinf(task.raw(), ffi::MSK_DINF_OPTIMIZER_TIME, &mut t) }
        == ffi::MSK_RES_OK
    {
        s.attributes
            .push(("MSK_DINF_OPTIMIZER_TIME".into(), ParamValue::Double(t)));
    }
    Ok((s, basic))
}

/// z = s_l − s_u of solution `st` (length n).
fn reduced_costs(task: &Task<'_>, st: i32, n: usize) -> Result<Vec<f64>> {
    let mut slx = vec![0.0; n];
    let mut sux = vec![0.0; n];
    read(task, st, &mut slx, ffi::MSK_getslx)?;
    read(task, st, &mut sux, ffi::MSK_getsux)?;
    Ok(slx.iter().zip(&sux).map(|(a, b)| a - b).collect())
}

/// Load-only report used by the benchmark's positive control and memory measurement
/// (constitution XIV-C, XIV-D). Not part of the solver-independent contract.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct LoadReport {
    /// `"single-call"` or `"blocked"`.
    pub path: &'static str,
    /// Number of `MSK_putacolslice64` blocks (0 for a single call).
    pub blocks: usize,
    /// Largest number of converted indices held at once.
    pub buffer_high_water: usize,
    /// Bytes of bound keys and bound-value copies.
    pub bound_bytes: usize,
    /// Seconds spent inside MOSEK's load calls.
    pub vendor_seconds: f64,
    /// Seconds of the whole load; the difference is optSolver's own loading work.
    pub total_seconds: f64,
}

impl Env {
    /// Loads `problem` into a fresh MOSEK task and deletes it, reporting how it was loaded.
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
            bound_bytes: loaded.bound_bytes,
            vendor_seconds: loaded.vendor_seconds,
            total_seconds: loaded.total_seconds,
        })
    }
}
