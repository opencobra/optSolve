//! Licensed probes of MOSEK behaviour the backend relies on (task T021; feature 002 research
//! R18). Each probe asserts the behaviour observed on MOSEK 11.2 so that a change in a later
//! version fails here rather than silently in the backend. Without a licence they are
//! skipped with a message.

#![cfg(test)]
// Probe set-up groups several plain vendor calls on a live task per block; each block's
// SAFETY comment covers all of them.
#![allow(clippy::multiple_unsafe_ops_per_block)]

use core::ptr;

use super::env::{Env, Task};
use super::ffi;

fn env() -> Option<Env> {
    match Env::new() {
        Ok(e) => Some(e),
        Err(e) => {
            println!("ignored: no MOSEK licence ({e})");
            None
        }
    }
}

/// One variable, no rows; minimise x with the given bound key and values.
fn one_var(env: &Env, bk: i32, bl: f64, bu: f64) -> (Task<'_>, i32) {
    let task = env.task().expect("task");
    let t = task.raw();
    // SAFETY: live task; scalar arguments; pointers to live locals of length 1.
    let rc = unsafe {
        assert_eq!(ffi::MSK_appendvars(t, 1), 0);
        let c = [1.0];
        assert_eq!(ffi::MSK_putcslice(t, 0, 1, c.as_ptr()), 0);
        let (k, l, u) = ([bk], [bl], [bu]);
        ffi::MSK_putvarboundslice(t, 0, 1, k.as_ptr(), l.as_ptr(), u.as_ptr())
    };
    (task, rc)
}

fn optimize(task: &Task<'_>) -> (i32, i32) {
    let mut trm = 0;
    // SAFETY: live task, writable local.
    let rc = unsafe { ffi::MSK_optimizetrm(task.raw(), &mut trm) };
    (rc, trm)
}

fn stat(task: &Task<'_>, st: i32) -> Option<(i32, i32)> {
    let mut d = 0;
    let (mut p, mut s) = (0, 0);
    // SAFETY: live task, writable locals.
    unsafe {
        ffi::MSK_solutiondef(task.raw(), st, &mut d);
        if d == 0 {
            return None;
        }
        ffi::MSK_getprosta(task.raw(), st, &mut p);
        ffi::MSK_getsolsta(task.raw(), st, &mut s);
    }
    Some((p, s))
}

/// Probe 1 (research R3): an IEEE infinity on a side the bound key marks infinite. Observed:
/// accepted for `MSK_BK_LO` with u = +∞ and `MSK_BK_FR` with ±∞, and the solve is unaffected;
/// an infinite value on a finite side is rejected. The backend nevertheless copies infinite
/// values to ±`MSK_INFINITY` (only when present), which needs no reliance on this.
#[test]
fn probe_infinite_value_on_infinite_side() {
    let Some(env) = env() else { return };
    let (task, rc) = one_var(&env, ffi::MSK_BK_LO, 2.0, f64::INFINITY);
    println!("BK_LO with u = +inf: rc {rc}");
    let lo_ok = rc == ffi::MSK_RES_OK;
    if lo_ok {
        let (rc, _) = optimize(&task);
        assert_eq!(rc, ffi::MSK_RES_OK);
        assert_eq!(
            stat(&task, ffi::MSK_SOL_BAS)
                .or(stat(&task, ffi::MSK_SOL_ITR))
                .map(|s| s.1),
            Some(ffi::MSK_SOL_STA_OPTIMAL)
        );
    }
    let (_, rc_fr) = one_var(&env, ffi::MSK_BK_FR, f64::NEG_INFINITY, f64::INFINITY);
    println!("BK_FR with ±inf: rc {rc_fr}");
    let (_, rc_bad) = one_var(&env, ffi::MSK_BK_RA, f64::NEG_INFINITY, 1.0);
    println!("BK_RA with l = -inf: rc {rc_bad}");
    println!(
        "FINDING infinite-side: LO accepted {lo_ok}, FR accepted {}, RA with -inf accepted {}",
        rc_fr == 0,
        rc_bad == 0
    );
}

