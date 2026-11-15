//! The MOSEK callback: cancellation and progress (task T025; feature 002 research R8;
//! constitution VII-C).
//!
//! One internal `extern "C"` trampoline is installed with `MSK_putcallbackfunc` only when a
//! cancellation flag or a progress closure is supplied. MOSEK has no termination function:
//! the trampoline returns non-zero when the flag is set, which ends the optimisation with
//! `MSK_RES_TRM_USER_CALLBACK`. On the interior-point, conic and simplex progress codes it
//! builds a solver-independent [`Progress`] from the information arrays MOSEK passes and calls
//! the closure inside `catch_unwind`; a caught panic is stored, the optimisation is ended,
//! and `mosek::solve` resumes the panic after `MSK_optimizetrm` returns. The callback calls
//! no MOSEK function (M§7.7).

use core::ffi::c_void;
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use super::env::Task;
use super::ffi;
use crate::error::Result;
use crate::options::{Progress, ProgressPhase};

/// State shared with the trampoline through the user handle. After [`install`] and until
/// [`uninstall`] it must not move, be dropped, or be accessed by anything but the trampoline.
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

    /// The panic caught in the progress closure, if any.
    pub(crate) fn take_panic(&mut self) -> Option<Box<dyn Any + Send>> {
        self.panic.take()
    }

    fn needed(&self) -> bool {
        self.cancel.is_some() || self.progress.is_some()
    }
}

/// Installs the trampoline on `task` with `state` as user handle when a cancellation flag or
/// progress closure is set; returns whether it did. `state` must stay in place, untouched,
/// until [`uninstall`].
pub(crate) fn install(task: &Task<'_>, state: &mut CallbackState<'_>) -> Result<bool> {
    if !state.needed() {
        return Ok(false);
    }
    let handle = (state as *mut CallbackState<'_>).cast::<c_void>();
    // SAFETY: the task is live; `handle` points to `state`, which the caller keeps alive and
    // in place until `uninstall`; the trampoline only casts it back to the same type.
    let rc = unsafe { ffi::MSK_putcallbackfunc(task.raw(), Some(trampoline), handle) };
    task.check(rc)?;
    Ok(true)
}

/// Removes the callback from `task`. Call after `MSK_optimizetrm` returns and before the
/// [`CallbackState`] is dropped or used.
pub(crate) fn uninstall(task: &Task<'_>) -> Result<()> {
    // SAFETY: the task is live; a null function removes the callback and the handle is not
    // stored.
    let rc = unsafe { ffi::MSK_putcallbackfunc(task.raw(), None, core::ptr::null_mut()) };
    task.check(rc)
}

/// Reads entry `i` of a MOSEK information array, `None` for a null array.
///
/// # Safety
/// A non-null `p` must point to an information array MOSEK passed to the callback, which
/// has an entry for every information item of its type (`i` is such an item).
unsafe fn at<T: Copy>(p: *const T, i: i32) -> Option<T> {
    if p.is_null() {
        return None;
    }
    let i = usize::try_from(i).ok()?;
    // SAFETY: by the function's contract the array covers item `i`, so the offset stays
    // inside it.
    let q = unsafe { p.add(i) };
    // SAFETY: `q` points to entry `i` of that array, initialised by MOSEK.
    Some(unsafe { *q })
}

fn read_progress(
    caller: i32,
    douinf: *const f64,
    intinf: *const i32,
    start: Instant,
) -> Option<Progress> {
    // SAFETY: (all `at` calls): MOSEK passes `douinf` and `intinf` indexed by MSK_DINF_* and
    // MSK_IINF_* items (MOSEK 11 C API, MSKcallbackfunc), or null, which `at` handles.
    let elapsed = unsafe { at(douinf, ffi::MSK_DINF_OPTIMIZER_TIME) }
        .filter(|t| t.is_finite() && *t >= 0.0)
        .unwrap_or_else(|| start.elapsed().as_secs_f64());
    let (phase, iterations, primal_objective, dual_objective) = match caller {
        ffi::MSK_CALLBACK_INTPNT | ffi::MSK_CALLBACK_CONIC => (
            ProgressPhase::Barrier,
            // SAFETY: see above.
            unsafe { at(intinf, ffi::MSK_IINF_INTPNT_ITER) }.map(f64::from),
            // SAFETY: see above.
            unsafe { at(douinf, ffi::MSK_DINF_INTPNT_PRIMAL_OBJ) },
            // SAFETY: see above.
            unsafe { at(douinf, ffi::MSK_DINF_INTPNT_DUAL_OBJ) },
        ),
        ffi::MSK_CALLBACK_UPDATE_PRIMAL_SIMPLEX
        | ffi::MSK_CALLBACK_UPDATE_DUAL_SIMPLEX
        | ffi::MSK_CALLBACK_UPDATE_SIMPLEX => {
            // SAFETY: see above.
            let primal = unsafe { at(intinf, ffi::MSK_IINF_SIM_PRIMAL_ITER) };
            // SAFETY: see above.
            let dual = unsafe { at(intinf, ffi::MSK_IINF_SIM_DUAL_ITER) };
            let iterations = match (primal, dual) {
                (None, None) => None,
                (p, d) => Some(f64::from(p.unwrap_or(0)) + f64::from(d.unwrap_or(0))),
            };
            (
                ProgressPhase::Simplex,
                iterations,
                // SAFETY: see above.
                unsafe { at(douinf, ffi::MSK_DINF_SIM_OBJ) },
                None,
            )
        }
        _ => return None,
    };
    Some(Progress {
        elapsed,
        primal_objective,
        dual_objective,
        iterations,
        phase,
    })
}

/// The `extern "C"` callback. Never unwinds; returns non-zero to end the optimisation.
unsafe extern "C" fn trampoline(
    _task: ffi::MSKtask_t,
    handle: *mut c_void,
    caller: i32,
    douinf: *const f64,
    intinf: *const i32,
    _lintinf: *const i64,
) -> i32 {
    if handle.is_null() {
        return 0;
    }
    // SAFETY: `handle` is the `CallbackState` registered by `install`, alive, in place and
    // accessed by nothing else until `uninstall`. MOSEK invokes the callback one call at a
    // time on the thread that called `MSK_optimizetrm` (established as behaviour by the
    // per-backend contract test `f002::t015_callback_thread`), so this reference is unique.
    let state = unsafe { &mut *handle.cast::<CallbackState<'_>>() };
    if state.panic.is_some() {
        return 1;
    }
    let stop = state
        .cancel
        .is_some_and(|flag| flag.load(Ordering::Relaxed));
    let start = state.start;
    if let Some(f) = state.progress.as_deref_mut() {
        if let Some(progress) = read_progress(caller, douinf, intinf, start) {
            if let Err(payload) = catch_unwind(AssertUnwindSafe(|| f(&progress))) {
                state.panic = Some(payload);
                return 1;
            }
        }
    }
    // The flag may have been set by the closure itself.
    let stop = stop
        || state
            .cancel
            .is_some_and(|flag| flag.load(Ordering::Relaxed));
    i32::from(stop)
}
