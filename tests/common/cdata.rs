//! Feature 005 (task T003): the shared C-interface test harness. Writes a problem file the C
//! driver (`tests/c/driver.c`) reads, compiles and runs C programs against the libraries Cargo
//! built, reads the driver's result file, and solves the same data through the Rust API so the
//! two can be compared with [`super::compare::agree_c_rust`].

#![allow(non_snake_case)] // Notation Convention (XII-C): A, Q, F are matrices.

use super::ccompiler;
use super::compare::{Observed, observe_rust};
use super::fixtures::repo_root;
use super::jtask::JtaskProblem;
use super::mps::MpsProblem;
use super::{Failed, ensure, fail};
use optsolver::{
    Backend, Cone, ConicRef, CscRef, Method, Options, ParamValue, Problem, RowBounds, Sense,
    Solution,
};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Row bounds of a C problem.
#[derive(Debug, Clone)]
pub enum Rows {
    SenseRhs(Vec<u8>, Vec<f64>),
    Ranges(Vec<f64>, Vec<f64>),
}

/// A CSC matrix's column pointers, row indices and values.
pub type Csc = (Vec<usize>, Vec<usize>, Vec<f64>);

/// k, F (column pointers, row indices, values) and g.
pub type Conic = (usize, Vec<usize>, Vec<usize>, Vec<f64>, Vec<f64>);

/// The caller buffers of a Rust buffer-path solve, by array name.
pub type NamedBuffers = Vec<(String, Vec<f64>)>;

/// A problem with every array owned, written for the driver and solved through Rust.
#[derive(Debug, Clone)]
pub struct CProblem {
    pub m: usize,
    pub n: usize,
    pub sense: Sense,
    pub gamma: f64,
    pub A_col_ptrs: Vec<usize>,
    pub A_row_indices: Vec<usize>,
    pub A_values: Vec<f64>,
    pub c: Vec<f64>,
    pub l: Vec<f64>,
    pub u: Vec<f64>,
    pub rows: Rows,
    pub Q: Option<Csc>,
    /// k, F's column pointers, row indices and values, and g.
    pub F: Option<Conic>,
    pub cones: Vec<Cone>,
}

/// How the driver solves it.
#[derive(Debug, Clone, Default)]
pub struct Run {
    pub backend: i32,
    pub index_i32: bool,
    pub buffers: bool,
    pub once: bool,
    pub repeats: usize,
    pub thread: bool,
    pub caps: bool,
    pub leak_result: bool,
    /// Options as driver keys (`opt_threads`, …) with numeric values.
    pub options: Vec<(&'static str, f64)>,
    /// Raw parameters: name, type code, number, string.
    pub raw: Vec<(String, i32, f64, Option<String>)>,
    /// Extra keys written verbatim (for deliberately invalid inputs).
    pub extra: Vec<(String, Vec<f64>)>,
    /// Keys to leave out (for deliberately invalid inputs).
    pub omit: Vec<&'static str>,
}

fn nums<T: Copy + Into<f64>>(v: &[T]) -> Vec<f64> {
    v.iter().map(|&x| x.into()).collect()
}

fn usz(v: &[usize]) -> Vec<f64> {
    #[allow(clippy::cast_precision_loss)] // test data: indices far below 2^53
    v.iter().map(|&x| x as f64).collect()
}

impl CProblem {
    /// From an MPS toy or corpus problem (QP when it has a quadratic section), with objective
    /// `c` (`None`: the file's). `ranges` writes the row bounds in the ranges form.
    pub fn from_mps(p: &MpsProblem, c: Option<&[f64]>, ranges: bool) -> Self {
        let rows = if ranges || p.has_ranges() {
            let (b_l, b_u) = p.row_ranges();
            Rows::Ranges(b_l, b_u)
        } else {
            Rows::SenseRhs(p.sense.clone(), p.b.clone())
        };
        Self {
            m: p.m,
            n: p.n,
            sense: p.objective_sense,
            gamma: p.gamma,
            A_col_ptrs: p.col_ptrs.clone(),
            A_row_indices: p.row_indices.clone(),
            A_values: p.values.clone(),
            c: c.map_or_else(|| p.c.clone(), <[f64]>::to_vec),
            l: p.l.clone(),
            u: p.u.clone(),
            rows,
            Q: (!p.q_col_ptrs.is_empty()).then(|| {
                (
                    p.q_col_ptrs.clone(),
                    p.q_row_indices.clone(),
                    p.q_values.clone(),
                )
            }),
            F: None,
            cones: Vec::new(),
        }
    }

