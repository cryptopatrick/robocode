//! Chapter 1: specs on integers.
use vstd::prelude::*;

verus! {

// Exercise 1.1: gun heat. Write `requires`/`ensures` so `check_gun_heat` verifies.
pub fn my_gun_heat(p: u32) -> (r: u32)
    requires
        p <= 3000,
    ensures
        1000 <= r <= 1600,
{
    1000 + p / 5
}

pub fn check_gun_heat() {
    let h = my_gun_heat(3000);
    assert(h <= 1600);
    let h = my_gun_heat(100);
    assert(h >= 1000);
}

// Exercise 1.2: bullet damage `4p + 2·max(p − 1000, 0)` in milli-units.
pub fn my_bullet_damage(p: u32) -> (r: u32)
    requires
        p <= 3000,
    ensures
        p >= 100 ==> r > 0,
        r <= 16000,
{
    let extra = if p > 1000 { 2 * (p - 1000) } else { 0 };
    4 * p + extra
}

pub fn check_bullet_damage(p: u32)
    requires
        100 <= p <= 3000,
{
    let d = my_bullet_damage(p);
    assert(d > 0);
    assert(d <= 16000);
}

} // verus!
