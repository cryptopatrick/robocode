# `requires` and `ensures`: SafeShooter

*Mirrors Verus guide: "Pre- and postconditions", "spec functions".*

This chapter is about the main proof of the prototype: **SafeShooter never fires unsafely.**

## Saying what "safe" means

The property is written as **spec functions**: pure mathematical definitions that exist only for verification. From `src/robots/safe_shooter.rs`:

```rust
pub open spec fn aimed(obs: Observation) -> bool {
    obs.target matches Some(t)
        && -AIM_TOLERANCE <= spec_angle_diff(t.bearing.0 as int, obs.gun_heading.0 as int) <= AIM_TOLERANCE
}

pub open spec fn safe_to_fire(obs: Observation, p: Power) -> bool {
    &&& MIN_POWER.0 <= p.0 <= MAX_POWER.0         // a legal bullet power (0.1 to 3.0)
    &&& obs.energy.0 > p.0                        // energy left over after the shot
    &&& obs.gun_heat.0 == 0                       // gun is cool
    &&& obs.target matches Some(t) && t.age == 0  // target scanned this turn
    &&& aimed(obs)                                // gun points at it
}
```

- `spec fn` bodies can use mathematical integers (`int`), which never overflow.
- `&&&` is a bulleted "and", handy for long conjunctions.
- `x matches Some(t) && …` checks the shape and binds `t` for the rest of the expression.
- Why `>` and not `>=`? Firing with *exactly* your remaining energy leaves you at 0, and the engine removes robots at 0. An early version of SafeShooter used `>=` and was proved "safe", and it shot itself dead. A proof is only as good as its spec.

## The theorem

```rust
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires well_formed(*obs),
    ensures r.fire matches Some(p) ==> safe_to_fire(*obs, p),
))]
pub fn decide(obs: &Observation) -> Action { … }
```

Read it as: *for every well-formed observation, if `decide` fires with power `p`, firing is safe.* `well_formed` just says the angles are normalized and the battlefield has a sensible size; the runner always produces observations like that.

Verus proves this for all observations, including every energy level, heat, angle and target combination. And this is the real `decide` that fights in `cargo run --example battle`.

## Watching the proof catch a bug

In `decide`, remove `obs.gun_heat.0 == 0 &&` from the firing condition and verify:

```text
error: postcondition not satisfied
```

Tests might never hit the case where the gun is hot and everything else is ready; Verus finds it at once.

## Using a proven function

Because `decide` has an `ensures`, other code can rely on it without looking inside:

```rust
pub fn check_safe_shooter(obs: &Observation)
    requires well_formed(*obs),
{
    let a = decide(obs);
    if let Some(p) = a.fire {
        assert(obs.gun_heat.0 == 0);
        assert(obs.energy.0 > p.0);
    }
}
```

## Exercises: `exercises/src/ch02.rs`

1. **`careful_fire`** is a simpler trigger that fires `Power(1000)` when everything is ready. Add the `requires` it needs (hint: what does `is_aimed` require?) and an `ensures` using `safe_to_fire`, so `check_careful_fire` verifies.
2. **`check_safe_shooter`** calls the real `decide` but is missing a `requires`. Add it.
