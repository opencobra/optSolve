//! Contract tests: validation, verification and options rules (User Story 2), plus the
//! harness self-test (T010a). Custom harness (research R14).

mod common;
#[path = "contract/f005.rs"]
mod f005;
#[path = "contract/qp_conic.rs"]
mod qp_conic;

use common::{Arguments, DeclaredBackend, Failed, Registry, Trial, ensure};

fn harness_self_test() -> Result<(), Failed> {
    // A backend that is not compiled, and one whose environment cannot be created, must both
    // yield ignored trials: never passed (SC-002).
    let declared = vec![
        DeclaredBackend {
            name: "absent",
            make: None,
        },
        DeclaredBackend {
            name: "unlicensed",
            make: Some(|| Err("no licence".to_string())),
        },
    ];
    let registry = Registry::from_declared(declared);
    let trials = registry.trials("probe", |_| Ok(()));
    ensure(trials.len() == 2, "one trial per declared backend")?;
    for t in &trials {
        ensure(
            t.has_ignored_flag(),
            format!("trial {} must be ignored", t.name()),
        )?;
    }
    ensure(
        trials[1].name().contains("no licence"),
        "the reason appears in the trial name",
    )?;
    // Heavy tier switched off: ignored with the reason, never passed (T007).
    let available = Registry::from_declared(vec![DeclaredBackend {
        name: "ok",
        make: Some(|| Ok(Box::new(common::fake_backend::FakeBackend::lp_only()))),
    }]);
    let heavy = available.heavy_trials_if(false, "heavy", std::sync::Arc::new(|_| Ok(())));
    ensure(
        heavy.len() == 1 && heavy[0].has_ignored_flag() && heavy[0].name().contains("heavy tier"),
        "heavy trials ignored with the reason when switched off",
    )?;
    let heavy = available.heavy_trials_if(true, "heavy", std::sync::Arc::new(|_| Ok(())));
    ensure(
        heavy.len() == 1 && !heavy[0].has_ignored_flag(),
        "heavy trials run when switched on",
    )?;
    // Pair trials: ignored unless both members are available (T007).
    let pairs = registry.pair_trials("pair", |_, _| Ok(()));
    ensure(
        pairs.len() == 1 && pairs[0].has_ignored_flag() && pairs[0].name().contains("absent"),
        format!(
            "pair with an absent backend is ignored: {:?}",
            pairs.iter().map(|t| t.name()).collect::<Vec<_>>()
        ),
    )?;
    let two = Registry::from_declared(vec![
        DeclaredBackend {
            name: "a",
            make: Some(|| Ok(Box::new(common::fake_backend::FakeBackend::lp_only()))),
        },
        DeclaredBackend {
            name: "b",
            make: Some(|| Ok(Box::new(common::fake_backend::FakeBackend::lp_only()))),
        },
    ]);
    let pairs = two.pair_trials("pair", |_, _| Ok(()));
    ensure(
        pairs.len() == 1 && !pairs[0].has_ignored_flag() && pairs[0].name() == "pair::a+b",
        "pair of available backends runs",
    )?;
    Ok(())
}

fn main() {
    let args = Arguments::from_args();
    let mut trials = vec![Trial::test(
        "harness_reports_unavailable_backends_as_ignored",
        harness_self_test,
    )];
    let _registry = Registry::detect();
    // User Story 2 trials are added below this line (T020-T024).
    trials.extend(us2_trials(&_registry));
    // Feature 002: QP and conic contract (T003, T004).
    trials.extend(qp_conic::trials());
    trials.extend(f005::trials(&_registry));
    libtest_mimic::run(&args, trials).exit();
}

// ---------------------------------------------------------------------------------------
// User Story 2 (T020-T024). These tests need no solver: validation, conversion, verification
// and options checks are solver-independent, so they run everywhere (constitution II).
// ---------------------------------------------------------------------------------------

type TestFn = fn() -> Result<(), Failed>;

fn us2_trials(_registry: &Registry) -> Vec<Trial> {
    let tests: Vec<(&str, TestFn)> = vec![
        // T020: validation, one failing input per rule.
        ("t020_col_ptrs_wrong_length", t020_col_ptrs_wrong_length),
        ("t020_values_wrong_length", t020_values_wrong_length),
        (
            "t020_col_ptrs_not_starting_at_zero",
            t020_col_ptrs_not_starting_at_zero,
        ),
        ("t020_col_ptrs_last_not_nnz", t020_col_ptrs_last_not_nnz),
        ("t020_col_ptrs_decreasing", t020_col_ptrs_decreasing),
        ("t020_row_index_out_of_range", t020_row_index_out_of_range),
        ("t020_row_indices_unsorted", t020_row_indices_unsorted),
        ("t020_row_indices_duplicate", t020_row_indices_duplicate),
        ("t020_row_index_negative_i32", t020_row_index_negative_i32),
        ("t020_values_non_finite", t020_values_non_finite),
        ("t020_dimension_too_large", t020_dimension_too_large),
        (
            "t020_c_wrong_length_or_non_finite",
            t020_c_wrong_length_or_non_finite,
        ),
        ("t020_vector_lengths", t020_vector_lengths),
        ("t020_nan_bounds", t020_nan_bounds),
        (
            "t020_infinite_bounds_sense_form",
            t020_infinite_bounds_sense_form,
        ),
        (
            "t020_infinite_bounds_range_form",
            t020_infinite_bounds_range_form,
        ),
        (
            "t020_infinite_variable_bounds",
            t020_infinite_variable_bounds,
        ),
        ("t020_invalid_sense_byte", t020_invalid_sense_byte),
        ("t020_gamma_and_var_types", t020_gamma_and_var_types),
        ("t020_valid_problem_accepted", t020_valid_problem_accepted),
        ("t026_canonical_row_bounds", t026_canonical_row_bounds),
        // T021: zero copy and block conversion.
        ("t021_i32_indices_not_copied", t021_i32_indices_not_copied),
        ("t021_usize_blocks_bounded", t021_usize_blocks_bounded),
        (
            "t021_oversized_column_and_empty_columns",
            t021_oversized_column_and_empty_columns,
        ),
        // T022: verifier.
        (
            "t022_optimal_minimise_verified",
            t022_optimal_minimise_verified,
        ),
        ("t022_row_bound_violation", t022_row_bound_violation),
        (
            "t022_variable_bound_violation",
            t022_variable_bound_violation,
        ),
        (
            "t022_reduced_cost_inconsistency",
            t022_reduced_cost_inconsistency,
        ),
        ("t022_dual_sign_minimise", t022_dual_sign_minimise),
        ("t022_dual_sign_maximise", t022_dual_sign_maximise),
        (
            "t022_complementary_slackness_and_gap",
            t022_complementary_slackness_and_gap,
        ),
        ("t022_gap_includes_gamma", t022_gap_includes_gamma),
        (
            "t022_not_evaluated_without_duals",
            t022_not_evaluated_without_duals,
        ),
        ("t022_farkas_certificate", t022_farkas_certificate),
        ("t022_unbounded_ray", t022_unbounded_ray),
        ("t022_malformed_lengths_fail", t022_malformed_lengths_fail),
        // T023: guard self-test (XV-C).
        ("t023_all_zero_solution_fails", t023_all_zero_solution_fails),
        (
            "t023_perturbation_beyond_factor_fails",
            t023_perturbation_beyond_factor_fails,
        ),
        // T024: options and capabilities.
        ("t024_unsupported_field", t024_unsupported_field),
        ("t024_parameter_conflict", t024_parameter_conflict),
        (
            "t024_unsupported_problem_class",
            t024_unsupported_problem_class,
        ),
        // T023 corpus-fixture variant (XV-C): the same guards on iDopaNeuroC_problem_LP.
        (
            "t023_corpus_all_zero_solution_fails",
            t023_corpus_all_zero_solution_fails,
        ),
        (
            "t023_corpus_row_perturbation_fails",
            t023_corpus_row_perturbation_fails,
        ),
    ];
    tests
        .into_iter()
        .map(|(name, f)| Trial::test(name, f))
        .collect()
}

