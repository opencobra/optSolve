//! A test backend with fixed capabilities (task T024). It solves nothing: after the shared
//! problem and options checks every backend runs, it returns `Status::NotSolved`.

use optsolver::backend::{Backend, Capabilities, OptionMapping, ProblemClass, ProblemRef};
use optsolver::options::check::{check, check_problem};
use optsolver::{Options, Solution, SolutionBuffers, SolverIdentity, Status};

/// A backend whose capabilities are whatever it was built with.
pub struct FakeBackend {
    pub caps: Capabilities,
}

impl FakeBackend {
    /// LP only; honours `time_limit` (vendor `TimeLimit`) and `threads` (vendor `Threads`).
    pub fn lp_only() -> Self {
        Self {
            caps: Capabilities {
                backend: "fake",
                problem_classes: vec![ProblemClass::Lp],
                cones: Vec::new(),
                options: vec![
                    OptionMapping {
                        field: "time_limit",
                        vendor_parameters: &["TimeLimit"],
                    },
                    OptionMapping {
                        field: "threads",
                        vendor_parameters: &["Threads"],
                    },
                ],
                certificates: false,
                basis: false,
                solver: SolverIdentity {
                    name: "fake".into(),
                    version: "0".into(),
                },
            },
        }
    }
}

impl Backend for FakeBackend {
    fn capabilities(&self) -> Capabilities {
        self.caps.clone()
    }

    fn solve(
        &self,
        problem: ProblemRef<'_, '_>,
        options: &mut Options<'_>,
    ) -> optsolver::error::Result<Solution> {
        check_problem(problem, &self.caps)?;
        check(options, &self.caps)?;
        Ok(Solution::empty(Status::NotSolved, self.caps.solver.clone()))
    }

    fn solve_into(
        &self,
        problem: ProblemRef<'_, '_>,
        options: &mut Options<'_>,
        _out: SolutionBuffers<'_>,
    ) -> optsolver::error::Result<Solution> {
        self.solve(problem, options)
    }
}
