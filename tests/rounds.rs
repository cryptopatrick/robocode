use std::cell::RefCell;
use std::rc::Rc;

use robocode::api::{Commands, Robot, RoundEndedEvent, Status};
use robocode::physics::BattleConfig;
use robocode::roster;
use robocode::runner::{Battle, boxed};
use robocode::robots::{SafeShooter, SittingDuck};

type Log = Rc<RefCell<Vec<String>>>;

/// Wraps a robot and logs round callbacks plus its start position each round.
struct Logged {
    inner: Box<dyn Robot>,
    log: Log,
}

impl Robot for Logged {
    fn name(&self) -> &str {
        self.inner.name()
    }
    fn run(&mut self, s: &Status, c: &mut Commands) {
        if s.time == 0 {
            self.log.borrow_mut().push(format!("start {:.0},{:.0}", s.x, s.y));
        }
        self.inner.run(s, c);
    }
    fn on_scanned_robot(&mut self, e: &robocode::api::ScannedRobotEvent, s: &Status, c: &mut Commands) {
        self.inner.on_scanned_robot(e, s, c);
    }
    fn on_robot_death(&mut self, e: &robocode::api::RobotDeathEvent, s: &Status, c: &mut Commands) {
        self.inner.on_robot_death(e, s, c);
    }
    fn on_death(&mut self, _s: &Status) {
        self.log.borrow_mut().push("death".into());
    }
    fn on_win(&mut self, _s: &Status) {
        self.log.borrow_mut().push("win".into());
    }
    fn on_round_ended(&mut self, e: &RoundEndedEvent) {
        self.log.borrow_mut().push(format!("round-ended {} of {}", e.round, e.total_rounds));
    }
}

fn duel(a_log: &Log, b_log: &Log) -> Battle {
    let robots: Vec<Box<dyn Robot>> = vec![
        Box::new(Logged { inner: boxed(SafeShooter), log: a_log.clone() }),
        Box::new(Logged { inner: boxed(SittingDuck), log: b_log.clone() }),
    ];
    Battle::new(BattleConfig::default(), &roster::spawns(2, 800.0, 600.0), robots)
}

#[test]
fn three_rounds_three_outcomes() {
    let (a, b): (Log, Log) = (Rc::default(), Rc::default());
    let results = duel(&a, &b).run_rounds(3, 5000);
    assert_eq!(results.rounds.len(), 3);
    assert_eq!(results.wins, vec![3, 0]);
    assert_eq!(results.survived, vec![3, 0]);
    assert!(results.bullet_damage[0] >= 300.0, "{:?}", results.bullet_damage);
    assert_eq!(results.bullet_damage[1], 0.0);
}

#[test]
fn spawns_rotate_each_round() {
    let (a, b): (Log, Log) = (Rc::default(), Rc::default());
    duel(&a, &b).run_rounds(3, 5000);
    let starts: Vec<String> = a.borrow().iter().filter(|l| l.starts_with("start")).cloned().collect();
    assert_eq!(starts, ["start 100,100", "start 700,500", "start 100,100"]);
}

#[test]
fn round_callbacks_once_per_round() {
    let (a, b): (Log, Log) = (Rc::default(), Rc::default());
    duel(&a, &b).run_rounds(2, 5000);
    let events = |log: &Log| -> Vec<String> { log.borrow().iter().filter(|l| !l.starts_with("start")).cloned().collect() };
    assert_eq!(events(&a), ["win", "round-ended 0 of 2", "win", "round-ended 1 of 2"]);
    assert_eq!(events(&b), ["death", "round-ended 0 of 2", "death", "round-ended 1 of 2"]);
}

#[test]
fn single_run_reports_win_and_round_end() {
    let (a, b): (Log, Log) = (Rc::default(), Rc::default());
    let outcome = duel(&a, &b).run(5000);
    assert_eq!(outcome.winner, Some(0));
    assert!(a.borrow().contains(&"win".to_string()));
    assert!(b.borrow().contains(&"death".to_string()));
}
