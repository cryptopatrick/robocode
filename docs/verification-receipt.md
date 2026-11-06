# Verification receipt

What is **proved** by Verus, what is only **tested** (a trusted assumption guarded by a named test), and what is **out of scope**. Modelled on the receipts in Microsoft Research's *Proof-Driven Development* (PDD Kit, 2026).

To check it:
- `cargo verus verify --features verus` (proofs);
- `cargo run -p verus-mutants` (every `ensures` clause rejects a planted bug);
- `cargo test` (the tests named below).

## Proved

| Requirement | Clause | Function | Mutant that must be killed |
|---|---|---|---|
| Firing heats the gun by 1.0–1.6 | `r.0 >= 1000 && r.0 <= 1600` | `units::gun_heat_for_power` | `gun-heat-too-low`, `gun-heat-too-high` |
| Every legal shot does damage | `p.0 >= 100 ==> r.0 > 0` | `units::bullet_damage` | `damage-zero-at-min-power` |
| Damage is at most 16.0 | `r.0 <= 16000` | `units::bullet_damage` | `damage-over-cap` |
| Hit bonus is 3 × power | `r.0 == 3 * p.0` | `units::bullet_hit_bonus` | `hit-bonus-off-by-one` |
| Shortest signed turn | `r == spec_angle_diff(..)`, range, congruence | `units::angle_diff` | `angle-diff-wrong-wrap` |
| "Aimed" means within 3.0° | `r == aimed(*obs)` | `safe_shooter::is_aimed` | `aimed-tolerance-too-wide` |
| SafeShooter never fires unsafely | `r.fire matches Some(p) ==> safe_to_fire(*obs, p)` | `safe_shooter::decide` | `fires-with-hot-gun` |
| SafeShooter never holds fire when ready | `r.fire is Some <==> ready(*obs)` | `safe_shooter::decide` | `never-fires` |
| SafeShooter never drives into a nearby wall (forward) | `a > 0 ==> !heading_into_near_wall(..)` | `safe_shooter::decide`, `movement` | `drives-into-near-wall` |
| … nor backward | `a < 0 ==> !heading_into_near_wall(.., reverse_heading(..))` | `safe_shooter::decide`, `movement` | `reverses-into-near-wall` |
| Wall check matches its spec | `r == heading_into_near_wall(*obs, h)` | `safe_shooter::toward_near_wall` | `wall-margin-off-by-one` |
| Radar turns at most 45° | `-MAX_RADAR_TURN <= r <= MAX_RADAR_TURN` | `safe_shooter::radar_turn` | `radar-turn-over-limit` |
| Closest target is nearest | `targets@[i].distance <= targets@[j].distance` | `targeting::closest_target` | (shared) |
| … and the first on ties | `forall j < i. targets@[j].distance > targets@[i].distance` | `targeting::closest_target` | `closest-picks-last-of-tie` |
| No target from nothing | `r is None <==> targets@.len() == 0` | `targeting::closest_target` | `closest-of-nothing` |
| Executable well-formedness check matches the spec | `r == well_formed(*obs)` | `strategy::is_well_formed` | — |

## Tested (trusted boundary)

These are assumed by the proofs and checked only by tests. Each one is a place where a bug could hide from Verus.

| # | Assumed clause | Where | Guarded by |
|---|---|---|---|
| A1 | `Angle::from_degrees` returns `0 <= r.0 < 3600` | `units.rs`, `external_body` | `tests/boundary.rs::angle_from_degrees_is_always_in_range` |
| A2 | The runner builds only `well_formed` observations (SafeShooter's precondition) | `runner::observe_status` (unverified) | `debug_assert!` in `StrategyRobot::run`; `tests/boundary.rs::runner_observations_are_well_formed` |
| A3 | Observed gun heat 0 ⇔ the engine accepts the shot | `runner::observe_status` (post-cooling conversion) | `tests/runner.rs::observed_cool_gun_matches_engine_firing` |
| A4 | Observed energy never exceeds the engine's (floor) | `units::Energy::from_f64` | `tests/units.rs::energy_rounds_down` |
| A5 | A target's bearing points at the target | `geometry::angle_to_point` (f64 `atan2`) | `tests/scanning.rs::radar_sweep_finds_robot_in_arc` |
| A6 | `energy > p` at the start of a turn keeps the robot alive after firing | engine semantics (`handle_firing`, `process_deaths`) | `tests/robots.rs::firing_never_kills_the_shooter` |

## Out of scope

- The `f64` physics engine (movement, collisions, bullets), checked only against pinned battle results (`golden_*` tests).
- Energy lost *within* a turn, for example to a wall hit during movement before firing. A6 only covers firing.
- Robot API robots (`robocode::api`), which use `f64` like the original game.
- Which power SafeShooter chooses, how its gun tracks, and how its radar locks. These are behaviour, not safety, and are covered by ordinary tests.
- Verus, Z3 and rustc themselves (the trusted computing base).

## Tests pruned because a proof now covers them

| Removed test | Replaced by |
|---|---|
| `fires_when_safe` | `r.fire is Some <==> ready(*obs)` |
| `never_fires_when_hot` | `safe_to_fire` (gun is cool) |
| `never_fires_without_target` | `safe_to_fire` (target seen this turn) |
| `aimed_across_zero` | `angle_diff` spec plus `is_aimed == aimed` |
| `not_aimed_outside_tolerance` | `is_aimed == aimed` |
| `energy_boundary` | `safe_to_fire` (`energy > p`) and `ready` (`energy > 1000`) |
| fire assertion of `never_fires_on_stale_target` | `safe_to_fire` (age 0); the gun-tracking check stays as `keeps_tracking_a_stale_target_with_the_gun` |