use common::fail;
use common::fake_backend::FakeBackend;
use optsolver::problem::MAX_DIMENSION;
use optsolver::problem::convert::{BLOCK_CAPACITY, IndexBuffer, i32_row_indices};
use optsolver::verify::names;
use optsolver::{
    Backend, Certificate, CscRef, Error, Measure, Options, Overall, ParamValue, Problem, RowBounds,
    Sense, SolutionReturned, SolutionView, Status, Tolerances, Verdict, VerificationReport, verify,
};

const INF: f64 = f64::INFINITY;

fn expect_invalid<T: std::fmt::Debug>(
    r: optsolver::error::Result<T>,
    what: &str,
    detail: &str,
) -> Result<(), Failed> {
    match r {
        Err(Error::Invalid { what: w, detail: d }) if w == what && d.contains(detail) => Ok(()),
        other => Err(fail(format!(
            "expected Invalid {{ what: {what}, detail ~ {detail:?} }}, got {other:?}"
        ))),
    }
}

// A valid 3x2 matrix: column 0 has rows 0 and 2, column 1 has row 1.
const CP: [usize; 3] = [0, 2, 3];
const RI: [usize; 3] = [0, 2, 1];
const VA: [f64; 3] = [1.0, 2.0, 3.0];

fn a3x2() -> CscRef<'static, usize> {
    CscRef::new(3, 2, &CP, &RI, &VA).expect("valid matrix")
}

fn t020_col_ptrs_wrong_length() -> Result<(), Failed> {
    expect_invalid(CscRef::new(3, 2, &[0, 3], &RI, &VA), "A.col_ptrs", "length")
}

fn t020_values_wrong_length() -> Result<(), Failed> {
    expect_invalid(
        CscRef::new(3, 2, &CP, &RI, &[1.0, 2.0]),
        "A.values",
        "length",
    )
}

fn t020_col_ptrs_not_starting_at_zero() -> Result<(), Failed> {
    expect_invalid(
        CscRef::new(3, 2, &[1, 2, 3], &RI, &VA),
        "A.col_ptrs",
        "col_ptrs[0]",
    )
}

fn t020_col_ptrs_last_not_nnz() -> Result<(), Failed> {
    expect_invalid(CscRef::new(3, 2, &[0, 2, 2], &RI, &VA), "A.col_ptrs", "nnz")
}

fn t020_col_ptrs_decreasing() -> Result<(), Failed> {
    expect_invalid(
        CscRef::new(3, 3, &[0, 2, 1, 3], &RI, &VA),
        "A.col_ptrs",
        "decreasing",
    )
}

fn t020_row_index_out_of_range() -> Result<(), Failed> {
    expect_invalid(
        CscRef::new(3, 2, &CP, &[0usize, 3, 1], &VA),
        "A.row_indices",
        "out of range",
    )
}

fn t020_row_indices_unsorted() -> Result<(), Failed> {
    expect_invalid(
        CscRef::new(3, 2, &CP, &[2usize, 0, 1], &VA),
        "A.row_indices",
        "unsorted",
    )
}

fn t020_row_indices_duplicate() -> Result<(), Failed> {
    expect_invalid(
        CscRef::new(3, 2, &CP, &[1usize, 1, 1], &VA),
        "A.row_indices",
        "duplicate",
    )
}

fn t020_row_index_negative_i32() -> Result<(), Failed> {
    expect_invalid(
        CscRef::new(3, 2, &CP, &[0i32, -1, 1], &VA),
        "A.row_indices",
        "negative",
    )
}

fn t020_values_non_finite() -> Result<(), Failed> {
    expect_invalid(
        CscRef::new(3, 2, &CP, &RI, &[1.0, f64::NAN, 3.0]),
        "A.values",
        "non-finite",
    )?;
    expect_invalid(
        CscRef::new(3, 2, &CP, &RI, &[1.0, 2.0, INF]),
        "A.values",
        "non-finite",
    )
}

fn t020_dimension_too_large() -> Result<(), Failed> {
    let empty: [usize; 0] = [];
    match CscRef::new(MAX_DIMENSION + 1, 0, &[0], &empty, &[]) {
        Err(Error::TooLarge { what: "m", value }) if value == MAX_DIMENSION + 1 => {}
        other => return Err(fail(format!("expected TooLarge m, got {other:?}"))),
    }
    // n is checked before the length of col_ptrs, so no huge array is needed.
    match CscRef::new(0, MAX_DIMENSION + 1, &[0], &empty, &[]) {
        Err(Error::TooLarge { what: "n", .. }) => {}
        other => return Err(fail(format!("expected TooLarge n, got {other:?}"))),
    }
    // The limit itself is accepted.
    CscRef::new(MAX_DIMENSION, 0, &[0], &empty, &[]).map_err(fail)?;
    Ok(())
}

const SENSE: &[u8] = b"=<>";
const B: [f64; 3] = [1.0, 1.0, 1.0];
const L: [f64; 2] = [0.0, 0.0];
const U: [f64; 2] = [1.0, 1.0];
const C: [f64; 2] = [1.0, 1.0];

fn rows_sense<'a>(sense: &'a [u8], b: &'a [f64]) -> RowBounds<'a> {
    RowBounds::SenseRhs { sense, b }
}

