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

/// An axis-aligned rectangle (a building or a road), in battlefield pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Rect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            min: Point::new(x, y),
            max: Point::new(x + width, y + height),
        }
    }

    /// The rectangle grown by `margin` on every side. A circle of that radius
    /// centred inside the grown rectangle touches the original.
    pub fn grow(&self, margin: f64) -> Self {
        Self {
            min: Point::new(self.min.x - margin, self.min.y - margin),
            max: Point::new(self.max.x + margin, self.max.y + margin),
        }
    }

    /// Where the segment `from → to` first touches the rectangle, as a fraction
    /// of the segment (`0.0` at `from`, `1.0` at `to`). `None` if it doesn't
    /// touch, or only leaves the rectangle: a body already touching a face and
    /// moving away is free to go. Slab method (Liang–Barsky).
    pub fn first_contact(&self, from: Point, to: Point) -> Option<f64> {
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        let length = (dx * dx + dy * dy).sqrt();
        let mut enter = 0.0_f64;
        let mut exit = 1.0_f64;
        for (start, delta, lo, hi) in [(from.x, dx, self.min.x, self.max.x), (from.y, dy, self.min.y, self.max.y)] {
            if delta.abs() < EPSILON {
                if start < lo || start > hi {
                    return None;
                }
            } else {
                let (a, b) = ((lo - start) / delta, (hi - start) / delta);
                let (near, far) = if a <= b { (a, b) } else { (b, a) };
                enter = enter.max(near);
                exit = exit.min(far);
                if enter > exit {
                    return None;
                }
            }
        }
        // Only a leave, not an entry: the rectangle is behind or the body is
        // already on its face moving away. Anything deeper than a rounding
        // error counts as inside.
        if (exit * length) <= 1e-6 {
            return None;
        }
        Some(enter)
    }
}
