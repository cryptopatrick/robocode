//! Chapter 3: loops and invariants.
use robocode::strategy::TargetInfo;
use vstd::prelude::*;

verus! {

// Exercise 3.1: find the weakest target (lowest energy). Write the loop invariant.
pub fn weakest_target(targets: &[TargetInfo]) -> (r: Option<usize>)
    ensures
        r matches Some(i) ==> i < targets@.len()
            && forall|j: int| 0 <= j < targets@.len() ==> targets@[i as int].energy.0 <= targets@[j].energy.0,
        r is None <==> targets@.len() == 0,
{
    if targets.len() == 0 {
        return None;
    }
    let mut best: usize = 0;
    let mut i: usize = 1;
    while i < targets.len()
        invariant
            1 <= i <= targets@.len(),
            best < i,
            forall|j: int| 0 <= j < i ==> targets@[best as int].energy.0 <= targets@[j].energy.0,
        decreases targets@.len() - i,
    {
        if targets[i].energy.0 < targets[best].energy.0 {
            best = i;
        }
        i += 1;
    }
    Some(best)
}

} // verus!
