use crate::geometry::Point;
use crate::physics::bullet::BulletId;
use crate::physics::robot::RobotId;

#[derive(Debug, Clone, Default)]
pub struct TurnEvents {
    pub bullet_fired: Vec<BulletFired>,
    pub bullet_hits: Vec<BulletImpact>,
    pub bullet_misses: Vec<BulletMissed>,
    pub wall_collisions: Vec<WallCollision>,
    pub robot_collisions: Vec<RobotCollision>,
    pub robot_deaths: Vec<RobotId>,
    pub scans: Vec<ScannedRobot>,
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

/// A bullet that left the battlefield without hitting anything.
#[derive(Debug, Clone, PartialEq)]
pub struct BulletMissed {
    pub shooter: RobotId,
    pub bullet_id: BulletId,
    pub power: f64,
}

#[derive(Debug, Clone)]
pub struct WallCollision {
    pub robot_id: RobotId,
    pub position: Point,
}

/// A robot detected by another robot's radar this turn.
#[derive(Debug, Clone, PartialEq)]
pub struct ScannedRobot {
    pub scanner: RobotId,
    pub target: RobotId,
    /// Absolute bearing from scanner to target, degrees in `0..360`.
    pub bearing: f64,
    pub distance: f64,
    pub energy: f64,
    pub heading: f64,
    pub velocity: f64,
}

#[derive(Debug, Clone)]
pub struct RobotCollision {
    pub robot_id: RobotId,
    pub other_robot_id: RobotId,
    pub at_fault: bool,
}
