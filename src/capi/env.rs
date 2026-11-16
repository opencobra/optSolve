//! Environment handles and capabilities (feature 005; FR-007, FR-008, FR-017).

use core::ffi::c_char;
use std::ffi::CString;

use super::*;
use crate::backend::{Backend, Capabilities};
use crate::error::{Error, Result};

/// The backend environment behind an `optsolver_env` handle.
pub(crate) enum Inner {
    #[cfg(feature = "gurobi")]
    Gurobi(crate::gurobi::Env),
    #[cfg(feature = "mosek")]
    Mosek(crate::mosek::Env),
    /// Keeps the enum valid when no backend is compiled in.
    #[allow(dead_code)]
    Never(core::convert::Infallible),
}

impl Inner {
    pub(crate) fn backend(&self) -> &dyn Backend {
        match self {
            #[cfg(feature = "gurobi")]
            Inner::Gurobi(e) => e,
            #[cfg(feature = "mosek")]
            Inner::Mosek(e) => e,
            Inner::Never(n) => match *n {},
        }
    }
}

/// An opaque environment handle (one backend environment, licence acquired once).
#[allow(non_camel_case_types)] // C type name (header).
pub struct optsolver_env {
    pub(crate) inner: Inner,
}

fn backend_name(backend: i32) -> Result<&'static str> {
    match backend {
        OPTSOLVER_BACKEND_GUROBI => Ok("gurobi"),
        OPTSOLVER_BACKEND_MOSEK => Ok("mosek"),
        other => Err(invalid("backend", format!("unknown backend code {other}"))),
    }
}

/// Whether `backend` is compiled into this library.
fn compiled(backend: i32) -> bool {
    (backend == OPTSOLVER_BACKEND_GUROBI && cfg!(feature = "gurobi"))
        || (backend == OPTSOLVER_BACKEND_MOSEK && cfg!(feature = "mosek"))
}

/// Creates the environment of `backend` (counted for the reuse test).
pub(crate) fn create(backend: i32) -> Result<Inner> {
    let name = backend_name(backend)?;
    if !compiled(backend) {
        return Err(Error::Unsupported {
            field: "backend",
            backend: name,
        });
    }
    let inner: Result<Inner> = match backend {
        #[cfg(feature = "gurobi")]
        OPTSOLVER_BACKEND_GUROBI => crate::gurobi::Env::new().map(Inner::Gurobi),
        #[cfg(feature = "mosek")]
        OPTSOLVER_BACKEND_MOSEK => crate::mosek::Env::new().map(Inner::Mosek),
        _ => Err(Error::Unsupported {
            field: "backend",
            backend: name,
        }),
    };
    if inner.is_ok() {
        count_environment();
    }
    inner
}

/// Creates an environment handle for `backend` (`OPTSOLVER_BACKEND_*`), acquiring the licence
/// once. Free it with [`optsolver_env_free`]. The handle may be used from any thread, by one
/// thread at a time.
///
/// # Safety
/// `out` must be a writable pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_env_create(backend: i32, out: *mut *mut optsolver_env) -> i32 {
    guard(|| {
        if out.is_null() {
            return Err(invalid("out", "NULL"));
        }
        let env = Box::new(optsolver_env {
            inner: create(backend)?,
        });
        // SAFETY: `out` is non-null and writable by the contract.
        unsafe { out.write(Box::into_raw(env)) };
        Ok(())
    })
}

/// Frees an environment handle. NULL is a no-op.
///
/// # Safety
/// `env` must be NULL or a handle from [`optsolver_env_create`], not yet freed and not in use on
/// another thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_env_free(env: *mut optsolver_env) {
    contain((), || {
        if !env.is_null() {
            // SAFETY: by the contract `env` came from Box::into_raw and is freed once.
            drop(unsafe { Box::from_raw(env) });
        }
    })
}

/// 1 if `backend` is compiled into this library, else 0.
#[unsafe(no_mangle)]
pub extern "C" fn optsolver_backend_compiled(backend: i32) -> i32 {
    contain(0, || i32::from(compiled(backend)))
}

