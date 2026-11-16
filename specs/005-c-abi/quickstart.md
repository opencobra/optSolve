# Quickstart: Feature 005 Validation

Prerequisites as feature 002: `GUROBI_HOME`, `MOSEK_BINDIR` and the licence variables set on the
command line; the vendor library directories on `LD_LIBRARY_PATH` for the licensed tier.

1. Unlicensed tier (FR-013, FR-022, SC-006): `cargo test --features capi --test c_interface`.
   Expected: header agreement, version, invalid-input codes, panic containment and leak check pass;
   licensed trials are listed as ignored, naming the missing backend.
2. Licensed tier (SC-001, SC-004, SC-005): `cargo test --features capi,gurobi,mosek --test
   c_interface`. Expected: every toy instance on every supporting backend agrees with the Rust API
   through both the owned and the caller-buffer path; one environment per backend in the reuse
   trial; Gurobi memory-limit trial `StoppedAtLimit`/`Memory`; MOSEK memory limit `Unsupported`.
3. Benchmarks (SC-002, SC-003): `cargo bench --features capi,gurobi,mosek --bench load_solve` (C-path scenarios);
   run bundle under `results/005-c-abi/runs/`.
4. Link demonstration (SC-007): build the release libraries, then compile `tests/c/minimal.c`
   with the flags in `docs/llm-interface.md` ("C interface: building and linking"), static and
   shared, and run it on `tests/fixtures/small/toy_lp_max` data. Commands recorded in the run
   bundle.
5. Whole suite and in-gate checks: `cargo fmt --check`; clippy with `-D warnings` and `cargo test`
   for every feature set — `""`, `gurobi`, `mosek`, `gurobi,mosek`, `capi`, `capi,gurobi`,
   `capi,mosek`, `capi,gurobi,mosek` (additivity: each `capi` set against its counterpart without); MSRV build `cargo +1.85 build --features capi` where the
   toolchain is installed.
