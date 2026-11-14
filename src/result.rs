//! The solver-independent result (data-model.md, Result and Status vocabulary;
//! constitution XVI-A, XVI-B).

#![forbid(unsafe_code)]

use crate::options::ParamValue;
use crate::verify::VerificationReport;

/// Solver-independent status: why the solver stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Status {
    /// Optimality proven within the solver's tolerances.
    Optimal,
    /// Infeasibility proven.
    Infeasible,
    /// Unboundedness proven.
    Unbounded,
    /// One of infeasible or unbounded, not determined which.
    InfeasibleOrUnbounded,
    /// Stopped at a limit; see [`Solution::limit_reason`].
    StoppedAtLimit,
    /// Numerical trouble, or tolerances could not be met.
    NumericalDifficulty,
    /// Stopped by the cancellation flag or the progress callback.
    Interrupted,
    /// Local optimum (non-convex problem classes only).
    LocallyOptimal,
    /// Locally infeasible (non-convex problem classes only).
    LocallyInfeasible,
    /// No optimisation has completed.
    NotSolved,
    /// The solver reported a code the translation does not recognise.
    Unknown,
}

/// Which limit stopped the solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LimitReason {
    /// Time limit.
    Time,
    /// Iteration limit.
    Iterations,
    /// Branch-and-bound node limit.
    Nodes,
    /// Solution-count limit.
    Solutions,
    /// Objective cutoff.
    ObjectiveCutoff,
    /// Objective target reached.
    ObjectiveTarget,
    /// Work limit.
    Work,
    /// Memory limit.
    Memory,
}

/// What solution is returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolutionReturned {
    /// No solution.
    None,
    /// A feasible, not proven optimal, solution.
    Feasible,
    /// A solution proven optimal.
    Optimal,
}

/// Basis status of one variable or row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BasisStatus {
    /// Basic.
    Basic,
    /// Non-basic at its lower bound.
    AtLower,
    /// Non-basic at its upper bound.
    AtUpper,
    /// Non-basic, between bounds.
    Superbasic,
    /// Fixed (lower = upper).
    Fixed,
}

/// Basis, when the solver provides one.
#[derive(Debug, Clone, PartialEq)]
pub struct Basis {
    /// Per variable, length n.
    pub variables: Vec<BasisStatus>,
    /// Per row, length m.
    pub rows: Vec<BasisStatus>,
}

/// Iteration counts.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Counts {
    /// Simplex iterations.
    pub simplex_iterations: Option<f64>,
    /// Barrier iterations.
    pub barrier_iterations: Option<i64>,
}

/// Wall-clock seconds per phase.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Timings {
    /// Validation, conversion and loading.
    pub load: f64,
    /// Solver optimisation call.
    pub solve: f64,
    /// Result extraction.
    pub extraction: f64,
    /// Independent verification.
    pub verification: f64,
}

/// Certificate of infeasibility or unboundedness.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Certificate {
    /// Farkas certificate of infeasibility: row multipliers y_F (length m) and bound
    /// multipliers z_F (length n).
    Farkas {
        /// Row multipliers.
        y: Vec<f64>,
        /// Bound multipliers.
        z: Vec<f64>,
    },
    /// Ray r (length n) along which the objective is unbounded.
    UnboundedRay {
        /// The ray.
        r: Vec<f64>,
    },
}

/// Solver name and version.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SolverIdentity {
    /// Solver name, for example `"gurobi"`.
    pub name: String,
    /// Solver version, for example `"13.0.3"`.
    pub version: String,
}

/// The solver-independent result of one solve.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Solution {
    /// Why the solver stopped.
    pub status: Status,
    /// Present only when `status` is [`Status::StoppedAtLimit`].
    pub limit_reason: Option<LimitReason>,
    /// What solution is returned.
    pub returned: SolutionReturned,
    /// Primal solution x, length n (empty if none, or written to caller buffers).
    pub x: Vec<f64>,
    /// Row duals y, length m (empty if unavailable or switched off).
    pub y: Vec<f64>,
    /// Reduced costs z, length n (empty if unavailable or switched off).
    pub z: Vec<f64>,
    /// Row activity Ax as reported by the solver, length m (empty if unavailable).
    pub row_activity: Vec<f64>,
    /// Basis, when available.
    pub basis: Option<Basis>,
    /// Primal objective value cᵀx + γ, when defined.
    pub objective: Option<f64>,
    /// Best bound, when defined.
    pub bound: Option<f64>,
    /// Iteration counts.
    pub counts: Counts,
    /// Phase timings.
    pub timings: Timings,
    /// Certificate, when requested and available.
    pub certificate: Option<Certificate>,
    /// Every raw status code the solver reported, by name.
    pub raw_status: Vec<(String, i64)>,
    /// Solver name and version.
    pub solver: SolverIdentity,
    /// Extra solver attributes, by name.
    pub attributes: Vec<(String, ParamValue)>,
    /// Every parameter the backend set, with its value (constitution VII-G).
    pub effective_parameters: Vec<(String, ParamValue)>,
    /// Independent verification report; `None` only when verification is switched off.
    pub verification: Option<VerificationReport>,
}

impl Solution {
    /// A result with the given status and everything else empty.
    pub fn empty(status: Status, solver: SolverIdentity) -> Self {
        Self {
            status,
            limit_reason: None,
            returned: SolutionReturned::None,
            x: Vec::new(),
            y: Vec::new(),
            z: Vec::new(),
            row_activity: Vec::new(),
            basis: None,
            objective: None,
            bound: None,
            counts: Counts::default(),
            timings: Timings::default(),
            certificate: None,
            raw_status: Vec::new(),
            solver,
            attributes: Vec::new(),
            effective_parameters: Vec::new(),
            verification: None,
        }
    }
    /// Borrowed view for verification.
    pub fn view(&self) -> SolutionView<'_> {
        SolutionView {
            status: self.status,
            returned: self.returned,
            x: if self.x.is_empty() {
                None
            } else {
                Some(&self.x)
            },
            y: if self.y.is_empty() {
                None
            } else {
                Some(&self.y)
            },
            z: if self.z.is_empty() {
                None
            } else {
                Some(&self.z)
            },
            certificate: self.certificate.as_ref(),
        }
    }
}

/// Borrowed solution data used by [`crate::verify::verify`].
#[derive(Debug, Clone, Copy)]
pub struct SolutionView<'a> {
    /// Status.
    pub status: Status,
    /// What solution is returned.
    pub returned: SolutionReturned,
    /// Primal x.
    pub x: Option<&'a [f64]>,
    /// Row duals y.
    pub y: Option<&'a [f64]>,
    /// Reduced costs z.
    pub z: Option<&'a [f64]>,
    /// Certificate.
    pub certificate: Option<&'a Certificate>,
}

/// Caller-supplied output buffers for `solve_into` (lengths: x and z n, y and row_activity m).
#[derive(Debug)]
pub struct SolutionBuffers<'b> {
    /// Primal x, length n.
    pub x: &'b mut [f64],
    /// Row duals y, length m.
    pub y: Option<&'b mut [f64]>,
    /// Reduced costs z, length n.
    pub z: Option<&'b mut [f64]>,
    /// Row activity, length m.
    pub row_activity: Option<&'b mut [f64]>,
}
