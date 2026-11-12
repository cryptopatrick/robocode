//! RoboCode physics, battle simulation and robot strategies in Rust.
//!
//! - [`physics`]: the deterministic `f64` engine that mirrors classic RoboCode.
//! - [`strategy`]: robots as `decide(&Observation) -> Action`, in the
//!   fixed-point units of [`units`].
//! - [`runner`]: connects strategies to the engine; [`robots`] has samples.
//!
//! # Optional Verus support
//!
//! With the `verus` feature, [Verus](https://github.com/verus-lang/verus) can
//! verify the specs attached to strategy code (see the RoboVerus guide in
//! `guide/` and `docs/verus-integration.md`). Without it, no Verus crate is
//! compiled and the specs vanish.
//!
//! **Floats vs. integers.** The `f64` physics matches Java. The fixed-point
//! model (strategies, [`units`]) is what gets proved. The two are not the
//! same code. Strategies use the integer units in every build, so a proved
//! `decide` is the code that fights; the runner converts at the boundary,
//! rounding energy down and gun heat up, so proofs hold for the engine state
//! at the start of each turn (not for damage taken later in that turn).

// Verus keeps loop-invariant attributes during verification; this unstable
// feature is enabled only then (`verus_keep_ghost` is set by Verus itself).
#![cfg_attr(verus_keep_ghost, feature(proc_macro_hygiene))]

mod geometry;
pub mod api;
pub mod live;
pub mod recording;
pub mod robots;
pub mod roster;
pub mod physics;
pub mod rules;
pub mod runner;
pub mod terrain;
pub mod strategy;
pub mod units;

pub use geometry::{Battlefield, Point, Rect};

/// Names the robot defined in a robot file, for `cargo robocode`.
///
/// ```ignore
/// robocode::robot!(MyRobot);            // a `Robot` (from robocode::api)
/// robocode::robot!(strategy MyStrategy); // a fixed-point `Strategy`
/// ```
///
/// The type must implement `Default`.
#[macro_export]
macro_rules! robot {
    (strategy $t:ty) => {
        pub fn __robocode_robot() -> ::std::boxed::Box<dyn $crate::api::Robot> {
            $crate::runner::boxed(<$t as ::std::default::Default>::default())
        }
    };
    ($t:ty) => {
        pub fn __robocode_robot() -> ::std::boxed::Box<dyn $crate::api::Robot> {
            ::std::boxed::Box::new(<$t as ::std::default::Default>::default())
        }
    };
}
