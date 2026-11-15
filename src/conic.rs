//! Affine conic constraints Fx + g ∈ K (data-model.md of feature 002, Cones; constitution
//! XII-C notation F, g, K, s, k).
//!
//! K is a product of cones K_1 × … × K_p; cone i applies to the consecutive block of rows of
//! Fx + g that it covers, and the dimensions sum to k, the number of rows of F. Every array
//! is borrowed from the caller and validated once by [`ConicRef::new`]. The block violation
//! functions here are used by [`crate::verify`] to judge primal (Fx + g ∈ K) and dual
//! (σs ∈ K*) cone membership.

#![forbid(unsafe_code)]
#![allow(non_snake_case)] // Notation Convention (XII-C): F is a matrix.

use crate::error::{Error, Result};
use crate::problem::CscRef;

/// One cone of the product K (data-model.md, Cones).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Cone {
    /// Quadratic (second-order) cone of dimension d ≥ 1: u₁ ≥ ‖u₂:d‖₂.
    Quadratic(usize),
    /// Rotated quadratic cone of dimension d ≥ 2: 2u₁u₂ ≥ ‖u₃:d‖₂², u₁, u₂ ≥ 0.
    RotatedQuadratic(usize),
    /// Primal exponential cone (dimension 3): closure of {u₁ ≥ u₂ exp(u₃/u₂), u₂ > 0}.
    PrimalExponential,
    /// Dual exponential cone (dimension 3): closure of {u₁ ≥ −u₃ exp(u₂/u₃ − 1), u₃ < 0}.
    DualExponential,
    /// Primal power cone: ∏_{i≤ℓ} u_i^{α_i} ≥ ‖u_{ℓ+1:d}‖₂, u_{1:ℓ} ≥ 0, with ℓ = α.len() and α
    /// normalised to sum 1.
    PrimalPower {
        /// Dimension d.
        dim: usize,
        /// Exponents of the leading ℓ coordinates (positive; normalised by optSolver).
        alpha: Vec<f64>,
    },
    /// Dual power cone: ∏_{i≤ℓ} (u_i/α_i)^{α_i} ≥ ‖u_{ℓ+1:d}‖₂, u_{1:ℓ} ≥ 0 (α normalised).
    DualPower {
        /// Dimension d.
        dim: usize,
        /// Exponents of the leading ℓ coordinates (positive; normalised by optSolver).
        alpha: Vec<f64>,
    },
    /// Nonnegative orthant of dimension d ≥ 1.
    Nonnegative(usize),
    /// The zero cone {0} of dimension d ≥ 1 (its dual is all of ℝ^d).
    Zero(usize),
    /// Positive semidefinite cone of order r ≥ 1, in the svec layout of dimension r(r+1)/2:
    /// the lower triangle column by column, off-diagonal entries multiplied by √2.
    PositiveSemidefinite {
        /// Order r.
        order: usize,
    },
}

/// The kind of a cone, without its parameters (feature 003, ADR-0003): what a backend declares
/// it accepts in [`crate::Capabilities::cones`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ConeKind {
    /// [`Cone::Quadratic`].
    Quadratic,
    /// [`Cone::RotatedQuadratic`].
    RotatedQuadratic,
    /// [`Cone::PrimalExponential`].
    PrimalExponential,
    /// [`Cone::DualExponential`].
    DualExponential,
    /// [`Cone::PrimalPower`].
    PrimalPower,
    /// [`Cone::DualPower`].
    DualPower,
    /// [`Cone::Nonnegative`].
    Nonnegative,
    /// [`Cone::Zero`].
    Zero,
    /// [`Cone::PositiveSemidefinite`].
    PositiveSemidefinite,
}

impl ConeKind {
    /// Every kind.
    pub const ALL: [ConeKind; 9] = [
        ConeKind::Quadratic,
        ConeKind::RotatedQuadratic,
        ConeKind::PrimalExponential,
        ConeKind::DualExponential,
        ConeKind::PrimalPower,
        ConeKind::DualPower,
        ConeKind::Nonnegative,
        ConeKind::Zero,
        ConeKind::PositiveSemidefinite,
    ];

    /// Stable name, used in errors and reports (for example `"rotated quadratic"`).
    pub fn name(self) -> &'static str {
        match self {
            ConeKind::Quadratic => "quadratic",
            ConeKind::RotatedQuadratic => "rotated quadratic",
            ConeKind::PrimalExponential => "primal exponential",
            ConeKind::DualExponential => "dual exponential",
            ConeKind::PrimalPower => "primal power",
            ConeKind::DualPower => "dual power",
            ConeKind::Nonnegative => "nonnegative",
            ConeKind::Zero => "zero",
            ConeKind::PositiveSemidefinite => "positive semidefinite",
        }
    }
}

