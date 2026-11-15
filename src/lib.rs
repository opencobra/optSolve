#![doc = include_str!("../docs/llm-interface.md")]
// The crate documentation is docs/llm-interface.md, the single interface document for client
// authors and LLMs (feature 003, ADR-0003); its Rust examples are documentation tests and
// tests/standards.rs checks that it names every public item. Design: the solver-independent
// core (problem, conic, options, result, verify, backend, error) never depends on a backend;
// backends are modules behind additive Cargo features (ADR-0001, ADR-0002).
#![deny(unsafe_code)]

// 64-bit targets only (constitution, Technical Constraints).
const _: () = assert!(
    core::mem::size_of::<usize>() == 8,
    "optsolver supports 64-bit targets only"
);

pub mod backend;
pub mod conic;
pub mod error;
pub mod options;
pub mod problem;
pub mod result;
pub mod verify;

#[cfg(feature = "gurobi")]
pub mod gurobi;

#[cfg(feature = "mosek")]
pub mod mosek;

pub use backend::{Backend, Capabilities, OptionMapping, ProblemClass, ProblemRef};
pub use conic::{Cone, ConeKind, ConicRef};
pub use error::Error;
pub use options::{Method, Options, ParamValue, Presolve, Progress, ProgressPhase};
pub use problem::{CscRef, Problem, RowBounds, RowIndex, Sense};
pub use result::{
    Basis, BasisStatus, Certificate, Counts, LimitReason, Solution, SolutionBuffers,
    SolutionReturned, SolutionView, SolverIdentity, Status, Timings,
};
pub use verify::{Measure, Overall, Tolerances, Verdict, VerificationReport, verify};
