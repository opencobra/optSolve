# Contract: Public API Changes In Feature 002

Additions to the `optsolver` crate's public API (feature 001's reconciled contract,
`specs/001-core-gurobi-lp/contracts/public-api.md`, otherwise unchanged). Entities and
conventions: data-model.md.

## Core (always compiled)

```rust
// src/conic.rs (new)
#[non_exhaustive]
pub enum Cone {
    Quadratic(usize), RotatedQuadratic(usize),
    PrimalExponential, DualExponential,                 // dimension 3
    PrimalPower { dim: usize, alpha: Vec<f64> },        // alpha.len() = number of leading coordinates
    DualPower { dim: usize, alpha: Vec<f64> },
    Nonnegative(usize), Zero(usize),
    PositiveSemidefinite { order: usize },              // dimension order(order + 1) / 2, svec layout
}
impl Cone { pub fn dim(&self) -> usize; }

pub struct ConicRef<'a> { /* private: F: CscRef<'a, usize>, g: &'a [f64], cones: &'a [Cone] */ }
// F's row indices index the k conic rows; MOSEK takes them as int64, so usize passes through.
impl<'a> ConicRef<'a> { pub fn new(F: CscRef<'a, usize>, g: &'a [f64], cones: &'a [Cone]) -> Result<Self, Error>; }

// src/problem.rs (changed)
impl<'a, I: RowIndex> Problem<'a, I> {
    pub fn with_Q(self, Q: CscRef<'a, usize>) -> Result<Self, Error>;   // now validated: lower triangle
    pub fn with_conic(self, conic: ConicRef<'a>) -> Result<Self, Error>;
    pub fn conic(&self) -> Option<&ConicRef<'a>>;
}

// src/result.rs (changed)
pub struct Solution { /* … existing fields … */ pub s: Vec<f64> }        // conic duals, length k
pub enum Certificate { Farkas { y, z, s }, UnboundedRay { r } }          // s added (empty if k = 0)
pub struct SolutionBuffers<'b> { /* … */ pub s: Option<&'b mut [f64]> }

// src/backend.rs (changed)
pub enum ProblemClass { Lp, Milp, Qp, Conic }                            // Conic added
```

## MOSEK backend (feature `mosek`)

```rust
pub mod mosek {
    pub struct Env { /* NonNull<MSKenv>; !Send, !Sync; licence checked out on creation */ }
    impl Env { pub fn new() -> Result<Env, Error>; }
    impl Backend for Env { /* LP, QP, Conic */ }
    #[doc(hidden)] pub use solve::LoadReport;     // benchmark positive control, as for Gurobi
}
```

## Gurobi backend

Capabilities unchanged (LP only); `check_problem` now also rejects conic problems for it.
