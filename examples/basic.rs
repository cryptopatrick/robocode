use robocode::Point;
use robocode::physics::{BattleConfig, BattleState, RobotState};

fn main() {
    let mut battle = BattleState::new(
        BattleConfig::default(),
        vec![{
            let mut robot = RobotState::new(0, Point::new(400.0, 300.0));
            robot.set_ahead(50.0);
            robot
        }],
    );

    for _ in 0..20 {
        battle.step();
    }

    let robot = &battle.robots[0];
    println!(
        "Robot {} now at ({:.2}, {:.2}) with energy {:.1}",
        robot.id, robot.position.x, robot.position.y, robot.energy
    );
}
