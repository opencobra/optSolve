# Design notes

Facts below were checked on the reference machine on 2026-11-14 unless marked
otherwise. Claims about third-party crates come from their published
documentation or source and should be re-checked before relying on them.

## 1. Starting point: why Gurobi and MOSEK do not work under RunMat

COBRA (`src/base/solvers/solveCobraLP.m`, `solveCobraQP.m`, `solveCobraMILP.m`)
calls the vendors' MATLAB interfaces directly: `gurobi(model, params)` and
`mosekopt(cmd, prob, param)`.

Installed on the reference machine:

| Solver | Location | Interfaces present |
|---|---|---|
| Gurobi 13.0.3 | `/usr/local/bin/gurobi1303/linux64` | `lib/libgurobi130.so`, `include/gurobi_c.h`, `gurobi_c++.h`, `matlab/gurobi.mexa64` (plus `gurobi_iis`, `gurobi_read`, `gurobi_write`, … MEX files); no `gurobipy` |
| MOSEK 11.2 | `/usr/local/bin/mosektoolslinux64x86/mosek/11.2` | `tools/platform/linux64x86/h/mosek.h`, `libmosek64.so.11.2`, `toolbox/r2019b/mosekopt.mexa64`, Python package under `tools/platform/linux64x86/python/3/mosek` |

Under RunMat, `initCobraToolbox` reports Gurobi as installed but not working,
and "Could not find installation of mosek".

The vendors' MEX binaries cannot load in RunMat. Their dynamic dependencies
are MATLAB's own runtime libraries:

- `gurobi.mexa64` needs `libgurobi130.so`, `libut.so`, `libmx.so`,
  `libmex.so`, `libmat.so`; it imports `mx*`/`mex*` symbols with the `_730`
  suffix (`mxCreateSparse_730`, `mxGetIr_730`, `mxGetJc_730`, …).
- `mosekopt.mexa64` needs `libmosek64.so.11.2`, `libut.so`, `libmx.so`,
  `libmex.so`, `libmat.so`; it imports `_800`-suffixed symbols
  (`mexCallMATLABWithTrap_800`, `mxCreateStructArray_800`, …).

RunMat's MEX support (`runmat-mex`) is source-compatible, not binary
compatible: gateways are compiled against RunMat's own `mex.h`/`matrix.h` with
`runmat mex`, and the loader requires RunMat host symbols
(`runmatMexHostAbiVersion`, `runmatMexBindHost`, `runmatMexInvoke`). Neither
vendor ships the source of its MATLAB gateway.

A separate, unexplained problem (COBRA finding X13): none of the free solvers
COBRA reports as installed (glpk, pdco, quadMinos, dqqMinos, lp_solve) passes
COBRA's validation solve under RunMat. If that is a RunMat defect in COBRA's
solver-validation path, it would affect any new interface too.

## 2. Options considered

| Option | Summary | Outcome |
|---|---|---|
| C gateway compiled with `runmat mex` against each vendor C API | Same calling convention as the vendor MEX | Works, but C code and MEX value conversion |
| MATLAB-code shims calling Python (`gurobipy`, `mosek`) through RunMat's Python adapter | No C | Conversion of every argument across languages; `gurobipy` not installed |
| File based: write MPS/LP, run `gurobi_cl` / MOSEK CLI, parse solution | Simplest | Slow per solve; depends on RunMat file and `system` builtins; loses detail |
| Emulate MATLAB's `libmx`/`libmex` ABI in RunMat so vendor MEX load unchanged | Every vendor MEX would work | Large; emulating MathWorks' binary interface needs legal review |
| **Native Rust, calling the vendors' C APIs** | In-process, typed | **Chosen**, as a standalone crate with a RunMat adapter |

## 3. Survey of Rust solver crates (as published, Nov 2026)

