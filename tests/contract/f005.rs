//! Feature 005: Rust-API trials (specs/005-c-abi/tasks.md T009, T011, T013, T014), written once
//! against the contract and run on every backend through the shared registry.

// Built for every feature combination; without a backend the registry gives ignored trials and
// the backend-specific arms below are compiled out, leaving code clippy sees as unreachable.
#![allow(unreachable_code, unused_variables, clippy::never_loop)]

use crate::common::toy::{Toy, ToyData};
use crate::common::{Failed, Registry, Trial, ensure, fail};
use optsolver::{Backend, Error, LimitReason, Options, Overall, ParamValue, Status};

pub fn trials(registry: &Registry) -> Vec<Trial> {
    let mut t = Vec::new();
    t.extend(registry.trials(
        "f005::t009_wrong_length_conic_dual_buffer",
        t009_conic_dual_buffer,
    ));
    t.extend(registry.trials("f005::t011_environments_across_threads", t011_threads));
    t.extend(registry.trials("f005::t013_memory_limit_option", t013_memory_option));
    t.extend(registry.trials("f005::t014_memory_limit_stops_solve", t014_memory_limit));
    t
}

/// The same backend kind as `b`, newly created (environments are `Send`, so they may be created
/// here and moved to another thread).
fn same_kind(b: &dyn Backend) -> Result<Box<dyn Backend + Send>, Failed> {
    match b.capabilities().backend {
        #[cfg(feature = "gurobi")]
        "gurobi" => Ok(Box::new(optsolver::gurobi::Env::new().map_err(fail)?)),
        #[cfg(feature = "mosek")]
        "mosek" => Ok(Box::new(optsolver::mosek::Env::new().map_err(fail)?)),
        other => Err(fail(format!("unknown backend {other}"))),
    }
}

fn t009_conic_dual_buffer(b: &dyn Backend) -> Result<(), Failed> {
    let toy = Toy::load("toy_clp_quad")?;
    let ToyData::Jtask(p) = &toy.data else {
        return Err(fail("toy_clp_quad is conic"));
    };
    let problem = p.problem().map_err(fail)?;
    let (m, n, k) = (p.m, p.n, p.k);
    let mut x = vec![0.0; n];
    let mut s = vec![0.0; k + 1];
    let out = optsolver::SolutionBuffers {
        x: &mut x,
        y: None,
        z: None,
        row_activity: None,
        s: Some(&mut s),
        z_l: None,
        z_u: None,
    };
    let _ = m;
    match b.solve_into((&problem).into(), &mut Options::default(), out) {
        Err(Error::Invalid { what: "out.s", .. }) => Ok(()),
        other => Err(fail(format!(
            "expected Invalid out.s, got {:?}",
            other.map(|s| s.status)
        ))),
    }
}

fn toy_solve(b: &dyn Backend) -> Result<(), Failed> {
    let toy = Toy::load("toy_lp_max")?;
    let sol = toy.solve(b, false)?.map_err(fail)?;
    ensure(
        sol.status == Status::Optimal
            && sol.verification.as_ref().map(|r| r.overall) == Some(Overall::Verified),
        format!("toy_lp_max: {:?}", sol.status),
    )
}

fn t011_threads(b: &dyn Backend) -> Result<(), Failed> {
    // Four threads each creating, solving with and freeing environments twenty times
    // concurrently (Gurobi creation and freeing are serialised by the library's lock).
    let kind = b.capabilities().backend;
    let handles: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(move || -> Result<(), String> {
                for _ in 0..20 {
                    let env: Box<dyn Backend + Send> = match kind {
                        #[cfg(feature = "gurobi")]
                        "gurobi" => {
                            Box::new(optsolver::gurobi::Env::new().map_err(|e| e.to_string())?)
                        }
                        #[cfg(feature = "mosek")]
                        "mosek" => {
                            Box::new(optsolver::mosek::Env::new().map_err(|e| e.to_string())?)
                        }
                        other => return Err(format!("unknown backend {other}")),
                    };
                    toy_solve(&*env).map_err(|e| format!("{e:?}"))?;
                }
                Ok(())
            })
        })
        .collect();
    for h in handles {
        h.join()
            .map_err(|_| fail("thread panicked"))?
            .map_err(fail)?;
    }
    // Created on this thread, used and freed on another.
    let env = same_kind(b)?;
    std::thread::spawn(move || -> Result<(), String> {
        toy_solve(&*env).map_err(|e| format!("{e:?}"))?;
        drop(env);
        Ok(())
    })
    .join()
    .map_err(|_| fail("thread panicked"))?
    .map_err(fail)
}

