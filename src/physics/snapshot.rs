use std::fmt::Write as _;
use std::io::{self, Write};

use super::bullet::BulletState;
use super::events::{RobotCollision, TurnEvents};
use super::robot::RobotState;
use super::{BattleConfig, BulletId, RobotId};
use crate::geometry::Point;

#[derive(Debug, Clone)]
pub struct TurnSnapshot {
    pub battle_id: String,
    pub turn: u64,
    pub config: SnapshotConfig,
    pub robots: Vec<RobotSnapshot>,
    pub bullets: Vec<BulletSnapshot>,
    pub events: EventSnapshot,
}

impl TurnSnapshot {
    pub fn to_json_string(&self) -> String {
        let mut out = String::new();
        self.write_json_to_string(&mut out);
        out
    }

    pub fn write_json<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let mut out = String::new();
        self.write_json_to_string(&mut out);
        writer.write_all(out.as_bytes())
    }

    fn write_json_to_string(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        append_string_field(out, &mut first, "battle_id", &self.battle_id);
        append_u64_field(out, &mut first, "turn", self.turn);
        append_config_field(out, &mut first, &self.config);
        append_robots_field(out, &mut first, &self.robots);
        append_bullets_field(out, &mut first, &self.bullets);
        append_events_field(out, &mut first, &self.events);
        out.push('}');
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SnapshotConfig {
    pub battlefield_width: f64,
    pub battlefield_height: f64,
    pub gun_cooling_rate: f64,
}

impl From<&BattleConfig> for SnapshotConfig {
    fn from(config: &BattleConfig) -> Self {
        Self {
            battlefield_width: config.battlefield.width,
            battlefield_height: config.battlefield.height,
            gun_cooling_rate: config.gun_cooling_rate,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RobotSnapshot {
    pub id: RobotId,
    pub position: Point,
    pub heading: f64,
    pub gun_heading: f64,
    pub radar_heading: f64,
    pub velocity: f64,
    pub energy: f64,
    pub gun_heat: f64,
    pub alive: bool,
    pub radius: f64,
    pub control: RobotControlSnapshot,
}

impl From<&RobotState> for RobotSnapshot {
    fn from(robot: &RobotState) -> Self {
        Self {
            id: robot.id,
            position: robot.position,
            heading: robot.heading,
            gun_heading: robot.gun_heading,
            radar_heading: robot.radar_heading,
            velocity: robot.velocity,
            energy: robot.energy,
            gun_heat: robot.gun_heat,
            alive: robot.alive,
            radius: robot.radius,
            control: RobotControlSnapshot {
                distance_remaining: robot.control.distance_remaining,
                turn_remaining: robot.control.turn_remaining,
                gun_turn_remaining: robot.control.gun_turn_remaining,
                radar_turn_remaining: robot.control.radar_turn_remaining,
                fire_power: robot.control.fire_power,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RobotControlSnapshot {
    pub distance_remaining: f64,
    pub turn_remaining: f64,
    pub gun_turn_remaining: f64,
    pub radar_turn_remaining: f64,
    pub fire_power: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct BulletSnapshot {
    pub id: BulletId,
    pub owner_id: RobotId,
    pub position: Point,
    pub heading: f64,
    pub power: f64,
    pub speed: f64,
    pub alive: bool,
}

impl From<&BulletState> for BulletSnapshot {
    fn from(bullet: &BulletState) -> Self {
        Self {
            id: bullet.id,
            owner_id: bullet.owner_id,
            position: bullet.position,
            heading: bullet.heading,
            power: bullet.power,
            speed: bullet.speed,
            alive: bullet.alive,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EventSnapshot {
    pub bullet_fired: Vec<BulletFiredSnapshot>,
    pub bullet_hits: Vec<BulletImpactSnapshot>,
    pub wall_collisions: Vec<WallCollisionSnapshot>,
    pub robot_collisions: Vec<RobotCollisionSnapshot>,
    pub robot_deaths: Vec<RobotId>,
}

impl From<&TurnEvents> for EventSnapshot {
    fn from(events: &TurnEvents) -> Self {
        Self {
            bullet_fired: events.bullet_fired.iter().map(Into::into).collect(),
            bullet_hits: events.bullet_hits.iter().map(Into::into).collect(),
            wall_collisions: events
                .wall_collisions
                .iter()
                .map(Into::into)
                .collect(),
            robot_collisions: events
                .robot_collisions
                .iter()
                .map(Into::into)
                .collect(),
            robot_deaths: events.robot_deaths.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BulletFiredSnapshot {
    pub shooter: RobotId,
    pub bullet_id: BulletId,
    pub power: f64,
}

impl From<&crate::physics::events::BulletFired> for BulletFiredSnapshot {
    fn from(event: &crate::physics::events::BulletFired) -> Self {
        Self {
            shooter: event.shooter,
            bullet_id: event.bullet_id,
            power: event.power,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BulletImpactSnapshot {
    pub shooter: RobotId,
    pub victim: RobotId,
    pub bullet_id: BulletId,
    pub power: f64,
}

impl From<&crate::physics::events::BulletImpact> for BulletImpactSnapshot {
    fn from(event: &crate::physics::events::BulletImpact) -> Self {
        Self {
            shooter: event.shooter,
            victim: event.victim,
            bullet_id: event.bullet_id,
            power: event.power,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WallCollisionSnapshot {
    pub robot_id: RobotId,
    pub position: Point,
}

impl From<&crate::physics::events::WallCollision> for WallCollisionSnapshot {
    fn from(event: &crate::physics::events::WallCollision) -> Self {
        Self { robot_id: event.robot_id, position: event.position }
    }
}

#[derive(Debug, Clone)]
pub struct RobotCollisionSnapshot {
    pub robot_id: RobotId,
    pub other_robot_id: RobotId,
    pub at_fault: bool,
}

impl From<&RobotCollision> for RobotCollisionSnapshot {
    fn from(event: &RobotCollision) -> Self {
        Self {
            robot_id: event.robot_id,
            other_robot_id: event.other_robot_id,
            at_fault: event.at_fault,
        }
    }
}

fn append_string_field(
    out: &mut String,
    first: &mut bool,
    name: &str,
    value: &str,
) {
    append_field_sep(out, first);
    push_string(out, name);
    out.push(':');
    push_string(out, value);
}

fn append_u64_field(
    out: &mut String,
    first: &mut bool,
    name: &str,
    value: u64,
) {
    append_field_sep(out, first);
    push_string(out, name);
    out.push(':');
    let _ = write!(out, "{}", value);
}

fn append_f64_field(
    out: &mut String,
    first: &mut bool,
    name: &str,
    value: f64,
) {
    append_field_sep(out, first);
    push_string(out, name);
    out.push(':');
    let _ = write!(out, "{}", value);
}

fn append_bool_field(
    out: &mut String,
    first: &mut bool,
    name: &str,
    value: bool,
) {
    append_field_sep(out, first);
    push_string(out, name);
    out.push(':');
    out.push_str(if value { "true" } else { "false" });
}

fn append_config_field(
    out: &mut String,
    first: &mut bool,
    config: &SnapshotConfig,
) {
    append_field_sep(out, first);
    push_string(out, "config");
    out.push(':');
    out.push('{');
    let mut inner_first = true;
    append_f64_field(
        out,
        &mut inner_first,
        "battlefield_width",
        config.battlefield_width,
    );
    append_f64_field(
        out,
        &mut inner_first,
        "battlefield_height",
        config.battlefield_height,
    );
    append_f64_field(
        out,
        &mut inner_first,
        "gun_cooling_rate",
        config.gun_cooling_rate,
    );
    out.push('}');
}

fn append_robots_field(
    out: &mut String,
    first: &mut bool,
    robots: &[RobotSnapshot],
) {
    append_field_sep(out, first);
    push_string(out, "robots");
    out.push(':');
    append_robot_array(out, robots);
}

fn append_bullets_field(
    out: &mut String,
    first: &mut bool,
    bullets: &[BulletSnapshot],
) {
    append_field_sep(out, first);
    push_string(out, "bullets");
    out.push(':');
    append_bullet_array(out, bullets);
}

fn append_events_field(
    out: &mut String,
    first: &mut bool,
    events: &EventSnapshot,
) {
    append_field_sep(out, first);
    push_string(out, "events");
    out.push(':');
    append_events_object(out, events);
}

fn append_robot_array(out: &mut String, robots: &[RobotSnapshot]) {
    out.push('[');
    for (idx, robot) in robots.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        append_robot(out, robot);
    }
    out.push(']');
}

fn append_robot(out: &mut String, robot: &RobotSnapshot) {
    out.push('{');
    let mut first = true;
    append_u64_field(out, &mut first, "id", robot.id as u64);
    append_point_field(out, &mut first, "position", robot.position);
    append_f64_field(out, &mut first, "heading", robot.heading);
    append_f64_field(out, &mut first, "gun_heading", robot.gun_heading);
    append_f64_field(out, &mut first, "radar_heading", robot.radar_heading);
    append_f64_field(out, &mut first, "velocity", robot.velocity);
    append_f64_field(out, &mut first, "energy", robot.energy);
    append_f64_field(out, &mut first, "gun_heat", robot.gun_heat);
    append_bool_field(out, &mut first, "alive", robot.alive);
    append_f64_field(out, &mut first, "radius", robot.radius);
    append_control_field(out, &mut first, &robot.control);
    out.push('}');
}

fn append_control_field(
    out: &mut String,
    first: &mut bool,
    control: &RobotControlSnapshot,
) {
    append_field_sep(out, first);
    push_string(out, "control");
    out.push(':');
    out.push('{');
    let mut inner_first = true;
    append_f64_field(
        out,
        &mut inner_first,
        "distance_remaining",
        control.distance_remaining,
    );
    append_f64_field(
        out,
        &mut inner_first,
        "turn_remaining",
        control.turn_remaining,
    );
    append_f64_field(
        out,
        &mut inner_first,
        "gun_turn_remaining",
        control.gun_turn_remaining,
    );
    append_f64_field(
        out,
        &mut inner_first,
        "radar_turn_remaining",
        control.radar_turn_remaining,
    );
    append_field_sep(out, &mut inner_first);
    push_string(out, "fire_power");
    out.push(':');
    match control.fire_power {
        Some(value) => {
            let _ = write!(out, "{}", value);
        }
        None => out.push_str("null"),
    }
    out.push('}');
}

fn append_bullet_array(out: &mut String, bullets: &[BulletSnapshot]) {
    out.push('[');
    for (idx, bullet) in bullets.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        append_bullet(out, bullet);
    }
    out.push(']');
}

fn append_bullet(out: &mut String, bullet: &BulletSnapshot) {
    out.push('{');
    let mut first = true;
    append_u64_field(out, &mut first, "id", bullet.id);
    append_u64_field(out, &mut first, "owner_id", bullet.owner_id as u64);
    append_point_field(out, &mut first, "position", bullet.position);
    append_f64_field(out, &mut first, "heading", bullet.heading);
    append_f64_field(out, &mut first, "power", bullet.power);
    append_f64_field(out, &mut first, "speed", bullet.speed);
    append_bool_field(out, &mut first, "alive", bullet.alive);
    out.push('}');
}

fn append_events_object(out: &mut String, events: &EventSnapshot) {
    out.push('{');
    let mut first = true;
    append_bullet_fired_array(out, &mut first, &events.bullet_fired);
    append_bullet_hits_array(out, &mut first, &events.bullet_hits);
    append_wall_collisions_array(out, &mut first, &events.wall_collisions);
    append_robot_collisions_array(out, &mut first, &events.robot_collisions);
    append_robot_deaths_array(out, &mut first, &events.robot_deaths);
    out.push('}');
}

fn append_bullet_fired_array(
    out: &mut String,
    first: &mut bool,
    events: &[BulletFiredSnapshot],
) {
    append_field_sep(out, first);
    push_string(out, "bullet_fired");
    out.push(':');
    out.push('[');
    for (idx, event) in events.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push('{');
        let mut inner_first = true;
        append_u64_field(
            out,
            &mut inner_first,
            "shooter",
            event.shooter as u64,
        );
        append_u64_field(out, &mut inner_first, "bullet_id", event.bullet_id);
        append_f64_field(out, &mut inner_first, "power", event.power);
        out.push('}');
    }
    out.push(']');
}

fn append_bullet_hits_array(
    out: &mut String,
    first: &mut bool,
    events: &[BulletImpactSnapshot],
) {
    append_field_sep(out, first);
    push_string(out, "bullet_hits");
    out.push(':');
    out.push('[');
    for (idx, event) in events.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push('{');
        let mut inner_first = true;
        append_u64_field(
            out,
            &mut inner_first,
            "shooter",
            event.shooter as u64,
        );
        append_u64_field(out, &mut inner_first, "victim", event.victim as u64);
        append_u64_field(out, &mut inner_first, "bullet_id", event.bullet_id);
        append_f64_field(out, &mut inner_first, "power", event.power);
        out.push('}');
    }
    out.push(']');
}

fn append_wall_collisions_array(
    out: &mut String,
    first: &mut bool,
    events: &[WallCollisionSnapshot],
) {
    append_field_sep(out, first);
    push_string(out, "wall_collisions");
    out.push(':');
    out.push('[');
    for (idx, event) in events.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push('{');
        let mut inner_first = true;
        append_u64_field(
            out,
            &mut inner_first,
            "robot_id",
            event.robot_id as u64,
        );
        append_point_field(out, &mut inner_first, "position", event.position);
        out.push('}');
    }
    out.push(']');
}

fn append_robot_collisions_array(
    out: &mut String,
    first: &mut bool,
    events: &[RobotCollisionSnapshot],
) {
    append_field_sep(out, first);
    push_string(out, "robot_collisions");
    out.push(':');
    out.push('[');
    for (idx, event) in events.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push('{');
        let mut inner_first = true;
        append_u64_field(
            out,
            &mut inner_first,
            "robot_id",
            event.robot_id as u64,
        );
        append_u64_field(
            out,
            &mut inner_first,
            "other_robot_id",
            event.other_robot_id as u64,
        );
        append_bool_field(out, &mut inner_first, "at_fault", event.at_fault);
        out.push('}');
    }
    out.push(']');
}

fn append_robot_deaths_array(
    out: &mut String,
    first: &mut bool,
    deaths: &[RobotId],
) {
    append_field_sep(out, first);
    push_string(out, "robot_deaths");
    out.push(':');
    out.push('[');
    for (idx, id) in deaths.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        let _ = write!(out, "{}", *id as u64);
    }
    out.push(']');
}

fn append_point_field(
    out: &mut String,
    first: &mut bool,
    name: &str,
    point: Point,
) {
    append_field_sep(out, first);
    push_string(out, name);
    out.push(':');
    out.push('{');
    let mut inner_first = true;
    append_f64_field(out, &mut inner_first, "x", point.x);
    append_f64_field(out, &mut inner_first, "y", point.y);
    out.push('}');
}

fn append_field_sep(out: &mut String, first: &mut bool) {
    if *first {
        *first = false;
    } else {
        out.push(',');
    }
}

fn push_string(out: &mut String, value: &str) {
    out.push('"');
    escape_json(value, out);
    out.push('"');
}

fn escape_json(input: &str, out: &mut String) {
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            other => out.push(other),
        }
    }
}
