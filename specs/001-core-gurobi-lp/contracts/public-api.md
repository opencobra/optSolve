# Contract: Public Rust API Of The `optsolver` Crate (Feature 001)

Signatures are the contract; bodies are not specified here. Field names follow the Notation
Convention (`A`, `c`, `b`, `l`, `u`, `x`, `y`, `z`, `gamma`, `b_l`, `b_u`); `non_snake_case` is
allowed on these public types only (constitution VII-F). Entities and rules: data-model.md.

## Solver-independent core (always compiled)

```rust
pub trait RowIndex: sealed::Sealed + Copy {}          // implemented for usize and i32 only
impl RowIndex for usize {}  impl RowIndex for i32 {}

pub struct CscRef<'a, I: RowIndex> { /* private: col_ptrs, row_indices, values */ }
impl<'a, I: RowIndex> CscRef<'a, I> {
    pub fn new(m: usize, n: usize, col_ptrs: &'a [usize], row_indices: &'a [I],
               values: &'a [f64]) -> Result<Self, Error>;
    pub unsafe fn new_unchecked(/* same */) -> Self;   // # Safety: all CscRef::new invariants
}

pub enum RowBounds<'a> {
    SenseRhs { sense: &'a [u8], b: &'a [f64] },        // b'=' | b'<' | b'>'
    Ranges   { b_l: &'a [f64], b_u: &'a [f64] },
}
pub enum Sense { Minimise, Maximise }

#[non_exhaustive]
pub struct Problem<'a, I: RowIndex> {
    pub A: CscRef<'a, I>, pub c: &'a [f64], pub gamma: f64, pub sense: Sense,
    pub rows: RowBounds<'a>, pub l: &'a [f64], pub u: &'a [f64],
    pub var_types: Option<&'a [u8]>,                   // reserved (MILP)
    pub Q: Option<QRef<'a>>,                           // reserved (QP)
}
impl<'a, I: RowIndex> Problem<'a, I> { pub fn new(/* … */) -> Result<Self, Error>; }

#[non_exhaustive] #[derive(Default)]
pub struct Options<'cb> {                              // defaults module: constitution VII-G
    pub time_limit: Option<f64>, pub iteration_limit: Option<f64>, pub threads: Option<u32>,
    pub primal_feasibility_tolerance: Option<f64>, pub dual_feasibility_tolerance: Option<f64>,
    pub method: Option<Method>, pub presolve: Option<Presolve>, pub random_seed: Option<i32>,
    pub solver_log: Option<bool>, pub certificates: bool, pub array_outputs: Option<bool>,
    pub verification: Option<bool>, pub raw: Vec<(String, ParamValue)>,
    pub cancel: Option<&'cb AtomicBool>,
    pub progress: Option<&'cb mut dyn FnMut(&Progress)>,
}
pub enum ParamValue { Int(i64), Double(f64), Str(String) }

#[non_exhaustive] pub enum Status { Optimal, Infeasible, Unbounded, InfeasibleOrUnbounded,
    StoppedAtLimit, NumericalDifficulty, Interrupted, LocallyOptimal, LocallyInfeasible,
    NotSolved, Unknown }
#[non_exhaustive] pub enum LimitReason { Time, Iterations, Nodes, Solutions, ObjectiveCutoff,
    ObjectiveTarget, Work, Memory }
pub enum SolutionReturned { None, Feasible, Optimal }

#[non_exhaustive]
pub struct Solution {
    pub status: Status, pub limit_reason: Option<LimitReason>, pub returned: SolutionReturned,
    pub x: Vec<f64>, pub y: Vec<f64>, pub z: Vec<f64>, pub row_activity: Vec<f64>,
    pub basis: Option<Basis>, pub objective: Option<f64>, pub bound: Option<f64>,
    pub counts: Counts, pub timings: Timings, pub certificate: Option<Certificate>,
    pub raw_status: Vec<(String, i64)>, pub solver: SolverIdentity,
    pub attributes: Vec<(String, ParamValue)>, pub effective_parameters: Vec<(String, ParamValue)>,
    pub verification: Option<VerificationReport>,
}
pub struct SolutionBuffers<'b> { pub x: &'b mut [f64], pub y: Option<&'b mut [f64]>,
    pub z: Option<&'b mut [f64]>, pub row_activity: Option<&'b mut [f64]> }

pub fn verify<I: RowIndex>(problem: &Problem<'_, I>, solution: &SolutionView<'_>,
                           tolerances: &Tolerances) -> VerificationReport;

pub trait Backend {                                    // introduced here; second implementation: MOSEK
    fn capabilities(&self) -> Capabilities;
    fn solve<I: RowIndex>(&self, p: &Problem<'_, I>, o: &mut Options<'_>) -> Result<Solution, Error>;
    fn solve_into<I: RowIndex>(&self, p: &Problem<'_, I>, o: &mut Options<'_>,
                               out: SolutionBuffers<'_>) -> Result<SolutionMeta, Error>;
}

#[non_exhaustive] pub enum Error { Invalid { what: &'static str, detail: String },
    TooLarge { what: &'static str, value: usize }, Unsupported { field: &'static str, backend: &'static str },
    ParameterConflict { name: String }, Solver { code: i32, message: String } }
```

## Gurobi backend (feature `gurobi`)

```rust
pub mod gurobi {
    pub struct Env { /* NonNull<GRBenv>; !Send, !Sync (research R15) */ }
    impl Env { pub fn new() -> Result<Env, Error>; }  // licence check happens here, once
    impl Backend for Env { /* … */ }
}
```

`Backend` is a trait over one implementation in this feature; constitution V forbids extracting
a cross-backend abstraction before a second backend exists. The plan therefore keeps it
**provisional**: it is what the shared test harness calls, and its shape is re-examined when the
MOSEK feature implements it (Complexity Tracking).
