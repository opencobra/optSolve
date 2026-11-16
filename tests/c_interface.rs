//! Feature 005: tests of the C interface (specs/005-c-abi/tasks.md, T004, T005, T007, T008,
//! T010, T012, T014 to T016). Custom harness over the shared registry: licensed trials are
//! ignored, with the reason, where a backend is not compiled in or not usable. C programs are
//! compiled against `src/capi/optsolver.h` and linked to the static or shared library Cargo
//! built.

#![allow(non_snake_case)] // Notation Convention (XII-C): A, F are matrices.

mod common;

use common::cdata::{self, CProblem, CResult, Link, Run, compare_paths, run_driver};
use common::toy::{Toy, ToyData};
use common::{Arguments, Failed, Registry, Trial, ensure, fail};
use optsolver::capi::*;
use optsolver::{Backend, Status};
use std::collections::{BTreeMap, BTreeSet};
use std::mem::{offset_of, size_of};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

static PREVIOUS_HOOK_CALLED: AtomicBool = AtomicBool::new(false);

fn main() {
    // Before any trial: a hook installed by the host before the library's first use must still
    // receive panics outside entry points, and stay unaware of contained ones (research R4).
    std::panic::set_hook(Box::new(|_| {
        PREVIOUS_HOOK_CALLED.store(true, Ordering::SeqCst)
    }));
    let _ = optsolver_abi_version(); // installs the library's wrapping hook
    let _ = std::panic::catch_unwind(|| panic!("outside an entry point"));
    let reached = PREVIOUS_HOOK_CALLED.load(Ordering::SeqCst);
    PREVIOUS_HOOK_CALLED.store(false, Ordering::SeqCst);
    let inside_silent = optsolver_test_panic() == OPTSOLVER_ERR_INTERNAL
        && !PREVIOUS_HOOK_CALLED.load(Ordering::SeqCst);

    let args = Arguments::from_args();
    let registry = Registry::detect();
    let mut trials = vec![
        Trial::test(
            "t005::panic_outside_entry_reaches_previous_hook",
            move || ensure(reached, "the previously installed hook was not called"),
        ),
        Trial::test("t005::panic_inside_entry_is_silent", move || {
            ensure(inside_silent, "a contained panic reached the previous hook")
        }),
        Trial::test(
            "t003::runner_rejects_a_program_that_prints",
            t003_runner_rejects_output,
        ),
        Trial::test("t004::layout_and_constants_agree_with_header", t004_layout),
        Trial::test("t004::prototypes_agree_with_header", t004_prototypes),
        Trial::test(
            "t004::prototype_check_fails_on_altered_header",
            t004_prototype_negative,
        ),
        Trial::test("t016::errors_program_static", || t016_errors(Link::Static)),
        Trial::test("t016::errors_program_shared", || t016_errors(Link::Shared)),
        Trial::test("t016::invalid_inputs_through_c", t016_invalid_inputs),
        Trial::test("t016::leak_check_unlicensed", t016_leaks_unlicensed),
    ];
    trials.extend(common::compare::agreement_self_tests());
    type Licensed = fn(&dyn Backend) -> Result<(), Failed>;
    let licensed: [(&str, Licensed); 10] = [
        ("t007::capabilities_match_rust", t007_capabilities),
        ("t007::not_usable_backend_reports_reason", t007_not_usable),
        ("t008::accessors_and_edge_cases", t008_accessors),
        ("t010::lp_qp_toys_agree", t010_lp_qp_toys),
        ("t010::certificates_agree", t010_certificates),
        ("t012::reuse_one_handle", t012_reuse),
        ("t012::handle_moves_between_threads", t012_thread),
        ("t014::memory_limit_through_c", t014_memory_limit),
        ("t015::conic_toys_agree", t015_conic_toys),
        ("t016::leak_check_licensed", t016_leaks_licensed),
    ];
    for (name, f) in licensed {
        trials.extend(registry.trials(name, f));
    }
    libtest_mimic::run(&args, trials).exit();
}

fn backend_code(b: &dyn Backend) -> i32 {
    match b.capabilities().backend {
        "gurobi" => OPTSOLVER_BACKEND_GUROBI,
        "mosek" => OPTSOLVER_BACKEND_MOSEK,
        _ => 0,
    }
}

fn run(b: &dyn Backend) -> Run {
    Run {
        backend: backend_code(b),
        ..Run::default()
    }
}

fn runs_dir() -> Result<std::path::PathBuf, Failed> {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("c_interface")
        .join("runs");
    std::fs::create_dir_all(&dir).map_err(fail)?;
    Ok(dir)
}

// ---------------------------------------------------------------------------------------------
// T003: the runner's no-output assertion rejects a program that prints.

fn t003_runner_rejects_output() -> Result<(), Failed> {
    let exe = cdata::program("printer", Link::Static, &[])?;
    let out = std::process::Command::new(&exe).output().map_err(fail)?;
    let ran = cdata::Ran {
        status: out.status,
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    };
    ensure(
        cdata::check_silent(&ran, false).is_err(),
        "printing program accepted",
    )?;
    let ran = cdata::Ran {
        stdout: String::new(),
        ..ran
    };
    ensure(
        cdata::check_silent(&ran, false).is_err(),
        "standard error output accepted",
    )
}

// ---------------------------------------------------------------------------------------------
// T004: header agreement.

fn header() -> Result<String, Failed> {
    std::fs::read_to_string(common::fixtures::repo_root().join("src/capi/optsolver.h"))
        .map_err(fail)
}

