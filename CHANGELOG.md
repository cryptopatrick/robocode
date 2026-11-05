## Unreleased
 - Unreleased changes.

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
