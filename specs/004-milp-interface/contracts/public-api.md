# Contract: Public API Changes In Feature 004

All additions extend existing types (reuse first, spec SC-003).

```rust
// src/options.rs — Options gains (all optional; solver defaults when unset):
pub mip_relative_gap: Option<f64>,
pub mip_absolute_gap: Option<f64>,
pub node_limit: Option<f64>,
pub solution_limit: Option<u32>,
pub integrality_tolerance: Option<f64>,
pub fixed_model_duals: bool,
#[non_exhaustive] pub enum ProgressPhase { Simplex, Barrier, BranchAndBound, Other }

// src/backend.rs — Capabilities gains:
pub integer_classes: Vec<ProblemClass>,
pub integer_cones: Vec<ConeKind>,

// src/result.rs
pub struct Counts { /* … */ pub nodes: Option<f64> }
impl Solution { pub fn mip_gap(&self) -> Option<(f64, f64)>; }   // (absolute, relative)

// src/verify.rs
pub struct Tolerances { /* … */ pub mip_relative_gap: f64, pub mip_absolute_gap: f64 }
pub mod names { pub const INTEGRALITY: &str; pub const MIP_GAP: &str; }
```

Behaviour: `Problem::with_var_types` (existing) is honoured by both backends; `check_problem`
applies the mixed-integer rule (data-model.md); `random_seed` maps to MOSEK's MIO seed for
mixed-integer problems.
