//! `Tactician`: picks a maneuver every turn by a fixed priority, with
//! hysteresis so it does not flip between maneuvers on every scan.

use crate::strategy::maneuver::memory::ManeuverMemory;
use crate::strategy::maneuver::{Intent, Maneuver, Risk, RiskParams};
use crate::strategy::{Action, Observation, Strategy};

/// Turns a maneuver is kept before a lower-priority rule may replace it.
const HOLD_TURNS: u32 = 8;

/// Turns in Search (or Patrol) with nothing seen before the other is tried.
const SEARCH_SWAP_TURNS: u32 = 150;

/// The maneuver for this turn.
///
/// 1. Evade if a shot was detected (Aggressive ignores it above its reserve);
///    Evade may interrupt at any time.
/// 2. Keep `current` for its first 8 turns (an Evade lasts `evade_turns`).
/// 3. Search if the target is stale or missing; after 150 turns of that,
///    Patrol, and back to Search after 150 more.
/// 4. Defensive if energy is below the reserve.
/// 5. Attack if the target was scanned this turn.
/// 6. Patrol.
pub fn choose(
    current: Maneuver,
    turns_in: u32,
    shot: bool,
    obs: &Observation,
    p: &RiskParams,
    risk: Risk,
) -> Maneuver {
    let low = obs.energy < p.reserve;
    if shot && (risk != Risk::Aggressive || low) {
        return Maneuver::Evade;
    }
    let hold =
        if current == Maneuver::Evade { p.evade_turns } else { HOLD_TURNS };
    if turns_in < hold {
        return current;
    }
    let lost = obs.target.is_none_or(|t| t.age > p.stale_after);
    if lost {
        // Search is a fixed orbit and may never find a hidden opponent, so
        // with nothing seen it alternates with Patrol (spec section 4).
        return match current {
            Maneuver::Search if turns_in >= SEARCH_SWAP_TURNS => {
                Maneuver::Patrol
            }
            Maneuver::Patrol if turns_in < SEARCH_SWAP_TURNS => {
                Maneuver::Patrol
            }
            _ => Maneuver::Search,
        };
    }
    match obs.target {
        _ if low => Maneuver::Defensive,
        Some(t) if t.age == 0 => Maneuver::Attack,
        _ => Maneuver::Patrol,
    }
}

pub struct Tactician {
    risk: Risk,
    mem: ManeuverMemory,
    current: Maneuver,
}

impl Tactician {
    pub fn new(risk: Risk, seed: u64) -> Self {
        Tactician {
            risk,
            mem: ManeuverMemory::new(seed),
            current: Maneuver::Search,
        }
    }
}

impl Default for Tactician {
    fn default() -> Self {
        Tactician::new(Risk::Normal, 1)
    }
}

impl Strategy for Tactician {
    fn name(&self) -> &str {
        "Tactician"
    }

    fn decide(&mut self, obs: &Observation) -> Action {
        let params = self.risk.params();
        let shot = self.mem.observe(obs);
        let next = choose(
            self.current,
            self.mem.turns_in_maneuver,
            shot,
            obs,
            &params,
            self.risk,
        );
        if next != self.current {
            self.mem.turns_in_maneuver = 0;
            if next == Maneuver::Evade {
                self.mem.clear_stuck();
            }
            self.current = next;
        }
        let action = next.run(obs, &mut self.mem, &params);
        self.mem.turns_in_maneuver += 1;
        action
    }

    fn intent(&self) -> Option<Intent> {
        Some(Intent { maneuver: self.current, risk: self.risk })
    }
}
