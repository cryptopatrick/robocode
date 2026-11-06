//! Tests guarding the trusted boundary: the clauses Verus assumes about the
//! unverified f64 side. See docs/verification-receipt.md.

use std::cell::Cell;
use std::rc::Rc;

use robocode::Battlefield;
use robocode::physics::BattleConfig;
use robocode::roster;
use robocode::runner::{Battle, boxed};
use robocode::strategy::{Action, Observation, Strategy, is_well_formed};
use robocode::units::Angle;

/// A1: `Angle::from_degrees` always lands in [0, 3600), even for extreme input.
#[test]
fn angle_from_degrees_is_always_in_range() {
    let inputs = [0.0, -0.0, 359.96, -0.04, 720.5, -1e-12, 1e300, -1e300, f64::INFINITY, f64::NEG_INFINITY, f64::NAN, f64::MAX, f64::MIN];
    for d in inputs {
        let a = Angle::from_degrees(d);
        assert!((0..3600).contains(&a.0), "{d} -> {}", a.0);
    }
    for i in -100_000..100_000 {
        let a = Angle::from_degrees(i as f64 * 0.37);
        assert!((0..3600).contains(&a.0));
    }
}

/// Wraps a strategy and checks every observation it is given.
struct Checked<S: Strategy> {
    inner: S,
    checked: Rc<Cell<u32>>,
}

impl<S: Strategy> Strategy for Checked<S> {
    fn name(&self) -> &str {
        self.inner.name()
    }
    fn decide(&mut self, obs: &Observation) -> Action {
        assert!(is_well_formed(obs), "runner built an ill-formed observation: {obs:?}");
        self.checked.set(self.checked.get() + 1);
        self.inner.decide(obs)
    }
}

/// A2: the runner only builds well-formed observations (SafeShooter's precondition).
#[test]
fn runner_observations_are_well_formed() {
    let checked = Rc::new(Cell::new(0));
    for (w, h) in [(400.0, 400.0), (800.0, 600.0), (5000.0, 5000.0)] {
        let robots = vec![
            boxed(Checked { inner: robocode::robots::SafeShooter, checked: checked.clone() }),
            roster::built_in("Circler").unwrap(),
            boxed(Checked { inner: robocode::robots::Spinner, checked: checked.clone() }),
            roster::built_in("SittingDuck").unwrap(),
        ];
        let config = BattleConfig { battlefield: Battlefield::new(w, h), ..BattleConfig::default() };
        Battle::new(config, &roster::spawns(4, w, h), robots).run(3000);
    }
    assert!(checked.get() > 1000, "only {} observations checked", checked.get());
}

/// The executable check agrees with the spec on the edges.
#[test]
fn is_well_formed_rejects_out_of_range() {
    let ok = Observation {
        turn: 0, energy: robocode::units::Energy(100_000), gun_heat: robocode::units::GunHeat(0),
        heading: Angle(0), gun_heading: Angle(3599), radar_heading: Angle(1800), velocity: 0,
        x: 10, y: 10, field_width: 100_000, field_height: 0, target: None,
    };
    assert!(is_well_formed(&ok));
    assert!(!is_well_formed(&Observation { heading: Angle(3600), ..ok }));
    assert!(!is_well_formed(&Observation { gun_heading: Angle(-1), ..ok }));
    assert!(!is_well_formed(&Observation { field_width: 100_001, ..ok }));
}
