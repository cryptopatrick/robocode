//! RoboCheck's extension of robocode 0.5.0 (see README.md): building
//! clearance, a per-turn decision log and an executable `ready`, built only on
//! robocode's public API so robocode itself stays unchanged.

pub mod ready;
pub mod robot;
pub mod surroundings;

pub use ready::is_ready;
pub use robot::{ArenaRobot, Decide, DecisionLog, DecisionRecord};
#[cfg(feature = "verus")]
pub use surroundings::surroundings_ok;
pub use surroundings::{OBSTACLE_MARGIN, Surroundings, clearance};
