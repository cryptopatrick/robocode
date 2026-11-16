# robocheck-arena

RoboCheck's extension of **robocode 0.5.0**. It adds what RoboCheck needs without changing robocode: it uses only
robocode's public API, and `robocheck/tests/core_untouched.rs` checks that robocode's own files still match the
`v0.5.0` tag.

| What | Why RoboCheck needs it |
|---|---|
| `Surroundings` (`clearance_ahead`, `clearance_behind`) and `clearance(…)` | "Don't drive into a building" can't be stated with 0.5.0's `Observation`, which knows the walls but not the buildings of the urban map |
| `ArenaRobot` | Runs a candidate `decide(&Observation, &Surroundings) -> Action` as a robocode `Robot`, computing `Surroundings` from the robot's status and the battle's terrain. Target tracking is the same as 0.5.0's `StrategyRobot`, so an `env`-blind robot fights exactly as before (tested) |
| `DecisionRecord`, `DecisionLog` | Every turn's full observation, surroundings and action (`decisions-<arena>.jsonl`), so RoboCheck can find the turn a rule was broken. 0.5.0's recordings keep only part of the observation |
| `is_ready` | An executable twin of SafeShooter's `ready` spec, proved equal to it by Verus, so a rule about readiness can be checked in a battle |
| `OBSTACLE_MARGIN` (80 px), `surroundings_ok` | The starter rule's margin, and the precondition Verus assumes about `Surroundings` |

Run: `cargo test -p robocheck-arena`; Verus: `cargo verus verify --features verus` in this folder.

## Porting into robocode later

1. Add `clearance_ahead` and `clearance_behind` to `Observation` (and `Status`), filled in by `runner::status_of`
   with `clearance` (which needs the battle's terrain), and require them non-negative in `well_formed`.
   New public fields break struct literals, so this is a minor version bump.
2. `ArenaRobot` collapses into `StrategyRobot`; `Decide` becomes `Strategy::decide(&Observation)`.
3. `DecisionRecord`'s fields go into the recording's per-turn decisions.
4. `is_ready` moves next to `is_aimed` in `robots::safe_shooter`.
5. RoboCheck's template then reads `obs.clearance_ahead` where it now reads `env.clearance_ahead`.
