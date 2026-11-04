//! RoboCode WASM physics and battle simulation primitives.
//!
//! The crate currently focuses on the deterministic core responsible for
//! replicating the classic RoboCode battle loop. Higher-level robot APIs,
//! events, or rendering layers can build on top of these modules without being
//! coupled to any specific runtime.

// Verus keeps loop-invariant attributes during verification; this unstable
// feature is enabled only then (`verus_keep_ghost` is set by Verus itself).
#![cfg_attr(verus_keep_ghost, feature(proc_macro_hygiene))]

mod geometry;
pub mod robots;
pub mod physics;
pub mod rules;
pub mod runner;
pub mod strategy;
pub mod units;

pub use geometry::{Battlefield, Point};
