//! Translation of MOSEK termination code, problem status and solution status into the
//! solver-independent status (task T024; feature 002 research R6; spec FR-011).
//!
//! The translation is total: every combination maps to a [`Status`], unknown values to
//! [`Status::Unknown`]; the caller keeps the raw codes in the result (constitution III). A
//! response code that is an error (not `MSK_RES_OK` and not a termination code) never reaches
//! this module: the backend returns it as an error.

#![forbid(unsafe_code)]

use super::ffi;
use crate::result::{LimitReason, SolutionReturned, Status};

/// Translates (termination code of `MSK_optimizetrm`, problem status, solution status of the
/// solution read) per research R6, in the table's order: certificates and optimality from
/// the solution status first, then termination codes, then the problem status.
pub fn translate(trm: i32, prosta: i32, solsta: i32) -> (Status, Option<LimitReason>) {
    match solsta {
        ffi::MSK_SOL_STA_OPTIMAL => return (Status::Optimal, None),
        ffi::MSK_SOL_STA_PRIM_INFEAS_CER => return (Status::Infeasible, None),
        ffi::MSK_SOL_STA_DUAL_INFEAS_CER if prosta == ffi::MSK_PRO_STA_DUAL_INFEAS => {
            return (Status::Unbounded, None);
        }
        ffi::MSK_SOL_STA_DUAL_INFEAS_CER => return (Status::InfeasibleOrUnbounded, None),
        _ => {}
    }
    let limit = |r| (Status::StoppedAtLimit, Some(r));
    match trm {
        ffi::MSK_RES_TRM_USER_CALLBACK => return (Status::Interrupted, None),
        ffi::MSK_RES_TRM_MAX_TIME | ffi::MSK_RES_TRM_SERVER_MAX_TIME => {
            return limit(LimitReason::Time);
        }
        ffi::MSK_RES_TRM_MAX_ITERATIONS => return limit(LimitReason::Iterations),
        ffi::MSK_RES_TRM_MIO_NUM_BRANCHES => return limit(LimitReason::Nodes),
        ffi::MSK_RES_TRM_MIO_NUM_RELAXS | ffi::MSK_RES_TRM_NUM_MAX_NUM_INT_SOLUTIONS => {
            return limit(LimitReason::Solutions);
        }
        ffi::MSK_RES_TRM_OBJECTIVE_RANGE => return limit(LimitReason::ObjectiveCutoff),
        ffi::MSK_RES_TRM_SERVER_MAX_MEMORY => return limit(LimitReason::Memory),
        ffi::MSK_RES_TRM_STALL
        | ffi::MSK_RES_TRM_NUMERICAL_PROBLEM
        | ffi::MSK_RES_TRM_MAX_NUM_SETBACKS
        | ffi::MSK_RES_TRM_INTERNAL
        | ffi::MSK_RES_TRM_INTERNAL_STOP
        | ffi::MSK_RES_TRM_LOST_RACE => return (Status::NumericalDifficulty, None),
        _ => {}
    }
    match prosta {
        ffi::MSK_PRO_STA_PRIM_INFEAS => (Status::Infeasible, None),
        ffi::MSK_PRO_STA_DUAL_INFEAS => (Status::Unbounded, None),
        ffi::MSK_PRO_STA_PRIM_AND_DUAL_INFEAS | ffi::MSK_PRO_STA_PRIM_INFEAS_OR_UNBOUNDED => {
            (Status::InfeasibleOrUnbounded, None)
        }
        _ => (Status::Unknown, None),
    }
}

/// What solution is returned (research R6): an optimal solution only with status Optimal;
/// Feasible for a primal-feasible solution status; otherwise None.
pub fn solution_returned(status: Status, solsta: i32) -> SolutionReturned {
    match solsta {
        ffi::MSK_SOL_STA_OPTIMAL if status == Status::Optimal => SolutionReturned::Optimal,
        ffi::MSK_SOL_STA_OPTIMAL
        | ffi::MSK_SOL_STA_PRIM_FEAS
        | ffi::MSK_SOL_STA_PRIM_AND_DUAL_FEAS => SolutionReturned::Feasible,
        _ => SolutionReturned::None,
    }
}

