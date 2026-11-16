//! Load-and-solve benchmark (User Story 4; tasks T047-T049; constitution XIV, XV-A).
//!
//! The parent process runs one child process per (instance, index type, kind) so that every
//! memory figure comes from a process dedicated to one scenario (XIV-C). Children:
//! - `mem`: peak of optSolver's own Rust heap while loading (dhat, exact; SC-004), plus the
//!   load path and conversion-buffer high-water mark (positive control, XIV-D);
//! - `time`: N = 5 solves, each with a fresh environment, timing environment creation, load,
//!   solve, extraction and verification separately (cold = first repetition), plus the rise in
//!   peak resident memory (`VmHWM`) and Gurobi's `MaxMemUsed` (cross-check, research R8).
//!
//! Every scenario runs for each compiled backend (feature 002, task T036: MOSEK with
//! `--features gurobi,mosek`). Records go to the active feature's run directory (the generated
//! output home declared in `project.yml` plus the feature named in `.specify/feature.json`) as
//! JSON lines, and a summary evaluating SC-004 and SC-005 per solver is printed. Run:
//! `cargo bench --features gurobi,mosek`.
//!
//! Feature 005 (task T017): with `--features capi` and `OPTSOLVER_BENCH_SUITE=capi`, the C-path
//! suite instead compares a solve through the C interface (`optsolver::capi`, in process, so
//! `dhat` measures it) with the same solve through the Rust API on `iDopaNeuroC_problem_LP`: the
//! heap criterion (SC-002) in a process per scenario, N = 3, with the allocation-count difference
//! as positive control, and the wall-time criterion (SC-003), N = 5 warm runs after one cold run,
//! environment creation timed separately.

#![allow(missing_docs, non_snake_case, unused_imports)]

#[path = "../tests/common/mod.rs"]
mod common;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use optsolver::{Backend, CscRef, Method, Options, ParamValue, Problem, RowBounds};
use std::process::Command;
use std::time::Instant;

const N_REPETITIONS: usize = 5;
const CHILD_ENV: &str = "OPTSOLVER_BENCH_CHILD";

/// One benchmark record (task T047: a record missing any field is refused by `validate`).
#[derive(Debug, Clone, Default)]
struct Record {
    solver: String,
    instance: String,
    index_type: String,
    kind: String,
    m: usize,
    n: usize,
    nnz: usize,
    n_repetitions: usize,
    // time records: per phase, seconds of each repetition
    env_creation: Vec<f64>,
    load: Vec<f64>,
    solve: Vec<f64>,
    extraction: Vec<f64>,
    verification: Vec<f64>,
    // SC-005 split (tasks T055, T056): validation, optSolver's own loading work, Gurobi's load calls
    validation: Vec<f64>,
    load_optsolver: Vec<f64>,
    load_vendor: Vec<f64>,
    // memory records
    heap_peak_during_load_bytes: Option<u64>,
    vmhwm_rise_kib: Option<u64>,
    gurobi_max_mem_used_gb: Option<f64>,
    // positive control
    load_path: String,
    q_terms: usize,
    buffer_high_water: usize,
    // configuration
    effective_parameters: String,
    solver_version: String,
    optsolver_commit: String,
    machine: String,
    profile: String,
}