fn t020_c_wrong_length_or_non_finite() -> Result<(), Failed> {
    let r = rows_sense(SENSE, &B);
    expect_invalid(
        Problem::new(a3x2(), &[1.0], Sense::Minimise, r, &L, &U),
        "c",
        "length",
    )?;
    expect_invalid(
        Problem::new(a3x2(), &[1.0, f64::NAN], Sense::Minimise, r, &L, &U),
        "c",
        "non-finite",
    )?;
    expect_invalid(
        Problem::new(a3x2(), &[INF, 1.0], Sense::Minimise, r, &L, &U),
        "c",
        "non-finite",
    )
}

fn t020_vector_lengths() -> Result<(), Failed> {
    let a = a3x2();
    let s = Sense::Minimise;
    expect_invalid(
        Problem::new(a, &C, s, rows_sense(b"=<", &B), &L, &U),
        "sense",
        "length",
    )?;
    expect_invalid(
        Problem::new(a, &C, s, rows_sense(SENSE, &[1.0]), &L, &U),
        "b",
        "length",
    )?;
    let r = RowBounds::Ranges {
        b_l: &[0.0; 2],
        b_u: &[1.0; 3],
    };
    expect_invalid(Problem::new(a, &C, s, r, &L, &U), "b_l", "length")?;
    let r = RowBounds::Ranges {
        b_l: &[0.0; 3],
        b_u: &[1.0; 4],
    };
    expect_invalid(Problem::new(a, &C, s, r, &L, &U), "b_u", "length")?;
    expect_invalid(
        Problem::new(a, &C, s, rows_sense(SENSE, &B), &[0.0], &U),
        "l",
        "length",
    )?;
    expect_invalid(
        Problem::new(a, &C, s, rows_sense(SENSE, &B), &L, &[1.0; 3]),
        "u",
        "length",
    )
}

fn t020_nan_bounds() -> Result<(), Failed> {
    let (a, s, nan) = (a3x2(), Sense::Minimise, f64::NAN);
    expect_invalid(
        Problem::new(a, &C, s, rows_sense(SENSE, &[1.0, nan, 1.0]), &L, &U),
        "b",
        "NaN",
    )?;
    let r = RowBounds::Ranges {
        b_l: &[0.0, nan, 0.0],
        b_u: &[1.0; 3],
    };
    expect_invalid(Problem::new(a, &C, s, r, &L, &U), "b_l", "NaN")?;
    let r = RowBounds::Ranges {
        b_l: &[0.0; 3],
        b_u: &[1.0, 1.0, nan],
    };
    expect_invalid(Problem::new(a, &C, s, r, &L, &U), "b_u", "NaN")?;
    let r = rows_sense(SENSE, &B);
    expect_invalid(Problem::new(a, &C, s, r, &[nan, 0.0], &U), "l", "NaN")?;
    expect_invalid(Problem::new(a, &C, s, r, &L, &[1.0, nan]), "u", "NaN")
}

fn t020_infinite_bounds_sense_form() -> Result<(), Failed> {
    let (a, s) = (a3x2(), Sense::Minimise);
    // Errors: '=' with ±inf, '<' with -inf, '>' with +inf.
    expect_invalid(
        Problem::new(a, &C, s, rows_sense(b"=<>", &[INF, 1.0, 1.0]), &L, &U),
        "b",
        "'='",
    )?;
    expect_invalid(
        Problem::new(a, &C, s, rows_sense(b"=<>", &[-INF, 1.0, 1.0]), &L, &U),
        "b",
        "'='",
    )?;
    expect_invalid(
        Problem::new(a, &C, s, rows_sense(b"=<>", &[1.0, -INF, 1.0]), &L, &U),
        "b",
        "'<'",
    )?;
    expect_invalid(
        Problem::new(a, &C, s, rows_sense(b"=<>", &[1.0, 1.0, INF]), &L, &U),
        "b",
        "'>'",
    )?;
    // Free rows: '<' with +inf and '>' with -inf are accepted.
    Problem::new(a, &C, s, rows_sense(b"=<>", &[1.0, INF, -INF]), &L, &U).map_err(fail)?;
    Ok(())
}

fn t020_infinite_bounds_range_form() -> Result<(), Failed> {
    let (a, s) = (a3x2(), Sense::Minimise);
    let r = RowBounds::Ranges {
        b_l: &[0.0, INF, 0.0],
        b_u: &[1.0, INF, 1.0],
    };
    expect_invalid(Problem::new(a, &C, s, r, &L, &U), "b_l", "+inf")?;
    let r = RowBounds::Ranges {
        b_l: &[0.0, -INF, -INF],
        b_u: &[1.0, 1.0, -INF],
    };
    expect_invalid(Problem::new(a, &C, s, r, &L, &U), "b_u", "-inf")?;
    // Free row (-inf, +inf) and an inverted finite range (passed to the solver) are accepted.
    let r = RowBounds::Ranges {
        b_l: &[-INF, 2.0, 0.0],
        b_u: &[INF, 1.0, 0.0],
    };
    Problem::new(a, &C, s, r, &L, &U).map_err(fail)?;
    Ok(())
}

fn t020_infinite_variable_bounds() -> Result<(), Failed> {
    let (a, s, r) = (a3x2(), Sense::Minimise, rows_sense(SENSE, &B));
    expect_invalid(
        Problem::new(a, &C, s, r, &[0.0, INF], &[INF, INF]),
        "l",
        "+inf",
    )?;
    expect_invalid(
        Problem::new(a, &C, s, r, &[-INF, -INF], &[-INF, 1.0]),
        "u",
        "-inf",
    )?;
    // Free variables and l > u are accepted.
    Problem::new(a, &C, s, r, &[-INF, 2.0], &[INF, 1.0]).map_err(fail)?;
    Ok(())
}

fn t020_invalid_sense_byte() -> Result<(), Failed> {
    let p = Problem::new(a3x2(), &C, Sense::Minimise, rows_sense(b"=<E", &B), &L, &U);
    expect_invalid(p, "sense", "invalid sense byte")
}

fn t020_gamma_and_var_types() -> Result<(), Failed> {
    let p =
        Problem::new(a3x2(), &C, Sense::Minimise, rows_sense(SENSE, &B), &L, &U).map_err(fail)?;
    expect_invalid(p.with_gamma(f64::NAN), "gamma", "not finite")?;
    expect_invalid(p.with_var_types(b"C"), "var_types", "length")?;
    expect_invalid(p.with_var_types(b"CX"), "var_types", "invalid type byte")?;
    p.with_var_types(b"CI").map_err(fail)?;
    Ok(())
}

