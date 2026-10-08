//! Deterministic physics engine for RoboCode battles.
//!
//! This module intentionally avoids references to any higher level robot API so
//! that it can be reused for the in-browser runtime as well as headless test
//! harnesses. Robots enqueue their intents on the [`RobotState`] structure and
//! the [`BattleState`] tick function simulates movement, collisions, bullets and
//! energy bookkeeping.

mod bullet;
mod engine;
mod events;
mod robot;
pub mod snapshot;

pub use bullet::{BulletId, BulletState};
pub use engine::{BattleConfig, BattleState, SpawnSpec};
pub use events::{
    BulletFired, BulletImpact, RobotCollision, TurnEvents, WallCollision,
};
pub use robot::{RobotControl, RobotId, RobotState};
pub use snapshot::{
    BulletSnapshot, EventSnapshot, RobotSnapshot, SnapshotConfig, TurnSnapshot,
};