impl Record {
    fn validate(&self) -> Result<(), String> {
        let mut missing = Vec::new();
        for (name, v) in [
            ("solver", &self.solver),
            ("instance", &self.instance),
            ("index_type", &self.index_type),
            ("kind", &self.kind),
            ("load_path", &self.load_path),
            ("solver_version", &self.solver_version),
            ("optsolver_commit", &self.optsolver_commit),
            ("machine", &self.machine),
            ("profile", &self.profile),
        ] {
            if v.is_empty() {
                missing.push(name);
            }
        }
        if self.m == 0 || self.n == 0 || self.nnz == 0 {
            missing.push("dimensions");
        }
        match self.kind.as_str() {
            "time" => {
                if self.n_repetitions != N_REPETITIONS {
                    missing.push("n_repetitions");
                }
                for (name, v) in [
                    ("env_creation", &self.env_creation),
                    ("load", &self.load),
                    ("solve", &self.solve),
                    ("extraction", &self.extraction),
                    ("verification", &self.verification),
                    ("validation", &self.validation),
                    ("load_optsolver", &self.load_optsolver),
                    ("load_vendor", &self.load_vendor),
                ] {
                    if v.len() != N_REPETITIONS || v.iter().any(|t| !t.is_finite() || *t < 0.0) {
                        missing.push(name);
                    }
                }
                if self.effective_parameters.is_empty() {
                    missing.push("effective_parameters");
                }
                if self.vmhwm_rise_kib.is_none() {
                    missing.push("vmhwm_rise_kib");
                }
                if self.solver == "gurobi" && self.gurobi_max_mem_used_gb.is_none() {
                    missing.push("gurobi_max_mem_used_gb");
                }
            }
            "mem" => {
                if self.heap_peak_during_load_bytes.is_none() {
                    missing.push("heap_peak_during_load_bytes");
                }
            }
            _ => missing.push("kind"),
        }
        if missing.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "record refused, missing or invalid: {}",
                missing.join(", ")
            ))
        }
    }

    fn to_json(&self) -> String {
        let list = |v: &[f64]| {
            format!(
                "[{}]",
                v.iter()
                    .map(|x| format!("{x:e}"))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        let opt_u = |v: Option<u64>| v.map_or("null".to_string(), |x| x.to_string());
        let opt_f = |v: Option<f64>| v.map_or("null".to_string(), |x| format!("{x:e}"));
        let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
        format!(
            "{{\"solver\":\"{}\",\"instance\":\"{}\",\"index_type\":\"{}\",\"kind\":\"{}\",\"m\":{},\"n\":{},\"nnz\":{},\"n_repetitions\":{},\"env_creation\":{},\"load\":{},\"solve\":{},\"extraction\":{},\"verification\":{},\"validation\":{},\"load_optsolver\":{},\"load_vendor\":{},\"heap_peak_during_load_bytes\":{},\"vmhwm_rise_kib\":{},\"gurobi_max_mem_used_gb\":{},\"load_path\":\"{}\",\"buffer_high_water\":{},\"q_terms\":{},\"effective_parameters\":\"{}\",\"solver_version\":\"{}\",\"optsolver_commit\":\"{}\",\"machine\":\"{}\",\"profile\":\"{}\"}}",
            self.solver,
            esc(&self.instance),
            self.index_type,
            self.kind,
            self.m,
            self.n,
            self.nnz,
            self.n_repetitions,
            list(&self.env_creation),
            list(&self.load),
            list(&self.solve),
            list(&self.extraction),
            list(&self.verification),
            list(&self.validation),
            list(&self.load_optsolver),
            list(&self.load_vendor),
            opt_u(self.heap_peak_during_load_bytes),
            opt_u(self.vmhwm_rise_kib),
            opt_f(self.gurobi_max_mem_used_gb),
            self.load_path,
            self.buffer_high_water,
            self.q_terms,
            esc(&self.effective_parameters),
            esc(&self.solver_version),
            esc(&self.optsolver_commit),
            esc(&self.machine),
            self.profile
        )
    }

    /// Minimal parser for the fields the summary needs (the format is ours, written above).
    fn from_json(line: &str) -> Record {
        let field = |name: &str| -> String {
            let key = format!("\"{name}\":");
            let start = line.find(&key).map(|i| i + key.len()).unwrap_or(line.len());
            let rest = &line[start..];
            if let Some(stripped) = rest.strip_prefix('"') {
                stripped.split('"').next().unwrap_or("").to_string()
            } else if let Some(stripped) = rest.strip_prefix('[') {
                stripped.split(']').next().unwrap_or("").to_string()
            } else {
                rest.split([',', '}']).next().unwrap_or("").to_string()
            }
        };
        let list = |s: String| -> Vec<f64> {
            s.split(',')
                .filter(|t| !t.is_empty())
                .filter_map(|t| t.parse().ok())
                .collect()
        };
        Record {
            solver: field("solver"),
            instance: field("instance"),
            index_type: field("index_type"),
            kind: field("kind"),
            m: field("m").parse().unwrap_or(0),
            n: field("n").parse().unwrap_or(0),
            nnz: field("nnz").parse().unwrap_or(0),
            n_repetitions: field("n_repetitions").parse().unwrap_or(0),
            env_creation: list(field("env_creation")),
            load: list(field("load")),
            solve: list(field("solve")),
            extraction: list(field("extraction")),
            verification: list(field("verification")),
            validation: list(field("validation")),
            load_optsolver: list(field("load_optsolver")),
            load_vendor: list(field("load_vendor")),
            heap_peak_during_load_bytes: field("heap_peak_during_load_bytes").parse().ok(),
            vmhwm_rise_kib: field("vmhwm_rise_kib").parse().ok(),
            gurobi_max_mem_used_gb: field("gurobi_max_mem_used_gb").parse().ok(),
            load_path: field("load_path"),
            buffer_high_water: field("buffer_high_water").parse().unwrap_or(0),
            q_terms: field("q_terms").parse().unwrap_or(0),
            effective_parameters: field("effective_parameters"),
            solver_version: field("solver_version"),
            optsolver_commit: field("optsolver_commit"),
            machine: field("machine"),
            profile: field("profile"),
        }
    }
}

/// The run bundle directory: `structure.generated` of `project.yml`, the active feature (from
/// `.specify/feature.json`), `runs/<stamp>-bench` (constitution IX-C, IX-E, IX-F).
fn run_dir(stamp: &str) -> std::path::PathBuf {
    let root = common::fixtures::repo_root();
    let yml = std::fs::read_to_string(common::fixtures::project_yml()).expect("project.yml");
    let generated = yml
        .lines()
        .find_map(|l| {
            l.trim()
                .strip_prefix("generated:")
                .map(|v| v.trim().to_string())
        })
        .expect("structure.generated in project.yml");
    let feature_json =
        std::fs::read_to_string(root.join(".specify/feature.json")).expect("feature.json");
    let feature = feature_json
        .split('"')
        .skip_while(|s| *s != "feature_directory")
        .nth(2)
        .and_then(|d| d.rsplit('/').next())
        .expect("feature_directory in .specify/feature.json")
        .to_string();
    root.join(generated)
        .join(feature)
        .join("runs")
        .join(format!("{stamp}-bench"))
}

fn median_and_spread(v: &[f64]) -> (f64, f64, f64) {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    (s[s.len() / 2], s[0], s[s.len() - 1])
}

fn vmhwm_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find(|l| l.starts_with("VmHWM:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

fn commit() -> String {
    let root = common::fixtures::repo_root();
    let out = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["rev-parse", "HEAD"])
        .output();
    let head = out
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let dirty = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["status", "--porcelain"])
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(true);
    if dirty {
        // The working tree's content as a tree hash (tracked and untracked, not ignored), so an
        // uncommitted run names exactly what it measured (feature 005, Gate 3).
        let index =
            std::env::temp_dir().join(format!("optsolver-bench-index-{}", std::process::id()));
        let git = |args: &[&str]| {
            Command::new("git")
                .arg("-C")
                .arg(&root)
                .env("GIT_INDEX_FILE", &index)
                .args(args)
                .output()
                .ok()
                .filter(|o| o.status.success())
        };
        let tree = git(&["add", "-A"])
            .and_then(|_| git(&["write-tree"]))
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        let _ = std::fs::remove_file(&index);
        format!("{head}+uncommitted (working tree {tree})")
    } else {
        head
    }
}

fn machine() -> String {
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|t| {
            t.lines()
                .find(|l| l.starts_with("model name"))
                .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
        })
        .unwrap_or_default();
    let host = std::fs::read_to_string("/proc/sys/kernel/hostname").unwrap_or_default();
    format!("{} | {}", host.trim(), cpu)
}

