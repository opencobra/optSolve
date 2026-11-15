//! Corpus tests: fixtures, provenance, MPS reader, seeded objectives (User Story 1), and the
//! per-backend solve tests (User Story 3). Custom harness (research R14).

mod common;

use common::{Arguments, Registry, Trial, ensure};

fn main() {
    let args = Arguments::from_args();
    let registry = Registry::detect();
    let mut trials: Vec<Trial> = Vec::new();
    trials.extend(us1_trials());
    trials.extend(us3_trials(&registry));
    trials.extend(f002_trials(&registry));
    trials.extend(toy_trials(&registry));
    // Feature 003 T017: cross-solver comparison (each pair of declared backends).
    trials.extend(common::compare::self_tests());
    match common::compare::items() {
        Ok(items) => trials.extend(common::compare::trials(&registry, &items)),
        Err(e) => trials.push(Trial::test("compare::items", move || Err(common::fail(&e)))),
    }
    libtest_mimic::run(&args, trials).exit();
}

fn us1_trials() -> Vec<Trial> {
    let mut trials = common::mps::self_tests();
    trials.extend(common::fixtures::self_tests());
    trials.extend(common::jtask::self_tests());
    trials.push(Trial::test(
        "us1::splitmix64_reference_vector",
        us1_splitmix64_reference_vector,
    ));
    trials.push(Trial::test(
        "us1::seeded_objective_reference_checksum",
        us1_seeded_objective_reference_checksum,
    ));
    trials.push(Trial::test(
        "us1::t013_corpus_document_matches_declaration",
        us1_corpus_document_matches_declaration,
    ));
    let instances = match common::fixtures::lp_instances() {
        Ok(v) => v,
        Err(e) => {
            trials.push(Trial::test("us1::corpus_declaration", move || {
                Err(common::fail(&e))
            }));
            return trials;
        }
    };
    for name in instances {
        let a = name.clone();
        trials.push(Trial::test(
            format!("us1::fixture_and_provenance::{name}"),
            move || us1_fixture_and_provenance(&a),
        ));
        let b = name.clone();
        trials.push(Trial::test(
            format!("us1::mps_matches_provenance::{name}"),
            move || us1_mps_matches_provenance(&b),
        ));
        trials.push(Trial::test(
            format!("us1::seeded_objectives_reproducible::{name}"),
            move || us1_seeded_objectives_reproducible(&name),
        ));
    }
    trials
}

/// Every declared instance has an MPS file and a complete provenance record whose MPS SHA-256
/// matches the file (task T011).
fn us1_fixture_and_provenance(instance: &str) -> Result<(), common::Failed> {
    use common::fixtures::{
        CORPUS_PROVENANCE_FIELDS, Provenance, mps_path, provenance_path, sha256_hex,
    };
    let mps = mps_path(instance).map_err(common::fail)?;
    let prov = provenance_path(instance).map_err(common::fail)?;
    ensure(mps.is_file(), format!("{} is missing", mps.display()))?;
    ensure(prov.is_file(), format!("{} is missing", prov.display()))?;
    let p = Provenance::load(&prov).map_err(common::fail)?;
    let missing = p.missing(CORPUS_PROVENANCE_FIELDS);
    ensure(
        missing.is_empty(),
        format!("{instance}: provenance lacks {missing:?}"),
    )?;
    ensure(
        p.str("instance").map_err(common::fail)? == instance,
        "provenance instance name",
    )?;
    ensure(
        !p.seeds().map_err(common::fail)?.is_empty(),
        "no seeds recorded",
    )?;
    let bytes = std::fs::read(&mps).map_err(common::fail)?;
    let actual = sha256_hex(&bytes);
    let recorded = p.str("mps_sha256").map_err(common::fail)?;
    ensure(
        actual == recorded,
        format!("{instance}: MPS SHA-256 {actual} differs from recorded {recorded}"),
    )
}

