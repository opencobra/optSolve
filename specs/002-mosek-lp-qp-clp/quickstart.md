# Quickstart: Validating Feature 002

Prerequisites as feature 001, plus MOSEK 11.2 with `MOSEKLM_LICENSE_FILE` set and its
`tools/platform/linux64x86/bin` directory on the build and run-time library paths
(`MOSEK_BINDIR` read by the build script), and the Rust 1.85.0 toolchain for the
minimum-version check.

1. `cargo test` — no solver: every Gurobi and MOSEK trial is reported ignored with the reason.
2. `cargo test --features gurobi,mosek` — every contract and corpus test runs on both backends;
   cross-solver LP agreement on iDopaNeuroC_problem_LP seeds 1, 2, 3; QP and CLP tests on MOSEK;
   the certificate-sign and callback-thread tests on both backends; heavy trials ignored.
3. `OPTSOLVER_HEAVY_TESTS=1 cargo test --features gurobi,mosek` — adds the iDopaNeuroC CLP
   problems; each reaches its recorded expected status and, when optimal, verifies.
4. `cargo +1.85.0 test --all-targets --features gurobi,mosek` — the minimum-version build.
5. `cargo bench --features gurobi,mosek --bench load_solve` — SC-004 and SC-005 per backend.