fn profile() -> String {
    if cfg!(debug_assertions) {
        "debug".into()
    } else {
        "release (bench profile)".into()
    }
}

fn deterministic_options<'a>(solver: &str, qp: bool) -> Options<'a> {
    let mut o = Options::default();
    if !qp {
        o.method = Some(Method::DualSimplex); // QP: each solver's default (barrier)
    }
    o.threads = Some(1);
    if solver == "gurobi" {
        o.random_seed = Some(0); // MOSEK maps no seed (feature 002 research R9)
    }
    o
}

/// The solvers compiled into this benchmark.
fn solvers() -> Vec<&'static str> {
    let mut v = vec!["gurobi"];
    if cfg!(feature = "mosek") {
        v.push("mosek");
    }
    v
}

/// A backend environment of either solver, with the load report both provide.
enum Solver {
    Gurobi(optsolver::gurobi::Env),
    #[cfg(feature = "mosek")]
    Mosek(optsolver::mosek::Env),
}

/// (path, blocks, buffer high-water, vendor seconds, total seconds, Q terms) of a load-only
/// call.
type LoadFigures = (&'static str, usize, usize, f64, f64, usize);

impl Solver {
    fn new(name: &str) -> Result<Self, String> {
        match name {
            "gurobi" => optsolver::gurobi::Env::new()
                .map(Solver::Gurobi)
                .map_err(|e| e.to_string()),
            #[cfg(feature = "mosek")]
            "mosek" => optsolver::mosek::Env::new()
                .map(Solver::Mosek)
                .map_err(|e| e.to_string()),
            other => Err(format!("solver {other} not compiled")),
        }
    }
    fn backend(&self) -> &dyn Backend {
        match self {
            Solver::Gurobi(e) => e,
            #[cfg(feature = "mosek")]
            Solver::Mosek(e) => e,
        }
    }
    fn load_report(&self, p: optsolver::ProblemRef<'_, '_>) -> Result<LoadFigures, String> {
        match self {
            Solver::Gurobi(e) => e.load_report(p).map(|r| {
                (
                    r.path,
                    r.blocks,
                    r.buffer_high_water,
                    r.vendor_seconds,
                    r.total_seconds,
                    r.q_terms,
                )
            }),
            #[cfg(feature = "mosek")]
            Solver::Mosek(e) => e.load_report(p).map(|r| {
                (
                    r.path,
                    r.blocks,
                    r.buffer_high_water,
                    r.vendor_seconds,
                    r.total_seconds,
                    r.q_terms,
                )
            }),
        }
        .map_err(|e| e.to_string())
    }
}

/// Child process body: one scenario, prints exactly one JSON record line on stdout.
fn child(spec: &str) -> Result<Record, String> {
    let mut parts = spec.split('|');
    let (instance, index_type, kind, solver) = (
        parts.next().unwrap_or(""),
        parts.next().unwrap_or(""),
        parts.next().unwrap_or(""),
        parts.next().unwrap_or(""),
    );
    // The derived QP (feature 003, T021) is its LP fixture with c = 0, minimise and
    // Q = q·I (feature 002 research R12); every other instance is an LP with seed 1.
    let qp = common::fixtures::qp_instances()?
        .iter()
        .any(|q| q == instance);
    let (base, q_diag) = if qp {
        let prov =
            common::fixtures::Provenance::load(&common::fixtures::qp_provenance_path(instance)?)?;
        let d = match prov.fields.get("q_diagonal") {
            Some(common::fixtures::Json::Number(v)) => *v,
            other => return Err(format!("q_diagonal is {other:?}")),
        };
        (prov.str("derived_from")?.to_string(), d)
    } else {
        (instance.to_string(), 0.0)
    };
    let f = common::fixtures::load_corpus_fixture(&base)?;
    let mp = &f.problem;
    let c = if qp {
        vec![0.0; mp.n]
    } else {
        common::objectives::seeded_objective(f.provenance.seeds()?[0], mp.n)
    };
    let sense = if qp {
        optsolver::Sense::Minimise
    } else {
        mp.objective_sense
    };
    let q_cp: Vec<usize> = (0..=mp.n).collect();
    let q_ri: Vec<usize> = (0..mp.n).collect();
    let q_va = vec![q_diag; mp.n];
    let rows_i32: Vec<i32> = mp
        .row_indices
        .iter()
        .map(|&i| i32::try_from(i).expect("row index fits i32"))
        .collect();
    let env = Solver::new(solver)?;
    let rows = RowBounds::SenseRhs {
        sense: &mp.sense,
        b: &mp.b,
    };
    let mut r = Record {
        solver: solver.into(),
        instance: instance.into(),
        index_type: index_type.into(),
        kind: kind.into(),
        m: mp.m,
        n: mp.n,
        nnz: mp.values.len(),
        solver_version: env.backend().capabilities().solver.version,
        optsolver_commit: commit(),
        machine: machine(),
        profile: profile(),
        ..Record::default()
    };
    macro_rules! with_problem {
        ($rows:expr, |$p:ident| $body:block) => {{
            let A = CscRef::new(mp.m, mp.n, &mp.col_ptrs, $rows, &mp.values)
                .map_err(|e| e.to_string())?;
            let mut $p =
                Problem::new(A, &c, sense, rows, &mp.l, &mp.u).map_err(|e| e.to_string())?;
            if qp {
                let q = CscRef::new(mp.n, mp.n, &q_cp, &q_ri, &q_va).map_err(|e| e.to_string())?;
                $p = $p.with_Q(q).map_err(|e| e.to_string())?;
            }
            $body
        }};
    }
    match kind {
        "mem" => {
            let run =
                |problem: optsolver::ProblemRef<'_, '_>, r: &mut Record| -> Result<(), String> {
                    let _profiler = dhat::Profiler::builder().testing().build();
                    let before = dhat::HeapStats::get();
                    let (path, blocks, high_water, _, _, q_terms) = env.load_report(problem)?;
                    r.q_terms = q_terms;
                    let after = dhat::HeapStats::get();
                    r.heap_peak_during_load_bytes =
                        Some((after.max_bytes.saturating_sub(before.curr_bytes)) as u64);
                    r.load_path = format!("{path} (blocks {blocks})");
                    r.buffer_high_water = high_water;
                    Ok(())
                };
            if index_type == "i32" {
                with_problem!(&rows_i32, |p| {
                    run((&p).into(), &mut r)?;
                })
            } else {
                with_problem!(&mp.row_indices, |p| {
                    run((&p).into(), &mut r)?;
                })
            }
        }
        "time" => {
            let hwm0 = vmhwm_kib();
            // Validation time (CscRef::new + Problem::new), N repetitions (task T056).
            for _ in 0..N_REPETITIONS {
                let t = Instant::now();
                if index_type == "i32" {
                    with_problem!(&rows_i32, |p| {
                        std::hint::black_box(&p);
                    })
                } else {
                    with_problem!(&mp.row_indices, |p| {
                        std::hint::black_box(&p);
                    })
                }
                r.validation.push(t.elapsed().as_secs_f64());
            }
            let run =
                |problem: optsolver::ProblemRef<'_, '_>, r: &mut Record| -> Result<(), String> {
                    let (path, blocks, _, _, _, q_terms) = env.load_report(problem)?;
                    r.q_terms = q_terms;
                    r.load_path = format!("{path} (blocks {blocks})");
                    // Load split (task T055), N repetitions of a load-only call.
                    for _ in 0..N_REPETITIONS {
                        let (_, _, _, vendor, total, _) = env.load_report(problem)?;
                        r.load_vendor.push(vendor);
                        r.load_optsolver.push((total - vendor).max(0.0));
                    }
                    for _ in 0..N_REPETITIONS {
                        let t = Instant::now();
                        let e = Solver::new(solver)?;
                        r.env_creation.push(t.elapsed().as_secs_f64());
                        let mut o = deterministic_options(solver, qp);
                        let s = e
                            .backend()
                            .solve(problem, &mut o)
                            .map_err(|e| e.to_string())?;
                        if s.status != optsolver::Status::Optimal {
                            return Err(format!("status {:?}", s.status));
                        }
                        r.load.push(s.timings.load);
                        r.solve.push(s.timings.solve);
                        r.extraction.push(s.timings.extraction);
                        r.verification.push(s.timings.verification);
                        r.effective_parameters = s
                            .effective_parameters
                            .iter()
                            .map(|(k, v)| {
                                format!(
                                    "{k}={}",
                                    match v {
                                        ParamValue::Int(i) => i.to_string(),
                                        ParamValue::Double(d) => d.to_string(),
                                        ParamValue::Str(t) => t.clone(),
                                    }
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(";");
                        r.gurobi_max_mem_used_gb = s
                            .attributes
                            .iter()
                            .find(|(k, _)| k == "MaxMemUsed")
                            .and_then(|(_, v)| {
                                if let ParamValue::Double(d) = v {
                                    Some(*d)
                                } else {
                                    None
                                }
                            });
                    }
                    r.n_repetitions = N_REPETITIONS;
                    Ok(())
                };
            if index_type == "i32" {
                with_problem!(&rows_i32, |p| {
                    run((&p).into(), &mut r)?;
                })
            } else {
                with_problem!(&mp.row_indices, |p| {
                    run((&p).into(), &mut r)?;
                })
            }
            r.vmhwm_rise_kib = match (hwm0, vmhwm_kib()) {
                (Some(a), Some(b)) => Some(b.saturating_sub(a)),
                _ => None,
            };
        }
        other => return Err(format!("unknown kind {other}")),
    }
    r.validate()?;
    Ok(r)
}

/// Guard self-test (XV-C, task T047): an incomplete record must be refused.
fn record_writer_self_test() {
    let good = Record {
        solver: "gurobi".into(),
        instance: "x".into(),
        index_type: "usize".into(),
        kind: "mem".into(),
        m: 1,
        n: 1,
        nnz: 1,
        heap_peak_during_load_bytes: Some(1),
        load_path: "single-call".into(),
        solver_version: "13".into(),
        optsolver_commit: "abc".into(),
        machine: "m".into(),
        profile: "p".into(),
        ..Record::default()
    };
    assert!(good.validate().is_ok(), "complete record accepted");
    let mut bad = good.clone();
    bad.optsolver_commit.clear();
    assert!(bad.validate().is_err(), "record without commit refused");
    let mut bad = good.clone();
    bad.kind = "time".into();
    assert!(
        bad.validate().is_err(),
        "time record without timings refused"
    );
    let mut bad = good;
    bad.heap_peak_during_load_bytes = None;
    assert!(
        bad.validate().is_err(),
        "mem record without heap peak refused"
    );
}

#[cfg(feature = "capi")]
mod capi_suite {
    //! Feature 005, task T017: C path versus Rust path (see the crate documentation above).
    use super::*;
    use optsolver::capi::{
        CEnv, CPathLp, OPTSOLVER_BACKEND_GUROBI, OPTSOLVER_BACKEND_MOSEK,
        OPTSOLVER_METHOD_DUAL_SIMPLEX, solve_lp_through_c,
    };

    pub const ENV: &str = "OPTSOLVER_BENCH_CAPI_CHILD";
    const INSTANCE: &str = "iDopaNeuroC_problem_LP";

    fn code(solver: &str) -> i32 {
        if solver == "mosek" {
            OPTSOLVER_BACKEND_MOSEK
        } else {
            OPTSOLVER_BACKEND_GUROBI
        }
    }

    /// One child: "<solver>|<path>|<kind>" with path rust, c (owned) or cbuf (caller buffers) and
    /// kind mem or time. Prints one line "key=value;...".
    pub fn child(spec: &str) -> Result<String, String> {
        let mut it = spec.split('|');
        let (solver, path, kind) = (
            it.next().unwrap_or(""),
            it.next().unwrap_or(""),
            it.next().unwrap_or(""),
        );
        let f = common::fixtures::load_corpus_fixture(INSTANCE)?;
        let mp = &f.problem;
        let c = common::objectives::seeded_objective(f.provenance.seeds()?[0], mp.n);
        let (b_l, b_u) = mp.row_ranges();
        let rust_env = Solver::new(solver)?;
        let c_env = CEnv::new(code(solver))?;
        let lp = CPathLp {
            m: mp.m,
            n: mp.n,
            A_col_ptrs: &mp.col_ptrs,
            A_row_indices: &mp.row_indices,
            A_values: &mp.values,
            c: &c,
            l: &mp.l,
            u: &mp.u,
            b_l: &b_l,
            b_u: &b_u,
            sense: mp.objective_sense,
        };
        let (mut x, mut y, mut z, mut act) = (
            vec![0.0; mp.n],
            vec![0.0; mp.m],
            vec![0.0; mp.n],
            vec![0.0; mp.m],
        );
        // One solve on the chosen path; returns (status ok, [load, solve, extraction]).
        let mut once = |path: &str| -> Result<[f64; 3], String> {
            match path {
                "rust" => {
                    let A = CscRef::new(mp.m, mp.n, &mp.col_ptrs, &mp.row_indices, &mp.values)
                        .map_err(|e| e.to_string())?;
                    let rows = RowBounds::Ranges {
                        b_l: &b_l,
                        b_u: &b_u,
                    };
                    let p = Problem::new(A, &c, mp.objective_sense, rows, &mp.l, &mp.u)
                        .map_err(|e| e.to_string())?;
                    let mut o = Options::default();
                    o.threads = Some(1);
                    o.method = Some(Method::DualSimplex);
                    let s = rust_env
                        .backend()
                        .solve((&p).into(), &mut o)
                        .map_err(|e| e.to_string())?;
                    // Read every array, as the C path's accessors do.
                    let n = s.x.len()
                        + s.y.len()
                        + s.z.len()
                        + s.z_l.len()
                        + s.z_u.len()
                        + s.s.len()
                        + s.row_activity.len();
                    std::hint::black_box(n);
                    if s.status != optsolver::Status::Optimal {
                        return Err(format!("status {:?}", s.status));
                    }
                    Ok([s.timings.load, s.timings.solve, s.timings.extraction])
                }
                "c" | "cbuf" => {
                    let bufs = (path == "cbuf").then_some((
                        &mut x[..],
                        &mut y[..],
                        &mut z[..],
                        &mut act[..],
                    ));
                    let o =
                        solve_lp_through_c(&c_env, &lp, 1, OPTSOLVER_METHOD_DUAL_SIMPLEX, bufs)?;
                    if o.status != 1 {
                        return Err(format!("status code {}", o.status));
                    }
                    Ok([o.timings.load, o.timings.solve, o.timings.extraction])
                }
                other => Err(format!("unknown path {other}")),
            }
        };
        match kind {
            "mem" => {
                let _profiler = dhat::Profiler::builder().testing().build();
                let before = dhat::HeapStats::get();
                once(path)?;
                let after = dhat::HeapStats::get();
                Ok(format!(
                    "peak_bytes={};blocks={}",
                    after.max_bytes.saturating_sub(before.curr_bytes),
                    after.total_blocks - before.total_blocks
                ))
            }
            "time" => {
                // The Rust and C paths alternate in this one process (path "both"), so drift of
                // the machine between processes does not enter the comparison.
                let t = Instant::now();
                let _e = Solver::new(solver)?;
                let env_s = t.elapsed().as_secs_f64();
                let mut walls: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
                let mut phases: [Vec<[f64; 3]>; 2] = [Vec::new(), Vec::new()];
                for _ in 0..=N_REPETITIONS {
                    for (k, p) in ["rust", "c"].iter().enumerate() {
                        let t = Instant::now();
                        let ph = once(p)?;
                        walls[k].push(t.elapsed().as_secs_f64());
                        phases[k].push(ph);
                    }
                }
                let fmt = |v: &[f64]| {
                    v.iter()
                        .map(|x| format!("{x:e}"))
                        .collect::<Vec<_>>()
                        .join(",")
                };
                // The effective parameters and solver version of the configuration measured.
                let A = CscRef::new(mp.m, mp.n, &mp.col_ptrs, &mp.row_indices, &mp.values)
                    .map_err(|e| e.to_string())?;
                let rows = RowBounds::Ranges {
                    b_l: &b_l,
                    b_u: &b_u,
                };
                let p = Problem::new(A, &c, mp.objective_sense, rows, &mp.l, &mp.u)
                    .map_err(|e| e.to_string())?;
                let mut o = Options::default();
                o.threads = Some(1);
                o.method = Some(Method::DualSimplex);
                let s = rust_env
                    .backend()
                    .solve((&p).into(), &mut o)
                    .map_err(|e| e.to_string())?;
                let params: Vec<String> = s
                    .effective_parameters
                    .iter()
                    .map(|(k, v)| match v {
                        ParamValue::Int(i) => format!("{k}={i}"),
                        ParamValue::Double(d) => format!("{k}={d}"),
                        ParamValue::Str(t) => format!("{k}={t}"),
                    })
                    .collect();
                eprintln!(
                    "config\t{} {}: {}",
                    s.solver.name,
                    s.solver.version,
                    params.join(" ")
                );
                let mut out = format!("env_creation={env_s:e}");
                for (k, p) in ["rust", "c"].iter().enumerate() {
                    out += &format!(
                        ";{p}_cold={:e};{p}_warm={};{p}_load={};{p}_solve={};{p}_extraction={}",
                        walls[k][0],
                        fmt(&walls[k][1..]),
                        fmt(&phases[k][1..].iter().map(|x| x[0]).collect::<Vec<_>>()),
                        fmt(&phases[k][1..].iter().map(|x| x[1]).collect::<Vec<_>>()),
                        fmt(&phases[k][1..].iter().map(|x| x[2]).collect::<Vec<_>>())
                    );
                }
                Ok(out)
            }
            other => Err(format!("unknown kind {other}")),
        }
    }

    fn field(line: &str, key: &str) -> Vec<f64> {
        let data = line.rsplit('|').next().unwrap_or("");
        data.split(';')
            .find_map(|kv| kv.strip_prefix(&format!("{key}=")))
            .map(|v| v.split(',').filter_map(|x| x.parse().ok()).collect())
            .unwrap_or_default()
    }

    pub fn main() {
        let exe = std::env::current_exe().expect("benchmark executable");
        let stamp = {
            let out = Command::new("date")
                .args(["-u", "+%Y%m%dT%H%M%SZ"])
                .output()
                .expect("date");
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };
        let dir = run_dir(&format!("{stamp}-capi"));
        std::fs::create_dir_all(&dir).expect("run bundle directory");
        let mut config = Vec::new();
        let mut run = |spec: String| -> String {
            let out = Command::new(&exe)
                .env(ENV, &spec)
                .output()
                .expect("child process");
            assert!(
                out.status.success(),
                "child {spec} failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            for l in String::from_utf8_lossy(&out.stderr).lines() {
                if let Some(rest) = l.strip_prefix("config\t") {
                    config.push(rest.to_string());
                }
            }
            let line = String::from_utf8_lossy(&out.stdout)
                .lines()
                .last()
                .unwrap_or("")
                .to_string();
            format!("{spec}|{line}")
        };
        let mut lines = Vec::new();
        let dims = {
            let f = common::fixtures::load_corpus_fixture(INSTANCE).expect("fixture");
            (f.problem.m, f.problem.n, f.problem.values.len())
        };
        let mut summary = format!(
            "# C path versus Rust path (feature 005, SC-002, SC-003)\n\nInstance {INSTANCE} (LP, m = {}, n = {}, nnz = {}; provenance record in tests/fixtures; seeded objective, seed 1, ranges form, one thread, dual simplex); commit {}; machine {}; profile {}\n\n",
            dims.0,
            dims.1,
            dims.2,
            commit(),
            machine(),
            profile()
        );
        for solver in solvers() {
            let mut peak = std::collections::BTreeMap::new();
            let mut blocks = std::collections::BTreeMap::new();
            for path in ["rust", "c", "cbuf"] {
                for _ in 0..3 {
                    let l = run(format!("{solver}|{path}|mem"));
                    peak.entry(path)
                        .or_insert_with(Vec::new)
                        .extend(field(&l, "peak_bytes"));
                    blocks
                        .entry(path)
                        .or_insert_with(Vec::new)
                        .extend(field(&l, "blocks"));
                    lines.push(l);
                }
            }
            let l = run(format!("{solver}|both|time"));
            let mut wall = std::collections::BTreeMap::new();
            for path in ["rust", "c"] {
                wall.insert(
                    path,
                    (
                        field(&l, &format!("{path}_warm")),
                        field(&l, &format!("{path}_cold")),
                        field(&l, "env_creation"),
                        field(&l, &format!("{path}_load")),
                        field(&l, &format!("{path}_solve")),
                        field(&l, &format!("{path}_extraction")),
                    ),
                );
            }
            lines.push(l);
            let (rp, _, _) = median_and_spread(&peak["rust"]);
            let mut s = format!("## {solver}\n\n");
            for path in ["c", "cbuf"] {
                let (cp, clo, chi) = median_and_spread(&peak[path]);
                let (rb, cb) = (
                    median_and_spread(&blocks["rust"]).0,
                    median_and_spread(&blocks[path]).0,
                );
                s += &format!(
                    "- SC-002 heap peak ({path}): C {cp:.0} B (min {clo:.0}, max {chi:.0}, N = 3) vs Rust {rp:.0} B (N = 3): difference {:.0} B, limit 4096 B → {}; allocation count C {cb:.0} vs Rust {rb:.0} (positive control: the C path ran)\n",
                    cp - rp,
                    if cp - rp <= 4096.0 { "PASS" } else { "FAIL" }
                );
            }
            let (rw, rlo, rhi) = median_and_spread(&wall["rust"].0);
            let (cw, clo, chi) = median_and_spread(&wall["c"].0);
            let limit = (0.01 * rw).max(1e-3);
            s += &format!(
                "- SC-003 warm wall time per solve call: C {cw:.4e} s (min {clo:.4e}, max {chi:.4e}) vs Rust {rw:.4e} s (min {rlo:.4e}, max {rhi:.4e}), N = 5 each, alternating in one process; difference {:.3e} s, limit max(1 %, 1 ms) = {limit:.3e} s → {}\n",
                cw - rw,
                if cw - rw <= limit { "PASS" } else { "FAIL" }
            );
            for path in ["rust", "c"] {
                let w = &wall[path];
                s += &format!(
                    "- {path}: environment creation {:.4e} s (N = 1), cold call {:.4e} s (N = 1); warm per-phase medians load {:.4e} s, solve {:.4e} s, extraction {:.4e} s (N = 5)\n",
                    w.2.first().copied().unwrap_or(f64::NAN),
                    w.1.first().copied().unwrap_or(f64::NAN),
                    median_and_spread(&w.3).0,
                    median_and_spread(&w.4).0,
                    median_and_spread(&w.5).0
                );
            }
            summary += &s;
            summary += "\n";
        }
        summary += "## Effective configuration (Rust path; the C path passes the same options)\n\n";
        config.sort();
        config.dedup();
        for c in config {
            summary += &format!("- {c}\n");
        }
        std::fs::write(dir.join("records.txt"), lines.join("\n") + "\n").expect("records");
        std::fs::write(dir.join("summary.md"), &summary).expect("summary");
        println!("{summary}");
    }
}

fn main() {
    #[cfg(feature = "capi")]
    {
        if let Ok(spec) = std::env::var(capi_suite::ENV) {
            match capi_suite::child(&spec) {
                Ok(line) => println!("{line}"),
                Err(e) => {
                    eprintln!("benchmark child {spec} failed: {e}");
                    std::process::exit(1);
                }
            }
            return;
        }
        if std::env::var("OPTSOLVER_BENCH_SUITE").is_ok_and(|v| v == "capi") {
            capi_suite::main();
            return;
        }
    }
    if let Ok(spec) = std::env::var(CHILD_ENV) {
        match child(&spec) {
            Ok(r) => println!("{}", r.to_json()),
            Err(e) => {
                eprintln!("benchmark child {spec} failed: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    record_writer_self_test();
    let mut instances =
        common::fixtures::lp_instances().expect("corpus instances from project.yml");
    // Feature 003 T021: the derived corpus QP as well.
    instances.extend(common::fixtures::qp_instances().expect("QP instances from project.yml"));
    let stamp = {
        let out = Command::new("date")
            .args(["-u", "+%Y%m%dT%H%M%SZ"])
            .output()
            .expect("date");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    let dir = run_dir(&stamp);
    std::fs::create_dir_all(&dir).expect("run bundle directory");
    let exe = std::env::current_exe().expect("benchmark executable");
    let mut lines = Vec::new();
    for solver in solvers() {
        for instance in &instances {
            for index_type in ["usize", "i32"] {
                let mut specs = vec![format!("{instance}|{index_type}|time|{solver}")];
                specs.extend(
                    (0..N_REPETITIONS).map(|_| format!("{instance}|{index_type}|mem|{solver}")),
                );
                for spec in specs {
                    let out = Command::new(&exe)
                        .env(CHILD_ENV, &spec)
                        .output()
                        .expect("child process");
                    if !out.status.success() {
                        panic!(
                            "child {spec} failed: {}",
                            String::from_utf8_lossy(&out.stderr)
                        );
                    }
                    let line = String::from_utf8_lossy(&out.stdout)
                        .lines()
                        .last()
                        .unwrap_or("")
                        .to_string();
                    lines.push(line);
                }
            }
        }
    }
    std::fs::write(dir.join("records.jsonl"), lines.join("\n") + "\n").expect("records");
    let records: Vec<Record> = lines.iter().map(|l| Record::from_json(l)).collect();
    let mut summary = format!("# Benchmark summary\n\nRun bundle: {}\n\n", dir.display());
    for solver in solvers() {
        for instance in &instances {
            for index_type in ["usize", "i32"] {
                let sel: Vec<&Record> = records
                    .iter()
                    .filter(|r| {
                        r.solver == solver && &r.instance == instance && r.index_type == index_type
                    })
                    .collect();
                let mem: Vec<f64> = sel
                    .iter()
                    .filter(|r| r.kind == "mem")
                    .filter_map(|r| r.heap_peak_during_load_bytes.map(|b| b as f64))
                    .collect();
                let Some(t) = sel.iter().find(|r| r.kind == "time") else {
                    continue;
                };
                let paths: Vec<&str> = sel
                    .iter()
                    .filter(|r| r.kind == "mem")
                    .map(|r| r.load_path.as_str())
                    .collect();
                let high_water = sel
                    .iter()
                    .filter(|r| r.kind == "mem")
                    .map(|r| r.buffer_high_water)
                    .max()
                    .unwrap_or(0);
                let (mm, mlo, mhi) = median_and_spread(&mem);
                // SC-004 rule plus the documented O(nnz(Q)) triplets: 12 B per term (two i32
                // index arrays and the halved values, Gurobi); MOSEK needs 8 B (feature 003
                // plan, Complexity Tracking).
                let limit = 32.0 * (t.m + t.n) as f64 + 1_048_576.0 + 12.0 * t.q_terms as f64;
                let added: Vec<f64> = (0..t.load.len())
                    .map(|k| {
                        (t.validation[k]
                            + t.load_optsolver[k]
                            + t.extraction[k]
                            + t.verification[k])
                            / t.solve[k]
                    })
                    .collect();
                let (am, alo, ahi) = median_and_spread(&added);
                let phase = |v: &[f64]| {
                    let (a, b, c) = median_and_spread(v);
                    format!("median {a:.3e} s (min {b:.3e}, max {c:.3e})")
                };
                summary += &format!(
                    "## {solver}: {instance}, {index_type} row indices (m = {}, n = {}, nnz = {})\n\n\
                 - Positive control (load path, every memory run): {:?}; buffer high-water {} entries; Q terms {}\n\
                 - SC-004 heap peak during load: median {mm:.0} B (min {mlo:.0}, max {mhi:.0}), N = {}; limit 32 B × (m + n) + 1 MiB + 12 B × Q terms = {limit:.0} B → {}\n\
                 - SC-005 (validation + optSolver load + extraction + verification) / solve: median {am:.3} (min {alo:.3}, max {ahi:.3}), N = {}; limit 0.05 → {} (validation + optSolver's own load work + extraction + verification; the solver's load calls excluded)\n\
                 - Validation: {}\n- Load, optSolver's own work: {}\n- Load, inside the solver's calls: {}\n\
                 - Environment creation: {}\n- Load: {}\n- Solve: {}\n- Extraction: {}\n- Verification: {}\n\
                 - Cold (first repetition) solve {:.3e} s; warm median {:.3e} s\n\
                 - VmHWM rise over the time run: {:?} KiB; Gurobi MaxMemUsed (Gurobi only) {:?} GB\n\
                 - Effective parameters: {}\n- {solver} {}, commit {}, machine {}, profile {}\n\n",
                    t.m,
                    t.n,
                    t.nnz,
                    paths,
                    high_water,
                    t.q_terms,
                    mem.len(),
                    if mm <= limit { "PASS" } else { "FAIL" },
                    added.len(),
                    if am <= 0.05 { "PASS" } else { "FAIL" },
                    phase(&t.validation),
                    phase(&t.load_optsolver),
                    phase(&t.load_vendor),
                    phase(&t.env_creation),
                    phase(&t.load),
                    phase(&t.solve),
                    phase(&t.extraction),
                    phase(&t.verification),
                    t.solve[0],
                    median_and_spread(&t.solve[1..]).0,
                    t.vmhwm_rise_kib,
                    t.gurobi_max_mem_used_gb,
                    t.effective_parameters,
                    t.solver_version,
                    t.optsolver_commit,
                    t.machine,
                    t.profile
                );
            }
        }
    }
    std::fs::write(dir.join("summary.md"), &summary).expect("summary");
    println!("{summary}");
}
