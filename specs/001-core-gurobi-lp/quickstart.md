# Quickstart: Validating Feature 001

Validation scenarios proving the feature end to end. Entities: data-model.md. API:
contracts/public-api.md.

## Prerequisites

- Rust 1.85 or later (reference machine: 1.91.1).
- For the Gurobi tier: Gurobi 13.0.3 installed, `GUROBI_HOME` set, a valid licence, and the
  Gurobi library directory on the run-time library path.
- For re-extracting fixtures only: MATLAB with Gurobi's MATLAB interface, and the variational
  kinetics repository with its frozen corpus.

## 1. Unlicensed tier (any machine)

```bash
cargo test                       # core, verification, MPS reader, seeded objectives
```

Expected: all pass; every Gurobi corpus test is listed as ignored with the reason
"gurobi: not compiled" (SC-002).

## 2. Licensed tier

```bash
cargo test --features gurobi
```

Expected: 3 corpus solves (iDopaNeuroC_problem_1 × seeds 1, 2, 3) report Optimal with a Verified report;
small infeasible and unbounded fixtures report Infeasible and Unbounded with verified
certificates; the status translation test covers codes 1–19 and an unknown code (SC-003,
SC-006).

## 3. Fixture provenance

```bash
cargo test --test corpus provenance   # hashes in each provenance record match the fixtures
```

Re-extraction (maintainer only): run the extraction script named in plan.md from MATLAB; it
rewrites each MPS file and provenance record and fails unless the Gurobi round trip is
bit-for-bit identical.

## 4. Benchmarks

```bash
cargo bench --features gurobi --bench load_solve   # writes a run bundle under results/
```

Expected: per instance, phase timings (median and spread over N = 5), Rust heap peak during
load, peak resident memory rise, Gurobi `MaxMemUsed`, effective parameters, versions, commit,
machine, profile; SC-004 and SC-005 evaluated and stated per instance.
