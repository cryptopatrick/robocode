//! Chapter 4: is your spec strong enough?
use vstd::prelude::*;

verus! {

// Exercise 4.1: the weak spec `100 <= r <= 3000` is also met by "always
// return 100". Strengthen it so `check_clamp` verifies.
pub fn clamp_power(p: u32) -> (r: u32)
    ensures
        100 <= r <= 3000,
        100 <= p <= 3000 ==> r == p,
        p < 100 ==> r == 100,
        p > 3000 ==> r == 3000,
{
    if p < 100 { 100 } else if p > 3000 { 3000 } else { p }
}

pub fn check_clamp() {
    let r = clamp_power(1500);
    assert(r == 1500);
    let r = clamp_power(50);
    assert(r == 100);
}

// Exercise 4.2: both readings. "Only fire when ready" is also met by "never
// fire". Add the other direction so `check_should_fire` verifies.
pub fn should_fire(heat: u32, energy: u32) -> (r: bool)
    ensures
        r <==> (heat == 0 && energy > 1000),
{
    heat == 0 && energy > 1000
}

pub fn check_should_fire() {
    let ready = should_fire(0, 5000);
    assert(ready);
    let hot = should_fire(1, 5000);
    assert(!hot);
}

} // verus!
