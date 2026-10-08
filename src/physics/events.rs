use crate::geometry::Point;
use crate::physics::bullet::BulletId;
use crate::physics::robot::RobotId;

#[derive(Debug, Clone, Default)]
pub struct TurnEvents {
    pub bullet_fired: Vec<BulletFired>,
    pub bullet_hits: Vec<BulletImpact>,
    pub wall_collisions: Vec<WallCollision>,
    pub robot_collisions: Vec<RobotCollision>,
    pub robot_deaths: Vec<RobotId>,
}

#[derive(Debug, Clone)]
pub struct BulletFired {
    pub shooter: RobotId,
    pub bullet_id: BulletId,
    pub power: f64,
}

#[derive(Debug, Clone)]
pub struct BulletImpact {
    pub shooter: RobotId,
    pub victim: RobotId,
    pub bullet_id: BulletId,
    pub power: f64,
}

#[derive(Debug, Clone)]
pub struct WallCollision {
    pub robot_id: RobotId,
    pub position: Point,
}

#[derive(Debug, Clone)]
pub struct RobotCollision {
    pub robot_id: RobotId,
    pub other_robot_id: RobotId,
    pub at_fault: bool,
}
