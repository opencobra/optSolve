//! Gurobi parameters (task T041; research R5, R6, R17; constitution III, VII-G).
//!
//! This is the only code in the crate that calls `GRBsetintparam`, `GRBsetdblparam` or
//! `GRBsetstrparam` (enforced by `tests/standards.rs`). Every default optSolver imposes comes
//! from [`crate::options::defaults`]; this module only maps options to Gurobi parameters.

use core::ffi::{CStr, c_double, c_int};
use std::ffi::CString;

use super::{env, ffi};
use crate::backend::{OptionMapping, ProblemClass};
use crate::error::{Error, Result};
use crate::options::{Method, Options, ParamValue, Presolve, defaults};
use crate::verify::Tolerances;

/// Gurobi mapping of every shared options field (research R5). `cancel` and `progress` set no
/// parameter: they are served by the callback (`gurobi::callback`, research R7).
pub(crate) const MAPPINGS: &[OptionMapping] = &[
    OptionMapping {
        field: "time_limit",
        vendor_parameters: &["TimeLimit"],
    },
    OptionMapping {
        field: "iteration_limit",
        vendor_parameters: &["IterationLimit"],
    },
    OptionMapping {
        field: "threads",
        vendor_parameters: &["Threads"],
    },
    OptionMapping {
        field: "primal_feasibility_tolerance",
        vendor_parameters: &["FeasibilityTol"],
    },
    OptionMapping {
        field: "dual_feasibility_tolerance",
        vendor_parameters: &["OptimalityTol"],
    },
    OptionMapping {
        field: "method",
        vendor_parameters: &["Method"],
    },
    OptionMapping {
        field: "presolve",
        vendor_parameters: &["Presolve"],
    },
    OptionMapping {
        field: "random_seed",
        vendor_parameters: &["Seed"],
    },
    OptionMapping {
        field: "solver_log",
        vendor_parameters: &["OutputFlag"],
    },
    OptionMapping {
        field: "certificates",
        vendor_parameters: &["InfUnbdInfo", "DualReductions"],
    },
    OptionMapping {
        field: "cancel",
        vendor_parameters: &[],
    },
    OptionMapping {
        field: "progress",
        vendor_parameters: &[],
    },
];

/// Adds the parameter name to a Gurobi error read from `env` right after the failing call.
fn named_error(env: *mut ffi::GRBenv, code: c_int, name: &CStr) -> Error {
    match env::error(env, code) {
        Error::Solver { code, message } => Error::Solver {
            code,
            message: format!("parameter {}: {message}", name.to_string_lossy()),
        },
        other => other,
    }
}

fn set_int(env: *mut ffi::GRBenv, name: &CStr, value: c_int) -> Result<()> {
    // SAFETY: `env` is a live Gurobi environment (caller's contract); `name` is a
    // NUL-terminated string that outlives the call.
    let code = unsafe { ffi::GRBsetintparam(env, name.as_ptr(), value) };
    if code != 0 {
        return Err(named_error(env, code, name));
    }
    Ok(())
}

fn set_dbl(env: *mut ffi::GRBenv, name: &CStr, value: c_double) -> Result<()> {
    // SAFETY: as in `set_int`.
    let code = unsafe { ffi::GRBsetdblparam(env, name.as_ptr(), value) };
    if code != 0 {
        return Err(named_error(env, code, name));
    }
    Ok(())
}

fn set_str(env: *mut ffi::GRBenv, name: &CStr, value: &CStr) -> Result<()> {
    // SAFETY: as in `set_int`; `value` is also NUL-terminated and outlives the call (Gurobi
    // copies it).
    let code = unsafe { ffi::GRBsetstrparam(env, name.as_ptr(), value.as_ptr()) };
    if code != 0 {
        return Err(named_error(env, code, name));
    }
    Ok(())
}

fn get_dbl(env: *mut ffi::GRBenv, name: &CStr) -> Result<f64> {
    let mut value: c_double = 0.0;
    // SAFETY: `env` is live (caller's contract), `name` is NUL-terminated, and `value` is a
    // writable local that Gurobi writes one double into.
    let code = unsafe { ffi::GRBgetdblparam(env, name.as_ptr(), &mut value) };
    if code != 0 {
        return Err(named_error(env, code, name));
    }
    Ok(value)
}