/// Backend-specific tests (task T017; SC-006): status translation is a MOSEK detail outside
/// the solver-independent contract. No licence and no MOSEK call.
#[cfg(test)]
mod tests {
    use super::*;

    const SOLSTA: [i32; 10] = [
        ffi::MSK_SOL_STA_UNKNOWN,
        ffi::MSK_SOL_STA_OPTIMAL,
        ffi::MSK_SOL_STA_PRIM_FEAS,
        ffi::MSK_SOL_STA_DUAL_FEAS,
        ffi::MSK_SOL_STA_PRIM_AND_DUAL_FEAS,
        ffi::MSK_SOL_STA_PRIM_INFEAS_CER,
        ffi::MSK_SOL_STA_DUAL_INFEAS_CER,
        ffi::MSK_SOL_STA_PRIM_ILLPOSED_CER,
        ffi::MSK_SOL_STA_DUAL_ILLPOSED_CER,
        ffi::MSK_SOL_STA_INTEGER_OPTIMAL,
    ];
    const PROSTA: [i32; 9] = [
        ffi::MSK_PRO_STA_UNKNOWN,
        ffi::MSK_PRO_STA_PRIM_AND_DUAL_FEAS,
        ffi::MSK_PRO_STA_PRIM_FEAS,
        ffi::MSK_PRO_STA_DUAL_FEAS,
        ffi::MSK_PRO_STA_PRIM_INFEAS,
        ffi::MSK_PRO_STA_DUAL_INFEAS,
        ffi::MSK_PRO_STA_PRIM_AND_DUAL_INFEAS,
        ffi::MSK_PRO_STA_ILL_POSED,
        ffi::MSK_PRO_STA_PRIM_INFEAS_OR_UNBOUNDED,
    ];
    const TRM: [i32; 16] = [
        ffi::MSK_RES_TRM_MAX_ITERATIONS,
        ffi::MSK_RES_TRM_MAX_TIME,
        ffi::MSK_RES_TRM_OBJECTIVE_RANGE,
        ffi::MSK_RES_TRM_STALL,
        ffi::MSK_RES_TRM_USER_CALLBACK,
        ffi::MSK_RES_TRM_MIO_NUM_RELAXS,
        ffi::MSK_RES_TRM_MIO_NUM_BRANCHES,
        ffi::MSK_RES_TRM_NUM_MAX_NUM_INT_SOLUTIONS,
        ffi::MSK_RES_TRM_MAX_NUM_SETBACKS,
        ffi::MSK_RES_TRM_NUMERICAL_PROBLEM,
        ffi::MSK_RES_TRM_LOST_RACE,
        ffi::MSK_RES_TRM_INTERNAL,
        ffi::MSK_RES_TRM_INTERNAL_STOP,
        ffi::MSK_RES_TRM_SERVER_MAX_TIME,
        ffi::MSK_RES_TRM_SERVER_MAX_MEMORY,
        ffi::MSK_RES_OK,
    ];

    #[test]
    fn solution_status_decides_first() {
        for &trm in &TRM {
            for &p in &PROSTA {
                assert_eq!(
                    translate(trm, p, ffi::MSK_SOL_STA_OPTIMAL).0,
                    Status::Optimal
                );
                assert_eq!(
                    translate(trm, p, ffi::MSK_SOL_STA_PRIM_INFEAS_CER).0,
                    Status::Infeasible
                );
                let expected = if p == ffi::MSK_PRO_STA_DUAL_INFEAS {
                    Status::Unbounded
                } else {
                    Status::InfeasibleOrUnbounded
                };
                assert_eq!(
                    translate(trm, p, ffi::MSK_SOL_STA_DUAL_INFEAS_CER).0,
                    expected
                );
            }
        }
    }