fn t020_valid_problem_accepted() -> Result<(), Failed> {
    let p =
        Problem::new(a3x2(), &C, Sense::Maximise, rows_sense(SENSE, &B), &L, &U).map_err(fail)?;
    ensure(p.m() == 3 && p.n() == 2 && p.A().nnz() == 3, "dimensions")?;
    // No copy: the problem borrows the caller's arrays.
    ensure(std::ptr::eq(p.c().as_ptr(), C.as_ptr()), "c is borrowed")?;
    ensure(
        std::ptr::eq(p.A().values().as_ptr(), VA.as_ptr()),
        "A.values is borrowed",
    )?;
    // A matrix with empty columns and m = 0 rows is valid.
    let empty: [usize; 0] = [];
    CscRef::new(0, 3, &[0, 0, 0, 0], &empty, &[]).map_err(fail)?;
    Ok(())
}

fn t026_canonical_row_bounds() -> Result<(), Failed> {
    let r = rows_sense(b"=<>", &[2.0, 3.0, 4.0]);
    ensure(r.bounds(0) == (2.0, 2.0), "'=' gives (b, b)")?;
    ensure(r.bounds(1) == (-INF, 3.0), "'<' gives (-inf, b)")?;
    ensure(r.bounds(2) == (4.0, INF), "'>' gives (b, +inf)")?;
    let r = rows_sense(b"<>", &[INF, -INF]);
    ensure(
        r.bounds(0) == (-INF, INF) && r.bounds(1) == (-INF, INF),
        "free rows",
    )?;
    let r = RowBounds::Ranges {
        b_l: &[-1.0],
        b_u: &[5.0],
    };
    ensure(r.bounds(0) == (-1.0, 5.0), "ranges as given")?;
    // Unvalidated data never panics.
    let (lo, hi) = rows_sense(b"?", &[1.0]).bounds(0);
    ensure(lo.is_nan() && hi.is_nan(), "invalid sense byte gives NaN")
}

// --- T021 -------------------------------------------------------------------------------

fn t021_i32_indices_not_copied() -> Result<(), Failed> {
    let ri: [i32; 3] = [0, 2, 1];
    let a = CscRef::new(3, 2, &CP, &ri, &VA).map_err(fail)?;
    let p = Problem::new(a, &C, Sense::Minimise, rows_sense(SENSE, &B), &L, &U).map_err(fail)?;
    let s = i32_row_indices(p.A()).ok_or_else(|| fail("i32 indices must be exposed"))?;
    ensure(
        std::ptr::eq(s.as_ptr(), ri.as_ptr()) && s.len() == 3,
        "same pointer as the caller's slice",
    )?;
    ensure(
        i32_row_indices(&a3x2()).is_none(),
        "usize indices are not exposed as i32",
    )
}

/// Walks every block, checking bounds, rebasing, zero-copy values and converted contents.
fn check_blocks(a: &CscRef<'_, usize>, buf: &mut IndexBuffer) -> Result<usize, Failed> {
    let cap = buf.capacity();
    let mut blocks = buf.blocks(a);
    let (mut next_col, mut count) = (0usize, 0usize);
    while let Some(b) = blocks.next_block() {
        count += 1;
        ensure(
            b.cols.start == next_col && b.cols.end > b.cols.start,
            "contiguous non-empty column ranges",
        )?;
        let base = a.col_ptrs()[b.cols.start];
        let nnz = b.row_indices.len();
        ensure(
            b.cols.len() == 1 || (nnz <= cap && b.cols.len() <= cap),
            "block within capacity",
        )?;
        ensure(b.starts.len() == b.cols.len() + 1, "starts length cols + 1")?;
        ensure(
            b.starts[0] == 0 && b.starts[b.cols.len()] == nnz,
            "rebased starts",
        )?;
        ensure(b.values.len() == nnz, "values length")?;
        ensure(
            std::ptr::eq(b.values.as_ptr(), a.values()[base..].as_ptr()),
            "values not copied",
        )?;
        for (k, j) in b.cols.clone().enumerate() {
            ensure(a.col_ptrs()[j] - base == b.starts[k], "start of column")?;
        }
        for (k, &i) in b.row_indices.iter().enumerate() {
            ensure(i as usize == a.row_indices()[base + k], "converted index")?;
        }
        next_col = b.cols.end;
    }
    ensure(next_col == a.n(), "blocks cover every column")?;
    Ok(count)
}

fn t021_usize_blocks_bounded() -> Result<(), Failed> {
    let (m, n) = (1000usize, 600usize);
    let nnz = m * n; // 600 000 > 2^18
    let col_ptrs: Vec<usize> = (0..=n).map(|j| j * m).collect();
    let row_indices: Vec<usize> = (0..nnz).map(|k| k % m).collect();
    let values: Vec<f64> = (0..nnz).map(|k| k as f64).collect();
    let a = CscRef::new(m, n, &col_ptrs, &row_indices, &values).map_err(fail)?;
    let mut buf = IndexBuffer::new();
    ensure(
        buf.capacity() == BLOCK_CAPACITY && BLOCK_CAPACITY == 1 << 18,
        "capacity 2^18",
    )?;
    let count = check_blocks(&a, &mut buf)?;
    ensure(
        count >= nnz.div_ceil(BLOCK_CAPACITY),
        format!("at least ceil(nnz/cap) blocks, got {count}"),
    )?;
    // The buffer is reusable for a second problem.
    let count2 = check_blocks(&a, &mut buf)?;
    ensure(count2 == count, "reuse gives the same blocks")
}

fn t021_oversized_column_and_empty_columns() -> Result<(), Failed> {
    // Columns: 3 nnz, 10 nnz (exceeds capacity 4), 2 nnz, then 9 empty columns.
    let col_ptrs: Vec<usize> = [0, 3, 13, 15]
        .into_iter()
        .chain(std::iter::repeat_n(15, 9))
        .collect();
    let n = col_ptrs.len() - 1;
    let mut row_indices: Vec<usize> = vec![0, 1, 2];
    row_indices.extend(0..10);
    row_indices.extend([4, 7]);
    let values = vec![1.0; 15];
    let a = CscRef::new(10, n, &col_ptrs, &row_indices, &values).map_err(fail)?;
    let mut buf = IndexBuffer::with_capacity(4);
    check_blocks(&a, &mut buf)?;
    let mut blocks = buf.blocks(&a);
    let sizes: Vec<(std::ops::Range<usize>, usize)> = std::iter::from_fn(|| {
        blocks
            .next_block()
            .map(|b| (b.cols.clone(), b.row_indices.len()))
    })
    .collect();
    ensure(
        sizes[1] == (1..2, 10),
        format!("oversized column alone in its block: {sizes:?}"),
    )?;
    ensure(
        sizes.iter().all(|(c, _)| c.len() <= 4),
        "at most capacity columns per block (empty columns)",
    )?;
    // n = 0 yields no block.
    let empty: [usize; 0] = [];
    let a0 = CscRef::new(3, 0, &[0], &empty, &[]).map_err(fail)?;
    ensure(buf.blocks(&a0).next_block().is_none(), "no block for n = 0")
}

