use crate::strategy::{Action, Observation, Strategy};
use crate::units::Power;

/// Drives in circles with its radar spinning; fires straight ahead of its gun
/// whenever it has just seen a target.
pub struct Spinner;

impl Strategy for Spinner {
    fn name(&self) -> &str {
        "Spinner"
    }
    fn decide(&mut self, obs: &Observation) -> Action {
        let fire = match obs.target {
            Some(t) if t.age == 0 => Some(Power(1000)),
            _ => None,
        };
        Action {
            fire,
            ahead: Some(100),
            turn_body: Some(100),
            turn_gun: Some(0),
            turn_radar: Some(450),
        }
    }
}