    /// From a JTASK conic problem.
    pub fn from_jtask(p: &JtaskProblem) -> Self {
        Self {
            m: p.m,
            n: p.n,
            sense: p.sense,
            gamma: p.gamma,
            A_col_ptrs: p.A_col_ptrs.clone(),
            A_row_indices: p.A_row_indices.clone(),
            A_values: p.A_values.clone(),
            c: p.c.clone(),
            l: p.l.clone(),
            u: p.u.clone(),
            rows: Rows::Ranges(p.b_l.clone(), p.b_u.clone()),
            Q: None,
            F: (p.k > 0).then(|| {
                (
                    p.k,
                    p.F_col_ptrs.clone(),
                    p.F_row_indices.clone(),
                    p.F_values.clone(),
                    p.g.clone(),
                )
            }),
            cones: p.cones.clone(),
        }
    }

    /// Solves through the Rust API with the options the run describes: `solve`, or on the
    /// buffer path `solve_into` with NaN-filled buffers, returned beside the solution.
    pub fn solve_rust(
        &self,
        b: &dyn Backend,
        run: &Run,
    ) -> optsolver::error::Result<(Solution, Option<NamedBuffers>)> {
        let mut o = rust_options(run);
        let rows = match &self.rows {
            Rows::SenseRhs(s, b) => RowBounds::SenseRhs { sense: s, b },
            Rows::Ranges(l, u) => RowBounds::Ranges { b_l: l, b_u: u },
        };
        let ri32: Vec<i32>;
        macro_rules! go {
            ($A:expr) => {{
                let mut p = Problem::new($A, &self.c, self.sense, rows, &self.l, &self.u)?
                    .with_gamma(self.gamma)?;
                if let Some((cp, ri, va)) = &self.Q {
                    p = p.with_Q(CscRef::new(self.n, self.n, cp, ri, va)?)?;
                }
                if let Some((k, cp, ri, va, g)) = &self.F {
                    let f = CscRef::new(*k, self.n, cp, ri, va)?;
                    p = p.with_conic(ConicRef::new(f, g, &self.cones)?)?;
                }
                if run.buffers {
                    let kk = self.F.as_ref().map_or(0, |f| f.0);
                    let (n, m) = (self.n, self.m);
                    let mut v: Vec<Vec<f64>> = [n, m, n, m, kk, n, n]
                        .iter()
                        .map(|&l| vec![f64::NAN; l])
                        .collect();
                    let [x, y, z, act, s, zl, zu] = &mut v[..] else {
                        unreachable!()
                    };
                    let out = optsolver::SolutionBuffers {
                        x,
                        y: Some(y),
                        z: Some(z),
                        row_activity: Some(act),
                        s: Some(s),
                        z_l: Some(zl),
                        z_u: Some(zu),
                    };
                    let sol = b.solve_into((&p).into(), &mut o, out)?;
                    let names = ["x", "y", "z", "row_activity", "s", "z_l", "z_u"];
                    Ok((
                        sol,
                        Some(names.iter().map(|n| n.to_string()).zip(v).collect()),
                    ))
                } else {
                    Ok((b.solve((&p).into(), &mut o)?, None))
                }
            }};
        }
        if run.index_i32 {
            ri32 = self
                .A_row_indices
                .iter()
                .map(|&i| i32::try_from(i).unwrap_or(-1))
                .collect();
            go!(CscRef::new(
                self.m,
                self.n,
                &self.A_col_ptrs,
                &ri32,
                &self.A_values
            )?)
        } else {
            go!(CscRef::new(
                self.m,
                self.n,
                &self.A_col_ptrs,
                &self.A_row_indices,
                &self.A_values
            )?)
        }
    }

