//! The owned MOSEK environment and the per-solve task (task T022; research R2; constitution
//! VII-B, VII-E).
//!
//! [`Env`] owns one `MSKenv_t`, checks a licence out at creation (`MSK_checkoutlicense`, so a
//! missing licence is reported by [`Env::new`] rather than at the first optimisation) and
//! deletes the environment in `Drop`. It is `Send` but not `Sync` (feature 005 research R3;
//! history at the end of the file).
//! A [`Task`] is created and deleted inside each solve and borrows the environment, so it
//! cannot outlive it. No solver parameter is set here (constitution VII-G).

use core::ffi::{CStr, c_char};
use core::marker::PhantomData;
use core::ptr::{self, NonNull};

use super::ffi;
use crate::error::{Error, Result};

/// An owned MOSEK environment with a licence checked out.
///
/// `Send`, not `Sync` (see the `unsafe impl Send` below).
#[derive(Debug)]
pub struct Env {
    raw: NonNull<ffi::MSKenv>,
    version: (i32, i32, i32),
}

// SAFETY: (constitution VII-B, feature 005 research R3) MOSEK 11.2 C API manual §10.3: "Sharing a
// task between threads is safe, as long as it is not accessed from more than one thread at a time.
// Multiple tasks can be created and used in parallel without any problems"; §10.6: "Calling
// MSK_optimize from different threads using the same MOSEK environment only consumes one license
// token". The environment may therefore be used from a thread other than its creator. Not `Sync`:
// the crate's solves take `&Env` without further synchronisation, and concurrent use of one
// environment is not needed. Concurrent creation and deletion of different environments is not
// documented either way (unverified).
unsafe impl Send for Env {}

/// Version of the MOSEK library loaded at run time (`MSK_getversion`; constitution VII-E).
pub(crate) fn library_version() -> (i32, i32, i32) {
    let (mut major, mut minor, mut revision) = (0, 0, 0);
    // SAFETY: MSK_getversion writes one int through each pointer; all three point to distinct
    // live, writable locals.
    unsafe { ffi::MSK_getversion(&mut major, &mut minor, &mut revision) };
    (major, minor, revision)
}