/// Each fixture parses into the dimensions of its provenance record, all rows equality,
/// objective sense maximise, and arrays whose SHA-256 equals that recorded from the source
/// model at extraction (task T012).
fn us1_mps_matches_provenance(instance: &str) -> Result<(), common::Failed> {
    use common::fixtures::{load_corpus_fixture, sha256_f64_le, sha256_hex, sha256_usize_le};
    let f = load_corpus_fixture(instance).map_err(common::fail)?;
    let (p, r) = (&f.problem, &f.provenance);
    let dims = |k: &str| r.usize(k).map_err(common::fail);
    ensure(
        p.m == dims("m")?,
        format!("m = {} but provenance says {}", p.m, dims("m")?),
    )?;
    ensure(
        p.n == dims("n")?,
        format!("n = {} but provenance says {}", p.n, dims("n")?),
    )?;
    ensure(
        p.nnz() == dims("nnz")?,
        format!("nnz = {} but provenance says {}", p.nnz(), dims("nnz")?),
    )?;
    ensure(
        p.sense.iter().all(|s| *s == b'='),
        "not every row is an equality",
    )?;
    ensure(!p.has_ranges(), "unexpected RANGES")?;
    ensure(
        p.objective_sense == optsolver::Sense::Maximise,
        "objective sense is not maximise",
    )?;
    ensure(
        r.str("objective_sense").map_err(common::fail)? == "max",
        "provenance objective sense is not max",
    )?;
    let checks = [
        ("A_col_ptrs", sha256_usize_le(&p.col_ptrs)),
        ("A_row_indices", sha256_usize_le(&p.row_indices)),
        ("A_values", sha256_f64_le(&p.values)),
        ("sense", sha256_hex(&p.sense)),
        ("b", sha256_f64_le(&p.b)),
        ("l", sha256_f64_le(&p.l)),
        ("u", sha256_f64_le(&p.u)),
        ("c", sha256_f64_le(&p.c)),
    ];
    for (key, actual) in checks {
        let recorded = r.nested_str("array_sha256", key).map_err(common::fail)?;
        ensure(
            actual == recorded,
            format!("{instance}: array {key} differs from the source model"),
        )?;
    }
    p.problem(&p.c).map(|_| ()).map_err(common::fail)
}

/// The seeded objective of every recorded seed is identical across two generations and has
/// length n (task T013).
fn us1_seeded_objectives_reproducible(instance: &str) -> Result<(), common::Failed> {
    use common::fixtures::{Provenance, provenance_path};
    use common::objectives::seeded_objective;
    let p = Provenance::load(&provenance_path(instance).map_err(common::fail)?)
        .map_err(common::fail)?;
    let n = p.usize("n").map_err(common::fail)?;
    for seed in p.seeds().map_err(common::fail)? {
        let (first, second) = (seeded_objective(seed, n), seeded_objective(seed, n));
        ensure(first.len() == n, "objective length")?;
        let same = first
            .iter()
            .zip(&second)
            .all(|(x, y)| x.to_bits() == y.to_bits());
        ensure(
            same,
            format!("{instance}: seed {seed} gives different objectives"),
        )?;
        ensure(
            first.iter().all(|v| (-1.0..1.0).contains(v)),
            "entry outside [-1, 1)",
        )?;
    }
    Ok(())
}

/// Reference checksum of the seed-1 objective of iDopaNeuroC_problem_LP (task T013): the
/// wrapping sum of the `to_bits()` values of all n entries. Obtained on 2026-11-14 from an
/// independent Python implementation of SplitMix64 and the R13 float conversion, with
/// n = 2942 (the instance's column count); first entries 0.1331231503445618,
/// 0.49156351452540226, 0.9420055071735924.
const LP_SEED_1_CHECKSUM: u64 = 13_446_950_058_244_376_948;

fn us1_seeded_objective_reference_checksum() -> Result<(), common::Failed> {
    use common::fixtures::{Provenance, provenance_path};
    use common::objectives::{bits_checksum, seeded_objective};
    let instance = "iDopaNeuroC_problem_LP";
    let p = Provenance::load(&provenance_path(instance).map_err(common::fail)?)
        .map_err(common::fail)?;
    ensure(
        p.seeds().map_err(common::fail)?.contains(&1),
        "seed 1 not recorded",
    )?;
    let c = seeded_objective(1, p.usize("n").map_err(common::fail)?);
    ensure(
        c[..3] == [0.1331231503445618, 0.49156351452540226, 0.9420055071735924],
        format!("first entries {:?}", &c[..3]),
    )?;
    let sum = bits_checksum(&c);
    ensure(
        sum == LP_SEED_1_CHECKSUM,
        format!("checksum {sum} != {LP_SEED_1_CHECKSUM}"),
    )
}

/// SplitMix64 reproduces the published reference outputs for seed 1234567.
fn us1_splitmix64_reference_vector() -> Result<(), common::Failed> {
    let mut g = common::objectives::SplitMix64::new(1_234_567);
    let got = [g.next_u64(), g.next_u64(), g.next_u64()];
    ensure(
        got == [
            6_457_827_717_110_365_317,
            3_203_168_211_198_807_973,
            9_817_491_932_198_370_423,
        ],
        format!("{got:?}"),
    )
}