    /// The problem file for the driver.
    pub fn file(&self, run: &Run) -> String {
        let mut keys: Vec<(String, Vec<f64>)> = Vec::new();
        let mut k = |key: &str, v: Vec<f64>| keys.push((key.to_string(), v));
        #[allow(clippy::cast_precision_loss)]
        {
            k("backend", vec![f64::from(run.backend)]);
            k("m", vec![self.m as f64]);
            k("n", vec![self.n as f64]);
        }
        k("sense", vec![self.sense.sigma()]);
        k("gamma", vec![self.gamma]);
        k("index_i32", vec![f64::from(u8::from(run.index_i32))]);
        k("A_col_ptrs", usz(&self.A_col_ptrs));
        k("A_row_indices", usz(&self.A_row_indices));
        k("A_values", self.A_values.clone());
        k("c", self.c.clone());
        k("l", self.l.clone());
        k("u", self.u.clone());
        match &self.rows {
            Rows::SenseRhs(s, b) => {
                k("row_sense", nums(s));
                k("b", b.clone());
            }
            Rows::Ranges(l, u) => {
                k("b_l", l.clone());
                k("b_u", u.clone());
            }
        }
        if let Some((cp, ri, va)) = &self.Q {
            k("Q_col_ptrs", usz(cp));
            k("Q_row_indices", usz(ri));
            k("Q_values", va.clone());
        }
        if let Some((kk, cp, ri, va, g)) = &self.F {
            #[allow(clippy::cast_precision_loss)]
            k("k", vec![*kk as f64]);
            k("F_col_ptrs", usz(cp));
            k("F_row_indices", usz(ri));
            k("F_values", va.clone());
            k("g", g.clone());
            k(
                "cone_kinds",
                self.cones
                    .iter()
                    .map(|c| f64::from(optsolver::capi::cone_code(c.kind())))
                    .collect(),
            );
            #[allow(clippy::cast_precision_loss)]
            k(
                "cone_dims",
                self.cones
                    .iter()
                    .map(|c| match c {
                        Cone::PositiveSemidefinite { order } => *order as f64,
                        c => c.dim() as f64,
                    })
                    .collect(),
            );
            for (i, c) in self.cones.iter().enumerate() {
                if let Cone::PrimalPower { alpha, .. } | Cone::DualPower { alpha, .. } = c {
                    k(&format!("cone_exponents_{i}"), alpha.clone());
                }
            }
        }
        let flag = |b: bool| vec![f64::from(u8::from(b))];
        k("buffers", flag(run.buffers));
        k("once", flag(run.once));
        #[allow(clippy::cast_precision_loss)]
        k("repeats", vec![run.repeats.max(1) as f64]);
        k("thread", flag(run.thread));
        k("caps", flag(run.caps));
        k("leak_result", flag(run.leak_result));
        if run.buffers {
            let kk = self.F.as_ref().map_or(0, |f| f.0);
            #[allow(clippy::cast_precision_loss)]
            for (key, len) in [
                ("buf_x", self.n),
                ("buf_y", self.m),
                ("buf_z", self.n),
                ("buf_row_activity", self.m),
                ("buf_s", kk),
                ("buf_z_l", self.n),
                ("buf_z_u", self.n),
            ] {
                k(key, vec![len as f64]);
            }
        }
        for (key, v) in &run.options {
            k(key, vec![*v]);
        }
        #[allow(clippy::cast_precision_loss)]
        k("raw_count", vec![run.raw.len() as f64]);
        for (i, (_, t, v, _)) in run.raw.iter().enumerate() {
            k(&format!("raw_value_{i}"), vec![f64::from(*t), *v]);
        }
        for (key, v) in &run.extra {
            keys.retain(|(k2, _)| k2 != key);
            keys.push((key.clone(), v.clone()));
        }
        keys.retain(|(key, _)| !run.omit.contains(&key.as_str()));
        let mut s = String::new();
        for (key, v) in &keys {
            let _ = write!(s, "{key} {}", v.len());
            for x in v {
                let _ = write!(s, " {}", fmt(*x));
            }
            s.push('\n');
        }
        for (i, (name, _, _, text)) in run.raw.iter().enumerate() {
            let _ = writeln!(s, "str_raw_name_{i} 1 {name}");
            if let Some(t) = text {
                let _ = writeln!(s, "str_raw_string_{i} 1 {t}");
            }
        }
        s
    }
}

/// A number the C reader parses back exactly.
fn fmt(x: f64) -> String {
    if x.is_nan() {
        "nan".into()
    } else if x.is_infinite() {
        if x > 0.0 { "inf".into() } else { "-inf".into() }
    } else {
        format!("{x:e}")
    }
}

/// The Rust options a run describes (the same keys the driver reads).
pub fn rust_options(run: &Run) -> Options<'static> {
    let mut o = Options::default();
    for (key, v) in &run.options {
        let v = *v;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        match *key {
            "opt_time_limit" => o.time_limit = Some(v),
            "opt_iteration_limit" => o.iteration_limit = Some(v),
            "opt_threads" => o.threads = Some(v as u32),
            "opt_primal_feasibility_tolerance" => o.primal_feasibility_tolerance = Some(v),
            "opt_dual_feasibility_tolerance" => o.dual_feasibility_tolerance = Some(v),
            "opt_method" => o.method = optsolver::capi::method_of(v as i32),
            "opt_presolve" => o.presolve = optsolver::capi::presolve_of(v as i32),
            "opt_random_seed" => o.random_seed = Some(v as i32),
            "opt_solver_log" => o.solver_log = Some(v != 0.0),
            "opt_array_outputs" => o.array_outputs = Some(v != 0.0),
            "opt_verification" => o.verification = Some(v != 0.0),
            "opt_certificates" => o.certificates = v != 0.0,
            "opt_memory_limit_gb" => o.memory_limit_gb = Some(v),
            other => panic!("unknown option key {other}"),
        }
    }
    for (name, t, v, s) in &run.raw {
        #[allow(clippy::cast_possible_truncation)]
        let value = match *t {
            1 => ParamValue::Int(*v as i64),
            2 => ParamValue::Double(*v),
            _ => ParamValue::Str(s.clone().unwrap_or_default()),
        };
        o.raw.push((name.clone(), value));
    }
    o
}

