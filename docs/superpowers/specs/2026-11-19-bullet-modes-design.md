# Bullet modes: realistic (default) and arcade

**Status:** draft for approval, 2026-11-19.
**Release:** robocode 0.7.0.
**Source:** the user's reference video `_wb/video01.mp4` (look only; the app never reads `_wb/`).

## Intent

The user finds it unrealistic that bullets drift slowly across the arena and that tanks can dodge them. In the reference video, a tank round is a bright white-yellow streak, seen for one or two frames (1/30–1/15 s). It is followed by a big yellow-white impact flash that fades over about a quarter of a second, sometimes with sparks.

robocode should offer two bullet modes:
- **realistic** (the new default): fast bullets drawn like the video;
- **arcade**: today's Robocode bullets, slow and dodgeable.

## Goals

- **realistic:** every bullet flies at a fixed **300 px per turn**, whatever its power, so it crosses an 800 px arena in about 3 turns. Power still sets the damage, the gun heat, the energy cost and the hit bonus, all unchanged.
- **arcade:** exactly today's rules (`20 − 3·power` px per turn). An arcade battle is byte-identical to the same battle in 0.6.0.
- **The mode is a battle rule.** It is chosen per battle, applied by the engine, recorded in the recording, and visible to robots.
- **New shot look, in both modes, in 2D and 3D:**
  - a glowing yellow-white tracer from the bullet's previous position to its current one;
  - a short muzzle flash on the turn a shot is fired;
  - an impact flash (with a few sparks in 2D) where a bullet hits a robot or a building.

## Non-goals

- No other physics changes: robot movement, gun heat, damage, radar and buildings stay as they are.
- No hitscan, meaning bullets that land the instant they're fired. Bullets still exist and still travel; they are just fast.
- No changes to robots' decision logic in this release. Tactician's Evade stays, though it matters little in realistic mode.

## Anti-goals (failure even if it works)

- **A bullet passing through a robot without hitting it.** At 300 px a turn, the engine must sweep the whole path each turn.
- **A bullet absorbed by a building that is behind the robot it hit**, or a robot hit through a building. The engine resolves the *first* thing along the path.
- **A replay that differs from its live battle**, or an old recording that replays in the wrong mode.
- **Drawn bullets that lie about where a bullet is.** The tracer runs between real recorded positions.

## Design

### Engine (`src/`)
- `BulletMode { Realistic, Arcade }`, with `Default = Realistic`. It is stored on the battle config and readable through `Battle`.
- `rules::bullet_speed(power, mode)`. Arcade returns `20 − 3·power`; realistic returns the constant `REALISTIC_BULLET_SPEED = 300.0`. The only caller is `src/physics/bullet.rs:35`.
- **Collision order.** Each turn, for each bullet, find the earliest contact along its segment from `previous_position` to `position`. The candidates are buildings (`Rect::first_contact`), robot circles and other bullets' segments. The bullet stops at the nearest one; ties go to the robot.
  - In arcade mode this must produce exactly today's outcomes. The open-arena fixture and every arcade snapshot stay byte-identical. If a strict reorder would change any arcade outcome, keep today's order in arcade and use earliest contact only in realistic. The builder decides which, based on the fixture tests.
  - Walls: a bullet that leaves the field is removed, as today.

### Robot API
- `Observation` gains `bullet_mode: BulletMode`, plus the derived `bullet_speed`, so that strategies aim correctly in either mode.
- Built-in robots that lead their aim must use the observed speed.
- `docs/robot-api.md` documents both.

### Recording and stream
- The header gains an optional `"bullets": "realistic" | "arcade"` field.
  - New recordings always write it.
  - A recording without it is arcade, because every recording made before 0.7.0 is arcade.
- The recording stays at version 2, since the field is optional.

### Choosing the mode
- **CLI:** `--bullets realistic|arcade`, default `realistic`.
- **wasm and web:** `BattleSpec.bullets`. New Battle gets a `Bullets: Realistic | Arcade` choice, defaulting to Realistic. The value is passed through the worker the same way `tanks` is.
- **Library:** `BattleConfig` defaults to realistic. This is a breaking behaviour change for crate users and is called out in the CHANGELOG.

### Viewer look
- **2D** (`web/src/draw.ts`):
  - Bullets become a glowing tracer: a bright core line plus a wider soft yellow line, from the previous to the current position. On a bullet's first turn, the line runs from the muzzle.
  - Muzzle flash: a small bright burst at the muzzle on the fire turn.
  - Impact flash: an expanding yellow-white disc plus a few spark lines, fading over about 4 turns. This replaces today's thin ring.
  - All of it respects the existing `effects` toggle.
- **3D** (`ArenaScene3D.tsx`): a thin emissive streak, a stretched box or cylinder, from the previous to the current position, replacing the grey sphere. Plus a brief point-light-free emissive flash sprite on impact.
- The previous position is derived from the recorded bullets of the previous turn, matched by bullet id.

### Tests that change
- Existing tests that rely on today's bullet speed pin `BulletMode::Arcade` explicitly, so they keep testing classic behaviour.
- The open-arena 0.5.0 fixture test runs in arcade.
- New tests:
  - a fast bullet hits a robot it would otherwise jump over;
  - a robot in front of a building is hit, not the building;
  - a building in front of a robot absorbs the bullet;
  - the default is realistic;
  - the header round-trip, including a missing field meaning arcade;
  - observed speed in both modes;
  - in the viewer: tracer endpoints, the muzzle flash on the fire turn, and the impact flash.

## Constraints

- PRD (`_dev/PRD.md`) requires physics identical to Robocode 1.9.x (lines 32, 85, 313, 398). Before any code changes, the PRD gets a "Bullet modes" section: arcade is the Robocode-identical mode, and realistic is a documented deviation and the default.
- Web: Tailwind only, no inline styles.
- No co-author trailers.

## What's left to the builder

- Exact tracer widths, glow alphas and flash timings, judged by eye against the video.
- Whether earliest-contact applies in both modes or only in realistic (see Engine).
- How RoboCheck handles the mode. Its stored runs predate 0.7.0, so they stay arcade (no field means arcade).

## Calls I made (check these)

- 300 px per turn, fixed, for realistic mode.
- The new look applies in both modes.
- Realistic is the default everywhere, including the Rust library.
- The mode is exposed to robots through `Observation`.
