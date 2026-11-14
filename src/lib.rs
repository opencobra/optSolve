//! optSolver: zero-copy loading of large matrix-form optimisation problems into native
//! solvers, behind one solver-independent contract (constitution XVI).
//!
//! The solver-independent core ([`problem`], [`options`], [`result`], [`verify`],
//! [`backend`], [`error`]) never depends on a backend. Backends are modules behind additive
//! Cargo features (ADR-0001).

#![deny(unsafe_code)]

// 64-bit targets only (constitution, Technical Constraints).
const _: () = assert!(
    core::mem::size_of::<usize>() == 8,
    "optsolver supports 64-bit targets only"
);

pub mod backend;
pub mod error;
pub mod options;
pub mod problem;
pub mod result;
pub mod verify;

#[cfg(feature = "gurobi")]
pub mod gurobi;

pub use backend::{Backend, Capabilities, OptionMapping, ProblemClass, ProblemRef};
pub use error::Error;
pub use options::{Method, Options, ParamValue, Presolve, Progress, ProgressPhase};
pub use problem::{CscRef, Problem, RowBounds, RowIndex, Sense};
pub use result::{
    Basis, BasisStatus, Certificate, Counts, LimitReason, Solution, SolutionBuffers,
    SolutionReturned, SolutionView, SolverIdentity, Status, Timings,
};
pub use verify::{Measure, Overall, Tolerances, Verdict, VerificationReport, verify};