/// The deterministic configuration of the comparison rule: one thread; dual simplex for LP,
/// barrier for QP and conic (spec, Clarifications).
pub fn deterministic(p: &CProblem) -> Vec<(&'static str, f64)> {
    let method = if p.Q.is_some() || p.F.is_some() {
        Method::Barrier
    } else {
        Method::DualSimplex
    };
    let code = match method {
        Method::Barrier => optsolver::capi::OPTSOLVER_METHOD_BARRIER,
        _ => optsolver::capi::OPTSOLVER_METHOD_DUAL_SIMPLEX,
    };
    vec![("opt_threads", 1.0), ("opt_method", f64::from(code))]
}

/// The driver's result file.
#[derive(Debug, Clone, Default)]
pub struct CResult {
    pub nums: BTreeMap<String, Vec<f64>>,
    pub strs: BTreeMap<String, String>,
}

impl CResult {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut r = CResult::default();
        for line in text.lines() {
            let mut it = line.splitn(3, ' ');
            let key = it.next().unwrap_or_default().to_string();
            let _count = it.next();
            let rest = it.next().unwrap_or_default();
            if let Some(k) = key.strip_prefix("str_") {
                r.strs.insert(k.to_string(), rest.to_string());
            } else {
                let v = rest
                    .split_whitespace()
                    .map(|t| t.parse::<f64>().map_err(|e| format!("{key}: {t}: {e}")))
                    .collect::<Result<Vec<_>, _>>()?;
                r.nums.insert(key, v);
            }
        }
        Ok(r)
    }
    pub fn num(&self, key: &str) -> Option<f64> {
        self.nums.get(key).and_then(|v| v.first().copied())
    }
    #[allow(clippy::cast_possible_truncation)]
    pub fn int(&self, key: &str) -> i32 {
        self.num(key).map_or(-999, |v| v as i32)
    }
    pub fn s(&self, key: &str) -> &str {
        self.strs.get(key).map_or("", String::as_str)
    }
    /// The observed form, for comparison. On the buffer path the arrays are the caller's
    /// buffers (an all-NaN buffer is an absent quantity).
    pub fn observe(&self, buffers: bool) -> Observed {
        let arr = |key: &str| -> Option<Vec<f64>> {
            let v = self.nums.get(key)?;
            (!v.is_empty() && !v.iter().all(|x| x.is_nan())).then(|| v.clone())
        };
        let a = |name: &str| {
            if buffers {
                let k = format!("out_buf_{name}");
                (name.to_string(), arr(&k))
            } else {
                (name.to_string(), arr(name))
            }
        };
        let mut failed = Vec::new();
        let nm = self.num("measure_count").unwrap_or(0.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        for i in 0..nm as usize {
            if self
                .nums
                .get(&format!("measure_{i}"))
                .and_then(|v| v.get(3))
                .copied()
                == Some(1.0)
            {
                failed.push(self.s(&format!("measure_name_{i}")).to_string());
            }
        }
        Observed {
            status: self.int("status").max(0),
            limit_reason: self.int("limit_reason"),
            returned: self.int("returned"),
            verification: self.int("verification"),
            failed_measures: failed,
            objective: self.num("objective"),
            bound: self.num("bound"),
            arrays: vec![
                a("x"),
                a("y"),
                a("z"),
                a("z_l"),
                a("z_u"),
                a("s"),
                a("row_activity"),
                ("cert_y".into(), arr("cert_y")),
                ("cert_z".into(), arr("cert_z")),
                ("cert_s".into(), arr("cert_s")),
                ("cert_ray".into(), arr("cert_ray")),
            ],
            primal_tolerance: None,
            dual_tolerance: None,
        }
    }
}

