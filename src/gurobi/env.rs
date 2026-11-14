//! The owned Gurobi environment (task T040; constitution VII-B, VII-E; research R15).
//!
//! [`Env`] owns one `GRBenv` and frees it in `Drop`. It is neither `Send` nor `Sync`: the
//! Gurobi 13.0 reference manual (C API, `GRBloadenv`) states that "environments are not thread
//! safe" and that "environment initialization is itself not thread-safe" (research R15, R17).
//!
//! No solver parameter is set here (constitution VII-G): parameters are set only in
//! `gurobi::params`.

use core::ffi::{CStr, c_int};
use core::ptr::{self, NonNull};

use super::ffi;
use crate::error::{Error, Result};

/// An owned, started Gurobi environment.
///
/// Holding a raw pointer makes `Env` `!Send` and `!Sync` automatically; no `unsafe impl` of
/// either exists (constitution VII-B). The following would fail to compile:
/// `fn assert_send<T: Send>() {} assert_send::<Env>();`
#[derive(Debug)]
pub struct Env {
    raw: NonNull<ffi::GRBenv>,
    version: (i32, i32, i32),
}

/// Version of the Gurobi library loaded at run time (`GRBversion`), never the header's
/// compile-time numbers (constitution VII-E).
fn library_version() -> (c_int, c_int, c_int) {
    let (mut major, mut minor, mut tech): (c_int, c_int, c_int) = (0, 0, 0);
    // SAFETY: GRBversion only writes one int through each of the three pointers, which point
    // to distinct, live, writable locals.
    unsafe { ffi::GRBversion(&mut major, &mut minor, &mut tech) };
    (major, minor, tech)
}

/// Reads the most recent error message of `env` into [`Error::Solver`] with `code`.
///
/// Must be called immediately after the failing call and before `env` is freed (constitution
/// III). A null `env` yields a message saying no environment was available.
pub(crate) fn error(env: *mut ffi::GRBenv, code: c_int) -> Error {
    let message = if env.is_null() {
        String::from("no Gurobi environment was created, so no error message is available")
    } else {
        // SAFETY: `env` is non-null and, by this function's contract, a live environment
        // returned by Gurobi. GRBgeterrormsg returns a pointer to a NUL-terminated string
        // owned by the environment (or null); it is copied before anything else touches env.
        let p = unsafe { ffi::GRBgeterrormsg(env) };
        if p.is_null() {
            String::new()
        } else {
            // SAFETY: `p` is non-null and points to a NUL-terminated string owned by `env`,
            // which stays alive for the duration of this copy.
            unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
        }
    };
    Error::Solver { code, message }
}

/// Creates an empty (unstarted) environment with `GRBemptyenvinternal`, passing the run-time
/// library version. On failure any partially created environment is freed after its error
/// message is read.
pub(crate) fn empty() -> Result<NonNull<ffi::GRBenv>> {
    let (major, minor, tech) = library_version();
    let mut raw: *mut ffi::GRBenv = ptr::null_mut();
    // SAFETY: `raw` is a live, writable local; Gurobi writes the new environment pointer
    // into it. The version numbers come from the loaded library (VII-E).
    let code = unsafe { ffi::GRBemptyenvinternal(&mut raw, major, minor, tech) };
    if code != 0 {
        let err = error(raw, code);
        if !raw.is_null() {
            // SAFETY: `raw` was produced by GRBemptyenvinternal, is not owned by any other
            // value, and its message has been read; it is freed exactly once here.
            unsafe { ffi::GRBfreeenv(raw) };
        }
        return Err(err);
    }
    NonNull::new(raw).ok_or_else(|| Error::Solver {
        code: 0,
        message: String::from("GRBemptyenv returned success but no environment"),
    })
}

/// Starts an empty environment (`GRBstartenv`; acquires the licence). On failure the error
/// message is read and the environment is freed, so `raw` must not be used afterwards.
pub(crate) fn start(raw: NonNull<ffi::GRBenv>) -> Result<NonNull<ffi::GRBenv>> {
    // SAFETY: `raw` is a live, unstarted environment from `empty`, owned by the caller and
    // passed here by value.
    let code = unsafe { ffi::GRBstartenv(raw.as_ptr()) };
    if code != 0 {
        let err = error(raw.as_ptr(), code);
        // SAFETY: the message has been read; per the 13.0 manual a failed GRBstartenv leaves
        // the environment in its previous (empty) state, which GRBfreeenv frees. Freed once.
        unsafe { ffi::GRBfreeenv(raw.as_ptr()) };
        return Err(err);
    }
    Ok(raw)
}

impl Env {
    /// Creates and starts a Gurobi environment.
    ///
    /// Fails with [`Error::Solver`] (vendor code and message) when the library cannot create
    /// the environment, for example when no licence is available.
    ///
    /// Gurobi prints a licence banner to standard output from `GRBstartenv` unless
    /// `OutputFlag` = 0 is set on the empty environment first (observed on 13.0.3, research
    /// R17); `gurobi::params::configure_new_environment` sets it (constitution III, VII-G).
    pub fn new() -> Result<Env> {
        let empty = empty()?;
        // OutputFlag = 0 between creation and start (VII-G, constitution III; T041).
        if let Err(err) = super::params::configure_new_environment(empty.as_ptr()) {
            // SAFETY: `empty` came from `empty()`, is owned here, its error message has been
            // read into `err`, and it is freed exactly once.
            unsafe { ffi::GRBfreeenv(empty.as_ptr()) };
            return Err(err);
        }
        let raw = start(empty)?;
        let (major, minor, tech) = library_version();
        Ok(Env {
            raw,
            version: (major, minor, tech),
        })
    }

    /// Raw pointer to the environment, valid while `self` is alive.
    #[allow(dead_code)] // Used by `gurobi::solve` (task T045).
    pub(crate) fn raw(&self) -> *mut ffi::GRBenv {
        self.raw.as_ptr()
    }

    /// Gurobi library version (major, minor, technical) reported by `GRBversion`.
    pub fn version(&self) -> (i32, i32, i32) {
        self.version
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        // SAFETY: `self.raw` was created by Gurobi, is exclusively owned by this `Env`, and is
        // freed exactly once here. Models borrow the `Env` and therefore are dropped first
        // (VII-B), as GRBfreeenv requires.
        unsafe { ffi::GRBfreeenv(self.raw.as_ptr()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_version_matches_declared_major_minor() {
        let (major, minor, _) = library_version();
        let declared = env!("OPTSOLVER_GUROBI_DECLARED_VERSION");
        let mut parts = declared.split('.');
        assert_eq!(parts.next().and_then(|s| s.parse().ok()), Some(major));
        assert_eq!(parts.next().and_then(|s| s.parse().ok()), Some(minor));
    }

    #[test]
    fn empty_environment_is_created_and_freed_without_licence() {
        let raw = empty().expect("GRBemptyenv needs no licence");
        // SAFETY: `raw` comes from `empty`, is owned here and freed exactly once.
        unsafe { ffi::GRBfreeenv(raw.as_ptr()) };
    }

    #[test]
    fn new_returns_env_or_solver_error() {
        match Env::new() {
            Ok(env) => assert_eq!(env.version().0, library_version().0),
            Err(Error::Solver { code, message }) => {
                assert_ne!(code, 0);
                assert!(!message.is_empty(), "error message read before freeing");
            }
            Err(other) => panic!("unexpected error kind: {other:?}"),
        }
    }
}