/// Every Rust constant, by header name.
fn rust_constants() -> BTreeMap<&'static str, i64> {
    macro_rules! table { ($($c:ident),* $(,)?) => { BTreeMap::from([$((stringify!($c), i64::from($c))),*]) }; }
    table!(
        OPTSOLVER_ABI_VERSION_MAJOR,
        OPTSOLVER_ABI_VERSION_MINOR,
        OPTSOLVER_OK,
        OPTSOLVER_ERR_INVALID,
        OPTSOLVER_ERR_TOO_LARGE,
        OPTSOLVER_ERR_UNSUPPORTED,
        OPTSOLVER_ERR_PARAMETER_CONFLICT,
        OPTSOLVER_ERR_SOLVER,
        OPTSOLVER_ERR_INTERNAL,
        OPTSOLVER_BACKEND_GUROBI,
        OPTSOLVER_BACKEND_MOSEK,
        OPTSOLVER_SENSE_MINIMISE,
        OPTSOLVER_SENSE_MAXIMISE,
        OPTSOLVER_UNSET,
        OPTSOLVER_STATUS_OPTIMAL,
        OPTSOLVER_STATUS_INFEASIBLE,
        OPTSOLVER_STATUS_UNBOUNDED,
        OPTSOLVER_STATUS_INFEASIBLE_OR_UNBOUNDED,
        OPTSOLVER_STATUS_STOPPED_AT_LIMIT,
        OPTSOLVER_STATUS_NUMERICAL_DIFFICULTY,
        OPTSOLVER_STATUS_INTERRUPTED,
        OPTSOLVER_STATUS_LOCALLY_OPTIMAL,
        OPTSOLVER_STATUS_LOCALLY_INFEASIBLE,
        OPTSOLVER_STATUS_NOT_SOLVED,
        OPTSOLVER_STATUS_UNKNOWN,
        OPTSOLVER_LIMIT_NONE,
        OPTSOLVER_LIMIT_TIME,
        OPTSOLVER_LIMIT_ITERATIONS,
        OPTSOLVER_LIMIT_NODES,
        OPTSOLVER_LIMIT_SOLUTIONS,
        OPTSOLVER_LIMIT_OBJECTIVE_CUTOFF,
        OPTSOLVER_LIMIT_OBJECTIVE_TARGET,
        OPTSOLVER_LIMIT_WORK,
        OPTSOLVER_LIMIT_MEMORY,
        OPTSOLVER_RETURNED_NONE,
        OPTSOLVER_RETURNED_FEASIBLE,
        OPTSOLVER_RETURNED_OPTIMAL,
        OPTSOLVER_CONE_QUADRATIC,
        OPTSOLVER_CONE_ROTATED_QUADRATIC,
        OPTSOLVER_CONE_PRIMAL_EXPONENTIAL,
        OPTSOLVER_CONE_DUAL_EXPONENTIAL,
        OPTSOLVER_CONE_PRIMAL_POWER,
        OPTSOLVER_CONE_DUAL_POWER,
        OPTSOLVER_CONE_NONNEGATIVE,
        OPTSOLVER_CONE_ZERO,
        OPTSOLVER_CONE_POSITIVE_SEMIDEFINITE,
        OPTSOLVER_CLASS_LP,
        OPTSOLVER_CLASS_MILP,
        OPTSOLVER_CLASS_QP,
        OPTSOLVER_CLASS_CONIC,
        OPTSOLVER_METHOD_AUTO,
        OPTSOLVER_METHOD_PRIMAL_SIMPLEX,
        OPTSOLVER_METHOD_DUAL_SIMPLEX,
        OPTSOLVER_METHOD_BARRIER,
        OPTSOLVER_METHOD_CONCURRENT,
        OPTSOLVER_PRESOLVE_AUTO,
        OPTSOLVER_PRESOLVE_OFF,
        OPTSOLVER_PRESOLVE_CONSERVATIVE,
        OPTSOLVER_PRESOLVE_AGGRESSIVE,
        OPTSOLVER_BASIS_BASIC,
        OPTSOLVER_BASIS_AT_LOWER,
        OPTSOLVER_BASIS_AT_UPPER,
        OPTSOLVER_BASIS_SUPERBASIC,
        OPTSOLVER_BASIS_FIXED,
        OPTSOLVER_VERIFICATION_NOT_RUN,
        OPTSOLVER_VERIFICATION_VERIFIED,
        OPTSOLVER_VERIFICATION_FAILED,
        OPTSOLVER_VERIFICATION_PARTIALLY_VERIFIED,
        OPTSOLVER_VERDICT_PASS,
        OPTSOLVER_VERDICT_FAIL,
        OPTSOLVER_VERDICT_NOT_EVALUATED,
        OPTSOLVER_ARRAY_X,
        OPTSOLVER_ARRAY_Y,
        OPTSOLVER_ARRAY_Z,
        OPTSOLVER_ARRAY_Z_L,
        OPTSOLVER_ARRAY_Z_U,
        OPTSOLVER_ARRAY_S,
        OPTSOLVER_ARRAY_ROW_ACTIVITY,
        OPTSOLVER_CERTIFICATE_NONE,
        OPTSOLVER_CERTIFICATE_FARKAS,
        OPTSOLVER_CERTIFICATE_UNBOUNDED_RAY,
        OPTSOLVER_CERT_Y,
        OPTSOLVER_CERT_Z,
        OPTSOLVER_CERT_S,
        OPTSOLVER_CERT_RAY,
        OPTSOLVER_PARAM_INT,
        OPTSOLVER_PARAM_DOUBLE,
        OPTSOLVER_PARAM_STRING,
    )
}

/// Every Rust structure field: "structure.field" → (offset, size), and structure sizes.
fn rust_layout() -> (BTreeMap<String, (usize, usize)>, BTreeMap<String, usize>) {
    let mut f = BTreeMap::new();
    let mut s = BTreeMap::new();
    macro_rules! layout {
        ($t:ident: $($field:ident => $cname:literal: $ft:ty),* $(,)?) => {{
            s.insert(stringify!($t).to_string(), size_of::<$t>());
            $(f.insert(format!("{}.{}", stringify!($t), $cname), (offset_of!($t, $field), size_of::<$ft>()));)*
        }};
    }
    layout!(optsolver_csc: rows => "rows": usize, cols => "cols": usize, col_ptrs => "col_ptrs": usize,
        row_indices => "row_indices": usize, values => "values": usize);
    layout!(optsolver_cone: kind => "kind": i32, dim => "dim": usize, exponents => "exponents": usize,
        exponents_len => "exponents_len": usize);
    layout!(optsolver_problem: struct_size => "struct_size": u32, A => "A": optsolver_csc,
        A_row_indices_i32 => "A_row_indices_i32": usize, c => "c": usize, sense => "sense": i32,
        gamma => "gamma": f64, l => "l": usize, u => "u": usize, row_sense => "row_sense": usize,
        b => "b": usize, b_l => "b_l": usize, b_u => "b_u": usize, Q => "Q": usize, F => "F": usize,
        g => "g": usize, cones => "cones": usize, num_cones => "num_cones": usize);
    layout!(optsolver_param: name => "name": usize, r#type => "type": i32, int_value => "int_value": i64,
        double_value => "double_value": f64, string_value => "string_value": usize);
    layout!(optsolver_options: struct_size => "struct_size": u32, time_limit => "time_limit": f64,
        iteration_limit => "iteration_limit": f64, threads => "threads": i64,
        primal_feasibility_tolerance => "primal_feasibility_tolerance": f64,
        dual_feasibility_tolerance => "dual_feasibility_tolerance": f64, method => "method": i32,
        presolve => "presolve": i32, random_seed => "random_seed": i64,
        random_seed_set => "random_seed_set": i32, solver_log => "solver_log": i32,
        array_outputs => "array_outputs": i32, verification => "verification": i32,
        certificates => "certificates": i32, memory_limit_gb => "memory_limit_gb": f64,
        raw => "raw": usize, num_raw => "num_raw": usize);
    layout!(optsolver_array: data => "data": usize, len => "len": usize);
    layout!(optsolver_buffers: struct_size => "struct_size": u32, x => "x": optsolver_array,
        y => "y": optsolver_array, z => "z": optsolver_array, row_activity => "row_activity": optsolver_array,
        s => "s": optsolver_array, z_l => "z_l": optsolver_array, z_u => "z_u": optsolver_array);
    layout!(optsolver_timings: load => "load": f64, solve => "solve": f64, extraction => "extraction": f64,
        verification => "verification": f64);
    layout!(optsolver_value: r#type => "type": i32, int_value => "int_value": i64,
        double_value => "double_value": f64, string_value => "string_value": usize);
    layout!(optsolver_measure: name => "name": usize, absolute => "absolute": f64, relative => "relative": f64,
        tolerance => "tolerance": f64, verdict => "verdict": i32, required => "required": i32);
    (f, s)
}

