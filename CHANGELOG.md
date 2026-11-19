# Changelog

## v0.6.0 - 2026-11-18
 - **Buildings block the radar** (centre-line rule): a robot is scanned only if the line between the two robot centres touches no building. This changes urban battles. On urban maps SafeShooter often never sees its opponent and so never fires. Open-arena battles are unchanged: a 0.5.0 open-arena recording is identical apart from `"version"`. New public `line_of_sight` and `physics::radar::radar_beam`.
 - **Radar beam** drawn as it is blocked: the 2D viewer shows the polygon the radar swept and could see, cut off at buildings. The 3D view does not draw it yet.
 - **Recording format version 2** (viewers read 1 and 2): adds `beam` to each robot in urban battles only, and an optional `intent` (`{"maneuver","risk"}`) in `decisions`. Urban recordings are about 61–78% larger gzipped because of the beam, which is recorded in full on every turn (an accepted cost). Intent adds about 4% when Tactician plays. Open-arena recordings without intent are the same as version 1 apart from `"version"`.
 - **Battlefield ratios** 4:3 (800 x 600), 2:1 (1200 x 600), 16:9 (1067 x 600) and 1:1 (600 x 600), each with its own urban map. Pick one in the viewer's New battle, or with `cargo robocode battle --ratio`.
 - *Note* `Terrain::urban(w, h)` now returns the ratio-specific map for 1200 x 600, 1067 x 600 and 600 x 600, so urban battles at those sizes get different buildings than in 0.5.0. Other sizes keep the 0.5.0 layout.
 - **Tactician**, the fifth built-in robot (with SafeShooter, Circler, Spinner and SittingDuck): it picks a maneuver each turn by priority with hysteresis, uses Search/Patrol when no target is seen, and has stuck recovery and a wall guard.
 - **Maneuver library** (`robocode::strategy::maneuver`): `Maneuver` (search, patrol, defensive, evade, attack), `Risk` profiles (cautious, normal, aggressive), `Intent`, a fire filter, and shot detection from the target's energy drop. The library includes stuck recovery and a 30 px wall guard on every maneuver. Shot detection is a heuristic: the energy drop is the only bullet information, so it causes false flinches (see `docs/robot-api.md`).
 - Viewer: each robot's maneuver shows in its label and in the robot panel.
 - *Breaking* `RobotSnapshot` gained a public `beam: Option<Vec<Point>>` field, so constructing it as a struct literal no longer compiles.
 - *Breaking* `roster::BUILT_IN` grew from `[&str; 4]` to `[&str; 5]` (Tactician).
 - New `Strategy::intent` hook (a default method returning `None`, so existing strategies are unaffected).
 - Release metadata test; CI builds the fuzz targets.

## v0.5.0 - 2026-11-12
 - *Breaking* New public field `TurnEvents::bullet_hit_obstacles` and new `BulletHitObstacle` type; `BattleState` and `EventSnapshot` gained fields.
 - **Urban arena** (opt-in): a fixed map of buildings and roads. Buildings stop tanks at their face (no damage) and absorb bullets, raising the new `BulletHitObstacle` event (`TurnEvents::bullet_hit_obstacles`). Roads are display only. Buildings do not block radar. Set with `runner::Battle::set_terrain(terrain::Terrain::urban(w, h))`. Recordings gain an optional `terrain` header key; battles without buildings are unchanged.
 - `cargo robocode battle --urban` fights on the city map.
 - Viewer: a **3D view** (three.js, loaded on demand) next to the 2D view, and an *Urban* option in New battle.

## v0.4.0 - 2026-11-12
 - **Live battles** (real-time with decoupled rendering, like Java Robocode): `live::Pacer`, `live::LiveSession`, and `recording::{Frame, RecordingBuilder, header_json, frame, initial_frame}`. The viewer runs new battles live in a Web Worker (pause, step, speed incl. Max, measured turns/s, Back to live). `cargo robocode battle --live [--tps N|max]` streams a native battle over Server-Sent Events. Live and precomputed battles are identical turn for turn.
 - Optional per-robot time budgets with Java-style skipped turns: `Battle::set_time_budget`, `on_skipped_turn`, `BattleResults::skipped_turns`.
 - `Battle::current_outcome`.
 - See `docs/architecture.md`.
 - *Breaking* New public field `BattleResults::skipped_turns`.
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
