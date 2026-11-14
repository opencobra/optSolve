//! The options structure (constitution XVI-C) and the single declaration of every default
//! optSolver imposes (constitution VII-G). Fields left unset request default behaviour: the
//! solver's own defaults, modified only by the overrides in [`defaults`].

#![forbid(unsafe_code)]

use std::sync::atomic::AtomicBool;

/// Every default optSolver imposes on a solver, each with its rationale (VII-G). Nothing else
/// in the crate may impose a default.
pub mod defaults {
    /// Solver log off unless requested. Rationale: constitution III (library code does not
    /// print; solver logs off by default).
    pub const SOLVER_LOG: bool = false;
    /// Array outputs (duals, reduced costs, slacks, basis) returned unless switched off.
    /// Rationale: constitution I (all available outputs returned by default).
    pub const ARRAY_OUTPUTS: bool = true;
    /// Independent verification on unless switched off. Rationale: spec FR-012.
    pub const VERIFICATION: bool = true;
    /// Factor applied to the effective solver tolerance when judging a verification measure.
    /// Rationale: spec clarification 2 (10×).
    pub const VERIFICATION_FACTOR: f64 = 10.0;
}

/// LP algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Solver's choice.
    Auto,
    /// Primal simplex.
    PrimalSimplex,
    /// Dual simplex.
    DualSimplex,
    /// Barrier (interior point).
    Barrier,
    /// Several methods concurrently.
    Concurrent,
}

/// Presolve level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presolve {
    /// Solver's choice.
    Auto,
    /// No presolve.
    Off,
    /// Conservative presolve.
    Conservative,
    /// Aggressive presolve.
    Aggressive,
}

/// Value of a raw vendor parameter.
#[derive(Debug, Clone, PartialEq)]
pub enum ParamValue {
    /// Integer.
    Int(i64),
    /// Double.
    Double(f64),
    /// String.
    Str(String),
}

/// Phase of the algorithm reporting progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressPhase {
    /// Simplex iterations.
    Simplex,
    /// Barrier iterations.
    Barrier,
    /// Other or unknown.
    Other,
}

/// Solver-independent progress record passed to the progress callback.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progress {
    /// Seconds since the solve started.
    pub elapsed: f64,
    /// Current primal objective, when known.
    pub primal_objective: Option<f64>,
    /// Current dual objective or bound, when known.
    pub dual_objective: Option<f64>,
    /// Iterations so far, when known.
    pub iterations: Option<f64>,
    /// Phase.
    pub phase: ProgressPhase,
}

/// Non-default solver behaviour. Every field is optional; `Options::default()` requests
/// default behaviour throughout.
#[derive(Default)]
#[non_exhaustive]
pub struct Options<'cb> {
    /// Wall-clock seconds the solver may spend optimising.
    pub time_limit: Option<f64>,
    /// Maximum simplex iterations.
    pub iteration_limit: Option<f64>,
    /// Maximum solver threads.
    pub threads: Option<u32>,
    /// Maximum violation of row and variable bounds the solver may accept.
    pub primal_feasibility_tolerance: Option<f64>,
    /// Maximum violation of reduced-cost sign conditions the solver may accept.
    pub dual_feasibility_tolerance: Option<f64>,
    /// LP algorithm.
    pub method: Option<Method>,
    /// Presolve level.
    pub presolve: Option<Presolve>,
    /// Solver random seed.
    pub random_seed: Option<i32>,
    /// Whether the solver's own log is produced (default [`defaults::SOLVER_LOG`]).
    pub solver_log: Option<bool>,
    /// Return infeasibility and unboundedness certificates.
    pub certificates: bool,
    /// Return array outputs (default [`defaults::ARRAY_OUTPUTS`]).
    pub array_outputs: Option<bool>,
    /// Run independent verification (default [`defaults::VERIFICATION`]).
    pub verification: Option<bool>,
    /// Raw vendor parameters: (vendor name, value).
    pub raw: Vec<(String, ParamValue)>,
    /// Setting this flag interrupts the solve.
    pub cancel: Option<&'cb AtomicBool>,
    /// Called with progress records during the solve.
    pub progress: Option<&'cb mut dyn FnMut(&Progress)>,
}

impl std::fmt::Debug for Options<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Options")
            .field("time_limit", &self.time_limit)
            .field("iteration_limit", &self.iteration_limit)
            .field("threads", &self.threads)
            .field(
                "primal_feasibility_tolerance",
                &self.primal_feasibility_tolerance,
            )
            .field(
                "dual_feasibility_tolerance",
                &self.dual_feasibility_tolerance,
            )
            .field("method", &self.method)
            .field("presolve", &self.presolve)
            .field("random_seed", &self.random_seed)
            .field("solver_log", &self.solver_log)
            .field("certificates", &self.certificates)
            .field("array_outputs", &self.array_outputs)
            .field("verification", &self.verification)
            .field("raw", &self.raw)
            .field("cancel", &self.cancel.is_some())
            .field("progress", &self.progress.is_some())
            .finish()
    }
}

impl Options<'_> {
    /// Effective solver-log setting.
    pub fn effective_solver_log(&self) -> bool {
        self.solver_log.unwrap_or(defaults::SOLVER_LOG)
    }
    /// Effective array-outputs setting.
    pub fn effective_array_outputs(&self) -> bool {
        self.array_outputs.unwrap_or(defaults::ARRAY_OUTPUTS)
    }
    /// Effective verification setting.
    pub fn effective_verification(&self) -> bool {
        self.verification.unwrap_or(defaults::VERIFICATION)
    }
    /// Names of the shared fields that are set, as used in
    /// [`crate::backend::OptionMapping::field`].
    pub fn set_shared_fields(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.time_limit.is_some() {
            v.push("time_limit");
        }
        if self.iteration_limit.is_some() {
            v.push("iteration_limit");
        }
        if self.threads.is_some() {
            v.push("threads");
        }
        if self.primal_feasibility_tolerance.is_some() {
            v.push("primal_feasibility_tolerance");
        }
        if self.dual_feasibility_tolerance.is_some() {
            v.push("dual_feasibility_tolerance");
        }
        if self.method.is_some() {
            v.push("method");
        }
        if self.presolve.is_some() {
            v.push("presolve");
        }
        if self.random_seed.is_some() {
            v.push("random_seed");
        }
        if self.solver_log.is_some() {
            v.push("solver_log");
        }
        if self.certificates {
            v.push("certificates");
        }
        if self.cancel.is_some() {
            v.push("cancel");
        }
        if self.progress.is_some() {
            v.push("progress");
        }
        v
    }
}

pub mod check;
