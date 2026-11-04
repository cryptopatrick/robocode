# Specs on integers

*Mirrors Verus guide: "Basic specifications", "Integers and arithmetic".*

## `requires` and `ensures`

A function's **precondition** (`requires`) is what callers must guarantee. Its **postcondition** (`ensures`) is what it promises back. Verus checks both: the body must establish the `ensures` whenever the `requires` hold, and every caller must establish the `requires`.

In the Verus guide's syntax:

```rust
verus! {
pub fn my_gun_heat(p: u32) -> (r: u32)
    requires p <= 3000,
    ensures 1000 <= r <= 1600,
{
    1000 + p / 5
}
}
```

The `(r: u32)` names the return value so the `ensures` can talk about it.

## Callers only see the spec

Verus checks each function on its own. When `check_gun_heat` calls `my_gun_heat(3000)`, Verus does **not** look inside `my_gun_heat`; it only knows the `ensures`. So this assertion verifies only because of the postcondition:

```rust
let h = my_gun_heat(3000);
assert(h <= 1600);
```

Delete the `ensures` and the assertion fails, even though the code didn't change. Specs are the interface.

## Arithmetic overflow

Verus checks that machine arithmetic can't overflow. In `4 * p` with `p: u32`, a large `p` overflows. RoboCode's rules say bullet power is at most 3.0 (`3000`), so `requires p <= 3000` both states the rule and lets Verus prove `4 * p` is safe.

## Implications in specs

`==>` means "implies". RoboCode allows powers from 0.1 upward, so "damage is positive" only holds from `100` up:

```rust
ensures p >= 100 ==> r > 0,
```

## Exercises: `exercises/src/ch01.rs`

1. **`my_gun_heat`**: add a `requires` and an `ensures` so `check_gun_heat` verifies.
2. **`my_bullet_damage`** (`4p + 2·max(p − 1000, 0)`): add a `requires` that rules out overflow, and an `ensures` strong enough for `check_bullet_damage`'s two assertions.

Check with `cargo verus verify -p roboverus-exercises`. The errors for chapter 1 disappear when you're done.
