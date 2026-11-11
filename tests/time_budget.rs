//! Java-style per-robot time budgets: a robot that takes too long skips turns.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use robocode::api::{Commands, Robot, SkippedTurnEvent, Status};
use robocode::physics::{BattleConfig, SpawnSpec};
use robocode::robots::SittingDuck;
use robocode::runner::{Battle, boxed};

/// Sleeps `ms` in every `run`; counts runs and skipped-turn notices.
struct Slow {
    ms: u64,
    runs: Rc<Cell<u32>>,
    skipped_notices: Rc<Cell<u32>>,
}

impl Robot for Slow {
    fn name(&self) -> &str { "Slow" }
    fn run(&mut self, _s: &Status, _c: &mut Commands) {
        self.runs.set(self.runs.get() + 1);
        std::thread::sleep(Duration::from_millis(self.ms));
    }
    fn on_skipped_turn(&mut self, _e: &SkippedTurnEvent, _s: &Status, _c: &mut Commands) {
        self.skipped_notices.set(self.skipped_notices.get() + 1);
    }
}

fn battle(ms: u64) -> (Battle, Rc<Cell<u32>>, Rc<Cell<u32>>) {
    let (runs, notices) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    let slow = Slow { ms, runs: runs.clone(), skipped_notices: notices.clone() };
    let spawns = [SpawnSpec::new(0, 100.0, 100.0, 0.0), SpawnSpec::new(1, 700.0, 500.0, 0.0)];
    (Battle::new(BattleConfig::default(), &spawns, vec![Box::new(slow), boxed(SittingDuck)]), runs, notices)
}

#[test]
fn without_a_budget_nothing_is_skipped() {
    let (mut b, runs, notices) = battle(2);
    for _ in 0..10 {
        b.step();
    }
    assert_eq!(runs.get(), 10);
    assert_eq!(notices.get(), 0);
    assert_eq!(b.skipped_turns(), &[0, 0]);
}

#[test]
fn a_slow_robot_skips_one_turn_per_budget_used() {
    // 5 ms per turn against a 2 ms budget: each run costs at least 2 skipped turns.
    let (mut b, runs, notices) = battle(5);
    b.set_time_budget(Some(Duration::from_millis(2)));
    for _ in 0..30 {
        b.step();
    }
    let skipped = b.skipped_turns()[0];
    assert!(runs.get() + skipped == 30, "runs {} + skipped {skipped}", runs.get());
    assert!(skipped >= 2 * (runs.get() - 1), "runs {} skipped {skipped}", runs.get());
    assert!(notices.get() >= runs.get() - 1, "every run after a skip hears about it");
    assert_eq!(b.skipped_turns()[1], 0);
}

#[test]
fn thirty_consecutive_skips_disable_the_robot() {
    // 40 ms against a 1 ms budget: at least 40 turns skipped in a row.
    let (mut b, _runs, _notices) = battle(40);
    b.set_time_budget(Some(Duration::from_millis(1)));
    for _ in 0..35 {
        b.step();
    }
    assert!(!b.state().robots[0].alive, "disabled after 30 consecutive skipped turns");
}