/// User Story 3 (T030–T035; analysis findings C1, C2): per-backend solve tests, each written
/// once in `common::us3` against `&dyn Backend` and registered here for every declared
/// backend. Spec US3 scenario 4 (status translation, SC-006) needs no licence and is tested
/// in the backend's own module, not here.
fn us3_trials(registry: &Registry) -> Vec<Trial> {
    use common::us3;
    let mut trials = vec![Trial::test("us3::problems_build", us3::problems_build)];

    // T030 (scenario 1, SC-003): every instance x seeds 1, 2, 3 from its provenance record,
    // deterministic options; plus one solve per instance with Options::default().
    let instances = match common::fixtures::lp_instances() {
        Ok(v) => v,
        Err(e) => {
            trials.push(Trial::test("us3::corpus_declaration", move || {
                Err(common::fail(&e))
            }));
            return trials;
        }
    };
    if instances.len() > us3::MAX_CORPUS_INSTANCES {
        let n = instances.len();
        trials.push(Trial::test("us3::corpus_table_size", move || {
            Err(common::fail(format!(
                "corpus declares {n} instances; common::us3 has {} slots",
                us3::MAX_CORPUS_INSTANCES
            )))
        }));
    }
    let (seeded, defaults) = (us3::seeded_tests(), us3::default_tests());
    for (i, name) in instances.iter().enumerate().take(us3::MAX_CORPUS_INSTANCES) {
        let seeds = common::fixtures::provenance_path(name)
            .and_then(|p| common::fixtures::Provenance::load(&p))
            .and_then(|p| p.seeds());
        match seeds {
            Ok(seeds) if seeds.len() == us3::SEEDS_PER_INSTANCE => {
                for (s, seed) in seeds.iter().enumerate() {
                    let label = format!("us3::t030_corpus_seeded::{name}::seed{seed}");
                    trials.extend(registry.trials(&label, seeded[i][s]));
                }
            }
            other => {
                let msg = format!(
                    "{name}: expected {} seeds, got {other:?}",
                    us3::SEEDS_PER_INSTANCE
                );
                trials.push(Trial::test(format!("us3::t030_seeds::{name}"), move || {
                    Err(common::fail(&msg))
                }));
            }
        }
        let label = format!("us3::t030_corpus_default_options::{name}");
        trials.extend(registry.trials(&label, defaults[i]));
    }

    let per_backend: Vec<(&str, common::BackendTest)> = vec![
        // T031 (scenario 2; edge cases).
        (
            "us3::t031_infeasible_certificate",
            us3::t031_infeasible_certificate,
        ),
        (
            "us3::t031_unbounded_certificate",
            us3::t031_unbounded_certificate,
        ),
        ("us3::t031_ranged_rows", us3::t031_ranged_rows),
        ("us3::t031_empty_column", us3::t031_empty_column),
        ("us3::t031_zero_rows", us3::t031_zero_rows),
        ("us3::t031_zero_columns", us3::t031_zero_columns),
        ("us3::t031_free_row", us3::t031_free_row),
        // T032 (scenario 3).
        ("us3::t032_time_limit_zero", us3::t032_time_limit_zero),
        // T033 (scenario 5; constitution III).
        (
            "us3::t033_raw_parameter_effective",
            us3::t033_raw_parameter_effective,
        ),
        (
            "us3::t033_default_solve_is_silent",
            us3::t033_default_solve_is_silent,
        ),
        (us3::QUIET_CHILD, us3::t033_quiet_child),
        // T034 (scenario 7).
        (
            "us3::t034_cancel_from_progress",
            us3::t034_cancel_from_progress,
        ),
        (
            "us3::t034_progress_panic_propagates",
            us3::t034_progress_panic_propagates,
        ),
        // T035 (scenario 6).
        (
            "us3::t035_one_backend_three_problems",
            us3::t035_one_backend_three_problems,
        ),
        // C1: caller buffers.
        (
            "us3::c1_solve_into_matches_solve",
            us3::c1_solve_into_matches_solve,
        ),
        (
            "us3::c1_solve_into_wrong_lengths",
            us3::c1_solve_into_wrong_lengths,
        ),
        // C2: problem classes outside the backend's capabilities.
        ("us3::c2_milp_rejected", us3::c2_milp_rejected),
        ("us3::c2_qp_rejected", us3::c2_qp_rejected),
    ];
    for (name, test) in per_backend {
        trials.extend(registry.trials(name, test));
    }
    trials
}

