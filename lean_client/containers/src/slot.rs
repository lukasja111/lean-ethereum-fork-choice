use ssz_derive::Ssz;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ssz, Default, Serialize, Deserialize)]
pub struct Slot(pub u64);

impl PartialOrd for Slot {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Slot {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl Slot {
    /// Port of 3SF-mini justifiability rule from the spec notes.
    pub fn is_justifiable_after(self, finalized: Slot) -> bool {
        assert!(self >= finalized, "candidate must not be before finalized");
        let delta = self.0 - finalized.0;
        // <=5 OR perfect square OR pronic (x^2 + x)
        delta <= 5
            || ((delta as f64).sqrt().fract() == 0.0)
            || ((((delta as f64) + 0.25).sqrt().fract() - 0.5).abs() < 1e-12)
    }
}