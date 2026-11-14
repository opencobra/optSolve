//! Translation of Gurobi status codes into the solver-independent status (task T043;
//! research R4; FR-017).
//!
//! The codes are the header's `GRB_LOADED` (1) to `GRB_LOCALLY_INFEASIBLE` (19), also declared
//! in `gurobi::ffi` and checked against the installed header by the header conformance test.
//! The translation is total: any other code maps to [`Status::Unknown`]; the caller keeps the
//! raw code in the result (constitution III).

#![forbid(unsafe_code)]

use super::ffi;
use crate::result::{LimitReason, SolutionReturned, Status};

/// Translates a Gurobi `Status` attribute value into the solver-independent status and, for
/// [`Status::StoppedAtLimit`], the limit that stopped the solver (research R4, exactly).
pub fn translate(code: i32) -> (Status, Option<LimitReason>) {
    match code {
        ffi::GRB_LOADED => (Status::NotSolved, None),
        ffi::GRB_OPTIMAL => (Status::Optimal, None),
        ffi::GRB_INFEASIBLE => (Status::Infeasible, None),
        ffi::GRB_INF_OR_UNBD => (Status::InfeasibleOrUnbounded, None),
        ffi::GRB_UNBOUNDED => (Status::Unbounded, None),
        ffi::GRB_CUTOFF => (Status::StoppedAtLimit, Some(LimitReason::ObjectiveCutoff)),
        ffi::GRB_ITERATION_LIMIT => (Status::StoppedAtLimit, Some(LimitReason::Iterations)),
        ffi::GRB_NODE_LIMIT => (Status::StoppedAtLimit, Some(LimitReason::Nodes)),
        ffi::GRB_TIME_LIMIT => (Status::StoppedAtLimit, Some(LimitReason::Time)),
        ffi::GRB_SOLUTION_LIMIT => (Status::StoppedAtLimit, Some(LimitReason::Solutions)),
        ffi::GRB_INTERRUPTED => (Status::Interrupted, None),
        ffi::GRB_NUMERIC => (Status::NumericalDifficulty, None),
        ffi::GRB_SUBOPTIMAL => (Status::NumericalDifficulty, None),
        ffi::GRB_INPROGRESS => (Status::NotSolved, None),
        ffi::GRB_USER_OBJ_LIMIT => (Status::StoppedAtLimit, Some(LimitReason::ObjectiveTarget)),
        ffi::GRB_WORK_LIMIT => (Status::StoppedAtLimit, Some(LimitReason::Work)),
        ffi::GRB_MEM_LIMIT => (Status::StoppedAtLimit, Some(LimitReason::Memory)),
        ffi::GRB_LOCALLY_OPTIMAL => (Status::LocallyOptimal, None),
        ffi::GRB_LOCALLY_INFEASIBLE => (Status::LocallyInfeasible, None),
        _ => (Status::Unknown, None),
    }
}

/// What solution is returned, from the translated status and Gurobi's `SolCount` attribute
/// (research R4): Optimal when the status is Optimal; otherwise Feasible when `SolCount` > 0;
/// None when `SolCount` = 0.
pub fn solution_returned(status: Status, sol_count: i32) -> SolutionReturned {
    if status == Status::Optimal {
        SolutionReturned::Optimal
    } else if sol_count > 0 {
        SolutionReturned::Feasible
    } else {
        SolutionReturned::None
    }
}

/// Backend-specific tests (task T036; SC-006). Status translation is a Gurobi detail outside
/// the solver-independent contract, so these tests live here rather than in the shared
/// contract harness (constitution II). They need no licence and no Gurobi library call.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_documented_code_translates_per_research_r4() {
        use LimitReason as L;
        use Status as S;
        let expected: [(i32, S, Option<L>); 19] = [
            (ffi::GRB_LOADED, S::NotSolved, None),
            (ffi::GRB_OPTIMAL, S::Optimal, None),
            (ffi::GRB_INFEASIBLE, S::Infeasible, None),
            (ffi::GRB_INF_OR_UNBD, S::InfeasibleOrUnbounded, None),
            (ffi::GRB_UNBOUNDED, S::Unbounded, None),
            (ffi::GRB_CUTOFF, S::StoppedAtLimit, Some(L::ObjectiveCutoff)),
            (
                ffi::GRB_ITERATION_LIMIT,
                S::StoppedAtLimit,
                Some(L::Iterations),
            ),
            (ffi::GRB_NODE_LIMIT, S::StoppedAtLimit, Some(L::Nodes)),
            (ffi::GRB_TIME_LIMIT, S::StoppedAtLimit, Some(L::Time)),
            (
                ffi::GRB_SOLUTION_LIMIT,
                S::StoppedAtLimit,
                Some(L::Solutions),
            ),
            (ffi::GRB_INTERRUPTED, S::Interrupted, None),
            (ffi::GRB_NUMERIC, S::NumericalDifficulty, None),
            (ffi::GRB_SUBOPTIMAL, S::NumericalDifficulty, None),
            (ffi::GRB_INPROGRESS, S::NotSolved, None),
            (
                ffi::GRB_USER_OBJ_LIMIT,
                S::StoppedAtLimit,
                Some(L::ObjectiveTarget),
            ),
            (ffi::GRB_WORK_LIMIT, S::StoppedAtLimit, Some(L::Work)),
            (ffi::GRB_MEM_LIMIT, S::StoppedAtLimit, Some(L::Memory)),
            (ffi::GRB_LOCALLY_OPTIMAL, S::LocallyOptimal, None),
            (ffi::GRB_LOCALLY_INFEASIBLE, S::LocallyInfeasible, None),
        ];
        for (i, (code, status, limit)) in expected.into_iter().enumerate() {
            assert_eq!(
                code,
                i32::try_from(i + 1).unwrap(),
                "table covers codes 1-19 in order"
            );
            assert_eq!(
                translate(code),
                (status, limit),
                "Gurobi status code {code}"
            );
        }
    }

    #[test]
    fn limit_reason_present_exactly_for_stopped_at_limit() {
        for code in 1..=19 {
            let (status, limit) = translate(code);
            assert_eq!(
                status == Status::StoppedAtLimit,
                limit.is_some(),
                "code {code}"
            );
        }
    }

    #[test]
    fn unknown_codes_translate_to_unknown() {
        for code in [99, 0, -1, 20, i32::MIN, i32::MAX] {
            assert_eq!(translate(code), (Status::Unknown, None), "code {code}");
        }
    }

    #[test]
    fn solution_returned_follows_status_and_solcount() {
        assert_eq!(
            solution_returned(Status::Optimal, 1),
            SolutionReturned::Optimal
        );
        assert_eq!(
            solution_returned(Status::Optimal, 0),
            SolutionReturned::Optimal
        );
        let (suboptimal, _) = translate(ffi::GRB_SUBOPTIMAL);
        assert_eq!(solution_returned(suboptimal, 1), SolutionReturned::Feasible);
        let (time_limit, _) = translate(ffi::GRB_TIME_LIMIT);
        assert_eq!(solution_returned(time_limit, 3), SolutionReturned::Feasible);
        assert_eq!(solution_returned(time_limit, 0), SolutionReturned::None);
        assert_eq!(
            solution_returned(Status::Infeasible, 0),
            SolutionReturned::None
        );
        assert_eq!(
            solution_returned(Status::Unknown, 0),
            SolutionReturned::None
        );
    }
}
