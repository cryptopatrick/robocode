use crate::geometry::{Point, normalize_absolute_angle};
use crate::rules::{EPSILON, ROBOT_RADIUS};

pub type RobotId = usize;

#[derive(Debug, Clone)]
pub struct RobotState {
    pub id: RobotId,
    pub position: Point,
    pub heading: f64,
    pub gun_heading: f64,
    pub radar_heading: f64,
    /// Radar heading at the start of the current turn (start of the scan arc).
    pub last_radar_heading: f64,
    pub velocity: f64,
    pub energy: f64,
    pub gun_heat: f64,
    pub alive: bool,
    pub radius: f64,
    pub control: RobotControl,
}

impl RobotState {
    pub fn new(id: RobotId, position: Point) -> Self {
        Self {
            id,
            position,
            heading: 0.0,
            gun_heading: 0.0,
            radar_heading: 0.0,
            last_radar_heading: 0.0,
            velocity: 0.0,
            energy: 100.0,
            gun_heat: 0.0,
            alive: true,
            radius: ROBOT_RADIUS,
            control: RobotControl::default(),
        }
    }

    pub fn reset_controls(&mut self) {
        self.control = RobotControl::default();
    }

    pub fn set_ahead(&mut self, distance: f64) {
        self.control.distance_remaining = distance;
    }

    pub fn set_turn_right(&mut self, degrees: f64) {
        self.control.turn_remaining = degrees;
    }

    pub fn set_turn_left(&mut self, degrees: f64) {
        self.set_turn_right(-degrees);
    }

    pub fn set_turn_gun_right(&mut self, degrees: f64) {
        self.control.gun_turn_remaining = degrees;
    }

    pub fn set_turn_radar_right(&mut self, degrees: f64) {
        self.control.radar_turn_remaining = degrees;
    }

    pub fn set_fire(&mut self, power: f64) {
        self.control.fire_power = Some(power);
    }

    pub fn heading_radians(&self) -> f64 {
        self.heading.to_radians()
    }

    pub fn gun_heading_radians(&self) -> f64 {
        self.gun_heading.to_radians()
    }

    pub fn apply_absolute_heading(&mut self, delta: f64) {
        self.heading = normalize_absolute_angle(self.heading + delta);
    }

    pub fn apply_gun_heading(&mut self, delta: f64) {
        self.gun_heading = normalize_absolute_angle(self.gun_heading + delta);
    }

    pub fn apply_radar_heading(&mut self, delta: f64) {
        self.radar_heading =
            normalize_absolute_angle(self.radar_heading + delta);
    }

    pub fn cool_gun(&mut self, cooling_rate: f64) {
        self.gun_heat = (self.gun_heat - cooling_rate).max(0.0);
    }

    pub fn kill(&mut self) {
        self.alive = false;
        self.velocity = 0.0;
        self.control.distance_remaining = 0.0;
    }
}

#[derive(Debug, Clone, Default)]
pub struct RobotControl {
    pub distance_remaining: f64,
    pub turn_remaining: f64,
    pub gun_turn_remaining: f64,
    pub radar_turn_remaining: f64,
    pub fire_power: Option<f64>,
}

impl RobotControl {
    pub fn has_pending_motion(&self) -> bool {
        self.distance_remaining.abs() > EPSILON
            || self.turn_remaining.abs() > EPSILON
            || self.gun_turn_remaining.abs() > EPSILON
            || self.radar_turn_remaining.abs() > EPSILON
            || self.fire_power.is_some()
    }
}