fn t013_memory_option(b: &dyn Backend) -> Result<(), Failed> {
    let toy = Toy::load("toy_lp_max")?;
    let ToyData::Mps(p) = &toy.data else {
        return Err(fail("toy_lp_max is MPS"));
    };
    let problem = p.problem(&p.c).map_err(fail)?;
    let with = |raw: Vec<(String, ParamValue)>| {
        let mut o = Options::default();
        o.memory_limit_gb = Some(100.0);
        o.raw = raw;
        b.solve((&problem).into(), &mut o)
    };
    match b.capabilities().backend {
        "gurobi" => {
            let sol = with(Vec::new()).map_err(fail)?;
            ensure(
                sol.effective_parameters
                    .iter()
                    .any(|(n, v)| n == "SoftMemLimit" && *v == ParamValue::Double(100.0)),
                format!("SoftMemLimit not applied: {:?}", sol.effective_parameters),
            )?;
            ensure(
                sol.verification.as_ref().map(|r| r.overall) == Some(Overall::Verified),
                "verdict",
            )?;
            match with(vec![("SoftMemLimit".into(), ParamValue::Double(50.0))]) {
                Err(Error::ParameterConflict { name }) if name == "SoftMemLimit" => {}
                other => {
                    return Err(fail(format!(
                        "expected a conflict, got {:?}",
                        other.map(|s| s.status)
                    )));
                }
            }
            let sol = with(vec![("MemLimit".into(), ParamValue::Double(200.0))]).map_err(fail)?;
            ensure(
                sol.status == Status::Optimal,
                "raw MemLimit beside the option is accepted",
            )
        }
        _ => match with(Vec::new()) {
            Err(Error::Unsupported {
                field: "memory_limit_gb",
                backend: "mosek",
            }) => Ok(()),
            other => Err(fail(format!(
                "expected Unsupported memory_limit_gb, got {:?}",
                other.map(|s| s.status)
            ))),
        },
    }
}

fn t014_memory_limit(b: &dyn Backend) -> Result<(), Failed> {
    let (f, seed) = crate::common::us3::corpus_with_seed(crate::common::us3::LARGEST_INSTANCE, 0)?;
    let c = crate::common::objectives::seeded_objective(seed, f.problem.n);
    let problem = f.problem.problem(&c).map_err(fail)?;
    let solve = |limit: Option<f64>| {
        let mut o = Options::default();
        o.threads = Some(1);
        o.memory_limit_gb = limit;
        b.solve((&problem).into(), &mut o)
    };
    if b.capabilities().backend == "mosek" {
        return match solve(Some(1e-6)) {
            Err(Error::Unsupported {
                field: "memory_limit_gb",
                ..
            }) => Ok(()),
            other => Err(fail(format!("MOSEK: {:?}", other.map(|s| s.status)))),
        };
    }
    for rep in 0..3 {
        let sol = solve(Some(1e-6)).map_err(fail)?;
        ensure(
            sol.status == Status::StoppedAtLimit && sol.limit_reason == Some(LimitReason::Memory),
            format!("repetition {rep}: {:?} {:?}", sol.status, sol.limit_reason),
        )?;
    }
    let sol = solve(None).map_err(fail)?;
    ensure(
        sol.status == Status::Optimal
            && sol.verification.as_ref().map(|r| r.overall) == Some(Overall::Verified),
        format!("without the limit: {:?}", sol.status),
    )
}