    #[test]
    fn termination_codes() {
        let s = ffi::MSK_SOL_STA_UNKNOWN;
        let p = ffi::MSK_PRO_STA_UNKNOWN;
        let at = |t| translate(t, p, s);
        assert_eq!(
            at(ffi::MSK_RES_TRM_USER_CALLBACK),
            (Status::Interrupted, None)
        );
        for (t, r) in [
            (ffi::MSK_RES_TRM_MAX_TIME, LimitReason::Time),
            (ffi::MSK_RES_TRM_SERVER_MAX_TIME, LimitReason::Time),
            (ffi::MSK_RES_TRM_MAX_ITERATIONS, LimitReason::Iterations),
            (ffi::MSK_RES_TRM_MIO_NUM_BRANCHES, LimitReason::Nodes),
            (ffi::MSK_RES_TRM_MIO_NUM_RELAXS, LimitReason::Solutions),
            (
                ffi::MSK_RES_TRM_NUM_MAX_NUM_INT_SOLUTIONS,
                LimitReason::Solutions,
            ),
            (
                ffi::MSK_RES_TRM_OBJECTIVE_RANGE,
                LimitReason::ObjectiveCutoff,
            ),
            (ffi::MSK_RES_TRM_SERVER_MAX_MEMORY, LimitReason::Memory),
        ] {
            assert_eq!(at(t), (Status::StoppedAtLimit, Some(r)), "{t}");
        }
        for t in [
            ffi::MSK_RES_TRM_STALL,
            ffi::MSK_RES_TRM_NUMERICAL_PROBLEM,
            ffi::MSK_RES_TRM_MAX_NUM_SETBACKS,
            ffi::MSK_RES_TRM_INTERNAL,
            ffi::MSK_RES_TRM_INTERNAL_STOP,
            ffi::MSK_RES_TRM_LOST_RACE,
        ] {
            assert_eq!(at(t), (Status::NumericalDifficulty, None), "{t}");
        }
    }

    #[test]
    fn problem_status_last() {
        let s = ffi::MSK_SOL_STA_UNKNOWN;
        let ok = ffi::MSK_RES_OK;
        assert_eq!(
            translate(ok, ffi::MSK_PRO_STA_PRIM_INFEAS, s).0,
            Status::Infeasible
        );
        assert_eq!(
            translate(ok, ffi::MSK_PRO_STA_DUAL_INFEAS, s).0,
            Status::Unbounded
        );
        assert_eq!(
            translate(ok, ffi::MSK_PRO_STA_PRIM_AND_DUAL_INFEAS, s).0,
            Status::InfeasibleOrUnbounded
        );
        assert_eq!(
            translate(ok, ffi::MSK_PRO_STA_PRIM_INFEAS_OR_UNBOUNDED, s).0,
            Status::InfeasibleOrUnbounded
        );
        for p in [
            ffi::MSK_PRO_STA_UNKNOWN,
            ffi::MSK_PRO_STA_PRIM_AND_DUAL_FEAS,
            ffi::MSK_PRO_STA_PRIM_FEAS,
            ffi::MSK_PRO_STA_DUAL_FEAS,
            ffi::MSK_PRO_STA_ILL_POSED,
        ] {
            assert_eq!(translate(ok, p, s), (Status::Unknown, None), "{p}");
        }
    }

    #[test]
    fn total_over_every_value_and_unknown_codes() {
        // Every combination translates without panicking; unknown values give Unknown.
        for &t in TRM.iter().chain(&[1234, -1, 999_999]) {
            for &p in PROSTA.iter().chain(&[42, -7]) {
                for &s in SOLSTA.iter().chain(&[42, -7]) {
                    let (status, limit) = translate(t, p, s);
                    assert_eq!(limit.is_some(), status == Status::StoppedAtLimit);
                }
            }
        }
        assert_eq!(translate(4242, 42, 42), (Status::Unknown, None));
    }

    #[test]
    fn solution_returned_rule() {
        use SolutionReturned::*;
        assert_eq!(
            solution_returned(Status::Optimal, ffi::MSK_SOL_STA_OPTIMAL),
            Optimal
        );
        assert_eq!(
            solution_returned(Status::StoppedAtLimit, ffi::MSK_SOL_STA_PRIM_FEAS),
            Feasible
        );
        assert_eq!(
            solution_returned(Status::Unknown, ffi::MSK_SOL_STA_PRIM_AND_DUAL_FEAS),
            Feasible
        );
        for s in SOLSTA {
            if ![
                ffi::MSK_SOL_STA_OPTIMAL,
                ffi::MSK_SOL_STA_PRIM_FEAS,
                ffi::MSK_SOL_STA_PRIM_AND_DUAL_FEAS,
            ]
            .contains(&s)
            {
                assert_eq!(solution_returned(Status::Unknown, s), None, "{s}");
            }
        }
    }
}
