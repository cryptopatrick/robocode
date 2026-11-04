//! RoboCode WASM physics and battle simulation primitives.
//!
//! The crate currently focuses on the deterministic core responsible for
//! replicating the classic RoboCode battle loop. Higher-level robot APIs,
//! events, or rendering layers can build on top of these modules without being
//! coupled to any specific runtime.

mod geometry;
pub mod physics;
pub mod rules;
pub mod runner;
pub mod strategy;
pub mod units;

pub use geometry::{Battlefield, Point};
