# Engine Additions for Porting AdvancedRobots: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Support what Java `AdvancedRobot`s rely on, so robots from the user's archives can be ported faithfully for a local verification experiment:
- `setMaxVelocity` / `setMaxTurnRate`;
- a Java turret mode (the gun and radar turn with the body, plus Java's three `setAdjust…` flags);
- robot-robot collisions with `onHitRobot`;
- bullet-on-bullet collisions with `onBulletHitBullet`;
- rounds, with `onDeath`, `onWin` and `onRoundEnded`.

**Source of semantics:** `_wb/latest_version_robocode-main/robocode.battle/.../peer/RobotPeer.java` and `BulletPeer.java`.

**Decisions (user, 2026-11-06):**
- Ports stay local and out of the repo.
- Only robots using `AdvancedRobot` are in scope.
- The turret coupling is an alternative mode.
- Add whatever else is missing.

## Global Constraints

- **The default behaviour stays as it is.** The turret stays independent unless a robot opts into Java mode, so the Verus proofs and the `Strategy` robots are untouched. Verus and the 15 mutants must stay green.
- **Collisions are a deliberate change to the physics.** If the golden battles or the recording fingerprint change, record the new values and the reason in the ledger and commit message; never update them silently.
- `robocode` keeps zero dependencies. Clippy passes with `-D warnings` on the workspace and on `-p robocode`.
- Commits carry no Claude attribution. Branch `engine-additions`.

## Tasks

### 1. `set_max_velocity` and `set_max_turn_rate`

- **Engine:** the caps live in `RobotControl` and persist until changed. Velocity is capped at `min(cap, 8)`, the body turn rate at `min(cap, 10 − 0.75·|v|)`.
- **API:** `Commands::set_max_velocity` and `set_max_turn_rate`.
- **Tests:** a robot with a cap of 5 never exceeds 5 px/turn; a turn-rate cap of 4 limits each turn to 4°.

### 2. Java turret mode

- **Engine:** `RobotState.turret: Turret { mode: Independent | Java, adjust_gun_for_body: bool, adjust_radar_for_body: bool, adjust_radar_for_gun: bool }`, with the flags defaulting to false as in Java.
- **In Java mode**, `apply_turns` follows `updateHeading`, `updateGunHeading` and `updateRadarHeading`: the body turn drags the gun and radar, and each `adjust` flag subtracts the drag from the dependent turn-remaining.
- **API:** `Commands::set_turret_mode(TurretMode)`, `set_adjust_gun_for_robot_turn`, `set_adjust_radar_for_robot_turn`, `set_adjust_radar_for_gun_turn`.
- **Tests:**
  - in Java mode a 10° body turn moves an untouched gun by 10°;
  - with adjust-gun the gun nets 0 over that turn;
  - in independent mode nothing changes;
  - the radar follows the gun in Java mode unless its adjust flag is set.

### 3. Robot-robot collisions and `on_hit_robot`

- **Engine** (after walls, per robot in order): two living robots collide when their 36×36 boxes overlap.
  - The robot moving *toward* the other is at fault. It stops (velocity 0, distance 0) and undoes this turn's move; both lose `ROBOT_HIT_DAMAGE` (0.6).
  - `RobotCollision` events are created for both robots (at-fault true and false).
  - `in_collision` skips the body turn on the next turn, as Java does.
- **API:** `HitRobotEvent { name, bearing (relative), energy (other's), at_fault }`.
- **Tests:**
  - a ram head-on gives 0.6 damage each and a stop;
  - the at-fault flags are correct;
  - a robot that moved away isn't at fault;
  - the event bearings are right.

### 4. Bullet-on-bullet collisions

- **Engine:** bullets remember their previous position. After all bullets move, two bullets whose segments (previous → current) intersect are both destroyed, and a `BulletHitBullet` event is created.
- **API:** `BulletHitBulletEvent { name (other owner), power, other_power }`.
- **Tests:** two bullets fired at each other collide midway, nothing hits the robots, and each owner gets the event. Parallel bullets don't collide.

### 5. Rounds

- **`Battle::run_rounds(rounds, max_turns_per_round) -> BattleResults`.** Each round resets the engine state and rotates the spawn assignment by the round number (deterministic and fair). Robot instances persist, like Java static fields.
- **API:** `on_death(&Status)` when the robot dies, `on_win(&Status)` for the last survivor, and `on_round_ended(&RoundEndedEvent { round, turns })` for all robots.
- **`BattleResults { rounds: Vec<BattleOutcome>, wins: Vec<u32>, survived: Vec<u32>, bullet_damage: Vec<f64> }`.** Full Java scoring is out of scope.
- **Tests:**
  - 3 rounds give 3 outcomes, and the wins add up;
  - the spawns rotate;
  - `on_win`, `on_death` and `on_round_ended` are called once per round with the right robots;
  - `run` (one round) is unchanged.

### 6. Docs and the full check

- Update `docs/robot-api.md`: the Java mapping rows for the new methods, the "not supported" list, and the turret mode explained.
- Add `--rounds N` to `cargo robocode battle`. It prints each round's winner and the totals; the recording covers round 1.
- **Full check:**
  - all tests;
  - golden/fingerprint (updated with a recorded reason if collisions changed them);
  - Verus, the solutions and the mutants;
  - the web viewer;
  - the CLI tests.
