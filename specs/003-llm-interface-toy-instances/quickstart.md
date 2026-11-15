# Quickstart: Feature 003 Validation

Prerequisites as feature 002 (`GUROBI_HOME`, `MOSEK_BINDIR`, licences on the command line).

1. Interface document examples (SC-001):
   `cargo test --doc` (compiles every example; solves are skipped without a backend), then
   `cargo test --doc --features gurobi,mosek` (runs them). Expected: all pass.
2. Drift test (FR-004): `cargo test --test standards` includes the document-coverage test.
   Expected: passes; adding an undocumented public item makes it fail.
3. Toy instances (SC-003): `cargo test --features gurobi,mosek --test corpus -- toy`.
   Expected: every toy instance reaches its recorded outcome on each backend supporting its
   class; unsupported combinations report `Unsupported`.
4. Comparison (SC-004): `cargo test --features gurobi,mosek --test corpus -- compare`.
   Expected: one trial per comparable instance passes; non-comparable ones are ignored with the
   reason; the report path is printed. With one feature only: every comparison ignored, naming
   the missing backend.
5. Independent tool checks (SC-005): `tools/corpus/make_toy_instances.m` (MATLAB, both solvers)
   regenerates files bit-identically and re-checks each against both tools.
6. LLM trial (SC-002): at Gate 3, per research R12.
