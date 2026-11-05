# Robot API

Write your own robot in a single Rust file with `robocode::api`. The API is modelled on classic RoboCode's `AdvancedRobot`: event handlers plus non-blocking `set_*` commands that run when the turn executes.

Quick start: copy `examples/my_robot.rs` and run `cargo run --example my_robot`.

## A robot

```rust
use robocode::api::{Commands, Robot, ScannedRobotEvent, Status, relative_bearing};

#[derive(Default)]
struct Hunter;

impl Robot for Hunter {
    fn name(&self) -> &str { "Hunter" }

    // Every turn, after the event handlers.
    fn run(&mut self, _status: &Status, cmd: &mut Commands) {
        cmd.set_ahead(100.0);
        cmd.set_turn_radar_right(45.0);
    }

    fn on_scanned_robot(&mut self, e: &ScannedRobotEvent, s: &Status, cmd: &mut Commands) {
        let aim = relative_bearing(e.absolute_bearing, s.gun_heading);
        cmd.set_turn_gun_right(aim);
        if aim.abs() < 3.0 { cmd.set_fire(1.0); }
    }
}
```

Battle it with the control API:

```rust
use robocode::physics::{BattleConfig, SpawnSpec};
use robocode::robots::SittingDuck;
use robocode::runner::{Battle, boxed};

let spawns = [SpawnSpec::new(0, 100.0, 100.0, 0.0), SpawnSpec::new(1, 700.0, 500.0, 0.0)];
let mut battle = Battle::new(BattleConfig::default(), &spawns,
    vec![Box::new(Hunter), boxed(SittingDuck)]);
let outcome = battle.run(5000);
```

## How a turn works

1. Your handlers are called for what happened **last turn**, in this order:
   - `on_scanned_robot`, nearest robot first;
   - `on_hit_by_bullet`;
   - `on_bullet_hit`;
   - `on_bullet_missed`;
   - `on_hit_wall`;
   - `on_robot_death`.
2. `run` is called.
3. The commands you set are carried out. For each kind (move, body turn, gun turn, radar turn, fire), the **last** call this turn wins. A kind you never set keeps whatever is still pending, so `set_ahead(500.0)` keeps the robot moving over many turns.

Dead robots get no calls.

## Units

- Pixels and degrees. 0° points up (north), and angles grow clockwise. y grows downward, as on a screen.
- Absolute angles are in `[0, 360)`. Relative bearings, like `ScannedRobotEvent::bearing`, are in `(-180, 180]`; `relative_bearing(absolute, heading)` converts one to the other.
- Bullet power is 0.1 to 3.0. The gun fires only when its heat is 0, and `set_fire` is ignored while it's hot.

## `Status`

Read-only facts at the start of the turn:
- `time`, `name`, `others` (opponents alive);
- `x`, `y`, `energy`, `velocity`, `gun_heat`;
- `heading`, `gun_heading`, `radar_heading`;
- `distance_remaining`, `turn_remaining`, `gun_turn_remaining`, `radar_turn_remaining`;
- `battlefield_width`, `battlefield_height`, `gun_cooling_rate`.

If several robots share a name, the battle numbers them like classic RoboCode: `SittingDuck`, `SittingDuck (2)`, ….

## From Java's `AdvancedRobot`

| Java | Rust |
|---|---|
| `run()` loop with `execute()` | `fn run(&mut self, status, cmd)`, called once per turn |
| `setAhead(d)` / `setBack(d)` | `cmd.set_ahead(d)` / `cmd.set_back(d)` |
| `setTurnRight(d)` / `setTurnLeft(d)` | `cmd.set_turn_right(d)` / `cmd.set_turn_left(d)` |
| `setTurnGunRight(d)`, `setTurnRadarRight(d)` (and `Left`) | `cmd.set_turn_gun_right(d)`, `cmd.set_turn_radar_right(d)` (and `_left`) |
| `setFire(p)` | `cmd.set_fire(p)` |
| `getX()`, `getEnergy()`, `getHeading()`, `getGunHeat()`, … | `status.x`, `status.energy`, `status.heading`, `status.gun_heat`, … |
| `getDistanceRemaining()`, `getTurnRemaining()`, … | `status.distance_remaining`, `status.turn_remaining`, … |
| `getOthers()`, `getTime()` | `status.others`, `status.time` |
| `onScannedRobot(e)`: `getBearing()`, `getDistance()`, `getEnergy()`, `getHeading()`, `getVelocity()`, `getName()` | `on_scanned_robot(e, …)`: `e.bearing`, `e.distance`, `e.energy`, `e.heading`, `e.velocity`, `e.name`; plus `e.absolute_bearing` |
| `onHitByBullet`, `onBulletHit`, `onBulletMissed`, `onHitWall`, `onRobotDeath` | `on_hit_by_bullet`, `on_bullet_hit`, `on_bullet_missed`, `on_hit_wall`, `on_robot_death` |

**Not supported yet:**
- the blocking `Robot` style (`ahead(100)` that waits);
- `onHitRobot` (the engine has no robot-to-robot collisions yet);
- `onWin` / `onDeath` / `onRoundEnded` (no rounds or scoring yet);
- custom events and `waitFor`;
- teams and messages;
- data files;
- colours;
- `onPaint`.

## Robots with proofs

`robocode::strategy` is the fixed-point, Verus-friendly way to write a robot: a single `decide(&Observation) -> Action`. `SafeShooter` is written this way, and Verus proves it never fires unsafely; see the RoboVerus guide in `guide/`. To put a strategy into a battle, wrap it with `runner::boxed(strategy)`. That uses `StrategyRobot`, which builds the strategy's `Observation` from the API events: the nearest robot scanned, how many turns ago, and the gun heat after cooling.

## Samples

| Robot | Written with | Behaviour |
|---|---|---|
| `Circler` | Robot API | Circles, locks radar and gun on its target, reverses at walls |
| `SafeShooter` | `Strategy` (Verus-proved) | Fires only when the safety spec holds |
| `Spinner` | `Strategy` | Drives in circles, fires on scan |
| `SittingDuck` | `Strategy` | Does nothing |
