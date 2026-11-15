//! RoboCheck Phase 0: a candidate strategy (standing in for AI output) is slotted
//! into a protected template, verified per property with Verus, and run in a
//! battle; verification and battle share one implementation hash.

pub mod battle;
pub mod candidate;
pub mod compile;
pub mod evidence;
pub mod pipeline;
pub mod template;
pub mod verify;
pub mod workspace;