| Crate | Scope | Notes |
|---|---|---|
| [`grb`](https://github.com/ada4a/rust-grb) | Gurobi wrapper | Latest 3.0.1 has features `gurobi10`–`gurobi12`; no Gurobi 13 |
| [`gurobi-rs`](https://crates.io/crates/gurobi-rs) | Gurobi wrapper | Version 1.1.0 has a `gurobi13` feature (used by `oximo`) |
| [`mosek`](https://docs.rs/crate/mosek/latest) | Official MOSEK Optimizer API for Rust | Version tracks MOSEK; 11.2.0 exists (in `oximo`'s `Cargo.lock`); thin wrapper over the C API |
| [`good_lp`](https://github.com/rust-or/good_lp) | LP/MILP modeller | Native backends CBC, HiGHS, SCIP (`russcip`), Clarabel, lpsolve, microlp, CPLEX; Gurobi only via `lp-solvers` (writes `.lp`, runs the CLI); no MOSEK; no QP |
| [`mosekcomodel`](https://docs.rs/mosekcomodel) | Fusion-style conic modeller | Backends MOSEK and HiGHS; no Gurobi |
| [`oximo`](https://github.com/oximo-rs/oximo) | Algebraic modelling, many backends | See section 4 |
| `highs`, `russcip`, `clarabel` | Single-solver crates | Candidates for later free backends |

No crate was found whose main purpose is zero-copy loading of matrix-form
problems into Gurobi or MOSEK.

## 4. Reading of `oximo` (commit `5033df3`, 2026-11-10, version 0.8.0)

- Workspace of 15 crates, MIT OR Apache-2.0, edition 2024, Rust ≥ 1.89. First
  release 0.1.0 on 2026-05-22.
- **Gurobi backend** (`crates/oximo-gurobi`): `gurobi-rs` 1.1.0 with feature
  `gurobi13`; requires Gurobi ≥ 13.0 and `GUROBI_HOME`. In-process C API
  (`Model::with_env`, `set_obj_attr_batch` for RHS/LB/UB/Obj,
  `add_qpterms`).
- **MOSEK backend** (`crates/oximo-mosek`): official `mosek` 11.2.0; requires
  MOSEK 11.2 (`MOSEK_BINDIR_112` if not in the default location). Task API
  (`append_vars`, `put_var_bound`, `put_c_j`, `append_cons`,
  `put_con_bound`, affine conic constraints). Supports LP, MILP, convex QP and
  MIQP, QCP, SOCP/MISOCP, continuous SDP.
- **No public matrix-form entry point.** Models are built from expressions
  (`variable!`, `objective!`, `add_constraint`). Its own MPS reader
  (`oximo-io/src/mps.rs`) uses `#[doc(hidden)]` builders (`__var`,
  `__add_constraint_interval`, `__minimize`, `__maximize`) and builds each row
  as a sum of `coefficient * variable` expression nodes. A source comment says
  these are hidden from the public builder API on purpose.
- **Results** (`SolverResult`): normalised `termination`, `raw_status`, primal
  solutions, row duals, `reduced_costs`, `best_bound`, `gap`, iterations,
  node count, solver version. **No basis status** (no `vbasis`/`cbasis`).
- Conclusion: a good multi-solver modeller, but using it for matrix-form data
  means relying on hidden internals and converting CSC → expressions → solver
  arrays. Not the fastest or lowest-memory path. Worth watching; a public
  matrix-form builder upstream would change this assessment.

## 5. The fastest, lowest-memory design

Pass caller-owned CSC buffers straight to each solver's **bulk** load call.

RunMat's `SparseTensor` (`runmat-value/src/array/sparse.rs`) already has the
layout both solvers load natively: `col_ptrs: usize` (length `cols + 1`),
`row_indices: usize` (zero-based, sorted within each column), values `f64`
(`as_f64_slice()`). The common Rust sparse crates (`sprs` `CsMat` in CSC,
`nalgebra-sparse` `CscMatrix`, `faer` `SparseColMat`) can lend the same three
slices.

| | Gurobi `GRBXloadmodel` | MOSEK `MSK_putacolslice64` + slices |
|---|---|---|
| Column starts | `size_t* vbeg` ← `col_ptrs` as is | `int64 ptrb`, `ptre` ← `col_ptrs[..n]`, `col_ptrs[1..]` as is |
| Row indices | `int* vind` ← convert to `i32` (4 bytes per nonzero, temporary) | `int32 asub` ← same conversion |
| Values | `double* vval` ← as is | `double aval` ← as is |
| Column lengths | `int* vlen` (4 bytes per column) | not needed |
| `c`, `lb`, `ub`, `rhs` | `double*` ← as is | `putcslice`, `putvarboundslice`, `putconboundslice` ← as is, plus bound-key arrays |
| Senses / types | `char*` (`sense`, `vtype`) | bound keys, `putvartypelist` |

Peak extra memory: the solver's internal copy (unavoidable for any in-process
binding) plus the temporary 32-bit index copy, which can be capped by loading
column blocks (`GRBXaddvars` / `MSK_putacolslice64` on ranges) through one
reused buffer.

Further rules:

1. **Raw C bindings** for the hot path (`bindgen` from the installed headers,
   or the raw layer of `gurobi-rs` / the official `mosek` crate), not
   per-variable or per-row wrapper objects. About 20 functions per solver.
2. **Reuse the environment.** `GRBenv` / `MSKenv` creation (including the
   licence check) can cost more than loading a typical model. Keep one per
   process or session; create only the model or task per solve.
3. **No names** unless requested (pass `NULL`).
4. **Results into caller-owned buffers**, or into `Vec`s the caller can adopt:
   `GRBgetdblattrarray` (X, Pi, RC, Slack), `MSK_getxx`, `MSK_gety`,
   `MSK_getslx`/`getsux`, … write directly.
5. **In process.** No subprocess, file exchange or IPC snapshot.
6. QP: `Q` as 32-bit triplets for `GRBaddqpterms`; lower triangle for MOSEK
   `putqobj`.
7. Infinite bounds: Gurobi treats values beyond ±1e100 as infinite (map ±Inf);
   MOSEK expresses them through bound keys.

## 6. Layering: independent crate plus RunMat adapter

1. **optSolver (this repository)**: no RunMat dependency. A borrowed,
   solver-neutral problem description, for example:

   ```rust
   pub struct LpRef<'a> {
       pub n_rows: usize, pub n_cols: usize,
       pub col_ptrs: &'a [usize],      // len n_cols + 1
       pub row_indices: &'a [usize],   // zero-based, sorted per column
       pub values: &'a [f64],
       pub c: &'a [f64], pub col_lb: &'a [f64], pub col_ub: &'a [f64],
       pub rows: RowBounds<'a>,        // sense + rhs, or lower/upper ranges
       pub var_types: Option<&'a [u8]>,// b'C' | b'I' | b'B'
       pub q: Option<CooRef<'a>>,
       pub sense: Sense,
   }
   pub fn solve(env: &GurobiEnv /* or MosekEnv */, lp: &LpRef, opts: &Options)
       -> Result<Solution, Error>;
   ```

   Each solver behind its own cargo feature (`gurobi`, `mosek`, later others).
2. **RunMat adapter** (in or beside RunMat, feature-gated): registers
   `gurobi` and `mosekopt` builtins through RunMat's `inventory`-based builtin
   registry (`runmat-builtins`; a crate outside `runmat-runtime` can register
   builtins this way), reads the struct fields COBRA passes, borrows slices
   from RunMat arrays, calls optSolver, and wraps result vectors as RunMat
   arrays (RunMat array constructors take ownership of a `Vec`).

Platform note: Gurobi's `size_t` column starts match `usize` everywhere;
MOSEK's `int64` matches `usize` only on 64-bit targets (all targets both
solvers support). Check at compile time.

Licensing: optSolver can be MIT/Apache-2.0 but links proprietary libraries.
Users need the vendor's headers, libraries and a licence; the repository must
not ship vendor files. Record the licence and provenance of every external
source.

## 7. Overhead of RunMat compatibility

Essentially none if the core API accepts the solvers' native input forms.

| Possible cost | Cause | Avoidance |
|---|---|---|
| Building row bound arrays | COBRA passes `b` + `csense` ('E'/'L'/'G') | Accept **sense + rhs** as well as ranges (Gurobi's native form) |
| Densifying or converting `A` | A full (dense) `A` | Unavoidable; COBRA's `S` is normally sparse |
| Value class conversion | Sparse logical/integer, integer-class bounds (no `f64` view) | One conversion in the adapter, only then |
| GPU-resident values | Must be gathered to host | Rare for COBRA |
| Blocking the runtime | RunMat builtins are `async`; solves are long | Run on a blocking thread holding cheap reference-counted clones |
| Variable types | COBRA's `vtype` is a char array | Accept bytes (Gurobi's native form) |

Field lookups and slice borrowing cost nanoseconds. The solvers' own internal
copies dominate in every case.

## 8. Other solvers

"Other solvers that provide a Rust interface" identified so far: HiGHS
(`highs` crate), SCIP (`russcip`), Clarabel (pure Rust), CPLEX (`cplex-rs`,
named by `good_lp`). Each needs the same check: does it have a bulk CSC (or
CSR) load call, and which input forms does it take natively?

## 9. Facts checked against the installed headers (2026-11-14)

Checked by reading Gurobi 13.0.3 `gurobi_c.h` and MOSEK 11.2 `mosek.h` on the reference
machine. Signatures are summarised, not copied.

| Fact | Gurobi | MOSEK | Consequence |
|---|---|---|---|
| Bulk column load | `GRBXloadmodel`: `size_t *vbeg`, `int *vlen`, `int *vind`, `double *vval`; `int numvars, numconstrs` | `MSK_putacolslice64`: `MSKint32t first, last`, `const MSKint64t *ptrb, *ptre`, `const MSKint32t *asub`, `const MSKrealt *aval` | Column starts pass through as `usize`/`i64`; row indices need `i32`; dimensions ≤ `i32::MAX` |
| Incremental column load | `GRBXaddvars(model, int numvars, size_t numnz, size_t *vbeg, int *vind, double *vval, ...)` | `MSK_appendvars`, `MSK_appendcons`, then `MSK_putacolslice64` per block; `MSK_putmaxnumanz`, `MSK_putmaxnumvar`, `MSK_putmaxnumcon` preallocate | Column-block chunking is possible on both |
| Const-ness of inputs | Input arrays declared non-`const` (`double *obj`, `size_t *vbeg`, …) | Inputs declared `const` | Passing pointers from shared slices to Gurobi needs vendor documentation that they are input-only (constitution VII-A) |
| Environment creation | `GRBloadenv` and `GRBemptyenv` are **macros** expanding to `GRBloadenvinternal` / `GRBemptyenvinternal` with `GRB_VERSION_MAJOR/MINOR/TECHNICAL`; `GRBstartenv`, `GRBfreeenv` are functions | — | Bindings call the `…internal` functions with version numbers from the header or library (VII-E) |
| Custom allocators | `GRBemptyenvadv` (macro) takes malloc, calloc, realloc, free and thread callbacks | — | A possible way to measure or cap the solver's own memory; not investigated |
| Infinity | `GRB_INFINITY` = `1e100` | `MSK_INFINITY` = `1.0e30`; infinite sides expressed by bound keys | Infinite bounds need backend-specific mapping |
| Solution extraction | `GRBgetdblattrarray(model, attrname, int first, int len, double *values)` | `MSK_getxx(task, whichsol, MSKrealt *xx)`, `MSK_getxxslice` | Results written directly into caller buffers |
| Objective and bounds slices | `GRBsetdblattrarray` | `MSK_putcslice`, `MSK_putvarboundslice(task, first, last, const MSKboundkeye *bkx, const MSKrealt *blx, *bux)` | Model modification without reload |
| Termination | `GRBterminate(model)` | Callback return value | Cancellation from a callback |
| Integer types | `int`, `size_t` | `MSKint32t` = `int32_t`, `MSKint64t` = `int64_t`, `MSKrealt` = `double` | 64-bit targets only |
| Calling convention | Functions declared `__stdcall` | `MSKAPI` | `extern "C"` on the supported 64-bit targets |

Not yet verified (vendor documentation, not headers): whether Gurobi writes through any input
pointer of `GRBXloadmodel`; thread-safety rules for `GRBenv`, `MSKenv_t` and `MSKtask_t`;
whether Gurobi accepts `inf` directly in bounds.
