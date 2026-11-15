# LLM Trial Protocol (SC-002; task T024 — maintainer-run, deferred)

Status: **deferred as debt** (maintainer, 2026-11-15: "will do LLM trial manually"). SC-002 is
unverified until this is run and recorded.

Protocol (research R12, adapted to a manual run): give an LLM of your choice **only**
`docs/llm-interface.md` (no access to the source) and one task statement below; ask for a binary
crate depending on optsolver by path with the `mosek` feature (and/or `gurobi`); build and run
it. Three independent attempts per task (fresh conversation each). Pass: compiles (record the
number of build attempts and any document gaps it exposed), runs, prints the expected status and
objective with verification `Verified`, and does not copy the problem arrays before building the
problem. Record model, prompts, code, compiler output and results under
`results/003-llm-interface-toy-instances/runs/<UTC>-llm-trial/`; a failure is a documentation
defect.

The tasks differ from the document's own examples:

| Task | Statement | Expected |
|---|---|---|
| LP | minimise 2x₁ + 3x₂ + x₃ s.t. x₁ + x₂ + x₃ = 4, x₁ − x₂ ≥ 1, x₂ + x₃ ≤ 3, 0 ≤ xᵢ ≤ 10; hold the matrix in your own arrays and pass them without copying; print status, objective, x, verification | Optimal, 5, x = (1, 0, 3) (toy_lp_mixed) |
| QP | maximise −½(x₁² + x₂²) − x₁ s.t. x₁ + x₂ = 2, x ≥ 0; print status, objective, x, verification | Optimal, −1.75, x = (½, 3/2) (toy_qp_max) |
| Conic | minimise x₀ s.t. x₁ = 1, x₂ = 2, (x₀, x₁, x₂) in the rotated quadratic cone 2x₀x₁ ≥ x₂², x₀, x₁ ≥ 0; print status, objective, x, conic duals, verification | Optimal, 2 (toy_clp_rquad) |

Build environment: `MOSEK_BINDIR=…/mosek/11.2/tools/platform/linux64x86/bin`, the same directory
on `LD_LIBRARY_PATH`, MOSEK licence as usual (Gurobi: `GUROBI_HOME`, `GRB_LICENSE_FILE`).