/// Records parameters in the order they are set, under Gurobi's names.
struct Recorder {
    env: *mut ffi::GRBenv,
    set: Vec<(String, ParamValue)>,
}

impl Recorder {
    fn int(&mut self, name: &CStr, value: c_int) -> Result<()> {
        set_int(self.env, name, value)?;
        self.set
            .push((name_string(name), ParamValue::Int(i64::from(value))));
        Ok(())
    }
    fn dbl(&mut self, name: &CStr, value: f64) -> Result<()> {
        set_dbl(self.env, name, value)?;
        self.set
            .push((name_string(name), ParamValue::Double(value)));
        Ok(())
    }
}

fn name_string(name: &CStr) -> String {
    name.to_string_lossy().into_owned()
}

/// Sets the parameters a new, unstarted environment needs before `GRBstartenv`: `OutputFlag`
/// = [`defaults::SOLVER_LOG`] (0), so that starting the environment prints no licence banner
/// (constitution III; research R17.8). `env` must be a live environment from `env::empty`.
pub(crate) fn configure_new_environment(env: *mut ffi::GRBenv) -> Result<()> {
    set_int(env, c"OutputFlag", c_int::from(defaults::SOLVER_LOG))
}

fn method_code(m: Method) -> c_int {
    match m {
        Method::Auto => -1,
        Method::PrimalSimplex => 0,
        Method::DualSimplex => 1,
        Method::Barrier => 2,
        Method::Concurrent => 3,
    }
}

fn presolve_code(p: Presolve) -> c_int {
    match p {
        Presolve::Auto => -1,
        Presolve::Off => 0,
        Presolve::Conservative => 1,
        Presolve::Aggressive => 2,
    }
}

fn invalid_raw(detail: String) -> Error {
    Error::Invalid {
        what: "options.raw",
        detail,
    }
}

/// Applies `options` to `model_env` (the model's own environment, `GRBgetenv`) and returns
/// every parameter set, with the value set, under Gurobi's parameter names.
///
/// Order: `OutputFlag` from [`Options::effective_solver_log`]; then each set shared field
/// (research R5); then `certificates` as `InfUnbdInfo` = 1 and `DualReductions` = 0 (research
/// R6, R17.3); then each raw parameter, type-checked against `GRBgetparamtype`. Conflicts
/// between raw and shared routes are rejected earlier by `options::check`.
///
/// Errors: [`Error::Invalid`] for an unknown raw name, a name with an interior NUL, a value of
/// the wrong type or out of range; [`Error::Solver`] (message naming the parameter) when
/// Gurobi rejects a value.
pub(crate) fn apply(
    model_env: *mut ffi::GRBenv,
    options: &Options<'_>,
    class: ProblemClass,
) -> Result<Vec<(String, ParamValue)>> {
    let mut r = Recorder {
        env: model_env,
        set: Vec::new(),
    };
    r.int(c"OutputFlag", c_int::from(options.effective_solver_log()))?;
    // Feature 003 (research R1, R3): a non-convex Q is an error, not a silent switch to spatial
    // branching; duals of second-order cones need QCPDual. Before the raw route, which may
    // override them.
    match class {
        ProblemClass::Qp => r.int(c"NonConvex", 0)?,
        ProblemClass::Conic => r.int(c"QCPDual", 1)?,
        _ => {}
    }
    if let Some(v) = options.time_limit {
        r.dbl(c"TimeLimit", v)?;
    }
    if let Some(v) = options.iteration_limit {
        r.dbl(c"IterationLimit", v)?;
    }
    if let Some(v) = options.threads {
        let v = c_int::try_from(v).map_err(|_| Error::Invalid {
            what: "options.threads",
            detail: format!(
                "{v} exceeds the largest value Gurobi accepts ({})",
                c_int::MAX
            ),
        })?;
        r.int(c"Threads", v)?;
    }
    if let Some(v) = options.primal_feasibility_tolerance {
        r.dbl(c"FeasibilityTol", v)?;
    }
    if let Some(v) = options.dual_feasibility_tolerance {
        r.dbl(c"OptimalityTol", v)?;
    }
    if let Some(m) = options.method {
        r.int(c"Method", method_code(m))?;
    }
    if let Some(p) = options.presolve {
        r.int(c"Presolve", presolve_code(p))?;
    }
    if let Some(v) = options.random_seed {
        r.int(c"Seed", v)?;
    }
    if options.certificates {
        r.int(c"InfUnbdInfo", 1)?;
        r.int(c"DualReductions", 0)?;
    }
    for (name, value) in &options.raw {
        apply_raw(&mut r, name, value)?;
    }
    Ok(r.set)
}

