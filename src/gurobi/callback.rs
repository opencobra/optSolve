//! The Gurobi callback: cancellation and progress (task T044; research R7; constitution
//! VII-C, FR-019).
//!
//! One internal `extern "C"` trampoline is installed with `GRBsetcallbackfunc` only when a
//! cancellation flag or a progress closure is supplied. It reads the flag with a relaxed
//! atomic load and calls `GRBterminate` when it is set; on `GRB_CB_SIMPLEX` and
//! `GRB_CB_BARRIER` it builds a solver-independent [`Progress`] and calls the closure inside
//! `catch_unwind`. A caught panic is stored, the solve is terminated, and the trampoline does
//! nothing more; `gurobi::solve` resumes the panic after `GRBoptimize` returns.
//!
//! Elapsed time is `GRB_CB_RUNTIME`, which the Gurobi 13.0 manual (Callback Codes, checked
//! 2026-11-14) makes available "for all `where` arguments except for POLLING"; if the query
//! fails, the time since [`CallbackState::new`] is used instead.

use core::ffi::{c_double, c_int, c_void};
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use super::{env, ffi};
use crate::error::Result;
use crate::options::{Progress, ProgressPhase};

/// State shared with the trampoline through `usrdata`.
///
/// After [`install`] and until [`uninstall`], the state must not move, be dropped, or be
/// accessed by anything but the trampoline.
pub(crate) struct CallbackState<'a> {
    cancel: Option<&'a AtomicBool>,
    progress: Option<&'a mut dyn FnMut(&Progress)>,
    start: Instant,
    panic: Option<Box<dyn Any + Send>>,
}

impl<'a> CallbackState<'a> {
    /// New state; the elapsed-time fallback clock starts now.
    pub(crate) fn new(
        cancel: Option<&'a AtomicBool>,
        progress: Option<&'a mut dyn FnMut(&Progress)>,
    ) -> Self {
        CallbackState {
            cancel,
            progress,
            start: Instant::now(),
            panic: None,
        }
    }

    /// Returns the progress closure so the caller can put it back into its options.
    pub(crate) fn into_progress(self) -> Option<&'a mut dyn FnMut(&Progress)> {
        self.progress
    }

    fn needed(&self) -> bool {
        self.cancel.is_some() || self.progress.is_some()
    }
}

