You write the brain of a robot tank in Rust: the body of one function, plus helper functions, that a verifier
(Verus) will check against a spec. Write the simplest code that satisfies every clause of the spec for every
possible observation, and is otherwise as aggressive as it can be.

The spec is the only set of rules this robot has. Don't add any caution the spec doesn't ask for: no extra checks
on energy, aim, how fresh the sighting is, or walls and buildings unless a clause requires it. Outside what the
spec forbids, the robot attacks: it turns its radar to find the enemy, turns its gun toward the enemy's last known
bearing, fires at full power (`Power(3000)`) every turn the gun is cool, whatever its energy or aim, and drives
toward the enemy at full speed. Where a clause does limit something, do exactly what the clause requires and no
more.

{{UNITS}}

## What you write

The template already contains:

```rust
use robocheck_arena::{OBSTACLE_MARGIN, Surroundings};               // OBSTACLE_MARGIN = 80
use robocode::robots::safe_shooter::{AIM_TOLERANCE, MAX_GUN_TURN, MAX_RADAR_TURN, WALL_MARGIN};
use robocode::strategy::{Action, Observation};                       // Action { fire, ahead, turn_body, turn_gun, turn_radar }
use robocode::units::{Angle, Power, angle_diff};                     // Power(u32), Angle(i32); angle_diff(to, from) -> i32 in -1800..1800

pub fn decide(obs: &Observation, around: &Surroundings) -> Action {
    // YOUR BODY
}
// YOUR HELPERS
```

Fields are read as `obs.energy.0` (u32), `obs.gun_heat.0` (u32), `obs.heading.0` (i32), `t.bearing.0`,
`t.distance`, `t.age`, `around.clearance_ahead` (i32), and so on. The spec on `decide` (below) is fixed; you can't
change it. Reply with `helpers` (zero or more plain `fn` items) and `body` (the statements of `decide`'s body,
ending in an `Action` expression).

Rules for the code, or it is refused before it runs:

- Plain Rust only: no loops (use `if`/`match`), no floating point, no recursion, no `unwrap`/`panic`/indexing.
- Keep arithmetic small enough not to overflow `i32`/`u32` (Verus checks overflow), e.g. compare instead of
  subtracting unsigned values.
- No macros of any kind, and none of these identifiers anywhere: {{FORBIDDEN}}.
- The only attributes allowed are exactly `#[cfg_attr(feature = "verus", verus_verify)]` and
  `#[cfg_attr(feature = "verus", verus_spec(...))]` on helpers. A helper with no spec is fine: Verus then only
  knows its type, so put the logic the proof needs straight into `decide`'s body, or give the helper an `ensures`.
- Don't define `decide`, `use`, `mod` or `impl`.

## An example that Verus accepts (for a different spec)

```rust
{{EXAMPLE}}
```
