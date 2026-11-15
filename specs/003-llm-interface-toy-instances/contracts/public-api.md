# Contract: Public API Changes In Feature 003

Additions to the `optsolver` public API (feature 002's contract otherwise unchanged).

```rust
// src/conic.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ConeKind {
    Quadratic, RotatedQuadratic, PrimalExponential, DualExponential,
    PrimalPower, DualPower, Nonnegative, Zero, PositiveSemidefinite,
}
impl ConeKind { pub fn name(self) -> &'static str; }
impl Cone { pub fn kind(&self) -> ConeKind; }

// src/backend.rs
pub struct Capabilities { /* … */ pub cones: Vec<ConeKind> }

// crate root
pub use conic::ConeKind;
//! crate-level documentation = docs/llm-interface.md (include_str!)
```

Behaviour changes:

- `check_problem` rejects a cone kind not in `Capabilities::cones` (`Error::Unsupported`, field =
  the kind's name).
- Gurobi: `problem_classes` gains `Qp` and `Conic`; `cones` = quadratic, rotated quadratic,
  nonnegative, zero. QP and conic solutions have no basis and no certificates (research R4).
- Gurobi effective parameters may include `NonConvex` = 0 (Q present) and `QCPDual` = 1 (cones
  present), set only in `gurobi::params::apply`.