/// The directory holding the libraries Cargo built for this test run: an upward search from the
/// test executable for the profile directory containing `liboptsolver.a` (constitution IX-I).
pub fn library_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut dir = exe.parent();
    while let Some(d) = dir {
        if d.join("liboptsolver.a").is_file() {
            return Ok(d.to_path_buf());
        }
        dir = d.parent();
    }
    Err(format!("liboptsolver.a not found above {}", exe.display()))
}

/// How a C program is linked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    Static,
    Shared,
}

/// Vendor link flags for the compiled-in backends, from the build's environment variables.
fn vendor_flags() -> Vec<String> {
    let mut v = Vec::new();
    if cfg!(feature = "gurobi") {
        if let Ok(h) = std::env::var("GUROBI_HOME") {
            v.push(format!("-L{h}/lib"));
            v.push(format!("-Wl,-rpath,{h}/lib"));
        }
        v.push("-lgurobi130".into());
    }
    if cfg!(feature = "mosek") {
        if let Ok(b) = std::env::var("MOSEK_BINDIR") {
            v.push(format!("-L{b}"));
            v.push(format!("-Wl,-rpath,{b}"));
        }
        v.push("-lmosek64".into());
    }
    v
}

/// The Rust runtime's native libraries for the static library (`--print native-static-libs`).
const NATIVE: [&str; 7] = [
    "-lgcc_s",
    "-lutil",
    "-lrt",
    "-lpthread",
    "-lm",
    "-ldl",
    "-lc",
];

