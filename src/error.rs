//! Error type of the crate (constitution VII-C: a non-optimal outcome is a status, not an
//! error; errors mean the solver could not be run or the input was invalid).

#![forbid(unsafe_code)]

use std::fmt;

/// Every way a call into optSolver can fail.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// Input failed validation before any solver call.
    Invalid {
        /// Which input was invalid (for example `"A.row_indices"`).
        what: &'static str,
        /// What is wrong with it.
        detail: String,
    },
    /// A dimension exceeds what the supported solvers accept.
    TooLarge {
        /// Which dimension.
        what: &'static str,
        /// Its value.
        value: usize,
    },
    /// An options field or problem feature the backend cannot honour.
    Unsupported {
        /// The field or feature.
        field: &'static str,
        /// The backend.
        backend: &'static str,
    },
    /// The same vendor parameter was set by two routes in one call.
    ParameterConflict {
        /// The vendor parameter name.
        name: String,
    },
    /// The solver reported an error.
    Solver {
        /// The vendor's error code.
        code: i32,
        /// The vendor's error message.
        message: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Invalid { what, detail } => write!(f, "invalid {what}: {detail}"),
            Error::TooLarge { what, value } => {
                write!(f, "{what} = {value} exceeds the supported solvers' limit")
            }
            Error::Unsupported { field, backend } => {
                write!(f, "{field} is not supported by the {backend} backend")
            }
            Error::ParameterConflict { name } => {
                write!(
                    f,
                    "solver parameter {name} is set by two routes in one call"
                )
            }
            Error::Solver { code, message } => write!(f, "solver error {code}: {message}"),
        }
    }
}

impl std::error::Error for Error {}

/// Convenience alias.
pub type Result<T> = std::result::Result<T, Error>;