fn t004_layout() -> Result<(), Failed> {
    let exe = cdata::program("layout", Link::Static, &[])?;
    let out = std::process::Command::new(&exe).output().map_err(fail)?;
    ensure(out.status.success(), "layout program failed")?;
    let text = String::from_utf8_lossy(&out.stdout);
    let (fields, sizes) = rust_layout();
    let consts = rust_constants();
    let (mut seen_c, mut seen_f, mut seen_s) = (BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
    for line in text.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        match w.as_slice() {
            ["const", name, v] => {
                let v: i64 = v.parse().map_err(fail)?;
                ensure(
                    consts.get(name) == Some(&v),
                    format!("{name}: header {v}, Rust {:?}", consts.get(name)),
                )?;
                seen_c.insert(name.to_string());
            }
            ["size", name, v] => {
                let v: usize = v.parse().map_err(fail)?;
                ensure(
                    sizes.get(*name) == Some(&v),
                    format!("sizeof {name}: header {v}, Rust {:?}", sizes.get(*name)),
                )?;
                seen_s.insert(name.to_string());
            }
            ["field", name, off, sz] => {
                let got = (
                    off.parse::<usize>().map_err(fail)?,
                    sz.parse::<usize>().map_err(fail)?,
                );
                ensure(
                    fields.get(*name) == Some(&got),
                    format!("{name}: header {got:?}, Rust {:?}", fields.get(*name)),
                )?;
                seen_f.insert(name.to_string());
            }
            _ => return Err(fail(format!("unexpected line {line:?}"))),
        }
    }
    for k in consts.keys() {
        ensure(
            seen_c.contains(*k),
            format!("{k} is defined in Rust but not printed from the header"),
        )?;
    }
    for k in fields.keys() {
        ensure(
            seen_f.contains(k),
            format!("{k} is laid out in Rust but not printed from the header"),
        )?;
    }
    for k in sizes.keys() {
        ensure(seen_s.contains(k), format!("{k} has no size line"))?;
    }
    let h = header()?;
    for line in h.lines() {
        if let Some(rest) = line.strip_prefix("#define OPTSOLVER_") {
            let name = format!(
                "OPTSOLVER_{}",
                rest.split_whitespace().next().unwrap_or_default()
            );
            if name != "OPTSOLVER_H" {
                ensure(
                    seen_c.contains(&name),
                    format!("{name} in the header is not checked"),
                )?;
            }
        }
    }
    let src = std::fs::read_to_string(common::fixtures::repo_root().join("src/capi/mod.rs"))
        .map_err(fail)?;
    for line in src.lines() {
        if let Some(rest) = line.trim().strip_prefix("pub const OPTSOLVER_") {
            let name = format!("OPTSOLVER_{}", rest.split(':').next().unwrap_or_default());
            ensure(
                consts.contains_key(name.as_str()),
                format!("{name} missing from the test's table"),
            )?;
        }
    }
    Ok(())
}

/// C type of a Rust parameter or return type, through one fixed table.
fn c_type(rust: &str) -> Result<String, String> {
    let t = rust.trim();
    if let Some(inner) = t.strip_prefix("*const ") {
        let i = c_type(inner)?;
        return Ok(if i.ends_with('*') {
            format!("{i}*")
        } else {
            format!("const {i} *")
        });
    }
    if let Some(inner) = t.strip_prefix("*mut ") {
        let i = c_type(inner)?;
        return Ok(if i.ends_with('*') {
            format!("{i}*")
        } else {
            format!("{i} *")
        });
    }
    Ok(match t {
        "i32" => "int32_t",
        "u32" => "uint32_t",
        "i64" => "int64_t",
        "u64" => "uint64_t",
        "usize" => "size_t",
        "f64" => "double",
        "c_char" => "char",
        "" | "()" => "void",
        x if x.starts_with("optsolver_") => x,
        other => return Err(format!("no C type for Rust type {other}")),
    }
    .to_string())
}

fn norm_type(t: &str) -> String {
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    t.replace(" *", "*")
        .replace('*', " *")
        .replace("* *", "**")
        .replace("  ", " ")
        .trim()
        .to_string()
}

type Prototypes = BTreeMap<String, (String, Vec<String>)>;