/// Compiles `tests/c/<name>.c` against the header and links it as `link`; `extra` are added
/// compiler flags (for example the sanitizer). Returns the executable.
pub fn build_c(name: &str, link: Link, extra: &[&str]) -> Result<PathBuf, String> {
    let root = repo_root();
    let lib = library_dir()?;
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("c_interface");
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let tag = if extra.is_empty() { "" } else { "-san" };
    let exe = out_dir.join(format!("{name}-{link:?}{tag}"));
    let tool = ccompiler::compiler(&out_dir);
    let mut cmd = tool.to_command();
    cmd.arg("-std=c11")
        .arg("-D_POSIX_C_SOURCE=200809L")
        .arg("-Wall")
        .arg("-Werror")
        .args(extra)
        .arg("-I")
        .arg(root.join("src/capi"))
        .arg(root.join(format!("tests/c/{name}.c")))
        .arg("-o")
        .arg(&exe);
    match link {
        Link::Static => {
            cmd.arg(lib.join("liboptsolver.a"));
        }
        Link::Shared => {
            cmd.arg(format!("-L{}", lib.display()))
                .arg(format!("-Wl,-rpath,{}", lib.display()))
                .arg("-l:liboptsolver.so");
        }
    }
    cmd.args(vendor_flags()).args(NATIVE).current_dir(&out_dir);
    let o = cmd.output().map_err(|e| e.to_string())?;
    if !o.status.success() {
        return Err(format!(
            "compiling tests/c/{name}.c failed:\n{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ));
    }
    Ok(exe)
}

/// The driver, compiled once per link kind per test process.
pub fn driver(link: Link) -> Result<PathBuf, Failed> {
    static S: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    static D: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    let cell = if link == Link::Static { &S } else { &D };
    cell.get_or_init(|| build_c("driver", link, &[]))
        .clone()
        .map_err(fail)
}

/// What a C program printed.
pub struct Ran {
    pub status: std::process::ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

/// Runs a program; the shared no-output rule: standard output and standard error must be empty
/// unless `allow_stderr` (the leak runs, whose sanitizer reports there).
pub fn run_program(
    exe: &Path,
    args: &[&Path],
    env: &[(&str, &str)],
    allow_stderr: bool,
) -> Result<Ran, Failed> {
    let mut cmd = Command::new(exe);
    cmd.args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let o = cmd.output().map_err(fail)?;
    let ran = Ran {
        status: o.status,
        stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    };
    check_silent(&ran, allow_stderr)?;
    Ok(ran)
}

/// The no-output assertion (FR-018), separate so that it can be tested against a program
/// that prints (XV-C).
pub fn check_silent(ran: &Ran, allow_stderr: bool) -> Result<(), Failed> {
    ensure(
        ran.stdout.is_empty(),
        format!("standard output not empty: {:?}", ran.stdout),
    )?;
    ensure(
        allow_stderr || ran.stderr.is_empty(),
        format!("standard error not empty: {:?}", ran.stderr),
    )
}

/// Writes the problem file, runs the driver, and reads its result.
pub fn run_driver(link: Link, p: &CProblem, run: &Run, label: &str) -> Result<CResult, Failed> {
    let exe = driver(link)?;
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("c_interface")
        .join("runs");
    std::fs::create_dir_all(&dir).map_err(fail)?;
    let safe: String = label
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let input = dir.join(format!("{safe}-{link:?}.in"));
    let output = dir.join(format!("{safe}-{link:?}.out"));
    std::fs::write(&input, p.file(run)).map_err(fail)?;
    let ran = run_program(&exe, &[&input, &output], &[], false)?;
    ensure(
        ran.status.success(),
        format!("driver exited with {}", ran.status),
    )?;
    CResult::parse(&std::fs::read_to_string(&output).map_err(fail)?).map_err(fail)
}

/// Solves `p` through the driver and through the Rust API on the same backend and compares
/// them by the agreement rule; returns whether they were bit-identical.
pub fn compare_paths(
    b: &dyn Backend,
    link: Link,
    p: &CProblem,
    run: &Run,
    label: &str,
) -> Result<bool, Failed> {
    let c = run_driver(link, p, run, label)?;
    let r = p.solve_rust(b, run);
    match r {
        Err(e) => {
            // Both paths must fail with the same code and message.
            ensure(
                c.int("rc") == optsolver::capi::code_of(&e) && c.s("message") == e.to_string(),
                format!(
                    "{label}: Rust error {e}; C rc {} {:?}",
                    c.int("rc"),
                    c.s("message")
                ),
            )?;
            Ok(true)
        }
        Ok((sol, bufs)) => {
            ensure(
                c.int("rc") == 0,
                format!("{label}: C rc {} {:?}", c.int("rc"), c.s("message")),
            )?;
            let mut want = observe_rust(&sol);
            if let Some(bufs) = bufs {
                // The buffer path on both sides: compare the caller arrays (all-NaN = absent).
                for (name, v) in bufs {
                    let v = (!v.is_empty() && !v.iter().all(|x| x.is_nan())).then_some(v);
                    if let Some(slot) = want.arrays.iter_mut().find(|(n, _)| *n == name) {
                        slot.1 = v;
                    }
                }
            }
            let got = c.observe(run.buffers);
            super::compare::agree_c_rust(&got, &want).map_err(|e| fail(format!("{label}: {e}")))
        }
    }
}

/// The C error-path program and others, compiled on demand.
pub fn program(name: &'static str, link: Link, extra: &[&str]) -> Result<PathBuf, Failed> {
    build_c(name, link, extra).map_err(fail)
}

/// The value of an effective parameter the driver wrote, by name.
pub fn parameter(c: &CResult, name: &str) -> Option<String> {
    let n = c.num("parameter_count").unwrap_or(0.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    (0..n as usize).find_map(|i| {
        let s = c.s(&format!("parameter_{i}"));
        s.strip_prefix(&format!("{name}=")).map(str::to_string)
    })
}

/// A Rust-side failure helper for missing data.
pub fn need<T>(v: Option<T>, what: &str) -> Result<T, Failed> {
    v.ok_or_else(|| fail(format!("missing {what}")))
}
