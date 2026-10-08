use crate::geometry::{Point, normalize_absolute_angle};
use crate::rules;

pub type BulletId = u64;

#[derive(Debug, Clone)]
pub struct BulletState {
    pub id: BulletId,
    pub owner_id: usize,
    pub position: Point,
    pub heading: f64,
    pub power: f64,
    pub speed: f64,
    pub alive: bool,
}

impl BulletState {
    pub fn new(
        id: BulletId,
        owner_id: usize,
        position: Point,
        heading: f64,
        power: f64,
    ) -> Self {
        let power = rules::clamp_bullet_power(power);
        Self {
            id,
            owner_id,
            position,
            heading: normalize_absolute_angle(heading),
            power,
            speed: rules::bullet_speed(power),
            alive: true,
        }
    }

    pub fn advance(&mut self) {
        let radians = self.heading.to_radians();
        self.position.x += self.speed * radians.sin();
        self.position.y += -self.speed * radians.cos();
    }
}
