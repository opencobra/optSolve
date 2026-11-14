# Open questions and suggested first features

## To decide

1. **Crate and repository name.** The repository is `optSolver`; the crate name
   (for example `optsolver`) and whether it is one crate with features or a
   workspace (`optsolver-core`, `optsolver-gurobi`, `optsolver-mosek`) is open.
2. **Bindings.** Generate our own `-sys` bindings with `bindgen` from the
   installed headers, or depend on `gurobi-rs` (raw layer, `gurobi13`) and
   the official `mosek` 11.2 crate. Criterion: access to the bulk loaders
   (`GRBXloadmodel`, `GRBXaddvars`, `MSK_putacolslice64`) without per-element
   wrapper overhead.
3. **Licence** of optSolver (proposed MIT OR Apache-2.0, as RunMat and
   `oximo`).
4. **Problem classes in scope for version 1**: LP and MILP first; QP (convex)
   next; QCP/SOCP later?
5. **Result content**: primal, duals, reduced costs, slacks, objective, raw
   and normalised status, MIP gap and bound, timings; whether basis status
   (Gurobi `VBasis`/`CBasis`, MOSEK `skc`/`skx`) is needed (COBRA usage to be
   checked).
6. **Index chunk size** for the 32-bit index conversion, and whether to offer
   a no-chunk path when memory is plentiful.
7. **Threading and environments**: one environment per process or per caller;
   thread safety of reusing `GRBenv` / `MSKenv` across solves.
8. **Where the RunMat adapter lives**: in the RunMat repository behind a
   feature, or in a separate crate depending on both.

## To verify

- Every C function used, against the vendors' documentation for Gurobi 13.0.3
  and MOSEK 11.2 (signatures, index widths, infinity handling).
- RunMat numeric array constructors take ownership of a `Vec<f64>` without
  copying (index buffers do: `HostIndexBuffer::new(Vec)` →
  `HostData::from_vec`).
- The fields of the Gurobi `model`/`params`/result structs and the MOSEK
  `prob`/`res` structs that COBRA's `solveCobraLP`, `solveCobraQP` and
  `solveCobraMILP` read and write. This list is the specification for the
  RunMat adapter.
- COBRA finding X13 (free solvers fail validation under RunMat): diagnose in
  the RunMat repository before relying on end-to-end COBRA tests.

## Benchmarks wanted

- Load time and peak resident memory for large models (for example Recon3D
  and larger genome-scale models) through: optSolver; the vendor MEX in MATLAB
  (outside this repository); `oximo` via its MPS reader.
- Environment creation cost versus model load cost.

## Suggested first features (SpecKit)

1. `/speckit-constitution`: principles drawn from this exploration
   (performance and memory first, zero-copy borrowed inputs, no RunMat
   dependency, solver backends behind features, provenance and licence
   recording, benchmarks as acceptance criteria).
2. Feature 001: core types (`LpRef`, row-bound forms, `Solution`, status) and
   the Gurobi LP/MILP backend with bulk load and environment reuse.
3. Feature 002: MOSEK LP/MILP backend.
4. Feature 003: QP for both.
5. Later: HiGHS, SCIP, Clarabel backends; the RunMat adapter (RunMat side).
