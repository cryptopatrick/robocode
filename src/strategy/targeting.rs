//! Choosing among scanned targets.

#[cfg(feature = "verus")]
use vstd::prelude::*;

use crate::strategy::TargetInfo;

/// Index of the nearest target (the first one on ties), or `None` if empty.
#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    ensures
        r matches Some(i) ==> i < targets@.len()
            && forall|j: int| 0 <= j < targets@.len() ==> targets@[i as int].distance <= targets@[j].distance,
        r is None <==> targets@.len() == 0,
))]
pub fn closest_target(targets: &[TargetInfo]) -> Option<usize> {
    if targets.is_empty() {
        return None;
    }
    let mut best: usize = 0;
    let mut i: usize = 1;
    // `verus_only` (set by Verus): plain builds with the feature never see this loop attribute.
    #[cfg_attr(verus_only, verus_spec(
        invariant
            1 <= i <= targets@.len(),
            best < i,
            forall|j: int| 0 <= j < i ==> targets@[best as int].distance <= targets@[j].distance,
        decreases targets@.len() - i
    ))]
    while i < targets.len() {
        if targets[i].distance < targets[best].distance {
            best = i;
        }
        i += 1;
    }
    Some(best)
}