// --- T022 / T023 ------------------------------------------------------------------------

const TOL: Tolerances = Tolerances {
    primal: 1e-6,
    dual: 1e-6,
    integrality: 1e-5,
};

fn measure<'r>(rep: &'r VerificationReport, name: &str) -> Result<&'r Measure, Failed> {
    rep.measures
        .iter()
        .find(|q| q.name == name)
        .ok_or_else(|| fail(format!("measure {name} missing: {rep:?}")))
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * (1.0 + b.abs())
}

fn check_measure(
    rep: &VerificationReport,
    name: &str,
    abs: f64,
    rel: f64,
    verdict: Verdict,
) -> Result<(), Failed> {
    let q = measure(rep, name)?;
    ensure(
        close(q.absolute, abs) && close(q.relative, rel) && q.verdict == verdict,
        format!("{name}: expected abs {abs}, rel {rel}, {verdict:?}; got {q:?}"),
    )
}

fn view<'a>(
    status: Status,
    x: Option<&'a [f64]>,
    y: Option<&'a [f64]>,
    z: Option<&'a [f64]>,
) -> SolutionView<'a> {
    let returned = if x.is_some() {
        SolutionReturned::Optimal
    } else {
        SolutionReturned::None
    };
    SolutionView {
        status,
        returned,
        x,
        y,
        z,
        z_l: None,
        z_u: None,
        s: None,
        basic: true,
        certificate: None,
    }
}

// LP1: minimise x1 + 2 x2 subject to x1 + x2 = 2, x >= 0. Optimum x = (2, 0), y = 1,
// z = (0, 1), p = d = 2 (+ gamma).
const LP1_CP: [usize; 3] = [0, 1, 2];
const LP1_RI: [usize; 2] = [0, 0];
const LP1_VA: [f64; 2] = [1.0, 1.0];
const LP1_C: [f64; 2] = [1.0, 2.0];
const LP1_B: [f64; 1] = [2.0];
const ZERO2: [f64; 2] = [0.0, 0.0];
const INF2: [f64; 2] = [INF, INF];

fn lp1(sense: Sense, row_sense: &'static [u8]) -> Problem<'static, usize> {
    let a = CscRef::new(1, 2, &LP1_CP, &LP1_RI, &LP1_VA).expect("valid");
    Problem::new(
        a,
        &LP1_C,
        sense,
        RowBounds::SenseRhs {
            sense: row_sense,
            b: &LP1_B,
        },
        &ZERO2,
        &INF2,
    )
    .expect("valid")
}

fn t022_optimal_minimise_verified() -> Result<(), Failed> {
    let p = lp1(Sense::Minimise, b"=");
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[2.0, 0.0]),
            Some(&[1.0]),
            Some(&[0.0, 1.0]),
        ),
        &TOL,
    );
    ensure(
        rep.overall == Overall::Verified,
        format!("expected Verified: {rep:?}"),
    )?;
    ensure(
        rep.measures.len() == 6 && rep.measures.iter().all(|q| q.required),
        "six required measures",
    )?;
    check_measure(&rep, names::GAP, 0.0, 0.0, Verdict::Pass)
}

fn t022_row_bound_violation() -> Result<(), Failed> {
    let p = lp1(Sense::Minimise, b"=");
    // Ax = 2.5 violates b = 2 by 0.5; scaling 1 + max |b| = 3.
    let rep = verify(
        &p,
        &view(Status::Optimal, Some(&[2.5, 0.0]), None, None),
        &TOL,
    );
    check_measure(&rep, names::ROW_BOUND, 0.5, 0.5 / 3.0, Verdict::Fail)?;
    check_measure(&rep, names::VARIABLE_BOUND, 0.0, 0.0, Verdict::Pass)?;
    ensure(rep.overall == Overall::Failed, "overall Failed")
}

fn t022_variable_bound_violation() -> Result<(), Failed> {
    let p = lp1(Sense::Minimise, b"=");
    // x2 = -0.5 violates l = 0; scaling 1 + max finite |l|, |u| = 1.
    let rep = verify(
        &p,
        &view(Status::Optimal, Some(&[2.5, -0.5]), None, None),
        &TOL,
    );
    check_measure(&rep, names::ROW_BOUND, 0.0, 0.0, Verdict::Pass)?;
    check_measure(&rep, names::VARIABLE_BOUND, 0.5, 0.5, Verdict::Fail)?;
    ensure(rep.overall == Overall::Failed, "overall Failed")
}

fn t022_reduced_cost_inconsistency() -> Result<(), Failed> {
    let p = lp1(Sense::Minimise, b"=");
    // c - A^T y = (0, 1); z2 = 1.5 is off by 0.5; per-column scaling for column 2 is
    // 1 + |c_2| + |A_12 y_1| = 1 + 2 + 1 = 4 (data-model.md, amended 2026-11-14).
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[2.0, 0.0]),
            Some(&[1.0]),
            Some(&[0.0, 1.5]),
        ),
        &TOL,
    );
    check_measure(&rep, names::REDUCED_COST, 0.5, 0.5 / 4.0, Verdict::Fail)?;
    ensure(rep.overall == Overall::Failed, "overall Failed")
}

fn t022_dual_sign_minimise() -> Result<(), Failed> {
    let p = lp1(Sense::Minimise, b"=");
    // y = 3 gives z = (-2, -1): consistent, but z must be >= 0 at finite l, infinite u.
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[2.0, 0.0]),
            Some(&[3.0]),
            Some(&[-2.0, -1.0]),
        ),
        &TOL,
    );
    check_measure(&rep, names::REDUCED_COST, 0.0, 0.0, Verdict::Pass)?;
    check_measure(&rep, names::DUAL_SIGN, 2.0, 2.0 / 3.0, Verdict::Fail)?;
    ensure(rep.overall == Overall::Failed, "overall Failed")
}

fn t022_dual_sign_maximise() -> Result<(), Failed> {
    // Maximise x1 + 2 x2 subject to x1 + x2 <= 2, x >= 0: x = (0, 2), y = 2, z = (-1, 0).
    let p = lp1(Sense::Maximise, b"<");
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[0.0, 2.0]),
            Some(&[2.0]),
            Some(&[-1.0, 0.0]),
        ),
        &TOL,
    );
    ensure(
        rep.overall == Overall::Verified,
        format!("maximise optimum verified: {rep:?}"),
    )?;
    // The minimise signs are wrong for maximise: y = -1, z = (2, 3). sigma*y = 1 > 0 where
    // b_l = -inf (violation 1); sigma*z = (-2, -3) < 0 where u = +inf (violation 3).
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[0.0, 2.0]),
            Some(&[-1.0]),
            Some(&[2.0, 3.0]),
        ),
        &TOL,
    );
    check_measure(&rep, names::DUAL_SIGN, 3.0, 1.0, Verdict::Fail)?; // 3 / (1 + ||c||)
    ensure(rep.overall == Overall::Failed, "overall Failed")
}

