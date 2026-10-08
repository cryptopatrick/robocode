//! Simple 2D geometry helpers tailored for the RoboCode battlefield model.

use crate::rules::EPSILON;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn translate(&self, heading_degrees: f64, distance: f64) -> Self {
        let radians = heading_degrees.to_radians();
        // 0 degrees points "up", so we invert cosine contribution for y.
        let dx = distance * radians.sin();
        let dy = -distance * radians.cos();
        Self { x: self.x + dx, y: self.y + dy }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Battlefield {
    pub width: f64,
    pub height: f64,
}

impl Battlefield {
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    pub fn clamp_point(&self, mut point: Point, radius: f64) -> (Point, bool) {
        let min_x = radius;
        let max_x = self.width - radius;
        let min_y = radius;
        let max_y = self.height - radius;
        let mut collided = false;

        if point.x < min_x {
            point.x = min_x;
            collided = true;
        } else if point.x > max_x {
            point.x = max_x;
            collided = true;
        }

        if point.y < min_y {
            point.y = min_y;
            collided = true;
        } else if point.y > max_y {
            point.y = max_y;
            collided = true;
        }

        (point, collided)
    }
}

pub fn distance_squared(a: Point, b: Point) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

pub fn normalize_absolute_angle(angle: f64) -> f64 {
    let mut result = angle % 360.0;
    if result < 0.0 {
        result += 360.0;
    }
    result
}

#[allow(dead_code)]
pub fn normalize_relative_angle(angle: f64) -> f64 {
    let mut result = angle % 360.0;
    if result <= -180.0 {
        result += 360.0;
    } else if result > 180.0 {
        result -= 360.0;
    }
    if result.abs() < EPSILON { 0.0 } else { result }
}

#[allow(dead_code)]
pub fn angle_to_point(from: Point, to: Point) -> f64 {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let radians = dx.atan2(-dy);
    normalize_absolute_angle(radians.to_degrees())
}