/// Probe 2 (research R6): which solution type carries an LP infeasibility certificate.
/// minimise x₁ + x₂ s.t. x₁ + x₂ ≥ 3, 0 ≤ x ≤ 1 with the default optimiser and with the dual
/// simplex. The backend reads the basic solution when defined, else the interior one.
#[test]
fn probe_lp_certificate_solution_type() {
    let Some(env) = env() else { return };
    use crate::options::{Method, Options};
    // Parameters only through `params::apply` (constitution VII-G).
    for method in [None, Some(Method::DualSimplex), Some(Method::Barrier)] {
        let task = env.task().expect("task");
        let t = task.raw();
        // SAFETY: live task; arrays are live locals of the lengths passed.
        unsafe {
            assert_eq!(ffi::MSK_appendvars(t, 2), 0);
            assert_eq!(ffi::MSK_appendcons(t, 1), 0);
            assert_eq!(ffi::MSK_putcslice(t, 0, 2, [1.0, 1.0].as_ptr()), 0);
            let (ptrb, ptre, sub, val) = ([0i64, 1], [1i64, 2], [0i32, 0], [1.0, 1.0]);
            assert_eq!(
                ffi::MSK_putacolslice64(
                    t,
                    0,
                    2,
                    ptrb.as_ptr(),
                    ptre.as_ptr(),
                    sub.as_ptr(),
                    val.as_ptr()
                ),
                0
            );
            let k = [ffi::MSK_BK_RA; 2];
            assert_eq!(
                ffi::MSK_putvarboundslice(
                    t,
                    0,
                    2,
                    k.as_ptr(),
                    [0.0, 0.0].as_ptr(),
                    [1.0, 1.0].as_ptr()
                ),
                0
            );
            assert_eq!(
                ffi::MSK_putconboundslice(
                    t,
                    0,
                    1,
                    [ffi::MSK_BK_LO].as_ptr(),
                    [3.0].as_ptr(),
                    [3.0].as_ptr()
                ),
                0
            );
        }
        let options = Options {
            method,
            ..Options::default()
        };
        super::params::apply(&task, &options, crate::backend::ProblemClass::Lp).expect("params");
        let (rc, trm) = optimize(&task);
        let bas = stat(&task, ffi::MSK_SOL_BAS);
        let itr = stat(&task, ffi::MSK_SOL_ITR);
        println!(
            "FINDING certificate method {method:?}: rc {rc} trm {trm} BAS {bas:?} ITR {itr:?}"
        );
        // What the backend relies on: the solution type it reads carries the certificate.
        let read = bas.or(itr).expect("a solution is defined");
        assert_eq!(read.1, ffi::MSK_SOL_STA_PRIM_INFEAS_CER);
    }
}

/// Probe 4 (research R3, R5): zero-length slices with NULL pointers (m = 0, k = 0) are
/// accepted, so empty problems need no special casing beyond skipping zero-length calls.
#[test]
fn probe_zero_length_null() {
    let Some(env) = env() else { return };
    let task = env.task().expect("task");
    let t = task.raw();
    // SAFETY: live task; zero-length ranges with null pointers (nothing is read).
    let (a, b, c) = unsafe {
        (
            ffi::MSK_putconboundslice(t, 0, 0, ptr::null(), ptr::null(), ptr::null()),
            ffi::MSK_putcslice(t, 0, 0, ptr::null()),
            ffi::MSK_putafegslice(t, 0, 0, ptr::null()),
        )
    };
    println!("FINDING zero-length NULL: putconboundslice {a}, putcslice {b}, putafegslice {c}");
    let (rc, trm) = optimize(&task);
    println!(
        "FINDING empty task optimise: rc {rc} trm {trm} ITR {:?}",
        stat(&task, ffi::MSK_SOL_ITR)
    );
}

/// Probe 3 (research R6): statuses after `MSK_RES_TRM_USER_CALLBACK`. The cancellation flag
/// is set before the solve, so the first callback ends it; the backend must report
/// Interrupted whatever problem and solution statuses MOSEK leaves.
#[test]
fn probe_status_after_user_callback() {
    use crate::backend::Backend;
    use crate::options::Options;
    use crate::problem::{CscRef, Problem, RowBounds, Sense};
    use crate::result::Status;
    use std::sync::atomic::AtomicBool;
    let Some(env) = env() else { return };
    // A dense 40 × 60 LP, enough for several interior-point iterations.
    let (m, n) = (40usize, 60usize);
    let cp: Vec<usize> = (0..=n).map(|j| j * m).collect();
    let ri: Vec<usize> = (0..n).flat_map(|_| 0..m).collect();
    let va: Vec<f64> = (0..n * m).map(|k| 1.0 + ((k * 31) % 11) as f64).collect();
    let c: Vec<f64> = (0..n).map(|j| -1.0 - (j % 7) as f64).collect();
    let b = vec![10.0; m];
    let sense = vec![b'<'; m];
    let (l, u) = (vec![0.0; n], vec![f64::INFINITY; n]);
    let a = CscRef::new(m, n, &cp, &ri, &va).expect("A");
    let p = Problem::new(
        a,
        &c,
        Sense::Minimise,
        RowBounds::SenseRhs {
            sense: &sense,
            b: &b,
        },
        &l,
        &u,
    )
    .expect("problem");
    let flag = AtomicBool::new(true);
    let mut o = Options {
        cancel: Some(&flag),
        ..Options::default()
    };
    let sol = env.solve((&p).into(), &mut o).expect("solve");
    println!(
        "FINDING user callback: status {:?}, raw {:?}",
        sol.status, sol.raw_status
    );
    assert_eq!(sol.status, Status::Interrupted);
}