/// Feature 002 per-backend tests, written once in `common::f002` and registered for every
/// declared backend.
fn f002_trials(registry: &Registry) -> Vec<Trial> {
    use common::f002;
    let per_backend: Vec<(&str, common::BackendTest)> = vec![
        // User Story 2 (T014, T015).
        ("f002::t014_certificate_sign", f002::t014_certificate_sign),
        ("f002::t015_callback_thread", f002::t015_callback_thread),
        // User Story 4 (T029).
        ("f002::t029_corpus_qp", f002::t029_corpus_qp),
        ("f002::t029_small_qps", f002::t029_small_qps),
        (
            "f003::t018_nonconvex_q_is_an_error",
            f002::t018_nonconvex_q_is_an_error,
        ),
        // User Story 5 (T034).
        ("f002::t034_conic_cases", f002::t034_conic_cases),
        ("f002::t034_conic_infeasible", f002::t034_conic_infeasible),
    ];
    let mut trials = Vec::new();
    for (name, test) in per_backend {
        trials.extend(registry.trials(name, test));
    }
    trials.extend(registry.trials("f002::t033_clp_solve_into", f002::t033_clp_solve_into));
    // User Story 5 (T033): CLP fixtures, licensed tier and heavy tier.
    match (
        common::fixtures::clp_instances(),
        common::fixtures::clp_heavy_instances(),
    ) {
        (Ok(licensed), Ok(heavy)) => {
            for name in licensed {
                let label = format!("f002::t033_clp_fixture::{name}");
                let test: common::BackendClosure =
                    std::sync::Arc::new(move |b| f002::t033_clp_fixture(b, &name));
                trials.extend(registry.trials_dyn(&label, test));
            }
            for name in heavy {
                let label = format!("f002::t033_clp_fixture_heavy::{name}");
                let test: common::BackendClosure =
                    std::sync::Arc::new(move |b| f002::t033_clp_fixture(b, &name));
                trials.extend(registry.heavy_trials_dyn(&label, test));
            }
        }
        (a, b) => {
            let msg = format!("CLP declaration: {:?} {:?}", a.err(), b.err());
            trials.push(Trial::test("f002::clp_declaration", move || {
                Err(common::fail(&msg))
            }));
        }
    }
    // User Story 3 (T019, SC-001): cross-solver agreement on the LP corpus.
    trials.extend(registry.pair_trials("f002::t019_lp_agreement", f002::t019_lp_agreement));
    trials
}

/// T013 (spec FR-002): every fixture declared in `project.yml` appears, in backticks, in
/// `docs/test-corpus.md`, and every fixture listed in the document's summary table exists
/// (LP and small fixtures as MPS, the derived QP as its provenance record, CLP fixtures in the
/// declared format).
fn us1_corpus_document_matches_declaration() -> Result<(), common::Failed> {
    use common::fixtures::{
        clp_heavy_instances, clp_instances, fixtures_dir, lp_instances, qp_instances, repo_root,
    };
    let path = repo_root().join("docs").join("test-corpus.md");
    let doc = std::fs::read_to_string(&path)
        .map_err(|e| common::fail(format!("{}: {e}", path.display())))?;
    let mut declared = Vec::new();
    for list in [
        lp_instances(),
        qp_instances(),
        clp_instances(),
        clp_heavy_instances(),
    ] {
        declared.extend(list.map_err(common::fail)?);
    }
    for name in &declared {
        ensure(
            doc.contains(&format!("`{name}`")),
            format!("{name} is declared in project.yml but not in docs/test-corpus.md"),
        )?;
    }
    // Summary table rows: | `name` | class | ...
    let dir = fixtures_dir().map_err(common::fail)?;
    let mut listed = 0;
    for line in doc.lines() {
        let Some(rest) = line.strip_prefix("| `") else {
            continue;
        };
        let Some((name, _)) = rest.split_once('`') else {
            continue;
        };
        let candidates = [
            dir.join(format!("{name}.mps")),
            dir.join("small").join(format!("{name}.mps")),
            dir.join("small").join(format!("{name}.jtask")),
            dir.join("qp").join(format!("{name}.provenance.json")),
            dir.join("clp").join(format!("{name}.jtask")),
        ];
        ensure(
            candidates.iter().any(|c| c.is_file()),
            format!("docs/test-corpus.md lists {name}, but no fixture file exists for it"),
        )?;
        listed += 1;
    }
    ensure(
        listed >= declared.len(),
        format!("only {listed} fixtures listed in the tables"),
    )
}

/// Feature 003 T014: every toy instance on every declared backend.
fn toy_trials(registry: &Registry) -> Vec<Trial> {
    match common::fixtures::toy_instances() {
        Ok(names) => names
            .into_iter()
            .flat_map(|name| {
                let label = format!("toy::{name}");
                let test: common::BackendClosure =
                    std::sync::Arc::new(move |b| common::toy::toy_trial(b, &name));
                registry.trials_dyn(&label, test)
            })
            .collect(),
        Err(e) => vec![Trial::test("toy::declaration", move || {
            Err(common::fail(&e))
        })],
    }
}