fn apply_raw(r: &mut Recorder, name: &str, value: &ParamValue) -> Result<()> {
    let cname = CString::new(name)
        .map_err(|_| invalid_raw(format!("parameter name {name:?} contains a NUL character")))?;
    // SAFETY: `r.env` is live (caller's contract of `apply`); `cname` is NUL-terminated and
    // outlives the call. GRBgetparamtype only reads the name (research R17.9).
    let kind = unsafe { ffi::GRBgetparamtype(r.env, cname.as_ptr()) };
    let wrong_type = |expected: &str| {
        invalid_raw(format!(
            "Gurobi parameter {name} takes {expected}, got {value:?}"
        ))
    };
    let set_value = match (kind, value) {
        (ffi::PARAM_TYPE_INT, ParamValue::Int(i)) => {
            let v = c_int::try_from(*i).map_err(|_| {
                invalid_raw(format!(
                    "value {i} of Gurobi parameter {name} is outside the int range"
                ))
            })?;
            set_int(r.env, &cname, v)?;
            ParamValue::Int(*i)
        }
        (ffi::PARAM_TYPE_INT, _) => return Err(wrong_type("an integer")),
        (ffi::PARAM_TYPE_DBL, ParamValue::Double(d)) => {
            set_dbl(r.env, &cname, *d)?;
            ParamValue::Double(*d)
        }
        (ffi::PARAM_TYPE_DBL, ParamValue::Int(i)) => {
            // An integer is exactly a double iff its odd part fits the 53-bit significand.
            let t = i.unsigned_abs();
            if t != 0 && (t >> t.trailing_zeros()) >= (1_u64 << 53) {
                return Err(invalid_raw(format!(
                    "value {i} of Gurobi parameter {name} is not exactly representable as a double"
                )));
            }
            let d = *i as f64;
            set_dbl(r.env, &cname, d)?;
            ParamValue::Double(d)
        }
        (ffi::PARAM_TYPE_DBL, _) => return Err(wrong_type("a double")),
        (ffi::PARAM_TYPE_STR, ParamValue::Str(s)) => {
            let cs = CString::new(s.as_str()).map_err(|_| {
                invalid_raw(format!(
                    "value of Gurobi parameter {name} contains a NUL character"
                ))
            })?;
            set_str(r.env, &cname, &cs)?;
            ParamValue::Str(s.clone())
        }
        (ffi::PARAM_TYPE_STR, _) => return Err(wrong_type("a string")),
        (ffi::PARAM_TYPE_UNKNOWN, _) => {
            return Err(invalid_raw(format!("unknown Gurobi parameter {name}")));
        }
        (other, _) => {
            return Err(invalid_raw(format!(
                "Gurobi parameter {name} has unrecognised type code {other}"
            )));
        }
    };
    r.set.push((name.to_owned(), set_value));
    Ok(())
}