fn t022_complementary_slackness_and_gap() -> Result<(), Failed> {
    let p = lp1(Sense::Minimise, b"=");
    // Feasible but not optimal: x = (1, 1), dual-feasible y = 1, z = (0, 1).
    // CS: |z2| * |x2 - l2| = 1; p = 3; d = 1*2 + 1*0 = 2; gap 1.
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[1.0, 1.0]),
            Some(&[1.0]),
            Some(&[0.0, 1.0]),
        ),
        &TOL,
    );
    check_measure(&rep, names::ROW_BOUND, 0.0, 0.0, Verdict::Pass)?;
    check_measure(&rep, names::DUAL_SIGN, 0.0, 0.0, Verdict::Pass)?;
    check_measure(
        &rep,
        names::COMPLEMENTARY_SLACKNESS,
        1.0,
        1.0 / 4.0,
        Verdict::Fail,
    )?;
    // Gap relative to max(1, min(|p|, |d|)) = 2.
    check_measure(&rep, names::GAP, 1.0, 0.5, Verdict::Fail)?;
    ensure(rep.overall == Overall::Failed, "overall Failed")?;
    // Judged against the tolerance supplied: a loose tolerance passes the same values.
    let loose = Tolerances {
        primal: 1e-6,
        dual: 0.06,
        integrality: 1e-5,
    };
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[1.0, 1.0]),
            Some(&[1.0]),
            Some(&[0.0, 1.0]),
        ),
        &loose,
    );
    check_measure(&rep, names::GAP, 1.0, 0.5, Verdict::Pass)?;
    check_measure(
        &rep,
        names::COMPLEMENTARY_SLACKNESS,
        1.0,
        0.25,
        Verdict::Pass,
    )
}

fn t022_gap_includes_gamma() -> Result<(), Failed> {
    let p = lp1(Sense::Minimise, b"=").with_gamma(5.0).map_err(fail)?;
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[2.0, 0.0]),
            Some(&[1.0]),
            Some(&[0.0, 1.0]),
        ),
        &TOL,
    );
    ensure(rep.overall == Overall::Verified, "gamma cancels in the gap")?;
    // x = (1, 1): p = 3 + 5 = 8, d = 2 + 5 = 7, gap 1, relative 1 / max(1, min(8, 7)).
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[1.0, 1.0]),
            Some(&[1.0]),
            Some(&[0.0, 1.0]),
        ),
        &TOL,
    );
    check_measure(&rep, names::GAP, 1.0, 1.0 / 7.0, Verdict::Fail)
}

fn t022_not_evaluated_without_duals() -> Result<(), Failed> {
    let p = lp1(Sense::Minimise, b"=");
    let rep = verify(
        &p,
        &view(Status::Optimal, Some(&[2.0, 0.0]), None, None),
        &TOL,
    );
    for name in [
        names::REDUCED_COST,
        names::DUAL_SIGN,
        names::COMPLEMENTARY_SLACKNESS,
        names::GAP,
    ] {
        let q = measure(&rep, name)?;
        ensure(
            q.verdict == Verdict::NotEvaluated && q.required,
            format!("{name} NotEvaluated: {q:?}"),
        )?;
    }
    ensure(
        rep.overall == Overall::PartiallyVerified,
        format!("PartiallyVerified: {rep:?}"),
    )?;
    // A non-optimal status with a feasible point does not require the dual measures.
    let mut v = view(Status::StoppedAtLimit, Some(&[2.0, 0.0]), None, None);
    v.returned = SolutionReturned::Feasible;
    let rep = verify(&p, &v, &TOL);
    ensure(
        rep.overall == Overall::Verified && rep.measures.len() == 2,
        format!("primal only: {rep:?}"),
    )
}

// LP2 (infeasible): x1 + x2 = -1, x >= 0.
const LP2_B: [f64; 1] = [-1.0];

fn t022_farkas_certificate() -> Result<(), Failed> {
    let a = CscRef::new(1, 2, &LP1_CP, &LP1_RI, &LP1_VA).map_err(fail)?;
    let p = Problem::new(
        a,
        &LP1_C,
        Sense::Minimise,
        rows_sense(b"=", &LP2_B),
        &ZERO2,
        &INF2,
    )
    .map_err(fail)?;
    let required = |rep: &VerificationReport| measure(rep, names::FARKAS).map(|q| q.required);
    // Valid: y = -1, z = (1, 1): A^T y + z = 0, z >= 0 at l, d_F = (-1)(-1) = 1 > 0.
    let cert = Certificate::Farkas {
        y: vec![-1.0],
        z: vec![1.0, 1.0],
        s: vec![],
    };
    let mut v = view(Status::Infeasible, None, None, None);
    v.certificate = Some(&cert);
    let rep = verify(&p, &v, &TOL);
    check_measure(&rep, names::FARKAS, 0.0, 0.0, Verdict::Pass)?;
    ensure(
        required(&rep)? && rep.overall == Overall::Verified,
        format!("Verified: {rep:?}"),
    )?;
    // Residual: z = (1, 1.5) gives ||A^T y + z|| = 0.5; scaling 1 + ||y|| = 2.
    let cert = Certificate::Farkas {
        y: vec![-1.0],
        z: vec![1.0, 1.5],
        s: vec![],
    };
    v.certificate = Some(&cert);
    let rep = verify(&p, &v, &TOL);
    check_measure(&rep, names::FARKAS, 0.5, 0.25, Verdict::Fail)?;
    // Wrong sign and d_F <= 0: y = 1, z = (-1, -1).
    let cert = Certificate::Farkas {
        y: vec![1.0],
        z: vec![-1.0, -1.0],
        s: vec![],
    };
    v.certificate = Some(&cert);
    let rep = verify(&p, &v, &TOL);
    let q = measure(&rep, names::FARKAS)?;
    ensure(
        q.absolute == 1.0 && q.relative == INF && q.verdict == Verdict::Fail,
        format!("bad certificate: {q:?}"),
    )?;
    // Infeasible without a certificate: required but not evaluated.
    let rep = verify(&p, &view(Status::Infeasible, None, None, None), &TOL);
    ensure(
        rep.overall == Overall::PartiallyVerified && rep.measures.len() == 1,
        format!("{rep:?}"),
    )
}

