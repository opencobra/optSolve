//! Checks shared by every backend before solving: unsupported fields and parameters set by
//! two routes (constitution XVI-C; FR-008), and problem features outside the backend's
//! capabilities (FR-013). Implemented by task T028.

#![forbid(unsafe_code)]
#![allow(non_snake_case)] // Notation Convention (XII-C): Q is a matrix.

use crate::backend::{Capabilities, ProblemClass, ProblemRef};
use crate::error::{Error, Result};
use crate::options::Options;

/// Rejects options the backend cannot honour, and vendor parameters set by two routes.
///
/// - Every shared field that is set (see [`Options::set_shared_fields`]) must appear in
///   `capabilities.options`; `certificates` additionally needs `capabilities.certificates`.
///   Otherwise [`Error::Unsupported`] names the field and the backend.
/// - A raw parameter whose vendor name is also set by a set shared field (through its
///   [`crate::backend::OptionMapping::vendor_parameters`]) is [`Error::ParameterConflict`].
///   Names are compared ignoring ASCII case, since solver parameter names are (Gurobi).
pub fn check(options: &Options<'_>, capabilities: &Capabilities) -> Result<()> {
    let set = options.set_shared_fields();
    for &field in &set {
        let supported = capabilities.options.iter().any(|o| o.field == field)
            && (field != "certificates" || capabilities.certificates);
        if !supported {
            return Err(Error::Unsupported {
                field,
                backend: capabilities.backend,
            });
        }
    }
    for (name, _) in &options.raw {
        let conflict = capabilities
            .options
            .iter()
            .filter(|o| set.contains(&o.field))
            .flat_map(|o| o.vendor_parameters.iter())
            .any(|p| p.eq_ignore_ascii_case(name));
        if conflict {
            return Err(Error::ParameterConflict { name: name.clone() });
        }
    }
    Ok(())
}

/// Rejects a problem the backend cannot solve, before any solver call (FR-013): variable
/// types need [`ProblemClass::Milp`], Q needs [`ProblemClass::Qp`], conic constraints need
/// [`ProblemClass::Conic`], and a problem with none of these needs [`ProblemClass::Lp`]. The
/// error is [`Error::Unsupported`] naming the field (`"var_types"`, `"Q"`, `"conic"` or
/// `"LP"`) and the backend.
pub fn check_problem(problem: ProblemRef<'_, '_>, capabilities: &Capabilities) -> Result<()> {
    let (has_var_types, has_Q, has_conic) = match problem {
        ProblemRef::Usize(p) => (
            p.var_types().is_some(),
            p.Q().is_some(),
            p.conic().is_some(),
        ),
        ProblemRef::I32(p) => (
            p.var_types().is_some(),
            p.Q().is_some(),
            p.conic().is_some(),
        ),
    };
    let supports = |c: ProblemClass| capabilities.problem_classes.contains(&c);
    let unsupported = |field| {
        Err(Error::Unsupported {
            field,
            backend: capabilities.backend,
        })
    };
    if has_var_types && !supports(ProblemClass::Milp) {
        return unsupported("var_types");
    }
    if has_Q && !supports(ProblemClass::Qp) {
        return unsupported("Q");
    }
    if has_conic && !supports(ProblemClass::Conic) {
        return unsupported("conic");
    }
    if !has_var_types && !has_Q && !has_conic && !supports(ProblemClass::Lp) {
        return unsupported("LP");
    }
    Ok(())
}