/// Symbolic name and description of a response code (`MSK_getcodedesc`), for example
/// `"MSK_RES_ERR_LICENSE: License error."`.
pub(crate) fn code_description(code: i32) -> String {
    let mut sym = [0 as c_char; ffi::MSK_MAX_STR_LEN];
    let mut desc = [0 as c_char; ffi::MSK_MAX_STR_LEN];
    // SAFETY: both buffers are MSK_MAX_STR_LEN bytes, the size the manual requires for
    // MSK_getcodedesc; MOSEK writes NUL-terminated strings into them.
    let rc = unsafe { ffi::MSK_getcodedesc(code, sym.as_mut_ptr(), desc.as_mut_ptr()) };
    if rc != ffi::MSK_RES_OK {
        return format!("response code {code}");
    }
    // SAFETY: MOSEK wrote NUL-terminated strings within the buffers (success above); the last
    // byte is forced to NUL as a guard before reading.
    let text = |b: &mut [c_char; ffi::MSK_MAX_STR_LEN]| {
        b[ffi::MSK_MAX_STR_LEN - 1] = 0;
        // SAFETY: as stated in the SAFETY note for this block of calls above.
        unsafe { CStr::from_ptr(b.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    };
    format!("{}: {}", text(&mut sym), text(&mut desc))
}

/// [`Error::Solver`] for a response code without a task (environment calls).
pub(crate) fn env_error(code: i32) -> Error {
    Error::Solver {
        code,
        message: code_description(code),
    }
}

/// [`Error::Solver`] for a failed task call: the task's last error message when MOSEK
/// recorded one (`MSK_getlasterror64`), else the code's description. Call it immediately
/// after the failing call (constitution III).
pub(crate) fn task_error(task: ffi::MSKtask_t, code: i32) -> Error {
    let mut last = 0;
    let mut len: i64 = 0;
    let mut buf = vec![0 as c_char; 4096];
    let size = i64::try_from(buf.len()).unwrap_or(i64::MAX);
    // SAFETY: `task` is a live task (caller's contract); `buf` has `size` writable bytes and
    // MOSEK writes at most `size` bytes including the terminating NUL; `last` and `len` are
    // writable locals.
    let rc = unsafe { ffi::MSK_getlasterror64(task, &mut last, size, &mut len, buf.as_mut_ptr()) };
    let detail = if rc == ffi::MSK_RES_OK && buf[0] != 0 {
        if let Some(b) = buf.last_mut() {
            *b = 0;
        }
        // SAFETY: NUL-terminated (guard above), within `buf`, which outlives the read.
        let s = unsafe { CStr::from_ptr(buf.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        s.trim().to_string()
    } else {
        String::new()
    };
    let desc = code_description(code);
    Error::Solver {
        code,
        message: if detail.is_empty() {
            desc
        } else {
            format!("{desc} ({detail})")
        },
    }
}

impl Env {
    /// Creates a MOSEK environment and checks out a licence for the base optimiser
    /// (`MSK_FEATURE_PTS`).
    ///
    /// Fails with [`Error::Solver`] (MOSEK response code and description) when the
    /// environment cannot be created or no licence is available. MOSEK prints nothing unless
    /// a stream is linked (research R2), so creation is silent.
    pub fn new() -> Result<Env> {
        let mut raw: ffi::MSKenv_t = ptr::null_mut();
        // SAFETY: `raw` is a live, writable local; a null debug file name is allowed (no
        // memory debug report).
        let rc = unsafe { ffi::MSK_makeenv(&mut raw, ptr::null()) };
        if rc != ffi::MSK_RES_OK {
            if !raw.is_null() {
                // SAFETY: `raw` was produced by MSK_makeenv, owned here, deleted once.
                unsafe { ffi::MSK_deleteenv(&mut raw) };
            }
            return Err(env_error(rc));
        }
        let Some(nn) = NonNull::new(raw) else {
            return Err(Error::Solver {
                code: 0,
                message: "MSK_makeenv returned success but no environment".into(),
            });
        };
        // From here `env` owns `raw` and deletes it on any early return.
        let env = Env {
            raw: nn,
            version: library_version(),
        };
        // SAFETY: `env.raw` is a live environment.
        let rc = unsafe { ffi::MSK_checkoutlicense(env.raw.as_ptr(), ffi::MSK_FEATURE_PTS) };
        if rc != ffi::MSK_RES_OK {
            return Err(env_error(rc));
        }
        Ok(env)
    }

    /// MOSEK library version (major, minor, revision) reported by `MSK_getversion`.
    pub fn version(&self) -> (i32, i32, i32) {
        self.version
    }

    /// A new empty task in this environment.
    pub(crate) fn task(&self) -> Result<Task<'_>> {
        let mut raw: ffi::MSKtask_t = ptr::null_mut();
        // SAFETY: `self.raw` is live for the lifetime of the returned task (borrow); `raw` is
        // a writable local.
        let rc = unsafe { ffi::MSK_makeemptytask(self.raw.as_ptr(), &mut raw) };
        if rc != ffi::MSK_RES_OK {
            return Err(env_error(rc));
        }
        NonNull::new(raw)
            .map(|raw| Task {
                raw,
                _env: PhantomData,
            })
            .ok_or_else(|| Error::Solver {
                code: 0,
                message: "MSK_makeemptytask returned success but no task".into(),
            })
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        let mut raw = self.raw.as_ptr();
        // SAFETY: `self.raw` is exclusively owned and deleted exactly once here. Tasks borrow
        // the `Env`, so they have already been deleted (VII-B).
        unsafe { ffi::MSK_deleteenv(&mut raw) };
    }
}

/// A MOSEK task, deleted in `Drop`; it borrows its environment.
#[derive(Debug)]
pub(crate) struct Task<'e> {
    raw: NonNull<ffi::MSKtask>,
    _env: PhantomData<&'e Env>,
}

impl Task<'_> {
    /// Raw pointer, valid while `self` is alive.
    pub(crate) fn raw(&self) -> ffi::MSKtask_t {
        self.raw.as_ptr()
    }

    /// Converts a response code of a call on this task into a `Result`.
    pub(crate) fn check(&self, rc: i32) -> Result<()> {
        if rc == ffi::MSK_RES_OK {
            Ok(())
        } else {
            Err(task_error(self.raw(), rc))
        }
    }
}

impl Drop for Task<'_> {
    fn drop(&mut self) {
        let mut raw = self.raw.as_ptr();
        // SAFETY: the task is exclusively owned and deleted exactly once, before its
        // environment (borrow).
        unsafe { ffi::MSK_deletetask(&mut raw) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Env>();
    }

    #[test]
    fn library_version_matches_declared_major_minor() {
        let (major, minor, _) = library_version();
        let declared = env!("OPTSOLVER_MOSEK_DECLARED_VERSION");
        let mut parts = declared.split('.');
        assert_eq!(parts.next().and_then(|s| s.parse().ok()), Some(major));
        assert_eq!(parts.next().and_then(|s| s.parse().ok()), Some(minor));
    }

    #[test]
    fn code_description_names_the_code() {
        let d = code_description(ffi::MSK_RES_TRM_STALL);
        assert!(d.starts_with("MSK_RES_TRM_STALL"), "{d}");
    }

    #[test]
    fn new_returns_env_or_solver_error() {
        match Env::new() {
            Ok(env) => {
                assert_eq!(env.version().0, library_version().0);
                let task = env.task().expect("empty task");
                drop(task);
            }
            Err(Error::Solver { code, message }) => {
                assert_ne!(code, 0);
                assert!(!message.is_empty());
            }
            Err(other) => panic!("unexpected error kind: {other:?}"),
        }
    }
}

// History
//
// Feature 002 research R2 kept `Env` neither `Send` nor `Sync`, as for Gurobi, pending a vendor
// check; feature 005 (research R3) made that check and made `Env` `Send`.
