//! The built-in robots and where robots spawn, shared by the command-line
//! tool and the browser engine so they never drift apart.

use crate::api::Robot;
use crate::physics::SpawnSpec;
use crate::robots::{Circler, SafeShooter, SittingDuck, Spinner};
use crate::runner::boxed;

/// Names of the built-in robots.
pub const BUILT_IN: [&str; 4] = ["SafeShooter", "Circler", "Spinner", "SittingDuck"];

/// A fresh built-in robot, or `None` for an unknown name.
pub fn built_in(name: &str) -> Option<Box<dyn Robot>> {
    match name {
        "SafeShooter" => Some(boxed(SafeShooter)),
        "Circler" => Some(Box::new(Circler::default())),
        "Spinner" => Some(boxed(Spinner)),
        "SittingDuck" => Some(boxed(SittingDuck)),
        _ => None,
    }
}

/// Spawn points for `n` robots (at most 4): the corners, inset 100 px, in the
/// order top-left, bottom-right, top-right, bottom-left; all heading 0°.
pub fn spawns(n: usize, width: f64, height: f64) -> Vec<SpawnSpec> {
    let corners = [(100.0, 100.0), (width - 100.0, height - 100.0), (width - 100.0, 100.0), (100.0, height - 100.0)];
    (0..n.min(corners.len())).map(|i| SpawnSpec::new(i, corners[i].0, corners[i].1, 0.0)).collect()
}
