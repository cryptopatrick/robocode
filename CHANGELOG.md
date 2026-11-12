## Unreleased
 - **Live battles** (real-time with decoupled rendering, like Java Robocode): `live::Pacer`, `live::LiveSession`, and `recording::{Frame, RecordingBuilder, header_json, frame, initial_frame}`. The viewer runs new battles live in a Web Worker (pause, step, speed incl. Max, measured turns/s, Back to live). `cargo robocode battle --live [--tps N|max]` streams a native battle over Server-Sent Events. Live and precomputed battles are identical turn for turn.
 - Optional per-robot time budgets with Java-style skipped turns: `Battle::set_time_budget`, `on_skipped_turn`, `BattleResults::skipped_turns`.
 - `Battle::current_outcome`.
 - See `docs/architecture.md`.
 - Robocode scoring: `runner::Score`, `BattleResults::scores`, `Battle::scores()`; `cargo robocode battle --rounds` prints scores.
 - Viewer: robot labels stay inside the arena's side edges.
 - `cargo verify` aliases keep Verus's build in `target/verus`.
 - CI on `actions/checkout` and `actions/setup-node` v7 (Node 24).

## v0.3.0 - 2026-11-06
 - *Breaking* New variants in the public `api::Event` enum (`HitRobot`, `BulletHitBullet`) and new public fields on `RobotControl`, `RobotState`, `TurnEvents` and `BulletState`.
 - Robot-to-robot collisions with Java's at-fault rule, and `on_hit_robot`.
 - Bullet-on-bullet collisions and `on_bullet_hit_bullet`.
 - Java turret mode (`TurretMode::Java`, `set_adjust_gun_for_robot_turn`, `set_adjust_radar_for_robot_turn`, `set_adjust_radar_for_gun_turn`) for robots ported from Java.
 - `set_max_velocity` and `set_max_turn_rate`, following Java (absolute value, NaN ignored).
 - Multi-round battles: `Battle::run_rounds`, `BattleResults`, and `on_death`, `on_win`, `on_round_ended`.
 - Verus: SafeShooter now provably fires if and only if it is ready, never drives into a nearby wall, and `closest_target` picks the first of a tie. An executable `strategy::is_well_formed` is proved equal to its spec. A trusted contract covers `Angle::from_degrees`.
 - Mutation checks for every `ensures` clause (development tool, `cargo run -p verus-mutants`), plus a verification receipt in `docs/verification-receipt.md`.

## v0.2.0 - 2026-11-05
 - *Breaking* License changed to EPL-1.0 (the license of the original Robocode).
 - *Breaking* `runner::Battle::new` takes `Vec<Box<dyn api::Robot>>`. Wrap fixed-point strategies with `runner::boxed`.
 - Robot API (`robocode::api`): write robots with event handlers (`on_scanned_robot`, `on_hit_by_bullet`, `on_bullet_hit`, `on_bullet_missed`, `on_hit_wall`, `on_robot_death`) and `set_*` commands, modelled on `AdvancedRobot`.
 - Strategy layer (`robocode::strategy`, `robocode::units`): fixed-point `decide(&Observation) -> Action` robots that Verus can verify.
 - Optional `verus` feature: Verus specs and proofs. `SafeShooter` is proved never to fire unsafely. Without the feature no Verus crate is compiled.
 - Radar scanning, missed-bullet events, and a battle runner with winner detection.
 - Sample robots: `SafeShooter`, `Circler`, `Spinner`, `SittingDuck`.
 - `recording` (battle recordings, format v1), `roster` (built-in robots and spawn points), and the `robot!` macro for `cargo robocode` robot files.
 - Snapshot JSON writes NaN and infinity as `null`.

## v0.1.0 - 2025-01-01
 - Initial Release

Flags: *Note*, *Breaking*:
