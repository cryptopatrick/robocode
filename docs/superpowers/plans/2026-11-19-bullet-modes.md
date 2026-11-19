# Bullet Modes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Two bullet modes: realistic (the default), where bullets fly at a fixed 300 px/turn, and arcade, which keeps today's rules. Both modes get a new look: a tracer, a muzzle flash and an impact flash.

**Architecture:** `BulletMode` lives on `BattleConfig`. It sets bullet speed in `rules`, is shown to robots through `Observation`, and is written to the recording header. The mode reaches the engine through the CLI, wasm and New Battle. The viewer draws tracers from the bullets' recorded positions.

**Tech Stack:** Rust (edition 2024, MSRV 1.85), wasm-bindgen, React 18 + TS + Vite + Vitest, three.js, Tailwind.

**Spec:** `docs/superpowers/specs/2026-11-19-bullet-modes-design.md`

## Global Constraints

- `BulletMode { Realistic, Arcade }`, with `Default = Realistic`, `name()` returning `"realistic"`/`"arcade"`, and `FromStr`.
- `REALISTIC_BULLET_SPEED = 300.0`. Arcade speed stays `20 − 3·power`. Power still sets damage, gun heat, energy cost and hit bonus, all unchanged.
- An arcade battle must be byte-identical to 0.6.0. `tests/fixtures/open_arena_0_5_0.json` and every existing snapshot pass with arcade pinned.
- Header field `"bullets"`: new recordings always write it. A missing field means arcade. The recording version stays 2.
- The PRD (`_dev/PRD.md`) gets a "Bullet modes" section in Task 1, before any code changes.
- Web: Tailwind only, no inline styles. Commits: no co-author trailers.

## Review Focus

1. A fast bullet must not tunnel through a robot (Task 2: `fast_bullet_hits_robot_it_would_jump`).
2. A robot in front of a building is hit, and a building in front of a robot absorbs the bullet (Task 2).
3. An old recording with no `bullets` field replays as arcade (Task 4: `missing_bullets_field_is_arcade`).
4. A live stream header carries `bullets` (Task 4: `live_header_carries_bullets`).
5. In 2D, a bullet's first turn draws the tracer from the muzzle, not from (0,0) (Task 5).

---

### Task 1: PRD, `BulletMode` and speed rule

**Files:**
- Modify: `_dev/PRD.md`
- Modify: `src/rules.rs`
- Modify: `src/physics/engine.rs` (`BattleConfig.bullet_mode`)
- Modify: `src/physics/bullet.rs:35`
- Modify: `src/lib.rs` (re-export)
- Modify: existing tests and benches that depend on the speed (pin Arcade)

**Produces:** `pub enum BulletMode`, `rules::bullet_speed(power: f64, mode: BulletMode) -> f64`, `rules::REALISTIC_BULLET_SPEED`, `BattleConfig { bullet_mode }` (default Realistic), and `Battle::bullet_mode()`.

- [ ] Add the PRD section. Arcade is the Robocode-identical mode. Realistic is a documented deviation and the default.
- [ ] Write the tests first:
  - `default_mode_is_realistic`
  - `realistic_speed_is_constant`: power 0.1 and 3.0 both give 300
  - `arcade_speed_matches_robocode`
  - `realistic_bullet_moves_300_per_turn`: a one-turn battle
- [ ] Implement. Pin `BulletMode::Arcade` wherever existing tests, snapshots, the 0.5.0 fixture comparison, examples and property tests assume classic speed. Add a short comment at each pin.
- [ ] Run `cargo test --workspace`, then clippy with `-D warnings`, then `cargo fmt --all -- --check`. All must pass.
- [ ] Commit: `feat: bullet modes, realistic by default`.

### Task 2: Earliest contact along the bullet's path

**Files:** `src/physics/engine.rs` (`apply_bullets` and robot hit checks), and `tests/bullets.rs` (create it, or extend an existing test file).

For each bullet, the outcome this turn is whichever comes first along the segment from `previous_position` to `position`: a building contact (`Rect::first_contact`), a robot circle (segment versus circle of radius `ROBOT_RADIUS`, excluding the bullet's owner), or bullet-on-bullet. On a tie, the robot wins. Leaving the field happens last.

In arcade mode, every existing fixture and snapshot must stay byte-identical. If the strict ordering breaks one, keep today's order in arcade and apply earliest-contact only in realistic. Record which you chose in the report.

- [ ] Write the tests first:
  - `fast_bullet_hits_robot_it_would_jump`: a robot sits midway along a 300 px step.
  - `robot_before_building_is_hit`
  - `building_before_robot_absorbs`
  - `arcade_outcomes_unchanged`: the existing fixture.
- [ ] Implement. Run the workspace tests, clippy and fmt.
- [ ] Commit: `feat: bullets stop at the first thing on their path`.

### Task 3: Robots see the mode

**Files:** `src/strategy/mod.rs` (`Observation.bullet_mode: BulletMode`, `Observation.bullet_speed_for(power: Power) -> f64` or a stored speed), wherever `Observation` is built in `src/`, `docs/robot-api.md`, and robocheck if it constructs `Observation`.

- [ ] Write the tests first:
  - the observation reports the battle's mode;
  - `bullet_speed_for` gives the right speed in both modes.
- [ ] Implement. Update every `Observation` literal in the tests, robocheck and the fuzz targets.
- [ ] Run `cargo test --workspace`, then clippy, then fmt.
- [ ] Commit: `feat: robots know the bullet mode`.

### Task 4: Mode in the header, the CLI, wasm and New Battle

**Files:**
- Rust:
  - `src/recording.rs`: `robots_and_field` writes `,"bullets":"<name>"`;
  - `cli/` `--bullets realistic|arcade`, default realistic;
  - `wasm/src/lib.rs`: `BattleSpec.bullets`, plus a `bullets: &str` argument on `run_battle`, `run_battle_native` and `LiveBattle::new`/`create`, where `""` means realistic.
- Web:
  - `web/src/recording.ts`: `bullets?: 'realistic' | 'arcade'`, and `bulletMode(rec)`, which returns `'arcade'` when the field is missing;
  - `wasm.ts`, `engine.worker.ts` and `live.ts` (`LiveHeader` Pick);
  - `NewBattle.tsx`: a Bullets select, default Realistic, with `BattleSetup.bullets`;
  - `App.tsx`.
- Docs: `docs/viewer.md` (the format field), `docs/` CLI usage.

The 0.5.0 fixture comparison: with the header now always writing `bullets`, the arcade fixture will differ by that field. Compare after removing `,"bullets":"arcade"` and the version number, and document this in the test.

- [ ] Write the tests first:
  - `bullets_in_header`
  - `live_header_carries_bullets`
  - the CLI flag parses, and the default is realistic
  - wasm `run_battle_passes_bullets`
  - TS `missing_bullets_field_is_arcade`
  - NewBattle: the default is Realistic, and the picked mode reaches `onRun`
- [ ] Implement. Run `npm run wasm`. Run every command separately: `cargo test --workspace`, clippy, fmt, `cd web && npm test`, `npm run build`.
- [ ] Commit: `feat: choose the bullet mode; recordings say which`.

### Task 5: 2D shot look

**Files:** `web/src/draw.ts`, `web/src/draw.test.ts`.

- **Tracer:** for each live bullet, find its previous position, meaning the same bullet id in `rec.turns[turn-1]`. If there is none, use the shooter's muzzle: the robot's position plus `barrelReach` along `gun_heading`, or 20 px for the plain tank. Draw a wide soft yellow line (`rgba(255,200,80,0.35)`, width 4), then a bright core line (`#fff6c8`, width 1.5).
- **Muzzle flash:** on the turn a bullet first appears, draw a 6 px bright disc at the muzzle that fades over 2 turns.
- **Impact flash:** for `bullet_hits` and `bullet_hit_obstacles`, draw an expanding yellow-white disc plus 6 short spark lines at fixed angles, fading over 4 turns. This replaces the current ring. All of these respect `view.effects`; the tracer core is always drawn.
- [ ] Write the tests first: the tracer endpoints use the previous recorded position; on the first turn the tracer starts at the muzzle; the impact flash appears on the hit turn and is gone after 4 turns.
- [ ] Implement. Run `npm test`, then `npm run build`.
- [ ] Commit: `feat: tracers and flashes in 2D`.

### Task 6: 3D shot look

**Files:** `web/src/components/ArenaScene3D.tsx`, and `web/src/scene.ts` (pure helpers plus tests).

- Replace the bullet sphere with a thin emissive box stretched from the previous position to the current one, using the same previous-position rule as 2D (put it in `scene.ts` as `bulletSegment(rec, turn, bulletIndex)` and share it with 2D if practical).
- Add an impact flash: a small additive-blended sprite or disc at the hit point that fades over 4 turns.
- [ ] Write the tests first: `bulletSegment` covers the previous-turn match and the first-turn muzzle fallback.
- [ ] Implement. Run `npm test`, then `npm run build`.
- [ ] Commit: `feat: tracers and flashes in 3D`.

### Task 7: Docs and release 0.7.0

**Files:** `README.md`, `CHANGELOG.md`, `docs/robot-api.md`, `docs/viewer.md`, `Cargo.toml` version 0.7.0, `robocheck/template/Cargo.lock.template`, and anything `tests/release_metadata` checks.

- [ ] CHANGELOG 0.7.0:
  - **Breaking:** realistic is the default everywhere (the library, the CLI and the web), `Observation` has new fields, and `bullet_speed` takes a mode.
  - Old recordings play as arcade.
- [ ] Run `cargo test --workspace` and `cargo publish --dry-run -p robocode`.
- [ ] Commit: `chore: release 0.7.0`. Do not tag, push or publish.
