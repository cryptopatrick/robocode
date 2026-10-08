use crate::geometry::{Battlefield, Point, distance_squared};
use crate::physics::bullet::BulletState;
use crate::physics::events::{
    BulletFired, BulletImpact, TurnEvents, WallCollision,
};
use crate::physics::robot::{RobotId, RobotState};
use crate::physics::snapshot::TurnSnapshot;
use crate::rules::{self, EPSILON};

#[derive(Debug, Clone, Copy)]
pub struct BattleConfig {
    pub battlefield: Battlefield,
    pub gun_cooling_rate: f64,
}

impl Default for BattleConfig {
    fn default() -> Self {
        Self {
            battlefield: Battlefield::new(800.0, 600.0),
            gun_cooling_rate: rules::GUN_COOLING_RATE,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SpawnSpec {
    pub id: RobotId,
    pub position: Point,
    pub heading: f64,
}

impl SpawnSpec {
    pub fn new(id: RobotId, x: f64, y: f64, heading: f64) -> Self {
        Self { id, position: Point::new(x, y), heading }
    }
}

pub struct BattleState {
    pub config: BattleConfig,
    pub turn: u64,
    pub robots: Vec<RobotState>,
    pub bullets: Vec<BulletState>,
    next_bullet_id: u64,
}

impl BattleState {
    pub fn new(config: BattleConfig, robots: Vec<RobotState>) -> Self {
        Self {
            config,
            turn: 0,
            robots,
            bullets: Vec::new(),
            next_bullet_id: 0,
        }
    }

    pub fn from_spawns(config: BattleConfig, spawns: &[SpawnSpec]) -> Self {
        let mut robots = Vec::with_capacity(spawns.len());
        for spawn in spawns {
            let mut robot = RobotState::new(spawn.id, spawn.position);
            robot.heading = spawn.heading;
            robots.push(robot);
        }
        Self::new(config, robots)
    }

    pub fn step(&mut self) -> TurnEvents {
        self.turn += 1;
        let mut events = TurnEvents::default();
        for robot in &mut self.robots {
            if robot.alive {
                robot.cool_gun(self.config.gun_cooling_rate);
            }
        }

        for idx in 0..self.robots.len() {
            if self.robots[idx].alive {
                apply_turns(&mut self.robots[idx]);
            }
        }

        for idx in 0..self.robots.len() {
            if self.robots[idx].alive {
                let wall_collision = apply_movement(
                    &mut self.robots[idx],
                    self.config.battlefield,
                );
                if wall_collision {
                    let id = self.robots[idx].id;
                    events.wall_collisions.push(WallCollision {
                        robot_id: id,
                        position: self.robots[idx].position,
                    });
                }
            }
        }

        self.handle_firing(&mut events);
        self.apply_bullets(&mut events);
        self.process_deaths(&mut events);

        events
    }

    pub fn snapshot<S: Into<String>>(
        &self,
        battle_id: S,
        events: &TurnEvents,
    ) -> TurnSnapshot {
        TurnSnapshot {
            battle_id: battle_id.into(),
            turn: self.turn,
            config: (&self.config).into(),
            robots: self.robots.iter().map(Into::into).collect(),
            bullets: self.bullets.iter().map(Into::into).collect(),
            events: events.into(),
        }
    }

    fn handle_firing(&mut self, events: &mut TurnEvents) {
        for idx in 0..self.robots.len() {
            let fire_request = {
                let robot = &mut self.robots[idx];
                if !robot.alive {
                    robot.control.fire_power = None;
                    None
                } else if robot.gun_heat > 0.0 || robot.energy <= 0.0 {
                    robot.control.fire_power = None;
                    None
                } else if let Some(raw_power) = robot.control.fire_power.take()
                {
                    let available_power = raw_power.min(robot.energy);
                    if available_power < rules::MIN_BULLET_POWER {
                        None
                    } else {
                        let power = rules::clamp_bullet_power(available_power);
                        robot.energy -= power;
                        robot.gun_heat += rules::gun_heat_for_power(power);
                        Some((
                            robot.id,
                            robot.position,
                            robot.gun_heading,
                            power,
                        ))
                    }
                } else {
                    None
                }
            };

            if let Some((robot_id, position, heading, power)) = fire_request {
                let bullet_id = self.next_bullet_id();
                let bullet = BulletState::new(
                    bullet_id, robot_id, position, heading, power,
                );
                self.bullets.push(bullet);
                events.bullet_fired.push(BulletFired {
                    shooter: robot_id,
                    bullet_id,
                    power,
                });
            }
        }
    }

    fn apply_bullets(&mut self, events: &mut TurnEvents) {
        for bullet in &mut self.bullets {
            if !bullet.alive {
                continue;
            }
            bullet.advance();
            let mut hit_victim = None;
            for robot in &mut self.robots {
                if !robot.alive || robot.id == bullet.owner_id {
                    continue;
                }
                let collision_radius = robot.radius + rules::BULLET_RADIUS;
                if distance_squared(robot.position, bullet.position)
                    <= collision_radius * collision_radius
                {
                    hit_victim = Some(robot.id);
                    apply_bullet_damage(robot, bullet.power);
                    events.bullet_hits.push(BulletImpact {
                        shooter: bullet.owner_id,
                        victim: robot.id,
                        bullet_id: bullet.id,
                        power: bullet.power,
                    });
                    if let Some(shooter) = self
                        .robots
                        .iter_mut()
                        .find(|r| r.id == bullet.owner_id && r.alive)
                    {
                        shooter.energy +=
                            rules::bullet_hit_bonus(bullet.power);
                    }
                    break;
                }
            }
            if hit_victim.is_some() {
                bullet.alive = false;
                continue;
            }
            if !is_within_field(bullet.position, self.config.battlefield) {
                bullet.alive = false;
            }
        }

        self.bullets.retain(|b| b.alive);
    }

    fn process_deaths(&mut self, events: &mut TurnEvents) {
        for robot in &mut self.robots {
            if robot.alive && robot.energy <= 0.0 {
                robot.kill();
                events.robot_deaths.push(robot.id);
            }
        }
    }

    fn next_bullet_id(&mut self) -> u64 {
        let id = self.next_bullet_id;
        self.next_bullet_id += 1;
        id
    }
}

fn apply_turns(robot: &mut RobotState) {
    if !robot.alive {
        robot.control.turn_remaining = 0.0;
        robot.control.gun_turn_remaining = 0.0;
        robot.control.radar_turn_remaining = 0.0;
        return;
    }
    let body_limit = rules::body_turn_rate_for_velocity(robot.velocity);
    let applied = clamp_turn(robot.control.turn_remaining, body_limit);
    robot.apply_absolute_heading(applied);
    robot.control.turn_remaining -= applied;
    if robot.control.turn_remaining.abs() < EPSILON {
        robot.control.turn_remaining = 0.0;
    }

    let gun_limit = rules::MAX_GUN_TURN_RATE;
    let gun_applied = clamp_turn(robot.control.gun_turn_remaining, gun_limit);
    robot.apply_gun_heading(gun_applied);
    robot.control.gun_turn_remaining -= gun_applied;
    if robot.control.gun_turn_remaining.abs() < EPSILON {
        robot.control.gun_turn_remaining = 0.0;
    }

    let radar_limit = rules::MAX_RADAR_TURN_RATE;
    let radar_applied =
        clamp_turn(robot.control.radar_turn_remaining, radar_limit);
    robot.apply_radar_heading(radar_applied);
    robot.control.radar_turn_remaining -= radar_applied;
    if robot.control.radar_turn_remaining.abs() < EPSILON {
        robot.control.radar_turn_remaining = 0.0;
    }
}

fn clamp_turn(requested: f64, limit: f64) -> f64 {
    requested.clamp(-limit, limit)
}

fn apply_movement(robot: &mut RobotState, battlefield: Battlefield) -> bool {
    if !robot.alive {
        robot.velocity = 0.0;
        robot.control.distance_remaining = 0.0;
        return false;
    }
    let desired_velocity = desired_velocity(robot.control.distance_remaining);
    robot.velocity = adjust_velocity(robot.velocity, desired_velocity);
    let previous_distance = robot.control.distance_remaining;
    let travel = robot.velocity;
    robot.position = robot.position.translate(robot.heading, travel);
    robot.control.distance_remaining -= travel;
    if previous_distance.signum() != robot.control.distance_remaining.signum()
        && robot.control.distance_remaining.abs() < rules::MAX_VELOCITY
    {
        robot.control.distance_remaining = 0.0;
    }

    let (clamped, collided) =
        battlefield.clamp_point(robot.position, robot.radius);
    robot.position = clamped;
    if collided {
        let damage = robot.velocity.abs() * 0.5 + 1.0;
        robot.energy -= damage;
        robot.velocity = 0.0;
        robot.control.distance_remaining = 0.0;
        return true;
    }
    false
}

fn desired_velocity(distance_remaining: f64) -> f64 {
    if distance_remaining.abs() < EPSILON {
        return 0.0;
    }
    let direction = distance_remaining.signum();
    let max_vel = rules::max_velocity_for_distance(distance_remaining.abs());
    direction * max_vel
}

fn adjust_velocity(current: f64, desired: f64) -> f64 {
    if (desired - current).abs() <= EPSILON {
        return desired;
    }
    if current == 0.0 {
        let delta = desired.clamp(-rules::ACCELERATION, rules::ACCELERATION);
        return delta;
    }
    let same_direction = current.signum() == desired.signum();
    let increasing_speed = desired.abs() > current.abs();
    let rate = if same_direction && increasing_speed {
        rules::ACCELERATION
    } else {
        rules::DECELERATION
    };
    if desired > current {
        (current + rate)
            .min(desired)
            .clamp(-rules::MAX_VELOCITY, rules::MAX_VELOCITY)
    } else {
        (current - rate)
            .max(desired)
            .clamp(-rules::MAX_VELOCITY, rules::MAX_VELOCITY)
    }
}

fn apply_bullet_damage(robot: &mut RobotState, power: f64) {
    let damage = rules::bullet_damage(power);
    robot.energy -= damage;
}

fn is_within_field(point: Point, battlefield: Battlefield) -> bool {
    point.x >= -50.0
        && point.x <= battlefield.width + 50.0
        && point.y >= -50.0
        && point.y <= battlefield.height + 50.0
}
