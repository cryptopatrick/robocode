//! Core Robocode physics constants and helper functions.
//!
//! These values mirror the classic JVM implementation and should remain stable
//! so that higher layers (robot API, event dispatch, rendering) can trust a
//! deterministic simulation surface.

/// Maximum linear velocity a robot can reach in pixels per turn.
pub const MAX_VELOCITY: f64 = 8.0;
/// Maximum angular velocity for the robot body in degrees per turn when the
/// robot is stationary.
pub const MAX_TURN_RATE: f64 = 10.0;
/// Maximum angular velocity for the gun in degrees per turn.
pub const MAX_GUN_TURN_RATE: f64 = 20.0;
/// Maximum angular velocity for the radar in degrees per turn.
pub const MAX_RADAR_TURN_RATE: f64 = 45.0;
/// Forward acceleration per turn (pixels / turn²).
pub const ACCELERATION: f64 = 1.0;
/// Braking / reverse acceleration per turn (pixels / turn²).
pub const DECELERATION: f64 = 2.0;
/// Energy lost when colliding with another robot.
pub const ROBOT_HIT_DAMAGE: f64 = 0.6;
/// Bonus score awarded when ramming kills an opponent (mirrors JVM API).
pub const ROBOT_HIT_BONUS: f64 = 2.0 * ROBOT_HIT_DAMAGE;
/// Minimum fire power allowed by the engine.
pub const MIN_BULLET_POWER: f64 = 0.1;
/// Maximum fire power allowed by the engine.
pub const MAX_BULLET_POWER: f64 = 3.0;
/// Cooling rate for the gun heat in heat units per turn.
pub const GUN_COOLING_RATE: f64 = 0.1;
/// Default robot radius (half of the 36px square body in classic RoboCode).
pub const ROBOT_RADIUS: f64 = 18.0;
/// Bullet collision radius approximation.
pub const BULLET_RADIUS: f64 = 3.0;
/// Natural epsilon used for floating point comparisons inside the physics core.
pub const EPSILON: f64 = 1e-9;

/// Returns the body turn rate allowed this turn for the provided velocity.
///
/// The faster a robot travels the slower it is allowed to rotate. This mirrors
/// the logic in the original game (`10 - 0.75 * |velocity|`).
pub fn body_turn_rate_for_velocity(velocity: f64) -> f64 {
    (MAX_TURN_RATE - 0.75 * velocity.abs()).max(0.0)
}

/// Converts bullet power to travel speed in pixels per turn.
pub fn bullet_speed(power: f64) -> f64 {
    20.0 - 3.0 * power
}

/// Returns how much damage a hit bullet deals to the victim robot.
pub fn bullet_damage(power: f64) -> f64 {
    4.0 * power + 2.0 * (power - 1.0).max(0.0)
}

/// Bonus energy awarded to the shooter when their bullet hits an opponent.
pub fn bullet_hit_bonus(power: f64) -> f64 {
    3.0 * power
}

/// Heat generated on the gun when firing.
pub fn gun_heat_for_power(power: f64) -> f64 {
    1.0 + power / 5.0
}

/// Clamp bullet power into the legal range.
pub fn clamp_bullet_power(power: f64) -> f64 {
    power.clamp(MIN_BULLET_POWER, MAX_BULLET_POWER)
}

/// Damage inflicted when hitting a wall at the provided velocity.
pub fn wall_hit_damage(velocity: f64) -> f64 {
    (velocity.abs() / 2.0 - 1.0).max(0.0)
}

/// Maximum velocity that still allows the robot to decelerate to zero before it
/// runs out of the provided distance.
pub fn max_velocity_for_distance(distance: f64) -> f64 {
    if distance <= 0.0 {
        return 0.0;
    }
    ((2.0 * DECELERATION * distance).sqrt()).min(MAX_VELOCITY)
}