fn t022_unbounded_ray() -> Result<(), Failed> {
    // Minimise -x1 subject to x1 - x2 = 0, x >= 0: ray r = (1, 1).
    let va: [f64; 2] = [1.0, -1.0];
    let a = CscRef::new(1, 2, &LP1_CP, &LP1_RI, &va).map_err(fail)?;
    let c = [-1.0, 0.0];
    let p = Problem::new(
        a,
        &c,
        Sense::Minimise,
        rows_sense(b"=", &[0.0]),
        &ZERO2,
        &INF2,
    )
    .map_err(fail)?;
    let mut v = view(Status::Unbounded, None, None, None);
    let good = Certificate::UnboundedRay { r: vec![1.0, 1.0] };
    v.certificate = Some(&good);
    let rep = verify(&p, &v, &TOL);
    check_measure(&rep, names::RAY, 0.0, 0.0, Verdict::Pass)?;
    ensure(rep.overall == Overall::Verified, "good ray verified")?;
    // r = (1, 0): A r = 1 must be 0 (equality row); scaling 1 + ||r|| = 2.
    let bad = Certificate::UnboundedRay { r: vec![1.0, 0.0] };
    v.certificate = Some(&bad);
    check_measure(&verify(&p, &v, &TOL), names::RAY, 1.0, 0.5, Verdict::Fail)?;
    // r = (-1, -1): not improving (and leaves x >= 0).
    let worse = Certificate::UnboundedRay {
        r: vec![-1.0, -1.0],
    };
    v.certificate = Some(&worse);
    let q = verify(&p, &v, &TOL).measures[0].clone();
    ensure(
        q.relative == INF && q.verdict == Verdict::Fail,
        format!("not improving: {q:?}"),
    )?;
    // Maximise x1 (c = (1, 0)): sigma c^T r = -1 < 0, so r = (1, 1) improves.
    let c_max = [1.0, 0.0];
    let pm = Problem::new(
        a,
        &c_max,
        Sense::Maximise,
        rows_sense(b"=", &[0.0]),
        &ZERO2,
        &INF2,
    )
    .map_err(fail)?;
    v.certificate = Some(&good);
    check_measure(&verify(&pm, &v, &TOL), names::RAY, 0.0, 0.0, Verdict::Pass)?;
    // Unbounded without a ray: required but not evaluated.
    let rep = verify(&p, &view(Status::Unbounded, None, None, None), &TOL);
    ensure(
        rep.overall == Overall::PartiallyVerified,
        format!("{rep:?}"),
    )
}

fn t022_malformed_lengths_fail() -> Result<(), Failed> {
    let p = lp1(Sense::Minimise, b"=");
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[2.0]),
            Some(&[1.0]),
            Some(&[0.0, 1.0]),
        ),
        &TOL,
    );
    ensure(rep.overall == Overall::Failed, "x of wrong length fails")?;
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[2.0, 0.0]),
            Some(&[1.0, 0.0]),
            Some(&[0.0, 1.0]),
        ),
        &TOL,
    );
    ensure(rep.overall == Overall::Failed, "y of wrong length fails")?;
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[2.0, f64::NAN]),
            Some(&[1.0]),
            Some(&[0.0, 1.0]),
        ),
        &TOL,
    );
    ensure(rep.overall == Overall::Failed, "NaN in x fails")
}

fn t023_all_zero_solution_fails() -> Result<(), Failed> {
    // "Everything failed": all-zero arrays claimed Optimal must never verify.
    let p = lp1(Sense::Minimise, b"=");
    let rep = verify(
        &p,
        &view(
            Status::Optimal,
            Some(&[0.0, 0.0]),
            Some(&[0.0]),
            Some(&[0.0, 0.0]),
        ),
        &TOL,
    );
    ensure(
        rep.overall == Overall::Failed,
        format!("all-zero solution must fail: {rep:?}"),
    )?;
    check_measure(&rep, names::ROW_BOUND, 2.0, 2.0 / 3.0, Verdict::Fail)
}

fn t023_perturbation_beyond_factor_fails() -> Result<(), Failed> {
    // Identity 2x2, rows x1 = 0 and x2 = 0, -1 <= x <= 1: row scaling 1 + 0 = 1, so the
    // relative row violation equals the perturbation of x1.
    let a = CscRef::new(2, 2, &[0, 1, 2], &[0usize, 1], &[1.0, 1.0]).map_err(fail)?;
    let p = Problem::new(
        a,
        &[0.0, 0.0],
        Sense::Minimise,
        rows_sense(b"==", &[0.0, 0.0]),
        &[-1.0, -1.0],
        &[1.0, 1.0],
    )
    .map_err(fail)?;
    let threshold = optsolver::options::defaults::VERIFICATION_FACTOR * TOL.primal;
    let run = |delta: f64| {
        let x = [delta, 0.0];
        let mut v = view(Status::StoppedAtLimit, Some(&x), None, None);
        v.returned = SolutionReturned::Feasible;
        verify(&p, &v, &TOL)
    };
    ensure(
        run(0.0).overall == Overall::Verified,
        "unperturbed solution verifies",
    )?;
    ensure(
        run(0.5 * threshold).overall == Overall::Verified,
        "within 10x tolerance passes",
    )?;
    let rep = run(1.5 * threshold);
    ensure(
        rep.overall == Overall::Failed,
        format!("beyond 10x tolerance fails: {rep:?}"),
    )?;
    check_measure(
        &rep,
        names::ROW_BOUND,
        1.5 * threshold,
        1.5 * threshold,
        Verdict::Fail,
    )
}

// --- T024 -------------------------------------------------------------------------------

fn t024_unsupported_field() -> Result<(), Failed> {
    let backend = FakeBackend::lp_only();
    let p = lp1(Sense::Minimise, b"=");
    let mut o = Options::default();
    o.time_limit = Some(10.0);
    backend.solve((&p).into(), &mut o).map_err(fail)?;
    let mut o = Options::default();
    o.iteration_limit = Some(10.0);
    match backend.solve((&p).into(), &mut o) {
        Err(Error::Unsupported {
            field: "iteration_limit",
            backend: "fake",
        }) => {}
        other => {
            return Err(fail(format!(
                "expected Unsupported iteration_limit: {other:?}"
            )));
        }
    }
    let mut o = Options::default();
    o.certificates = true;
    match backend.solve((&p).into(), &mut o) {
        Err(Error::Unsupported {
            field: "certificates",
            backend: "fake",
        }) => Ok(()),
        other => Err(fail(format!(
            "expected Unsupported certificates: {other:?}"
        ))),
    }
}

