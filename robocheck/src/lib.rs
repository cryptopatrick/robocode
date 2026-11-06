//! RoboCheck Phase 0: a candidate strategy (standing in for AI output) is slotted
//! into a protected template, verified per property with Verus, and run in a
//! battle; verification and battle share one implementation hash.

pub mod candidate;
pub mod template;
pub mod workspace;
pub mod compile;
pub mod verify;
pub mod battle;
pub mod evidence;
pub mod pipeline;
