# Quickstart: Feature 004 Validation

Prerequisites as feature 003.

1. Contract (no solver): `cargo test --test contract` — mixed-integer capability rule, gap,
   integrality and gap measures, fixed-problem duals in verification.
2. Readers: `cargo test --test corpus -- mps jtask` — integer markers, BV/LI/UI, JTASK `"int"`.
3. Toy instances and comparison: `cargo test --features gurobi,mosek --test corpus -- toy compare`
   — every mixed-integer toy reaches its outcome on each supporting backend; comparisons agree
   (compatible statuses reported).
4. Fixed-model duals and limits: `cargo test --features gurobi,mosek --test corpus -- f004`.
5. Document: `cargo test --doc --features gurobi,mosek` (MILP example) and
   `cargo test --test standards` (drift test).
6. Benchmark: `cargo bench --features gurobi,mosek` (MILP scenario).