/// `name → (return, params)` of the Rust `extern "C" fn` definitions in src/capi/.
fn rust_prototypes() -> Result<Prototypes, Failed> {
    let mut out = BTreeMap::new();
    for f in ["mod.rs", "problem.rs", "env.rs", "result.rs"] {
        let text = std::fs::read_to_string(common::fixtures::repo_root().join("src/capi").join(f))
            .map_err(fail)?;
        let mut rest = text.as_str();
        while let Some(i) = rest.find("extern \"C\" fn ") {
            let d = &rest[i + "extern \"C\" fn ".len()..];
            let open = d.find('(').ok_or_else(|| fail("("))?;
            let name = d[..open].trim().to_string();
            let close = d.find(')').ok_or_else(|| fail(")"))?;
            let params = d[open + 1..close]
                .split(',')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(|p| {
                    p.split_once(':')
                        .map(|(_, t)| c_type(t).map(|t| norm_type(&t)))
                        .ok_or_else(|| format!("param {p}"))?
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(fail)?;
            let after = &d[close + 1..];
            let brace = after.find('{').ok_or_else(|| fail("{"))?;
            let ret = after[..brace]
                .trim()
                .strip_prefix("->")
                .unwrap_or("")
                .trim();
            out.insert(name, (norm_type(&c_type(ret).map_err(fail)?), params));
            rest = &d[close..];
        }
    }
    Ok(out)
}

/// `name → (return, params)` of the header's prototypes, parameter names removed.
fn header_prototypes(h: &str) -> Prototypes {
    let mut code = String::new();
    let mut rest = h;
    while let Some(i) = rest.find("/*") {
        code.push_str(&rest[..i]);
        rest = rest[i..].find("*/").map_or("", |j| &rest[i + j + 2..]);
    }
    code.push_str(rest);
    let code: String = code
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
        .replace("extern \"C\" {", "");
    let mut out = BTreeMap::new();
    for stmt in code.split(';') {
        let s = stmt.split_whitespace().collect::<Vec<_>>().join(" ");
        let s = s.rsplit('}').next().unwrap_or_default().trim().to_string();
        if s.contains("typedef") || !s.ends_with(')') {
            continue;
        }
        let Some(open) = s.find('(') else { continue };
        let head = &s[..open];
        let Some(name_start) = head.rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        else {
            continue;
        };
        let name = head[name_start + 1..].to_string();
        if !name.starts_with("optsolver_") {
            continue;
        }
        let ret = norm_type(&head[..=name_start]);
        let params: Vec<String> = s[open + 1..s.len() - 1]
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty() && *p != "void")
            .map(|p| {
                let cut = p
                    .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .map_or(0, |i| i + 1);
                norm_type(&p[..cut])
            })
            .collect();
        out.insert(name, (ret, params));
    }
    out
}

fn prototype_mismatches(h: &str) -> Result<Vec<String>, Failed> {
    let rust = rust_prototypes()?;
    let c = header_prototypes(h);
    let mut bad = Vec::new();
    for (name, (ret, params)) in &rust {
        match c.get(name) {
            None => bad.push(format!(
                "{name}: defined in Rust, not declared in the header"
            )),
            Some((cret, cparams)) => {
                if ret != cret || params != cparams {
                    bad.push(format!(
                        "{name}: Rust ({} -> {ret}), header ({} -> {cret})",
                        params.join(", "),
                        cparams.join(", ")
                    ));
                }
            }
        }
    }
    for name in c.keys() {
        if !rust.contains_key(name) {
            bad.push(format!(
                "{name}: declared in the header, not defined in Rust"
            ));
        }
    }
    Ok(bad)
}

fn t004_prototypes() -> Result<(), Failed> {
    let bad = prototype_mismatches(&header()?)?;
    ensure(bad.is_empty(), format!("{bad:#?}"))?;
    ensure(rust_prototypes()?.len() >= 55, "too few functions parsed")
}

fn t004_prototype_negative() -> Result<(), Failed> {
    let h = header()?;
    let altered = h.replacen(
        "int32_t optsolver_result_status(",
        "int64_t optsolver_result_status(",
        1,
    );
    ensure(altered != h, "alteration did not apply")?;
    let bad = prototype_mismatches(&altered)?;
    ensure(
        bad.iter().any(|b| b.contains("optsolver_result_status")),
        "altered return type not caught",
    )?;
    let removed = h.replacen("void optsolver_caps_free(optsolver_caps *caps);", "", 1);
    ensure(removed != h, "removal did not apply")?;
    let bad = prototype_mismatches(&removed)?;
    ensure(
        bad.iter().any(|b| b.contains("optsolver_caps_free")),
        "removed prototype not caught",
    )
}

// ---------------------------------------------------------------------------------------------
// T016: error paths, invalid inputs and leaks.

fn run_errors(
    link: Link,
    extra: &[&str],
    leak: bool,
    allow_stderr: bool,
) -> Result<(String, cdata::Ran), Failed> {
    let exe = cdata::program("errors", link, extra)?;
    let out = runs_dir()?.join(format!("errors-{link:?}-{}-{leak}.out", extra.len()));
    let leak_arg = Path::new("leak");
    let mut args: Vec<&Path> = vec![out.as_path()];
    if leak {
        args.push(leak_arg);
    }
    let ran = cdata::run_program(
        &exe,
        &args,
        &[("ASAN_OPTIONS", "detect_leaks=1")],
        allow_stderr,
    )?;
    Ok((std::fs::read_to_string(&out).unwrap_or_default(), ran))
}

fn t016_errors(link: Link) -> Result<(), Failed> {
    let (text, ran) = run_errors(link, &[], false, false)?;
    ensure(
        ran.status.success() && text.trim() == "ok",
        format!("errors.c: {text} ({})", ran.status),
    )
}

fn asan_available() -> bool {
    cdata::program("printer", Link::Static, &["-fsanitize=address"]).is_ok()
}

fn t016_leaks_unlicensed() -> Result<(), Failed> {
    if !asan_available() {
        return Err(fail("AddressSanitizer unavailable: leak check unverified"));
    }
    let san = ["-fsanitize=address", "-fno-omit-frame-pointer"];
    let (text, ran) = run_errors(Link::Static, &san, false, true)?;
    ensure(
        ran.status.success() && text.trim() == "ok",
        format!("clean run: {text} {}", ran.stderr),
    )?;
    ensure(
        !ran.stderr.contains("LeakSanitizer"),
        format!("leaks reported:\n{}", ran.stderr),
    )?;
    let (_, ran) = run_errors(Link::Static, &san, true, true)?;
    ensure(
        ran.stderr.contains("LeakSanitizer") && !ran.status.success(),
        "positive control: the deliberate leak was not reported",
    )
}

/// A valid 3x2 problem (the Rust contract tests' matrix) for invalid-input cases.
fn small() -> CProblem {
    CProblem {
        m: 3,
        n: 2,
        sense: optsolver::Sense::Minimise,
        gamma: 0.0,
        A_col_ptrs: vec![0, 2, 3],
        A_row_indices: vec![0, 2, 1],
        A_values: vec![1.0, 2.0, 3.0],
        c: vec![1.0, 1.0],
        l: vec![0.0, 0.0],
        u: vec![1.0, 1.0],
        rows: cdata::Rows::SenseRhs(b"=<>".to_vec(), vec![1.0, 1.0, 1.0]),
        Q: None,
        F: None,
        cones: Vec::new(),
    }
}

type Change = fn(&mut CProblem, &mut Run);

fn t016_invalid_inputs() -> Result<(), Failed> {
    // Each `expect_invalid` case of tests/contract.rs that a C caller can express, plus the C
    // layer's own checks. Not expressible from C (lengths are derived from the dimensions, and
    // variable types are not in interface 1.0): t020_col_ptrs_wrong_length,
    // t020_values_wrong_length, t020_col_ptrs_last_not_nnz (nnz is col_ptrs[n] by definition), the length cases of t020_c_wrong_length_or_non_finite and
    // t020_vector_lengths, and the var_types cases of t020_gamma_and_var_types. The too-large
    // dimension cases are in tests/c/errors.c.
    let cases: Vec<(&str, Change, &str, &str)> = vec![
        (
            "col_ptrs_not_starting_at_zero",
            |p, _| p.A_col_ptrs = vec![1, 2, 3],
            "A.col_ptrs",
            "col_ptrs[0]",
        ),
        (
            "col_ptrs_decreasing",
            |p, _| {
                p.n = 3;
                p.A_col_ptrs = vec![0, 2, 1, 3];
                p.c = vec![1.0; 3];
                p.l = vec![0.0; 3];
                p.u = vec![1.0; 3];
            },
            "A.col_ptrs",
            "decreasing",
        ),
        (
            "row_index_out_of_range",
            |p, _| p.A_row_indices = vec![0, 3, 1],
            "A.row_indices",
            "out of range",
        ),
        (
            "row_indices_unsorted",
            |p, _| p.A_row_indices = vec![2, 0, 1],
            "A.row_indices",
            "unsorted",
        ),
        (
            "row_indices_duplicate",
            |p, _| p.A_row_indices = vec![1, 1, 1],
            "A.row_indices",
            "duplicate",
        ),
        (
            "row_index_negative_i32",
            |_, r| {
                r.index_i32 = true;
                r.extra.push(("A_row_indices".into(), vec![0.0, -1.0, 1.0]));
            },
            "A.row_indices",
            "negative",
        ),
        (
            "values_nan",
            |p, _| p.A_values[1] = f64::NAN,
            "A.values",
            "non-finite",
        ),
        (
            "values_inf",
            |p, _| p.A_values[2] = f64::INFINITY,
            "A.values",
            "non-finite",
        ),
        ("c_nan", |p, _| p.c[0] = f64::NAN, "c", "non-finite"),
        ("c_inf", |p, _| p.c[1] = f64::INFINITY, "c", "non-finite"),
        (
            "b_nan",
            |p, _| p.rows = cdata::Rows::SenseRhs(b"=<>".to_vec(), vec![f64::NAN, 1.0, 1.0]),
            "b",
            "NaN",
        ),
        (
            "b_l_nan",
            |p, _| p.rows = cdata::Rows::Ranges(vec![f64::NAN, 0.0, 0.0], vec![1.0; 3]),
            "b_l",
            "NaN",
        ),
        (
            "b_u_nan",
            |p, _| p.rows = cdata::Rows::Ranges(vec![0.0; 3], vec![1.0, f64::NAN, 1.0]),
            "b_u",
            "NaN",
        ),
        ("l_nan", |p, _| p.l[0] = f64::NAN, "l", "NaN"),
        ("u_nan", |p, _| p.u[1] = f64::NAN, "u", "NaN"),
        (
            "eq_plus_inf",
            |p, _| p.rows = cdata::Rows::SenseRhs(b"=<>".to_vec(), vec![f64::INFINITY, 1.0, 1.0]),
            "b",
            "'='",
        ),
        (
            "eq_minus_inf",
            |p, _| {
                p.rows = cdata::Rows::SenseRhs(b"=<>".to_vec(), vec![f64::NEG_INFINITY, 1.0, 1.0])
            },
            "b",
            "'='",
        ),
        (
            "le_minus_inf",
            |p, _| {
                p.rows = cdata::Rows::SenseRhs(b"=<>".to_vec(), vec![1.0, f64::NEG_INFINITY, 1.0])
            },
            "b",
            "'<'",
        ),
        (
            "ge_plus_inf",
            |p, _| p.rows = cdata::Rows::SenseRhs(b"=<>".to_vec(), vec![1.0, 1.0, f64::INFINITY]),
            "b",
            "'>'",
        ),
        (
            "b_l_plus_inf",
            |p, _| {
                p.rows = cdata::Rows::Ranges(vec![f64::INFINITY, 0.0, 0.0], vec![f64::INFINITY; 3])
            },
            "b_l",
            "+inf",
        ),
        (
            "b_u_minus_inf",
            |p, _| {
                p.rows = cdata::Rows::Ranges(
                    vec![f64::NEG_INFINITY; 3],
                    vec![f64::NEG_INFINITY, 1.0, 1.0],
                )
            },
            "b_u",
            "-inf",
        ),
        ("l_plus_inf", |p, _| p.l[0] = f64::INFINITY, "l", "+inf"),
        (
            "u_minus_inf",
            |p, _| p.u[1] = f64::NEG_INFINITY,
            "u",
            "-inf",
        ),
        (
            "invalid_sense_byte",
            |p, _| p.rows = cdata::Rows::SenseRhs(b"=?>".to_vec(), vec![1.0; 3]),
            "sense",
            "invalid sense byte",
        ),
        (
            "gamma_non_finite",
            |p, _| p.gamma = f64::INFINITY,
            "gamma",
            "not finite",
        ),
        // The C layer's own checks.
        (
            "both_row_forms",
            |_, r| r.extra.push(("b_l".into(), vec![0.0; 3])),
            "row bounds",
            "both forms",
        ),
        (
            "neither_row_form",
            |_, r| r.omit.extend(["row_sense", "b"]),
            "row bounds",
            "neither form",
        ),
        ("null_c", |_, r| r.omit.push("c"), "c", "NULL"),
        (
            "neither_index_width",
            |_, r| r.omit.push("A_row_indices"),
            "A.row_indices",
            "neither",
        ),
        (
            "bad_sense_code",
            |_, r| r.extra.push(("sense".into(), vec![3.0])),
            "sense",
            "is not 1",
        ),
        (
            "bad_method",
            |_, r| r.options.push(("opt_method", 9.0)),
            "options.method",
            "unknown code",
        ),
        (
            "bad_presolve",
            |_, r| r.options.push(("opt_presolve", 9.0)),
            "options.presolve",
            "unknown code",
        ),
        (
            "threads_too_large",
            |_, r| r.options.push(("opt_threads", 1e12)),
            "options.threads",
            "is not",
        ),
        (
            "seed_outside_int32",
            |_, r| r.options.push(("opt_random_seed", 1e12)),
            "options.random_seed",
            "outside",
        ),
        (
            "bad_param_type",
            |_, r| r.raw.push(("Method".into(), 7, 0.0, None)),
            "options.raw.type",
            "unknown code",
        ),
    ];
    let mut failures = Vec::new();
    for (name, change, what, detail) in cases {
        let mut p = small();
        let mut r = Run {
            backend: OPTSOLVER_BACKEND_GUROBI,
            once: true,
            ..Run::default()
        };
        change(&mut p, &mut r);
        let c = run_driver(Link::Static, &p, &r, &format!("invalid-{name}"))?;
        let msg = c.s("message");
        let ok = c.int("rc") == OPTSOLVER_ERR_INVALID
            && msg.starts_with(&format!("invalid {what}"))
            && msg.contains(detail)
            && c.num("environments_created") == Some(0.0);
        if !ok {
            failures.push(format!(
                "{name}: rc {} {msg:?} envs {:?}",
                c.int("rc"),
                c.num("environments_created")
            ));
        }
    }
    ensure(failures.is_empty(), format!("{failures:#?}"))
}

// ---------------------------------------------------------------------------------------------
// Licensed trials.

fn toy_problem(toy: &Toy, ranges: bool) -> CProblem {
    match &toy.data {
        ToyData::Mps(p) => CProblem::from_mps(p, None, ranges),
        ToyData::Jtask(p) => CProblem::from_jtask(p),
    }
}

fn toys() -> Result<Vec<Toy>, Failed> {
    common::fixtures::toy_instances()
        .map_err(fail)?
        .iter()
        .map(|n| Toy::load(n))
        .collect()
}

fn t007_capabilities(b: &dyn Backend) -> Result<(), Failed> {
    let r = Run {
        caps: true,
        ..run(b)
    };
    let c = run_driver(Link::Static, &small(), &r, &format!("caps-{}", r.backend))?;
    let caps = b.capabilities();
    ensure(
        c.int("rc") == 0 && c.int("caps_usable") == 1 && c.s("caps_reason").is_empty(),
        "usable",
    )?;
    let classes: u32 = caps.problem_classes.iter().map(|&k| class_bit(k)).sum();
    let cones: u32 = caps.cones.iter().map(|&k| 1u32 << cone_code(k)).sum();
    ensure(c.num("caps_classes") == Some(f64::from(classes)), "classes")?;
    ensure(c.num("caps_cones") == Some(f64::from(cones)), "cones")?;
    ensure(
        c.int("caps_certificates") == i32::from(caps.certificates),
        "certificates",
    )?;
    ensure(c.int("caps_basis") == i32::from(caps.basis), "basis")?;
    let names: Vec<String> = (0..caps.options.len())
        .map(|i| c.s(&format!("caps_option_{i}")).to_string())
        .collect();
    let want: Vec<String> = caps.options.iter().map(|o| o.field.to_string()).collect();
    ensure(names == want, format!("options {names:?} vs {want:?}"))?;
    ensure(
        c.s("caps_solver_version") == caps.solver.version
            && c.s("caps_solver_name") == caps.solver.name,
        "identity",
    )?;
    ensure(
        c.int("env_create_rc") == OPTSOLVER_OK,
        "environment creation",
    )
}

fn t007_not_usable(b: &dyn Backend) -> Result<(), Failed> {
    let r = Run {
        caps: true,
        ..run(b)
    };
    let exe = cdata::driver(Link::Static)?;
    let input = runs_dir()?.join(format!("notusable-{}.in", r.backend));
    let output = runs_dir()?.join(format!("notusable-{}.out", r.backend));
    std::fs::write(&input, small().file(&r)).map_err(fail)?;
    let missing = "/nonexistent/optsolver-test/licence.lic";
    let ran = cdata::run_program(
        &exe,
        &[&input, &output],
        &[
            ("GRB_LICENSE_FILE", missing),
            ("MOSEKLM_LICENSE_FILE", missing),
        ],
        false,
    )?;
    ensure(ran.status.success(), "driver failed")?;
    let c = CResult::parse(&std::fs::read_to_string(&output).map_err(fail)?).map_err(fail)?;
    if c.int("caps_usable") == 1 {
        return Err(fail(
            "the vendor library did not honour the licence variable: not-usable path unverified",
        ));
    }
    ensure(
        c.int("rc") == 0 && !c.s("caps_reason").is_empty(),
        "not usable with a reason",
    )?;
    ensure(
        c.int("env_create_rc") == OPTSOLVER_ERR_SOLVER,
        format!("env_create rc {}", c.int("env_create_rc")),
    )
}

fn t008_accessors(b: &dyn Backend) -> Result<(), Failed> {
    let toy = Toy::load("toy_lp_max")?;
    let p = toy_problem(&toy, false);
    let mut r = run(b);
    r.options = cdata::deterministic(&p);
    for link in [Link::Static, Link::Shared] {
        compare_paths(b, link, &p, &r, &format!("t008-{}", r.backend))?;
    }
    let c = run_driver(Link::Static, &p, &r, &format!("t008-fields-{}", r.backend))?;
    let (sol, _) = p.solve_rust(b, &r).map_err(fail)?;
    let t = c.nums.get("timings").ok_or_else(|| fail("timings"))?;
    ensure(
        t.len() == 4 && t.iter().all(|v| v.is_finite() && *v >= 0.0),
        "timings",
    )?;
    let params: Vec<String> = sol
        .effective_parameters
        .iter()
        .map(|(n, v)| match v {
            optsolver::ParamValue::Int(i) => format!("{n}=i{i}"),
            optsolver::ParamValue::Double(d) => format!("{n}=d{}", format_g17(*d)),
            optsolver::ParamValue::Str(s) => format!("{n}=s{s}"),
        })
        .collect();
    let cparams: Vec<String> = (0..params.len())
        .map(|i| c.s(&format!("parameter_{i}")).to_string())
        .collect();
    ensure(
        params == cparams,
        format!("parameters {cparams:?} vs {params:?}"),
    )?;
    ensure(
        c.s("solver_name") == sol.solver.name && c.s("solver_version") == sol.solver.version,
        "identity",
    )?;
    for (i, (n, code)) in sol.raw_status.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let want = *code as f64;
        ensure(
            c.s(&format!("raw_status_{i}")) == n && c.num(&format!("raw_code_{i}")) == Some(want),
            "raw status",
        )?;
    }
    ensure(
        c.s("status_name") == format!("{:?}", sol.status),
        "status name",
    )?;
    // Empty problems (zero rows; zero columns) behave as in Rust.
    let mut zero_rows = p.clone();
    zero_rows.m = 0;
    zero_rows.A_col_ptrs = vec![0; p.n + 1];
    zero_rows.A_row_indices.clear();
    zero_rows.A_values.clear();
    zero_rows.rows = cdata::Rows::Ranges(Vec::new(), Vec::new());
    compare_paths(
        b,
        Link::Static,
        &zero_rows,
        &r,
        &format!("t008-zero-rows-{}", r.backend),
    )?;
    let mut zero_cols = small();
    zero_cols.n = 0;
    zero_cols.A_col_ptrs = vec![0];
    zero_cols.A_row_indices.clear();
    zero_cols.A_values.clear();
    zero_cols.c.clear();
    zero_cols.l.clear();
    zero_cols.u.clear();
    zero_cols.rows = cdata::Rows::Ranges(vec![-1.0; 3], vec![1.0; 3]);
    compare_paths(
        b,
        Link::Static,
        &zero_cols,
        &r,
        &format!("t008-zero-cols-{}", r.backend),
    )?;
    // Wrong-length output buffer: INVALID naming it, before any solver call; every supplied
    // array holds NaN afterwards.
    let mut rb = Run {
        buffers: true,
        ..r.clone()
    };
    rb.extra.push(("buf_y".into(), vec![99.0]));
    let c = run_driver(
        Link::Static,
        &p,
        &rb,
        &format!("t008-wrong-buffer-{}", r.backend),
    )?;
    ensure(
        c.int("rc") == OPTSOLVER_ERR_INVALID && c.s("message").contains("out.y"),
        format!("wrong buffer: {} {:?}", c.int("rc"), c.s("message")),
    )?;
    for k in [
        "out_buf_x",
        "out_buf_y",
        "out_buf_z",
        "out_buf_row_activity",
    ] {
        let v = c.nums.get(k).ok_or_else(|| fail(k))?;
        ensure(
            v.iter().all(|x| x.is_nan()),
            format!("{k} not NaN after the error"),
        )?;
    }
    // A raw parameter the solver rejects: solver error with the vendor code (Gurobi).
    if r.backend == OPTSOLVER_BACKEND_GUROBI {
        let mut rr = r.clone();
        rr.raw
            .push(("TimeLimit".into(), OPTSOLVER_PARAM_DOUBLE, -1.0, None));
        let c = run_driver(Link::Static, &p, &rr, "t008-solver-error")?;
        let vendor = match p.solve_rust(b, &rr) {
            Err(optsolver::Error::Solver { code, .. }) => code,
            other => {
                return Err(fail(format!(
                    "expected a solver error, got {:?}",
                    other.map(|_| ())
                )));
            }
        };
        ensure(
            c.int("rc") == OPTSOLVER_ERR_SOLVER && c.int("vendor_code") == vendor,
            format!(
                "solver error: rc {} vendor {} (Rust {vendor})",
                c.int("rc"),
                c.int("vendor_code")
            ),
        )?;
    }
    // Every option field through C (except the memory limit: T013, T014), compared with Rust.
    let mut all = r.clone();
    let caps = b.capabilities();
    let honoured = |f: &str| caps.options.iter().any(|o| o.field == f);
    for (key, field, v) in [
        ("opt_time_limit", "time_limit", 100.0),
        ("opt_iteration_limit", "iteration_limit", 1e6),
        (
            "opt_primal_feasibility_tolerance",
            "primal_feasibility_tolerance",
            1e-7,
        ),
        (
            "opt_dual_feasibility_tolerance",
            "dual_feasibility_tolerance",
            1e-7,
        ),
        (
            "opt_presolve",
            "presolve",
            f64::from(OPTSOLVER_PRESOLVE_OFF),
        ),
        ("opt_random_seed", "random_seed", 3.0),
        ("opt_solver_log", "solver_log", 0.0),
        ("opt_certificates", "certificates", 1.0),
    ] {
        if honoured(field) {
            all.options.push((key, v));
        }
    }
    all.options.push(("opt_array_outputs", 1.0));
    compare_paths(
        b,
        Link::Static,
        &p,
        &all,
        &format!("t008-all-options-{}", r.backend),
    )?;
    let c = run_driver(
        Link::Static,
        &p,
        &all,
        &format!("t008-all-options-params-{}", r.backend),
    )?;
    let (sol, _) = p.solve_rust(b, &all).map_err(fail)?;
    ensure(
        c.num("parameter_count") == Some(sol.effective_parameters.len() as f64),
        "every option reached the solver as in Rust",
    )?;
    // Verification switched off.
    let mut off = r.clone();
    off.options.push(("opt_verification", 0.0));
    let c = run_driver(
        Link::Static,
        &p,
        &off,
        &format!("t008-verification-off-{}", r.backend),
    )?;
    ensure(
        c.int("verification") == OPTSOLVER_VERIFICATION_NOT_RUN
            && c.s("verification_name") == "NotRun",
        "verification off",
    )
}

