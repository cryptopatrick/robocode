# robocode

**Robot Combat Simulator**: program a robot tank in Rust, battle it, and, if you like, prove it safe with Verus.

[![Crates.io](https://img.shields.io/crates/v/robocode.svg)](https://crates.io/crates/robocode)
[![Documentation](https://docs.rs/robocode/badge.svg)](https://docs.rs/robocode)
[![License](https://img.shields.io/badge/license-EPL--1.0-blue.svg)](https://github.com/cryptopatrick/robocode)

## Overview

`robocode` is a Rust take on [Robocode](https://robocode.sourceforge.io/), the classic programming game where you write the brain of a robot tank and send it into an arena. It has:

- a deterministic battle engine that follows the original game's physics and rules;
- a **Robot API** modelled on Robocode's `AdvancedRobot`;
- **RoboVerus**: an optional `verus` feature and guide for learning formal verification with [Verus](https://github.com/verus-lang/verus) by proving things about robots.

## Key Features

- **Robot API** (`robocode::api`): event handlers (`on_scanned_robot`, `on_hit_by_bullet`, `on_hit_wall`, …) plus `set_ahead`, `set_turn_gun_right`, `set_fire` and friends, in pixels and degrees.
- **Verifiable strategies** (`robocode::strategy`): robots written as a single fixed-point `decide(&Observation) -> Action`, the same code whether Verus proves it or a battle runs it.
- **Optional Verus support:** with `--features verus`, Verus checks the crate's specs. The sample robot `SafeShooter` is proved never to fire unless it has the energy, a cool gun and an aimed gun. Without the feature, no Verus crate is compiled.
- **Battles and recordings:** run battles headless, and save turn-by-turn recordings for the browser viewer.
- **Sample robots:** `SafeShooter`, `Circler`, `Spinner`, `SittingDuck`.
- **No dependencies** in the default build.

## Architecture

| Module | What it does |
|---|---|
| `physics` | The `f64` engine: movement, turning, gun heat, bullets, radar scans, collisions with walls |
| `api` | The Robot API: `Robot` trait, `Status`, events, `Commands` |
| `runner` | Runs battles between robots; `boxed` turns a `Strategy` into a robot |
| `strategy`, `units` | Fixed-point observations and actions for verifiable robots |
| `robots` | Sample robots |
| `recording`, `roster` | Battle recordings (format v1) and the built-in robot list |

The engine uses `f64` and follows the Java original. Verifiable strategies see the world in whole-number units (milli-energy, decidegrees, pixels), so the `decide` that Verus proves is exactly the code that fights. The runner converts between the two, rounding in the safe direction.

## Quick Start

```toml
[dependencies]
robocode = "0.4"
```

```rust
use robocode::api::{Commands, HitWallEvent, Robot, ScannedRobotEvent, Status, relative_bearing};
use robocode::physics::{BattleConfig, SpawnSpec};
use robocode::robots::SittingDuck;
use robocode::runner::{Battle, boxed};

#[derive(Default)]
struct Hunter {
    backward: bool,
}

impl Robot for Hunter {
    fn name(&self) -> &str { "Hunter" }

    fn run(&mut self, _status: &Status, cmd: &mut Commands) {
        cmd.set_ahead(if self.backward { -100.0 } else { 100.0 });
        cmd.set_turn_radar_right(45.0);
    }

    fn on_hit_wall(&mut self, _e: &HitWallEvent, _s: &Status, _cmd: &mut Commands) {
        self.backward = !self.backward; // turn around at walls
    }

    fn on_scanned_robot(&mut self, e: &ScannedRobotEvent, s: &Status, cmd: &mut Commands) {
        let aim = relative_bearing(e.absolute_bearing, s.gun_heading);
        cmd.set_turn_gun_right(aim);
        if aim.abs() < 3.0 { cmd.set_fire(1.0); }
    }
}

fn main() {
    let spawns = [SpawnSpec::new(0, 100.0, 100.0, 0.0), SpawnSpec::new(1, 700.0, 500.0, 0.0)];
    let mut battle = Battle::new(BattleConfig::default(), &spawns,
        vec![Box::new(Hunter::default()), boxed(SittingDuck)]);
    let outcome = battle.run(5000);
    println!("winner: {:?} after {} turns", outcome.winner.map(|i| battle.name(i)), outcome.turns);
}
```

## Documentation

- [API documentation](https://docs.rs/robocode)
- [Robot API guide](https://github.com/cryptopatrick/robocode/blob/master/docs/robot-api.md), with a table mapping Java's `AdvancedRobot` methods to the Rust API
- [`cargo robocode`](https://github.com/cryptopatrick/robocode/blob/master/docs/cargo-robocode.md): write a robot in one file, battle it, and watch it in your browser (installed from the repository for now)
- [Battle viewer](https://github.com/cryptopatrick/robocode/blob/master/docs/viewer.md): watch and replay battles in the browser
- [RoboVerus guide](https://github.com/cryptopatrick/robocode/tree/master/guide): learn Verus by proving things about robots
- [Verus integration](https://github.com/cryptopatrick/robocode/blob/master/docs/verus-integration.md): how the optional `verus` feature works
- [RoboCheck](https://github.com/cryptopatrick/robocode/blob/master/docs/robocheck.md): verify a candidate strategy (such as AI-generated code) against a human-written Verus spec, then battle the same code

## Examples

```sh
cargo run --example my_robot                  # a robot defined in the example file, vs SittingDuck
cargo run --example battle -- circler         # SafeShooter vs Circler, Spinner or SittingDuck
```

## Contributing
Contributions are welcome!
Please see our [contributing guidelines](https://github.com/cryptopatrick/robocode/blob/master/CONTRIBUTING.md) for details on:
- Code style and testing requirements
- Submitting bug reports and feature requests
- Development setup and workflow

## License
This project is licensed under the Eclipse Public License v1.0 (EPL-1.0), the license of the original [Robocode](https://robocode.sourceforge.io/). See [LICENSE](https://github.com/cryptopatrick/robocode/blob/master/LICENSE) for details.