fn t024_parameter_conflict() -> Result<(), Failed> {
    let backend = FakeBackend::lp_only();
    let p = lp1(Sense::Minimise, b"=");
    // A raw parameter alone, or with an unrelated shared field, is fine.
    let mut o = Options::default();
    o.raw = vec![("TimeLimit".into(), ParamValue::Double(5.0))];
    backend.solve((&p).into(), &mut o).map_err(fail)?;
    o.threads = Some(2);
    backend.solve((&p).into(), &mut o).map_err(fail)?;
    // The same vendor parameter by two routes (names compared ignoring case).
    let mut o = Options::default();
    o.time_limit = Some(10.0);
    o.raw = vec![("timelimit".into(), ParamValue::Double(5.0))];
    match backend.solve((&p).into(), &mut o) {
        Err(Error::ParameterConflict { name }) if name == "timelimit" => Ok(()),
        other => Err(fail(format!("expected ParameterConflict: {other:?}"))),
    }
}

fn t024_unsupported_problem_class() -> Result<(), Failed> {
    let backend = FakeBackend::lp_only();
    let p = lp1(Sense::Minimise, b"=");
    let pi = p.with_var_types(b"CI").map_err(fail)?;
    match backend.solve((&pi).into(), &mut Options::default()) {
        Err(Error::Unsupported {
            field: "var_types",
            backend: "fake",
        }) => {}
        other => return Err(fail(format!("expected Unsupported var_types: {other:?}"))),
    }
    let q = CscRef::new(2, 2, &[0, 1, 2], &[0usize, 1], &[1.0, 1.0]).map_err(fail)?;
    let pq = p.with_Q(q).map_err(fail)?;
    match backend.solve((&pq).into(), &mut Options::default()) {
        Err(Error::Unsupported {
            field: "Q",
            backend: "fake",
        }) => {}
        other => return Err(fail(format!("expected Unsupported Q: {other:?}"))),
    }
    // The same check is available to every backend directly.
    optsolver::options::check::check_problem((&p).into(), &backend.capabilities()).map_err(fail)
}

// --- T023 corpus-fixture variant ---------------------------------------------------------

const T023_CORPUS_INSTANCE: &str = "iDopaNeuroC_problem_LP";

fn t023_corpus_all_zero_solution_fails() -> Result<(), Failed> {
    // All-zero x, y, z claimed Optimal on a corpus instance with its first seeded objective.
    // The instance has b = 0, so x = 0 meets the rows; but z = 0 while c - A^T y = c != 0, so
    // reduced-cost consistency fails, and the whole report must be Failed.
    let f = common::fixtures::load_corpus_fixture(T023_CORPUS_INSTANCE).map_err(fail)?;
    let (m, n) = (f.problem.m, f.problem.n);
    let seed = f.provenance.seeds().map_err(fail)?[0];
    let c = common::objectives::seeded_objective(seed, n);
    let p = f.problem.problem(&c).map_err(fail)?;
    let (x, y, z) = (vec![0.0; n], vec![0.0; m], vec![0.0; n]);
    let rep = verify(
        &p,
        &view(Status::Optimal, Some(&x), Some(&y), Some(&z)),
        &TOL,
    );
    ensure(
        rep.overall == Overall::Failed,
        format!("all-zero solution must fail: {:?}", rep.overall),
    )?;
    let q = measure(&rep, names::REDUCED_COST)?;
    ensure(
        q.verdict == Verdict::Fail,
        format!("reduced-cost measure must fail: {q:?}"),
    )
}

fn t023_corpus_row_perturbation_fails() -> Result<(), Failed> {
    // No solver is available here, so a feasible point is made rather than solved for: x0 is 0
    // clamped to the instance's variable bounds, and the rows are the instance's matrix A with
    // b_l = b_u = A x0 (computed in column order, as the verifier does). x0 then verifies
    // exactly. Then one variable x_j is moved within its bounds so that the largest row
    // violation (in the row with the largest |a_ij| of column j; the instance has no
    // single-entry column) is 0.5x (passes) and 1.5x (fails) the verification threshold
    // VERIFICATION_FACTOR x primal tolerance, relative to 1 + max |row bound|.
    let f = common::fixtures::load_corpus_fixture(T023_CORPUS_INSTANCE).map_err(fail)?;
    let fx = &f.problem;
    let (m, n) = (fx.m, fx.n);
    let x0: Vec<f64> = (0..n).map(|j| 0.0f64.max(fx.l[j]).min(fx.u[j])).collect();
    let mut ax = vec![0.0; m];
    for (j, &xj) in x0.iter().enumerate() {
        for k in fx.col_ptrs[j]..fx.col_ptrs[j + 1] {
            ax[fx.row_indices[k]] += fx.values[k] * xj;
        }
    }
    let scale = ax.iter().fold(0.0f64, |s, v| s.max(v.abs()));
    let a = fx.A().map_err(fail)?;
    let c = vec![0.0; n];
    let rows = RowBounds::Ranges { b_l: &ax, b_u: &ax };
    let p = Problem::new(a, &c, fx.objective_sense, rows, &fx.l, &fx.u).map_err(fail)?;
    let threshold = optsolver::options::defaults::VERIFICATION_FACTOR * TOL.primal * (1.0 + scale);
    // A column with entries and room on one side of x0 for the largest move below; a move of
    // x_j by t changes row i by a_ij t, so the largest row violation is max_i |a_ij| t.
    let (j, amax, dir) = (0..n)
        .filter(|&j| fx.col_ptrs[j + 1] > fx.col_ptrs[j])
        .find_map(|j| {
            let amax = fx.values[fx.col_ptrs[j]..fx.col_ptrs[j + 1]]
                .iter()
                .fold(0.0f64, |s, v| s.max(v.abs()));
            let need = 2.0 * threshold / amax;
            if fx.u[j] - x0[j] >= need {
                Some((j, amax, 1.0))
            } else if x0[j] - fx.l[j] >= need {
                Some((j, amax, -1.0))
            } else {
                None
            }
        })
        .ok_or_else(|| fail("no column with room to move"))?;
    let run = |violation: f64| {
        let mut x = x0.clone();
        x[j] += dir * violation / amax;
        let mut v = view(Status::StoppedAtLimit, Some(&x), None, None);
        v.returned = SolutionReturned::Feasible;
        verify(&p, &v, &TOL)
    };
    let rep = run(0.0);
    ensure(
        rep.overall == Overall::Verified,
        format!("x0 must verify: {rep:?}"),
    )?;
    let rep = run(0.5 * threshold);
    ensure(
        rep.overall == Overall::Verified,
        format!("within the threshold passes: {rep:?}"),
    )?;
    let rep = run(1.5 * threshold);
    ensure(
        rep.overall == Overall::Failed,
        format!("beyond the threshold fails: {rep:?}"),
    )?;
    let q = measure(&rep, names::ROW_BOUND)?;
    ensure(
        q.verdict == Verdict::Fail,
        format!("row-bound measure must fail: {q:?}"),
    )?;
    let q = measure(&rep, names::VARIABLE_BOUND)?;
    ensure(
        q.verdict == Verdict::Pass,
        format!("variable bounds still hold: {q:?}"),
    )
}
