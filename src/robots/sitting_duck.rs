use crate::strategy::{Action, Observation, Strategy};

/// Never moves, turns or fires.
pub struct SittingDuck;

impl Strategy for SittingDuck {
    fn name(&self) -> &str {
        "SittingDuck"
    }
    fn decide(&mut self, _obs: &Observation) -> Action {
        Action::default()
    }
}
