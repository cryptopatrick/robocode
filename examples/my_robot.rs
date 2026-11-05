//! Template for writing your own robot. Copy this file and change `MyRobot`.
//!
//! ```sh
//! cargo run --example my_robot
//! ```

use robocode::api::{Commands, HitWallEvent, Robot, ScannedRobotEvent, Status, relative_bearing};
use robocode::physics::{BattleConfig, SpawnSpec};
use robocode::robots::SittingDuck;
use robocode::runner::{Battle, boxed};

/// Patrols back and forth, sweeping its radar, and shoots whatever it sees.
#[derive(Default)]
struct MyRobot {
    backward: bool,
}

impl Robot for MyRobot {
    fn name(&self) -> &str {
        "MyRobot"
    }

    fn run(&mut self, _status: &Status, cmd: &mut Commands) {
        if self.backward { cmd.set_back(150.0) } else { cmd.set_ahead(150.0) }
        cmd.set_turn_radar_right(45.0);
    }

    fn on_scanned_robot(&mut self, e: &ScannedRobotEvent, s: &Status, cmd: &mut Commands) {
        let aim = relative_bearing(e.absolute_bearing, s.gun_heading);
        cmd.set_turn_gun_right(aim);
        if aim.abs() < 3.0 {
            cmd.set_fire(if e.distance < 200.0 { 3.0 } else { 1.0 });
        }
    }

    fn on_hit_wall(&mut self, _e: &HitWallEvent, _s: &Status, _cmd: &mut Commands) {
        self.backward = !self.backward;
    }
}

fn main() {
    let spawns = [SpawnSpec::new(0, 100.0, 300.0, 90.0), SpawnSpec::new(1, 650.0, 450.0, 0.0)];
    let robots = vec![Box::new(MyRobot::default()) as Box<dyn Robot>, boxed(SittingDuck)];
    let mut battle = Battle::new(BattleConfig::default(), &spawns, robots);
    let outcome = battle.run(5000);
    let winner = outcome.winner.map_or("nobody", |i| battle.name(i));
    println!("{winner} won after {} turns", outcome.turns);
}