/// Effective verification tolerances read back from `model_env` after [`apply`] (research
/// R5, CHK011; feature 003 research R5): `FeasibilityTol` (primal), `OptimalityTol` (dual;
/// for QP the larger of it and `BarConvTol`, for conic problems of it and `BarQCPConvTol`,
/// since those are solved by barrier), `IntFeasTol` (integrality).
pub(crate) fn effective_tolerances(
    model_env: *mut ffi::GRBenv,
    class: ProblemClass,
) -> Result<Tolerances> {
    let optimality = get_dbl(model_env, c"OptimalityTol")?;
    let dual = match class {
        ProblemClass::Qp => optimality.max(get_dbl(model_env, c"BarConvTol")?),
        ProblemClass::Conic => optimality.max(get_dbl(model_env, c"BarQCPConvTol")?),
        _ => optimality,
    };
    Ok(Tolerances {
        primal: get_dbl(model_env, c"FeasibilityTol")?,
        dual,
        integrality: get_dbl(model_env, c"IntFeasTol")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gurobi::Env;
    use core::ffi::c_char;

    fn get_int(env: *mut ffi::GRBenv, name: &CStr) -> c_int {
        let mut v: c_int = 0;
        // SAFETY: live env, NUL-terminated name, writable local.
        let code = unsafe { ffi::GRBgetintparam(env, name.as_ptr(), &mut v) };
        assert_eq!(code, 0, "GRBgetintparam {name:?}");
        v
    }

    fn get_str(env: *mut ffi::GRBenv, name: &CStr) -> String {
        let mut buf = [0 as c_char; ffi::GRB_MAX_STRLEN];
        // SAFETY: live env, NUL-terminated name, buffer of GRB_MAX_STRLEN bytes as required.
        let code = unsafe { ffi::GRBgetstrparam(env, name.as_ptr(), buf.as_mut_ptr()) };
        assert_eq!(code, 0, "GRBgetstrparam {name:?}");
        // SAFETY: Gurobi wrote a NUL-terminated string into `buf`.
        unsafe { CStr::from_ptr(buf.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    }

    fn started_env() -> Env {
        Env::new().expect("Gurobi environment (licence required for these tests)")
    }

    #[test]
    fn mappings_cover_every_shared_field() {
        let flag = std::sync::atomic::AtomicBool::new(false);
        let mut f = |_: &crate::options::Progress| {};
        let o = Options {
            time_limit: Some(1.0),
            iteration_limit: Some(1.0),
            threads: Some(1),
            primal_feasibility_tolerance: Some(1e-6),
            dual_feasibility_tolerance: Some(1e-6),
            method: Some(Method::Auto),
            presolve: Some(Presolve::Auto),
            random_seed: Some(0),
            solver_log: Some(false),
            certificates: true,
            cancel: Some(&flag),
            progress: Some(&mut f),
            ..Options::default()
        };
        let fields = o.set_shared_fields();
        assert_eq!(fields.len(), MAPPINGS.len());
        for field in fields {
            assert!(
                MAPPINGS.iter().any(|m| m.field == field),
                "{field} unmapped"
            );
        }
    }

    #[test]
    fn configure_new_environment_sets_output_flag_zero_before_start() {
        let raw = env::empty().expect("GRBemptyenv needs no licence");
        configure_new_environment(raw.as_ptr()).expect("OutputFlag settable");
        assert_eq!(get_int(raw.as_ptr(), c"OutputFlag"), 0);
        // SAFETY: `raw` comes from `env::empty`, is owned here and freed exactly once.
        unsafe { ffi::GRBfreeenv(raw.as_ptr()) };
    }

    #[test]
    fn started_env_has_output_flag_zero() {
        let env = started_env();
        assert_eq!(get_int(env.raw(), c"OutputFlag"), 0);
    }

    #[test]
    fn apply_sets_shared_fields_certificates_and_raw_parameters() {
        let env = started_env();
        let e = env.raw();
        let o = Options {
            time_limit: Some(12.5),
            iteration_limit: Some(1000.0),
            threads: Some(2),
            primal_feasibility_tolerance: Some(1e-7),
            dual_feasibility_tolerance: Some(1e-8),
            method: Some(Method::DualSimplex),
            presolve: Some(Presolve::Off),
            random_seed: Some(7),
            certificates: true,
            raw: vec![
                ("NumericFocus".into(), ParamValue::Int(2)),
                ("MIPGap".into(), ParamValue::Int(0)),
                ("BarConvTol".into(), ParamValue::Double(1e-9)),
                ("LogFile".into(), ParamValue::Str(String::new())),
            ],
            ..Options::default()
        };
        let set = apply(e, &o, ProblemClass::Lp).expect("parameters accepted");

        assert_eq!(get_int(e, c"OutputFlag"), 0);
        assert_eq!(get_dbl(e, c"TimeLimit").unwrap(), 12.5);
        assert_eq!(get_dbl(e, c"IterationLimit").unwrap(), 1000.0);
        assert_eq!(get_int(e, c"Threads"), 2);
        assert_eq!(get_dbl(e, c"FeasibilityTol").unwrap(), 1e-7);
        assert_eq!(get_dbl(e, c"OptimalityTol").unwrap(), 1e-8);
        assert_eq!(get_int(e, c"Method"), 1);
        assert_eq!(get_int(e, c"Presolve"), 0);
        assert_eq!(get_int(e, c"Seed"), 7);
        assert_eq!(get_int(e, c"InfUnbdInfo"), 1);
        assert_eq!(get_int(e, c"DualReductions"), 0);
        assert_eq!(get_int(e, c"NumericFocus"), 2);
        assert_eq!(get_dbl(e, c"MIPGap").unwrap(), 0.0);
        assert_eq!(get_dbl(e, c"BarConvTol").unwrap(), 1e-9);
        assert_eq!(get_str(e, c"LogFile"), "");

        let names: Vec<&str> = set.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            [
                "OutputFlag",
                "TimeLimit",
                "IterationLimit",
                "Threads",
                "FeasibilityTol",
                "OptimalityTol",
                "Method",
                "Presolve",
                "Seed",
                "InfUnbdInfo",
                "DualReductions",
                "NumericFocus",
                "MIPGap",
                "BarConvTol",
                "LogFile",
            ]
        );
        assert!(set.contains(&("MIPGap".into(), ParamValue::Double(0.0))));
        assert!(set.contains(&("OutputFlag".into(), ParamValue::Int(0))));

        let tol = effective_tolerances(e, ProblemClass::Lp).unwrap();
        assert_eq!(tol.primal, 1e-7);
        assert_eq!(tol.dual, 1e-8);
        assert!(tol.integrality > 0.0);
    }

    #[test]
    fn default_options_set_only_output_flag() {
        let env = started_env();
        let set = apply(env.raw(), &Options::default(), ProblemClass::Lp).unwrap();
        assert_eq!(set, vec![("OutputFlag".to_owned(), ParamValue::Int(0))]);
    }

    #[test]
    fn solver_log_true_sets_output_flag_one() {
        let env = started_env();
        let o = Options {
            solver_log: Some(true),
            ..Options::default()
        };
        apply(env.raw(), &o, ProblemClass::Lp).unwrap();
        assert_eq!(get_int(env.raw(), c"OutputFlag"), 1);
        // Restore so that later calls on this environment print nothing.
        configure_new_environment(env.raw()).unwrap();
    }

    fn raw_err(raw: Vec<(String, ParamValue)>) -> Error {
        let env = started_env();
        let o = Options {
            raw,
            ..Options::default()
        };
        apply(env.raw(), &o, ProblemClass::Lp).expect_err("rejected")
    }

    #[test]
    fn unknown_raw_name_is_invalid() {
        match raw_err(vec![("NoSuchParam".into(), ParamValue::Int(1))]) {
            Error::Invalid { what, detail } => {
                assert_eq!(what, "options.raw");
                assert!(detail.contains("NoSuchParam"), "{detail}");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn type_mismatches_are_invalid() {
        for (name, value) in [
            ("Threads", ParamValue::Double(1.0)),
            ("Threads", ParamValue::Str("1".into())),
            ("Threads", ParamValue::Int(i64::MAX)),
            ("TimeLimit", ParamValue::Str("1".into())),
            ("TimeLimit", ParamValue::Int(i64::MAX)),
            ("LogFile", ParamValue::Int(1)),
        ] {
            match raw_err(vec![(name.into(), value.clone())]) {
                Error::Invalid { detail, .. } => assert!(detail.contains(name), "{detail}"),
                other => panic!("{name} {value:?}: unexpected {other:?}"),
            }
        }
    }

    #[test]
    fn interior_nul_is_invalid() {
        assert!(matches!(
            raw_err(vec![("Thr\0eads".into(), ParamValue::Int(1))]),
            Error::Invalid { .. }
        ));
    }

    #[test]
    fn out_of_range_value_is_solver_error_naming_parameter() {
        match raw_err(vec![("Method".into(), ParamValue::Int(99))]) {
            Error::Solver { code, message } => {
                assert_ne!(code, 0);
                assert!(message.contains("Method"), "{message}");
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}