/// Installs the trampoline on `model` with `state` as user data, only when a cancellation flag
/// or progress closure is set (otherwise does nothing). Returns `Ok(true)` when installed.
///
/// `model` must be a live model; `state` must stay in place, untouched, until [`uninstall`]
/// has been called on `model`.
pub(crate) fn install(model: *mut ffi::GRBmodel, state: &mut CallbackState<'_>) -> Result<bool> {
    if !state.needed() {
        return Ok(false);
    }
    let usrdata = (state as *mut CallbackState<'_>).cast::<c_void>();
    // SAFETY: `model` is a live model (caller's contract). `usrdata` points to `state`, which
    // the caller keeps alive and in place until `uninstall`; the trampoline only casts it back
    // to the same type.
    let code = unsafe { ffi::GRBsetcallbackfunc(model, Some(trampoline), usrdata) };
    if code != 0 {
        return Err(model_error(model, code));
    }
    Ok(true)
}

/// Removes any callback from `model` (`GRBsetcallbackfunc` with `NULL`). Call it after
/// `GRBoptimize` returns and before the [`CallbackState`] is dropped or used.
pub(crate) fn uninstall(model: *mut ffi::GRBmodel) -> Result<()> {
    // SAFETY: `model` is a live model (caller's contract); a null callback removes the
    // existing one and Gurobi stores no user data.
    let code = unsafe { ffi::GRBsetcallbackfunc(model, None, core::ptr::null_mut()) };
    if code != 0 {
        return Err(model_error(model, code));
    }
    Ok(())
}

/// The panic caught in the progress closure, if any, for `std::panic::resume_unwind` after
/// `GRBoptimize` returns.
pub(crate) fn take_panic(state: &mut CallbackState<'_>) -> Option<Box<dyn Any + Send>> {
    state.panic.take()
}

fn model_error(model: *mut ffi::GRBmodel, code: c_int) -> crate::error::Error {
    // SAFETY: `model` is live; GRBgetenv returns the model's environment (or null, which
    // `env::error` handles).
    let menv = unsafe { ffi::GRBgetenv(model) };
    env::error(menv, code)
}

fn cb_double(cbdata: *mut c_void, where_: c_int, what: c_int) -> Option<f64> {
    let mut v: c_double = 0.0;
    // SAFETY: called only from the trampoline with Gurobi's `cbdata` and `where_`; `what` is
    // a double-valued code for this `where_` and `v` is a writable double.
    let code = unsafe { ffi::GRBcbget(cbdata, where_, what, (&mut v as *mut c_double).cast()) };
    (code == 0).then_some(v)
}

fn cb_int(cbdata: *mut c_void, where_: c_int, what: c_int) -> Option<c_int> {
    let mut v: c_int = 0;
    // SAFETY: as in `cb_double`, for an int-valued code and a writable int.
    let code = unsafe { ffi::GRBcbget(cbdata, where_, what, (&mut v as *mut c_int).cast()) };
    (code == 0).then_some(v)
}

/// Progress for `where_`, or `None` for where-codes that carry no progress information.
fn read_progress(cbdata: *mut c_void, where_: c_int, start: Instant) -> Option<Progress> {
    let (phase, iterations, primal_objective, dual_objective) = match where_ {
        ffi::GRB_CB_SIMPLEX => (
            ProgressPhase::Simplex,
            cb_double(cbdata, where_, ffi::GRB_CB_SPX_ITRCNT),
            // Gurobi's "current simplex objective value" (for dual simplex, a dual bound).
            cb_double(cbdata, where_, ffi::GRB_CB_SPX_OBJVAL),
            None,
        ),
        ffi::GRB_CB_BARRIER => (
            ProgressPhase::Barrier,
            cb_int(cbdata, where_, ffi::GRB_CB_BARRIER_ITRCNT).map(f64::from),
            cb_double(cbdata, where_, ffi::GRB_CB_BARRIER_PRIMOBJ),
            cb_double(cbdata, where_, ffi::GRB_CB_BARRIER_DUALOBJ),
        ),
        _ => return None,
    };
    let elapsed = cb_double(cbdata, where_, ffi::GRB_CB_RUNTIME)
        .unwrap_or_else(|| start.elapsed().as_secs_f64());
    Some(Progress {
        elapsed,
        primal_objective,
        dual_objective,
        iterations,
        phase,
    })
}

/// The `extern "C"` callback. Never unwinds; always returns 0.
pub(super) unsafe extern "C" fn trampoline(
    model: *mut ffi::GRBmodel,
    cbdata: *mut c_void,
    where_: c_int,
    usrdata: *mut c_void,
) -> c_int {
    if usrdata.is_null() {
        return 0;
    }
    // SAFETY: `usrdata` is the `CallbackState` registered by `install`, which by its contract
    // is alive, in place, and accessed by nothing else until `uninstall`; Gurobi invokes the
    // callback from one thread at a time, on the thread that called optimise (established as
    // behaviour by the per-backend contract test `f002::t015_callback_thread`, four threads,
    // barrier), so this exclusive reference is unique.
    let state = unsafe { &mut *usrdata.cast::<CallbackState<'_>>() };
    if state.panic.is_some() {
        return 0;
    }
    if state
        .cancel
        .is_some_and(|flag| flag.load(Ordering::Relaxed))
    {
        // SAFETY: `model` is the model being optimised; GRBterminate may be called from a
        // callback (13.0 manual, research R17.7).
        unsafe { ffi::GRBterminate(model) };
    }
    let start = state.start;
    let Some(f) = state.progress.as_deref_mut() else {
        return 0;
    };
    let Some(progress) = read_progress(cbdata, where_, start) else {
        return 0;
    };
    if let Err(payload) = catch_unwind(AssertUnwindSafe(|| f(&progress))) {
        state.panic = Some(payload);
        // SAFETY: as above.
        unsafe { ffi::GRBterminate(model) };
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gurobi::{Env, params};
    use crate::options::{Method, Options, Presolve};
    use core::ffi::c_char;
    use core::ptr;

    /// A small LP, min −x0 − x1 − … subject to a few dense rows, loaded into a new model.
    struct Model(*mut ffi::GRBmodel);

    impl Drop for Model {
        fn drop(&mut self) {
            // SAFETY: model owned here, freed once, before its Env (declared earlier).
            unsafe { ffi::GRBfreemodel(self.0) };
        }
    }

    fn load(env: &Env, n: usize, m: usize) -> Model {
        let mut obj: Vec<f64> = (0..n).map(|j| -1.0 - (j % 7) as f64).collect();
        let mut sense = vec![ffi::GRB_LESS_EQUAL; m];
        let mut rhs = vec![10.0; m];
        let mut vbeg: Vec<usize> = (0..n).map(|j| j * m).collect();
        let mut vlen = vec![c_int::try_from(m).unwrap(); n];
        let mut vind: Vec<c_int> = (0..n)
            .flat_map(|_| 0..c_int::try_from(m).unwrap())
            .collect();
        let mut vval: Vec<f64> = (0..n * m).map(|k| 1.0 + ((k * 31) % 11) as f64).collect();
        let mut lb = vec![0.0; n];
        let mut ub = vec![ffi::GRB_INFINITY; n];
        let mut model = ptr::null_mut();
        // SAFETY: every array has the length GRBXloadmodel reads for n columns, m rows and
        // n*m nonzeros; names, vtype are null; `model` is a writable local.
        let code = unsafe {
            ffi::GRBXloadmodel(
                env.raw(),
                &mut model,
                ptr::null(),
                c_int::try_from(n).unwrap(),
                c_int::try_from(m).unwrap(),
                ffi::GRB_MINIMIZE,
                0.0,
                obj.as_mut_ptr(),
                sense.as_mut_ptr(),
                rhs.as_mut_ptr(),
                vbeg.as_mut_ptr(),
                vlen.as_mut_ptr(),
                vind.as_mut_ptr(),
                vval.as_mut_ptr(),
                lb.as_mut_ptr(),
                ub.as_mut_ptr(),
                ptr::null_mut::<c_char>(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        assert_eq!(code, 0);
        let model = Model(model);
        let o = Options {
            presolve: Some(Presolve::Off),
            threads: Some(1),
            ..Options::default()
        };
        // SAFETY: live model.
        params::apply(unsafe { ffi::GRBgetenv(model.0) }, &o).unwrap();
        model
    }

    fn set_method(model: &Model, method: Method) {
        let o = Options {
            presolve: Some(Presolve::Off),
            threads: Some(1),
            method: Some(method),
            ..Options::default()
        };
        // SAFETY: live model.
        params::apply(unsafe { ffi::GRBgetenv(model.0) }, &o).unwrap();
    }

    fn optimize(model: &Model) -> c_int {
        // SAFETY: live model.
        assert_eq!(unsafe { ffi::GRBoptimize(model.0) }, 0);
        let mut status = 0;
        // SAFETY: live model, NUL-terminated name, writable int.
        let code = unsafe { ffi::GRBgetintattr(model.0, c"Status".as_ptr(), &mut status) };
        assert_eq!(code, 0);
        status
    }

    fn env() -> Env {
        Env::new().expect("Gurobi environment (licence required for these tests)")
    }

    #[test]
    fn nothing_installed_without_cancel_or_progress() {
        let env = env();
        let model = load(&env, 3, 2);
        let mut state = CallbackState::new(None, None);
        assert!(!install(model.0, &mut state).unwrap());
        assert_eq!(optimize(&model), ffi::GRB_OPTIMAL);
        uninstall(model.0).unwrap();
    }

    #[test]
    fn set_cancel_flag_interrupts_solve() {
        let env = env();
        let model = load(&env, 200, 100);
        let flag = AtomicBool::new(true);
        let mut state = CallbackState::new(Some(&flag), None);
        assert!(install(model.0, &mut state).unwrap());
        let status = optimize(&model);
        uninstall(model.0).unwrap();
        assert_eq!(status, ffi::GRB_INTERRUPTED);
        assert!(take_panic(&mut state).is_none());
    }

    #[test]
    fn unset_cancel_flag_does_not_interrupt() {
        let env = env();
        let model = load(&env, 20, 10);
        let flag = AtomicBool::new(false);
        let mut state = CallbackState::new(Some(&flag), None);
        install(model.0, &mut state).unwrap();
        let status = optimize(&model);
        uninstall(model.0).unwrap();
        assert_eq!(status, ffi::GRB_OPTIMAL);
    }

    #[test]
    fn progress_reports_simplex_and_barrier() {
        let env = env();
        for (method, phase) in [
            (Method::PrimalSimplex, ProgressPhase::Simplex),
            (Method::Barrier, ProgressPhase::Barrier),
        ] {
            let model = load(&env, 200, 100);
            set_method(&model, method);
            let mut seen: Vec<Progress> = Vec::new();
            let mut f = |p: &Progress| seen.push(*p);
            let mut state = CallbackState::new(None, Some(&mut f));
            install(model.0, &mut state).unwrap();
            let status = optimize(&model);
            uninstall(model.0).unwrap();
            assert!(take_panic(&mut state).is_none());
            drop(state);
            assert_eq!(status, ffi::GRB_OPTIMAL);
            assert!(!seen.is_empty(), "{method:?}: no progress reported");
            // Barrier is followed by crossover, which reports simplex progress.
            assert!(
                seen.iter().any(|p| p.phase == phase),
                "{method:?}: no {phase:?} record"
            );
            assert!(seen.iter().all(|p| p.elapsed >= 0.0));
            assert!(seen.iter().any(|p| p.iterations.is_some()));
            if phase == ProgressPhase::Barrier {
                assert!(seen.iter().any(|p| p.dual_objective.is_some()));
            }
        }
    }

    #[test]
    fn panic_in_progress_is_caught_terminates_and_is_returned() {
        let env = env();
        let model = load(&env, 200, 100);
        set_method(&model, Method::PrimalSimplex);
        let mut calls = 0_u32;
        let mut f = |_: &Progress| {
            calls += 1;
            panic!("progress closure panicked");
        };
        let mut state = CallbackState::new(None, Some(&mut f));
        install(model.0, &mut state).unwrap();
        let status = optimize(&model);
        uninstall(model.0).unwrap();
        let payload = take_panic(&mut state).expect("panic stored");
        drop(state);
        assert_eq!(status, ffi::GRB_INTERRUPTED);
        assert_eq!(calls, 1, "closure not called again after its panic");
        assert_eq!(
            payload.downcast_ref::<&str>(),
            Some(&"progress closure panicked")
        );
    }
}