/// What a backend supports, and whether it is usable.
#[allow(non_camel_case_types)] // C type name (header).
pub struct optsolver_caps {
    usable: bool,
    reason: CString,
    classes: u32,
    cones: u32,
    certificates: bool,
    basis: bool,
    options: Vec<CString>,
    solver_name: CString,
    solver_version: CString,
}

fn cstring(s: &str) -> CString {
    CString::new(s.replace('\0', " ")).unwrap_or_default()
}

fn caps_of(c: &Capabilities) -> optsolver_caps {
    optsolver_caps {
        usable: true,
        reason: CString::default(),
        classes: c
            .problem_classes
            .iter()
            .map(|&k| class_bit(k))
            .fold(0, |a, b| a | b),
        cones: c
            .cones
            .iter()
            .map(|&k| 1u32 << cone_code(k))
            .fold(0, |a, b| a | b),
        certificates: c.certificates,
        basis: c.basis,
        options: c.options.iter().map(|o| cstring(o.field)).collect(),
        solver_name: cstring(&c.solver.name),
        solver_version: cstring(&c.solver.version),
    }
}

/// Capabilities of `backend`, creating and freeing a temporary environment. A compiled-in
/// backend whose environment cannot be created gives `OPTSOLVER_OK` and a not-usable object whose
/// reason is the error message; a backend not compiled in gives `OPTSOLVER_ERR_UNSUPPORTED`.
///
/// # Safety
/// `out` must be a writable pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_capabilities(
    backend: i32,
    out: *mut *mut optsolver_caps,
) -> i32 {
    guard(|| {
        if out.is_null() {
            return Err(invalid("out", "NULL"));
        }
        let name = backend_name(backend)?;
        if !compiled(backend) {
            return Err(Error::Unsupported {
                field: "backend",
                backend: name,
            });
        }
        let caps = match create(backend) {
            Ok(inner) => caps_of(&inner.backend().capabilities()),
            Err(e) => optsolver_caps {
                usable: false,
                reason: cstring(&e.to_string()),
                classes: 0,
                cones: 0,
                certificates: false,
                basis: false,
                options: Vec::new(),
                solver_name: cstring(name),
                solver_version: CString::default(),
            },
        };
        // SAFETY: `out` is non-null and writable by the contract.
        unsafe { out.write(Box::into_raw(Box::new(caps))) };
        Ok(())
    })
}

/// Capabilities of an existing environment (no new environment is created).
///
/// # Safety
/// `env` must be a live handle; `out` a writable pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_env_capabilities(
    env: *const optsolver_env,
    out: *mut *mut optsolver_caps,
) -> i32 {
    guard(|| {
        if env.is_null() || out.is_null() {
            return Err(invalid(if env.is_null() { "env" } else { "out" }, "NULL"));
        }
        // SAFETY: a live handle by the contract.
        let caps = caps_of(&unsafe { &*env }.inner.backend().capabilities());
        // SAFETY: `out` is non-null and writable by the contract.
        unsafe { out.write(Box::into_raw(Box::new(caps))) };
        Ok(())
    })
}

/// Reads a field of a capabilities object (NULL gives `default`).
fn with_caps<T>(c: *const optsolver_caps, default: T, f: impl FnOnce(&optsolver_caps) -> T) -> T {
    if c.is_null() {
        return default;
    }
    // SAFETY: a live capabilities object by the accessor's contract.
    f(unsafe { &*c })
}

/// 1 if the backend is usable.
///
/// # Safety
/// `caps` must be NULL or a live capabilities object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_caps_usable(caps: *const optsolver_caps) -> i32 {
    contain(0, || with_caps(caps, 0, |c| i32::from(c.usable)))
}

/// Why the backend is not usable (`""` when usable).
///
/// # Safety
/// `caps` must be NULL or a live capabilities object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_caps_reason(caps: *const optsolver_caps) -> *const c_char {
    contain(c"".as_ptr(), || {
        with_caps(caps, c"".as_ptr(), |c| c.reason.as_ptr())
    })
}

