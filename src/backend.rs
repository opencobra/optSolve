//! The provisional `Backend` trait (ADR-0001): one call shape every solver implements, so
//! tests are written once (constitution II). Re-examined when a second backend exists.

#![forbid(unsafe_code)]

use crate::error::Result;
use crate::options::Options;
use crate::problem::Problem;
use crate::result::{Solution, SolutionBuffers, SolverIdentity};

/// A problem with either index type, so that [`Backend`] is object-safe.
#[derive(Debug, Clone, Copy)]
pub enum ProblemRef<'p, 'a> {
    /// 64-bit row indices.
    Usize(&'p Problem<'a, usize>),
    /// 32-bit row indices.
    I32(&'p Problem<'a, i32>),
}

impl<'p, 'a> From<&'p Problem<'a, usize>> for ProblemRef<'p, 'a> {
    fn from(p: &'p Problem<'a, usize>) -> Self {
        ProblemRef::Usize(p)
    }
}

impl<'p, 'a> From<&'p Problem<'a, i32>> for ProblemRef<'p, 'a> {
    fn from(p: &'p Problem<'a, i32>) -> Self {
        ProblemRef::I32(p)
    }
}

/// Problem classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProblemClass {
    /// Linear programme.
    Lp,
    /// Mixed-integer linear programme.
    Milp,
    /// Convex quadratic programme.
    Qp,
    /// Conic linear programme (affine conic constraints Fx + g ∈ K).
    Conic,
}

/// How a shared options field maps to a vendor parameter in one backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionMapping {
    /// Options field name (as in [`Options::set_shared_fields`]).
    pub field: &'static str,
    /// Vendor parameter(s) it sets; empty when it sets none (for example a callback).
    pub vendor_parameters: &'static [&'static str],
}

/// What a backend supports.
#[derive(Debug, Clone, PartialEq)]
pub struct Capabilities {
    /// Backend name, for example `"gurobi"`.
    pub backend: &'static str,
    /// Problem classes it solves.
    pub problem_classes: Vec<ProblemClass>,
    /// Cone kinds it accepts in conic problems (empty without [`ProblemClass::Conic`]).
    pub cones: Vec<crate::conic::ConeKind>,
    /// Shared options fields it honours, with their vendor mapping.
    pub options: Vec<OptionMapping>,
    /// Whether it returns certificates on request.
    pub certificates: bool,
    /// Whether it returns a basis.
    pub basis: bool,
    /// Solver identity.
    pub solver: SolverIdentity,
}

/// A solver backend.
pub trait Backend {
    /// What this backend supports.
    fn capabilities(&self) -> Capabilities;
    /// Solves and returns owned results.
    fn solve(&self, problem: ProblemRef<'_, '_>, options: &mut Options<'_>) -> Result<Solution>;
    /// Solves, writing x (and y, z, row activity when given) into caller buffers; the
    /// returned [`Solution`] has those arrays empty.
    fn solve_into(
        &self,
        problem: ProblemRef<'_, '_>,
        options: &mut Options<'_>,
        out: SolutionBuffers<'_>,
    ) -> Result<Solution>;
}