impl Cone {
    /// The kind of this cone.
    pub fn kind(&self) -> ConeKind {
        match self {
            Cone::Quadratic(_) => ConeKind::Quadratic,
            Cone::RotatedQuadratic(_) => ConeKind::RotatedQuadratic,
            Cone::PrimalExponential => ConeKind::PrimalExponential,
            Cone::DualExponential => ConeKind::DualExponential,
            Cone::PrimalPower { .. } => ConeKind::PrimalPower,
            Cone::DualPower { .. } => ConeKind::DualPower,
            Cone::Nonnegative(_) => ConeKind::Nonnegative,
            Cone::Zero(_) => ConeKind::Zero,
            Cone::PositiveSemidefinite { .. } => ConeKind::PositiveSemidefinite,
        }
    }

    /// Dimension of the cone (number of rows of Fx + g it covers).
    pub fn dim(&self) -> usize {
        match self {
            Cone::Quadratic(d)
            | Cone::RotatedQuadratic(d)
            | Cone::Nonnegative(d)
            | Cone::Zero(d)
            | Cone::PrimalPower { dim: d, .. }
            | Cone::DualPower { dim: d, .. } => *d,
            Cone::PrimalExponential | Cone::DualExponential => 3,
            Cone::PositiveSemidefinite { order } => order.saturating_mul(order + 1) / 2,
        }
    }

    /// The dual cone K_i* (data-model.md, Cones). The dual of the zero cone is the free cone,
    /// which has no variant: [`Cone::dual_violation`] handles it.
    fn dual_kind(&self) -> Option<Cone> {
        Some(match self {
            Cone::PrimalExponential => Cone::DualExponential,
            Cone::DualExponential => Cone::PrimalExponential,
            Cone::PrimalPower { dim, alpha } => Cone::DualPower {
                dim: *dim,
                alpha: alpha.clone(),
            },
            Cone::DualPower { dim, alpha } => Cone::PrimalPower {
                dim: *dim,
                alpha: alpha.clone(),
            },
            Cone::Zero(_) => return None,
            other => other.clone(),
        })
    }