/// Problem classes (`OPTSOLVER_CLASS_*` bits).
///
/// # Safety
/// `caps` must be NULL or a live capabilities object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_caps_problem_classes(caps: *const optsolver_caps) -> u32 {
    contain(0, || with_caps(caps, 0, |c| c.classes))
}

/// Cone kinds, bit `1 << kind` per accepted kind.
///
/// # Safety
/// `caps` must be NULL or a live capabilities object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_caps_cones(caps: *const optsolver_caps) -> u32 {
    contain(0, || with_caps(caps, 0, |c| c.cones))
}

/// 1 if certificates are returned on request.
///
/// # Safety
/// `caps` must be NULL or a live capabilities object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_caps_certificates(caps: *const optsolver_caps) -> i32 {
    contain(0, || with_caps(caps, 0, |c| i32::from(c.certificates)))
}

/// 1 if a basis is returned.
///
/// # Safety
/// `caps` must be NULL or a live capabilities object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_caps_basis(caps: *const optsolver_caps) -> i32 {
    contain(0, || with_caps(caps, 0, |c| i32::from(c.basis)))
}

/// Number of shared options fields honoured.
///
/// # Safety
/// `caps` must be NULL or a live capabilities object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_caps_option_count(caps: *const optsolver_caps) -> usize {
    contain(0, || with_caps(caps, 0, |c| c.options.len()))
}

/// Name of honoured option `i` (NULL when out of range).
///
/// # Safety
/// `caps` must be NULL or a live capabilities object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_caps_option(
    caps: *const optsolver_caps,
    i: usize,
) -> *const c_char {
    contain(core::ptr::null(), || {
        with_caps(caps, core::ptr::null(), |c| {
            c.options.get(i).map_or(core::ptr::null(), |s| s.as_ptr())
        })
    })
}

/// Solver name.
///
/// # Safety
/// `caps` must be NULL or a live capabilities object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_caps_solver_name(caps: *const optsolver_caps) -> *const c_char {
    contain(c"".as_ptr(), || {
        with_caps(caps, c"".as_ptr(), |c| c.solver_name.as_ptr())
    })
}

/// Solver version (`""` when not usable).
///
/// # Safety
/// `caps` must be NULL or a live capabilities object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_caps_solver_version(
    caps: *const optsolver_caps,
) -> *const c_char {
    contain(c"".as_ptr(), || {
        with_caps(caps, c"".as_ptr(), |c| c.solver_version.as_ptr())
    })
}

/// Frees a capabilities object. NULL is a no-op.
///
/// # Safety
/// `caps` must be NULL or an object from the library, not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn optsolver_caps_free(caps: *mut optsolver_caps) {
    contain((), || {
        if !caps.is_null() {
            // SAFETY: by the contract it came from Box::into_raw and is freed once.
            drop(unsafe { Box::from_raw(caps) });
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_and_not_compiled_backends() {
        let mut out: *mut optsolver_env = core::ptr::null_mut();
        // SAFETY: writable out-pointer.
        let rc = unsafe { optsolver_env_create(9, &mut out) };
        assert_eq!(rc, OPTSOLVER_ERR_INVALID);
        for b in [OPTSOLVER_BACKEND_GUROBI, OPTSOLVER_BACKEND_MOSEK] {
            if !compiled(b) {
                // SAFETY: writable out-pointer.
                let rc = unsafe { optsolver_env_create(b, &mut out) };
                assert_eq!(rc, OPTSOLVER_ERR_UNSUPPORTED);
                let mut c: *mut optsolver_caps = core::ptr::null_mut();
                // SAFETY: writable out-pointer.
                let rc = unsafe { optsolver_capabilities(b, &mut c) };
                assert_eq!(rc, OPTSOLVER_ERR_UNSUPPORTED);
                assert_eq!(optsolver_backend_compiled(b), 0);
            } else {
                assert_eq!(optsolver_backend_compiled(b), 1);
            }
        }
        // SAFETY: NULL is a no-op.
        unsafe { optsolver_env_free(core::ptr::null_mut()) };
        // SAFETY: NULL is a no-op.
        unsafe { optsolver_caps_free(core::ptr::null_mut()) };
    }
}