/// `%.17g` as the C driver writes it.
fn format_g17(d: f64) -> String {
    let s = format!("{d:.16e}");
    let (mant, exp) = s.split_once('e').unwrap_or((&s, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    if (-5..17).contains(&exp) {
        let digits = usize::try_from((16 - exp).max(0)).unwrap_or(0);
        let f = format!("{d:.digits$}");
        if f.contains('.') {
            f.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            f
        }
    } else {
        let m = mant.trim_end_matches('0').trim_end_matches('.');
        format!("{m}e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs())
    }
}

fn supported(toy: &Toy, b: &dyn Backend) -> bool {
    toy.unsupported_field(b).is_none()
}

fn t010_lp_qp_toys(b: &dyn Backend) -> Result<(), Failed> {
    let mut failures = Vec::new();
    let (mut exact, mut total) = (0, 0);
    for toy in toys()? {
        let ToyData::Mps(mps) = &toy.data else {
            continue;
        };
        if !supported(&toy, b) {
            continue;
        }
        let forms: &[bool] = if mps.has_ranges() {
            &[true]
        } else {
            &[false, true]
        };
        for &ranges in forms {
            let p = toy_problem(&toy, ranges);
            for buffers in [false, true] {
                for index_i32 in [false, true] {
                    for link in [Link::Static, Link::Shared] {
                        let mut r = run(b);
                        r.options = cdata::deterministic(&p);
                        r.buffers = buffers;
                        r.index_i32 = index_i32;
                        let label = format!(
                            "t010-{}-{}-r{ranges}-b{buffers}-i{index_i32}",
                            toy.name, r.backend
                        );
                        total += 1;
                        match compare_paths(b, link, &p, &r, &label) {
                            Ok(true) => exact += 1,
                            Ok(false) => {}
                            Err(e) => failures.push(format!("{label}-{link:?}: {e:?}")),
                        }
                    }
                }
            }
        }
    }
    ensure(total > 0, "no LP or QP toy instance ran")?;
    eprintln!(
        "t010 ({}): {total} C-versus-Rust runs, {exact} bit-identical",
        b.capabilities().backend
    );
    ensure(failures.is_empty(), format!("{failures:#?}"))
}

fn t010_certificates(b: &dyn Backend) -> Result<(), Failed> {
    let mut seen = 0;
    for name in ["infeasible", "unbounded"] {
        let toy = Toy::load(name)?;
        if !supported(&toy, b) {
            continue;
        }
        let p = toy_problem(&toy, false);
        for buffers in [false, true] {
            let mut r = run(b);
            r.options = cdata::deterministic(&p);
            r.options.push(("opt_certificates", 1.0));
            r.buffers = buffers;
            compare_paths(
                b,
                Link::Static,
                &p,
                &r,
                &format!("t010-cert-{name}-{}-{buffers}", r.backend),
            )?;
            let c = run_driver(
                Link::Static,
                &p,
                &r,
                &format!("t010-cert-kind-{name}-{}-{buffers}", r.backend),
            )?;
            let want = match toy.expected()?.status {
                Status::Infeasible => OPTSOLVER_CERTIFICATE_FARKAS,
                _ => OPTSOLVER_CERTIFICATE_UNBOUNDED_RAY,
            };
            let kind = c.int("certificate");
            ensure(
                kind == want,
                format!("{name}: certificate kind {kind}, expected {want}"),
            )?;
            if buffers && c.int("returned") == OPTSOLVER_RETURNED_NONE {
                let x = c.nums.get("out_buf_x").ok_or_else(|| fail("out_buf_x"))?;
                ensure(
                    x.iter().all(|v| v.is_nan()),
                    format!("{name}: x not NaN without a solution"),
                )?;
            }
            seen += 1;
        }
    }
    ensure(seen > 0, "no certificate instance ran")
}

fn t012_reuse(b: &dyn Backend) -> Result<(), Failed> {
    let exe = cdata::driver(Link::Static)?;
    let dir = runs_dir()?;
    let supported_toys: Vec<Toy> = toys()?.into_iter().filter(|t| supported(t, b)).collect();
    ensure(!supported_toys.is_empty(), "no supported toy")?;
    let (mut files, mut problems) = (Vec::new(), Vec::new());
    for i in 0..20 {
        let toy = &supported_toys[i % supported_toys.len()];
        let p = toy_problem(toy, false);
        let mut r = run(b);
        r.options = cdata::deterministic(&p);
        let input = dir.join(format!("reuse-{}-{i}.in", r.backend));
        let output = dir.join(format!("reuse-{}-{i}.out", r.backend));
        std::fs::write(&input, p.file(&r)).map_err(fail)?;
        files.push((input, output));
        problems.push((toy.name.clone(), p, r));
    }
    let args: Vec<&Path> = files
        .iter()
        .flat_map(|(i, o)| [i.as_path(), o.as_path()])
        .collect();
    let t0 = std::time::Instant::now();
    let ran = cdata::run_program(&exe, &args, &[], false)?;
    let seq = t0.elapsed().as_secs_f64();
    ensure(ran.status.success(), "sequence run failed")?;
    let mut created = None;
    for ((name, p, r), (_, output)) in problems.iter().zip(&files) {
        let c = CResult::parse(&std::fs::read_to_string(output).map_err(fail)?).map_err(fail)?;
        created = c.num("environments_created");
        let (sol, _) = p.solve_rust(b, r).map_err(fail)?;
        common::compare::agree_c_rust(&c.observe(false), &common::compare::observe_rust(&sol))
            .map_err(|e| fail(format!("{name}: {e}")))?;
    }
    ensure(
        created == Some(1.0),
        format!("environments created for 20 solves: {created:?}"),
    )?;
    let t0 = std::time::Instant::now();
    for (name, p, r) in &problems {
        let once = Run {
            once: true,
            ..r.clone()
        };
        let c = run_driver(
            Link::Static,
            p,
            &once,
            &format!("reuse-once-{name}-{}", r.backend),
        )?;
        ensure(
            c.num("environments_created") == Some(1.0),
            "a one-shot solve creates one environment",
        )?;
    }
    let one_shot = t0.elapsed().as_secs_f64();
    eprintln!(
        "t012 ({}): 20 problems ({} distinct toy instances in order) through one handle: {seq:.3} s in one process; 20 one-shot solves: {one_shot:.3} s in 20 processes (N = 1, informational)",
        b.capabilities().backend,
        supported_toys.len().min(20)
    );
    Ok(())
}

fn t012_thread(b: &dyn Backend) -> Result<(), Failed> {
    let toy = Toy::load("toy_lp_max")?;
    let p = toy_problem(&toy, false);
    let mut r = run(b);
    r.options = cdata::deterministic(&p);
    r.thread = true;
    let c = run_driver(Link::Static, &p, &r, &format!("thread-{}", r.backend))?;
    ensure(c.int("rc") == 0, format!("{:?}", c.s("message")))?;
    ensure(
        c.int("verification") == OPTSOLVER_VERIFICATION_VERIFIED,
        "verification verdict",
    )?;
    let plain = Run {
        thread: false,
        ..r.clone()
    };
    let (sol, _) = p.solve_rust(b, &plain).map_err(fail)?;
    common::compare::agree_c_rust(&c.observe(false), &common::compare::observe_rust(&sol))
        .map(|_| ())
        .map_err(fail)
}

fn t014_memory_limit(b: &dyn Backend) -> Result<(), Failed> {
    let (f, seed) = common::us3::corpus_with_seed(common::us3::LARGEST_INSTANCE, 0)?;
    let c = common::objectives::seeded_objective(seed, f.problem.n);
    let p = CProblem::from_mps(&f.problem, Some(&c), false);
    let mut r = run(b);
    r.options = vec![("opt_threads", 1.0), ("opt_memory_limit_gb", 1e-6)];
    if r.backend == OPTSOLVER_BACKEND_MOSEK {
        let c = run_driver(Link::Static, &p, &r, "memlimit-mosek")?;
        return ensure(
            c.int("rc") == OPTSOLVER_ERR_UNSUPPORTED && c.s("message").contains("memory_limit_gb"),
            format!("MOSEK: rc {} {:?}", c.int("rc"), c.s("message")),
        );
    }
    for rep in 0..3 {
        let c = run_driver(Link::Static, &p, &r, &format!("memlimit-{rep}"))?;
        ensure(
            c.int("rc") == 0,
            format!("rc {} {:?}", c.int("rc"), c.s("message")),
        )?;
        ensure(
            c.int("status") == OPTSOLVER_STATUS_STOPPED_AT_LIMIT
                && c.int("limit_reason") == OPTSOLVER_LIMIT_MEMORY,
            format!(
                "repetition {rep}: status {} limit {}",
                c.s("status_name"),
                c.s("limit_reason_name")
            ),
        )?;
        let soft = cdata::parameter(&c, "SoftMemLimit");
        ensure(
            soft.as_deref().is_some_and(|v| v.starts_with("d")),
            format!("SoftMemLimit {soft:?}"),
        )?;
    }
    r.options = vec![("opt_threads", 1.0)];
    let c = run_driver(Link::Static, &p, &r, "memlimit-none")?;
    ensure(
        c.int("status") == OPTSOLVER_STATUS_OPTIMAL
            && c.int("verification") == OPTSOLVER_VERIFICATION_VERIFIED,
        format!(
            "without the limit: {} {}",
            c.s("status_name"),
            c.s("verification_name")
        ),
    )
}

fn t015_conic_toys(b: &dyn Backend) -> Result<(), Failed> {
    let mut failures = Vec::new();
    let mut ran = 0;
    for toy in toys()? {
        let ToyData::Jtask(_) = &toy.data else {
            continue;
        };
        let p = toy_problem(&toy, false);
        if let Some(field) = toy.unsupported_field(b) {
            let r = Run {
                options: cdata::deterministic(&p),
                ..run(b)
            };
            let c = run_driver(
                Link::Static,
                &p,
                &r,
                &format!("t015-unsupported-{}-{}", toy.name, r.backend),
            )?;
            if !(c.int("rc") == OPTSOLVER_ERR_UNSUPPORTED && c.s("message").contains(field)) {
                failures.push(format!(
                    "{}: expected UNSUPPORTED {field}, got {} {:?}",
                    toy.name,
                    c.int("rc"),
                    c.s("message")
                ));
            }
            continue;
        }
        for buffers in [false, true] {
            let mut r = run(b);
            r.options = cdata::deterministic(&p);
            r.buffers = buffers;
            let label = format!("t015-{}-{}-b{buffers}", toy.name, r.backend);
            ran += 1;
            if let Err(e) = compare_paths(b, Link::Static, &p, &r, &label) {
                failures.push(format!("{label}: {e:?}"));
            }
        }
    }
    ensure(ran > 0, "no conic toy ran")?;
    ensure(failures.is_empty(), format!("{failures:#?}"))
}

fn t016_leaks_licensed(b: &dyn Backend) -> Result<(), Failed> {
    if !asan_available() {
        return Err(fail(
            "AddressSanitizer unavailable: licensed leak check unverified",
        ));
    }
    let exe = cdata::program(
        "driver",
        Link::Static,
        &["-fsanitize=address", "-fno-omit-frame-pointer"],
    )?;
    let supp = common::fixtures::repo_root().join("tests/c/lsan-vendor.supp");
    let lsan = format!("suppressions={}", supp.display());
    let toy = Toy::load("toy_lp_max")?;
    let p = toy_problem(&toy, false);
    let dir = runs_dir()?;
    let go = |leak: bool, caps: bool| -> Result<cdata::Ran, Failed> {
        let mut r = run(b);
        r.options = cdata::deterministic(&p);
        r.leak_result = leak;
        r.caps = caps;
        let input = dir.join(format!("leak-{}-{leak}-{caps}.in", r.backend));
        let output = dir.join(format!("leak-{}-{leak}-{caps}.out", r.backend));
        std::fs::write(&input, p.file(&r)).map_err(fail)?;
        let env = [
            ("ASAN_OPTIONS", "detect_leaks=1"),
            ("LSAN_OPTIONS", lsan.as_str()),
        ];
        let ran = cdata::run_program(&exe, &[&input, &output], &env, true)?;
        let c = CResult::parse(&std::fs::read_to_string(&output).map_err(fail)?).map_err(fail)?;
        ensure(
            c.int("rc") == 0,
            format!("leak-checked run rc {} {:?}", c.int("rc"), c.s("message")),
        )?;
        if !caps {
            ensure(
                c.int("verification") == OPTSOLVER_VERIFICATION_VERIFIED,
                "the leak-checked solve's verdict",
            )?;
        }
        Ok(ran)
    };
    for caps in [false, true] {
        let clean = go(false, caps)?;
        ensure(
            clean.status.success() && !clean.stderr.contains("LeakSanitizer"),
            format!("leaks reported (caps {caps}):\n{}", clean.stderr),
        )?;
    }
    let leaky = go(true, false)?;
    ensure(
        leaky.stderr.contains("LeakSanitizer"),
        "positive control: the unfreed result was not reported",
    )
}