    /// Checks the cone's own parameters (data-model.md, Cones).
    fn validate(&self, index: usize) -> Result<()> {
        let bad = |detail: String| {
            Err(Error::Invalid {
                what: "cones",
                detail: format!("cone {index}: {detail}"),
            })
        };
        match self {
            Cone::Quadratic(0) | Cone::Nonnegative(0) | Cone::Zero(0) => bad("dimension 0".into()),
            Cone::RotatedQuadratic(d) if *d < 2 => bad(format!(
                "rotated quadratic cone of dimension {d} (minimum 2)"
            )),
            Cone::PositiveSemidefinite { order: 0 } => bad("semidefinite cone of order 0".into()),
            Cone::PositiveSemidefinite { order } if order.checked_mul(order + 1).is_none() => {
                bad(format!("semidefinite cone order {order} too large"))
            }
            Cone::PrimalPower { dim, alpha } | Cone::DualPower { dim, alpha } => {
                if alpha.is_empty() {
                    return bad("power cone without exponents".into());
                }
                if alpha.len() >= *dim {
                    return bad(format!(
                        "power cone of dimension {dim} with {} exponents (must be fewer)",
                        alpha.len()
                    ));
                }
                if let Some(a) = alpha.iter().find(|a| !(a.is_finite() && **a > 0.0)) {
                    return bad(format!(
                        "power cone exponent {a} is not positive and finite"
                    ));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Violation of membership u ∈ K_i for a block u of length [`Cone::dim`] (data-model.md,
    /// Verification, block violations): 0 inside the cone, positive outside, +∞ for NaN or a
    /// block of the wrong length.
    pub fn violation(&self, u: &[f64]) -> f64 {
        if u.len() != self.dim() || u.iter().any(|v| v.is_nan()) {
            return f64::INFINITY;
        }
        let v = match self {
            Cone::Quadratic(_) => (norm2(&u[1..]) - u[0]).max(0.0),
            Cone::RotatedQuadratic(_) => {
                let t = norm2(&u[2..]);
                let q = (t * t - 2.0 * u[0] * u[1]) / (1.0 + norm_inf(u));
                q.max(-u[0]).max(-u[1]).max(0.0)
            }
            Cone::PrimalExponential => exp_violation(u[0], u[1], u[2]),
            Cone::DualExponential => dual_exp_violation(u[0], u[1], u[2]),
            Cone::PrimalPower { alpha, .. } => power_violation(u, alpha, false),
            Cone::DualPower { alpha, .. } => power_violation(u, alpha, true),
            Cone::Nonnegative(_) => u.iter().fold(0.0_f64, |a, &v| a.max(-v)),
            Cone::Zero(_) => norm_inf(u),
            Cone::PositiveSemidefinite { order } => {
                (-min_eigenvalue(&unsvec(u, *order), *order)).max(0.0)
            }
        };
        if v.is_nan() { f64::INFINITY } else { v }
    }

    /// Violation of membership u ∈ K_i* (the dual cone).
    pub fn dual_violation(&self, u: &[f64]) -> f64 {
        match self.dual_kind() {
            Some(dual) => dual.violation(u),
            // The dual of {0} is ℝ^d: only NaN or a wrong length can violate it.
            None if u.len() == self.dim() && !u.iter().any(|v| v.is_nan()) => 0.0,
            None => f64::INFINITY,
        }
    }
}

/// Primal exponential cone, closure included: for u₂ > 0, u₁ ≥ u₂ exp(u₃/u₂); for u₂ ≤ 0 the
/// closure {u₁ ≥ 0, u₂ = 0, u₃ ≤ 0}.
fn exp_violation(u1: f64, u2: f64, u3: f64) -> f64 {
    if u2 > 0.0 {
        let need = u2 * (u3 / u2).exp();
        (need - u1).max(-u1).max(0.0)
    } else {
        (-u2).max(-u1).max(u3).max(0.0)
    }
}

/// Dual exponential cone, closure included: for u₃ < 0, u₁ ≥ −u₃ exp(u₂/u₃ − 1); for u₃ ≥ 0
/// the closure {u₁ ≥ 0, u₂ ≥ 0, u₃ = 0}.
fn dual_exp_violation(u1: f64, u2: f64, u3: f64) -> f64 {
    if u3 < 0.0 {
        let need = -u3 * (u2 / u3 - 1.0).exp();
        (need - u1).max(-u1).max(0.0)
    } else {
        u3.max(-u1).max(-u2).max(0.0)
    }
}

/// Power cones with exponents normalised to sum 1; `dual` divides each leading coordinate by
/// its exponent first.
fn power_violation(u: &[f64], alpha: &[f64], dual: bool) -> f64 {
    let l = alpha.len();
    let total: f64 = alpha.iter().sum();
    let mut negative = 0.0_f64;
    let mut log_product = 0.0;
    for (&ui, &ai) in u[..l].iter().zip(alpha) {
        let a = ai / total;
        negative = negative.max(-ui);
        let base = if dual { ui / a } else { ui };
        log_product += if base > 0.0 {
            a * base.ln()
        } else {
            f64::NEG_INFINITY
        };
    }
    let product = log_product.exp();
    (norm2(&u[l..]) - product).max(negative).max(0.0)
}

fn norm2(v: &[f64]) -> f64 {
    let scale = norm_inf(v);
    if scale == 0.0 || !scale.is_finite() {
        return scale;
    }
    scale * v.iter().map(|x| (x / scale).powi(2)).sum::<f64>().sqrt()
}

fn norm_inf(v: &[f64]) -> f64 {
    v.iter().fold(0.0_f64, |a, &x| a.max(x.abs()))
}

/// svec⁻¹: the symmetric r × r matrix (row-major, full) whose svec is u.
fn unsvec(u: &[f64], r: usize) -> Vec<f64> {
    let mut x = vec![0.0; r * r];
    let mut k = 0;
    for j in 0..r {
        for i in j..r {
            let v = if i == j {
                u[k]
            } else {
                u[k] / std::f64::consts::SQRT_2
            };
            x[i * r + j] = v;
            x[j * r + i] = v;
            k += 1;
        }
    }
    x
}

/// Smallest eigenvalue of a symmetric r × r matrix (row-major) by cyclic Jacobi rotations.
fn min_eigenvalue(a: &[f64], r: usize) -> f64 {
    if r == 0 || a.len() != r * r {
        return 0.0;
    }
    let mut a = a.to_vec();
    for _sweep in 0..100 {
        let off: f64 = (0..r)
            .flat_map(|i| (0..r).filter(move |&j| j != i).map(move |j| (i, j)))
            .map(|(i, j)| a[i * r + j] * a[i * r + j])
            .sum();
        let diag: f64 = (0..r).map(|i| a[i * r + i] * a[i * r + i]).sum();
        if off <= 1e-30 * diag.max(f64::MIN_POSITIVE) || off == 0.0 {
            break;
        }
        for p in 0..r {
            for q in p + 1..r {
                let apq = a[p * r + q];
                if apq == 0.0 {
                    continue;
                }
                let theta = (a[q * r + q] - a[p * r + p]) / (2.0 * apq);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..r {
                    let akp = a[k * r + p];
                    let akq = a[k * r + q];
                    a[k * r + p] = c * akp - s * akq;
                    a[k * r + q] = s * akp + c * akq;
                }
                for k in 0..r {
                    let apk = a[p * r + k];
                    let aqk = a[q * r + k];
                    a[p * r + k] = c * apk - s * aqk;
                    a[q * r + k] = s * apk + c * aqk;
                }
            }
        }
    }
    (0..r).map(|i| a[i * r + i]).fold(f64::INFINITY, f64::min)
}

/// Borrowed, validated affine conic constraints Fx + g ∈ K.
#[derive(Debug, Clone, Copy)]
pub struct ConicRef<'a> {
    pub(crate) F: CscRef<'a, usize>,
    pub(crate) g: &'a [f64],
    pub(crate) cones: &'a [Cone],
}

impl<'a> ConicRef<'a> {
    /// Validates and borrows conic data: F is k × n (validated as any CSC matrix), g has
    /// length k with finite values, every cone is well formed (data-model.md) and the cone
    /// dimensions sum to k. The number of columns n is checked against the problem by
    /// [`crate::Problem::with_conic`].
    pub fn new(F: CscRef<'a, usize>, g: &'a [f64], cones: &'a [Cone]) -> Result<Self> {
        let k = F.m;
        if g.len() != k {
            return Err(Error::Invalid {
                what: "g",
                detail: format!("length is {}, expected k = {k} (rows of F)", g.len()),
            });
        }
        if let Some((i, v)) = g.iter().enumerate().find(|(_, v)| !v.is_finite()) {
            return Err(Error::Invalid {
                what: "g",
                detail: format!("non-finite value {v} at index {i}"),
            });
        }
        let mut total: usize = 0;
        for (i, cone) in cones.iter().enumerate() {
            cone.validate(i)?;
            total = total.checked_add(cone.dim()).ok_or(Error::Invalid {
                what: "cones",
                detail: "dimensions overflow".into(),
            })?;
        }
        if total != k {
            return Err(Error::Invalid {
                what: "cones",
                detail: format!("cone dimensions sum to {total}, expected k = {k} (rows of F)"),
            });
        }
        Ok(Self { F, g, cones })
    }
    /// The matrix F (k × n).
    pub fn F(&self) -> &CscRef<'a, usize> {
        &self.F
    }
    /// The offset g (length k).
    pub fn g(&self) -> &'a [f64] {
        self.g
    }
    /// The cones, in row order.
    pub fn cones(&self) -> &'a [Cone] {
        self.cones
    }
    /// Number of conic rows k.
    pub fn k(&self) -> usize {
        self.F.m
    }
    /// The cones with the first row each covers.
    pub fn blocks(&self) -> impl Iterator<Item = (usize, &'a Cone)> + 'a {
        let cones: &'a [Cone] = self.cones;
        cones.iter().scan(0usize, |start, cone| {
            let s = *start;
            *start += cone.dim();
            Some((s, cone))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_eigenvalue_of_known_matrices() {
        // [[2, 1], [1, 2]] has eigenvalues 1 and 3.
        let e = min_eigenvalue(&[2.0, 1.0, 1.0, 2.0], 2);
        assert!((e - 1.0).abs() < 1e-12, "{e}");
        // diag(3, -1, 2).
        let e = min_eigenvalue(&[3.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 2.0], 3);
        assert!((e + 1.0).abs() < 1e-12, "{e}");
        // [[4,1,0],[1,3,1],[0,1,2]]: eigenvalues 3 ± √3 and 3.
        let e = min_eigenvalue(&[4.0, 1.0, 0.0, 1.0, 3.0, 1.0, 0.0, 1.0, 2.0], 3);
        assert!((e - (3.0 - 3f64.sqrt())).abs() < 1e-12, "{e}");
    }

    #[test]
    fn svec_round_trip() {
        // X = [[1, 2], [2, 5]]: svec = (1, 2√2, 5).
        let x = unsvec(&[1.0, 2.0 * std::f64::consts::SQRT_2, 5.0], 2);
        assert!((x[1] - 2.0).abs() < 1e-15 && (x[2] - 2.0).abs() < 1e-15);
    }
}
