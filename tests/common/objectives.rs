//! Seeded objectives for the frozen corpus (research R13).
//!
//! The corpus instances store a zero objective; tests solve them with random objectives
//! generated here from the seeds recorded in each provenance record. The generator is
//! SplitMix64 (integer arithmetic only), so an objective is identical on every platform and
//! does not depend on a random-number crate's float conversion.

/// The SplitMix64 pseudo-random generator.
#[derive(Debug, Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// A generator started from `seed`.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next 64-bit output.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// The next value, uniform on [−1, 1): `(next_u64() >> 11) · 2⁻⁵³ · 2 − 1`.
    pub fn next_signed_unit(&mut self) -> f64 {
        const TWO_POW_MINUS_53: f64 = 1.0 / (1u64 << 53) as f64;
        (self.next_u64() >> 11) as f64 * TWO_POW_MINUS_53 * 2.0 - 1.0
    }
}

/// The objective c of length n generated from `seed`.
pub fn seeded_objective(seed: u64, n: usize) -> Vec<f64> {
    let mut g = SplitMix64::new(seed);
    (0..n).map(|_| g.next_signed_unit()).collect()
}

/// Wrapping sum of the IEEE 754 bit patterns of `v`: a compact checksum that changes when
/// any entry changes in any bit.
pub fn bits_checksum(v: &[f64]) -> u64 {
    v.iter().fold(0u64, |acc, x| acc.wrapping_add(x.to_bits()))
}
