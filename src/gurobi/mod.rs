//! Gurobi backend (feature `gurobi`; ADR-0001). `ffi` holds every C declaration; `params`
//! is the only code that sets Gurobi parameters (constitution VII-A, VII-G).

#![allow(unsafe_code)] // FFI backend: every unsafe block carries a SAFETY comment (VII-A).

pub(crate) mod callback;
pub(crate) mod env;
pub(crate) mod ffi;
pub(crate) mod load;
pub(crate) mod load_qc;
pub(crate) mod params;
pub(crate) mod solve;
pub mod status;

pub use env::Env;
#[doc(hidden)]
pub use solve::LoadReport;
