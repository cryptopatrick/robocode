use robocode::api::{Commands, relative_bearing};

#[test]
fn commands_last_call_wins() {
    let mut c = Commands::default();
    c.set_ahead(50.0);
    c.set_back(20.0);
    assert_eq!(c.ahead(), Some(-20.0));
    c.set_fire(1.0);
    c.set_fire(2.5);
    assert_eq!(c.fire(), Some(2.5));
}

#[test]
fn left_is_negative_right() {
    let mut c = Commands::default();
    c.set_turn_gun_left(30.0);
    c.set_turn_left(10.0);
    c.set_turn_radar_left(45.0);
    assert_eq!((c.turn_gun(), c.turn_body(), c.turn_radar()), (Some(-30.0), Some(-10.0), Some(-45.0)));
}

#[test]
fn unset_commands_are_none() {
    let c = Commands::default();
    assert_eq!((c.ahead(), c.turn_body(), c.turn_gun(), c.turn_radar(), c.fire()), (None, None, None, None, None));
}

#[test]
fn relative_bearing_wraps() {
    assert_eq!(relative_bearing(10.0, 350.0), 20.0);
    assert_eq!(relative_bearing(170.0, 350.0), 180.0);
    assert_eq!(relative_bearing(0.0, 180.0), 180.0);
    assert_eq!(relative_bearing(350.0, 10.0), -20.0);
}

// ---- Runner integration -------------------------------------------------

use std::cell::RefCell;
use std::rc::Rc;

use robocode::api::{
    BulletHitEvent, BulletMissedEvent, HitByBulletEvent, HitWallEvent, Robot, RobotDeathEvent,
    ScannedRobotEvent, Status,
};
use robocode::physics::{BattleConfig, SpawnSpec};
use robocode::runner::Battle;

type Log = Rc<RefCell<Vec<String>>>;
type Script = Box<dyn FnMut(&Status, &mut Commands)>;

/// Logs every call; `run` executes `script`.
struct Recorder {
    name: String,
    log: Log,
    scans: Rc<RefCell<Vec<ScannedRobotEvent>>>,
    statuses: Rc<RefCell<Vec<Status>>>,
    script: Script,
}

impl Recorder {
    fn new(name: &str, script: Script) -> (Self, Log) {
        let log: Log = Rc::default();
        let r = Recorder { name: name.into(), log: log.clone(), scans: Rc::default(), statuses: Rc::default(), script };
        (r, log)
    }
    fn idle(name: &str) -> (Self, Log) {
        Self::new(name, Box::new(|_, _| {}))
    }
}

impl Robot for Recorder {
    fn name(&self) -> &str {
        &self.name
    }
    fn run(&mut self, status: &Status, cmd: &mut Commands) {
        self.log.borrow_mut().push("run".into());
        self.statuses.borrow_mut().push(status.clone());
        (self.script)(status, cmd);
    }
    fn on_scanned_robot(&mut self, e: &ScannedRobotEvent, _s: &Status, _c: &mut Commands) {
        self.log.borrow_mut().push(format!("scan {}", e.name));
        self.scans.borrow_mut().push(e.clone());
    }
    fn on_hit_by_bullet(&mut self, e: &HitByBulletEvent, _s: &Status, _c: &mut Commands) {
        self.log.borrow_mut().push(format!("hit-by {} {:.0}", e.name, e.bearing));
    }
    fn on_bullet_hit(&mut self, e: &BulletHitEvent, _s: &Status, _c: &mut Commands) {
        self.log.borrow_mut().push(format!("bullet-hit {}", e.name));
    }
    fn on_bullet_missed(&mut self, _e: &BulletMissedEvent, _s: &Status, _c: &mut Commands) {
        self.log.borrow_mut().push("missed".into());
    }
    fn on_hit_wall(&mut self, e: &HitWallEvent, _s: &Status, _c: &mut Commands) {
        self.log.borrow_mut().push(format!("wall {:.0}", e.bearing));
    }
    fn on_robot_death(&mut self, e: &RobotDeathEvent, _s: &Status, _c: &mut Commands) {
        self.log.borrow_mut().push(format!("death {}", e.name));
    }
}

/// A robot and where it spawns: (x, y, heading).
type Entry = (Box<dyn Robot>, (f64, f64, f64));

fn battle(robots: Vec<Entry>) -> Battle {
    let spawns: Vec<SpawnSpec> = robots.iter().enumerate().map(|(i, (_, (x, y, h)))| SpawnSpec::new(i, *x, *y, *h)).collect();
    Battle::new(BattleConfig::default(), &spawns, robots.into_iter().map(|(r, _)| r).collect())
}

fn sweep(deg: f64) -> Script {
    Box::new(move |_, c: &mut Commands| c.set_turn_radar_right(deg))
}

#[test]
fn run_is_called_every_turn_after_handlers() {
    let (a, log) = Recorder::new("A", sweep(45.0));
    let (b, _) = Recorder::idle("B");
    let mut bt = battle(vec![(Box::new(a), (400.0, 500.0, 0.0)), (Box::new(b), (400.0, 200.0, 0.0))]);
    bt.step();
    bt.step();
    assert_eq!(log.borrow()[..3], ["run", "scan B", "run"]);
}

#[test]
fn scan_event_fields() {
    let (a, _) = Recorder::new("A", sweep(45.0));
    let scans = a.scans.clone();
    let (b, _) = Recorder::idle("B");
    let mut bt = battle(vec![(Box::new(a), (400.0, 500.0, 0.0)), (Box::new(b), (400.0, 200.0, 0.0))]);
    bt.step();
    bt.step();
    let s = scans.borrow()[0].clone();
    assert_eq!((s.name.as_str(), s.bearing, s.absolute_bearing), ("B", 0.0, 0.0));
    assert!((s.distance - 300.0).abs() < 1e-9);
    assert_eq!(s.energy, 100.0);

    // Straight behind: relative bearing 180, not -180.
    let (a, _) = Recorder::new("A", sweep(20.0));
    let scans = a.scans.clone();
    let (b, _) = Recorder::idle("B");
    let mut bt = battle(vec![(Box::new(a), (400.0, 100.0, 0.0)), (Box::new(b), (400.0, 400.0, 0.0))]);
    bt.state_mut().robots[0].radar_heading = 170.0;
    bt.step();
    bt.step();
    let s = scans.borrow()[0].clone();
    assert_eq!((s.bearing, s.absolute_bearing), (180.0, 180.0));
}

#[test]
fn two_fire_calls_one_bullet() {
    let fire_twice: Script = Box::new(|_, c: &mut Commands| {
        c.set_fire(1.0);
        c.set_fire(3.0);
    });
    let (a, _) = Recorder::new("A", fire_twice);
    let (b, _) = Recorder::idle("B");
    let mut bt = battle(vec![(Box::new(a), (100.0, 300.0, 0.0)), (Box::new(b), (700.0, 300.0, 0.0))]);
    let fired = bt.step().bullet_fired;
    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].power, 3.0);
}

#[test]
fn duplicate_names_are_numbered() {
    let (a, _) = Recorder::idle("SittingDuck");
    let (b, _) = Recorder::idle("SittingDuck");
    let statuses = b.statuses.clone();
    let (c, _) = Recorder::idle("SittingDuck");
    let mut bt = battle(vec![
        (Box::new(a), (100.0, 100.0, 0.0)),
        (Box::new(b), (700.0, 500.0, 0.0)),
        (Box::new(c), (700.0, 100.0, 0.0)),
    ]);
    bt.step();
    assert_eq!((bt.name(0), bt.name(1), bt.name(2)), ("SittingDuck", "SittingDuck (2)", "SittingDuck (3)"));
    assert_eq!(statuses.borrow()[0].name, "SittingDuck (2)");
    assert_eq!(statuses.borrow()[0].others, 2);
}

#[test]
fn death_is_reported_once_and_dead_get_no_calls() {
    let shoot: Script = Box::new(|_, c: &mut Commands| c.set_fire(3.0));
    let (a, _) = Recorder::new("A", shoot);
    let (b, b_log) = Recorder::idle("B");
    let (c, c_log) = Recorder::idle("C");
    let mut bt = battle(vec![
        (Box::new(a), (400.0, 500.0, 0.0)),
        (Box::new(b), (400.0, 200.0, 0.0)),
        (Box::new(c), (100.0, 100.0, 90.0)),
    ]);
    bt.state_mut().robots[1].energy = 0.5;
    for _ in 0..60 {
        bt.step();
    }
    assert!(!bt.state().robots[1].alive);
    let calls_after_death = b_log.borrow().iter().skip_while(|l| !l.starts_with("hit-by")).count();
    assert_eq!(calls_after_death, 0, "B log: {:?}", b_log.borrow());
    let deaths: Vec<_> = c_log.borrow().iter().filter(|l| l.starts_with("death")).cloned().collect();
    assert_eq!(deaths, ["death B"]);
}

#[test]
fn hit_by_bullet_and_bullet_hit_events() {
    let shoot_once: Script = Box::new(|s: &Status, c: &mut Commands| {
        if s.time == 0 {
            c.set_fire(3.0);
        }
    });
    let (a, a_log) = Recorder::new("A", shoot_once);
    let (b, b_log) = Recorder::idle("B");
    let mut bt = battle(vec![(Box::new(a), (400.0, 500.0, 0.0)), (Box::new(b), (400.0, 200.0, 0.0))]);
    for _ in 0..30 {
        bt.step();
    }
    assert!(a_log.borrow().contains(&"bullet-hit B".to_string()), "{:?}", a_log.borrow());
    assert!(b_log.borrow().contains(&"hit-by A 180".to_string()), "{:?}", b_log.borrow());
    assert!(!a_log.borrow().contains(&"missed".to_string()));
}

#[test]
fn bullet_missed_event() {
    let shoot_once: Script = Box::new(|s: &Status, c: &mut Commands| {
        if s.time == 0 {
            c.set_fire(3.0);
        }
    });
    let (a, a_log) = Recorder::new("A", shoot_once);
    let (b, _) = Recorder::idle("B");
    let mut bt = battle(vec![(Box::new(a), (400.0, 300.0, 0.0)), (Box::new(b), (100.0, 500.0, 0.0))]);
    for _ in 0..40 {
        bt.step();
    }
    assert_eq!(a_log.borrow().iter().filter(|l| *l == "missed").count(), 1);
}

#[test]
fn hit_wall_event() {
    // Facing left into the left wall: the wall is dead ahead (0).
    let forward: Script = Box::new(|_, c: &mut Commands| c.set_ahead(100.0));
    let (a, a_log) = Recorder::new("A", forward);
    let (b, _) = Recorder::idle("B");
    let mut bt = battle(vec![(Box::new(a), (19.0, 300.0, 270.0)), (Box::new(b), (700.0, 300.0, 0.0))]);
    for _ in 0..5 {
        bt.step();
    }
    assert!(a_log.borrow().contains(&"wall 0".to_string()), "{:?}", a_log.borrow());

    // Reversing into it while facing right: the wall is behind (180).
    let backward: Script = Box::new(|_, c: &mut Commands| c.set_back(100.0));
    let (a, a_log) = Recorder::new("A", backward);
    let (b, _) = Recorder::idle("B");
    let mut bt = battle(vec![(Box::new(a), (19.0, 300.0, 90.0)), (Box::new(b), (700.0, 300.0, 0.0))]);
    for _ in 0..5 {
        bt.step();
    }
    assert!(a_log.borrow().contains(&"wall 180".to_string()), "{:?}", a_log.borrow());
}

#[test]
fn status_reports_remaining_motion() {
    let go: Script = Box::new(|s: &Status, c: &mut Commands| {
        if s.time == 0 {
            c.set_ahead(100.0);
        }
    });
    let (a, _) = Recorder::new("A", go);
    let statuses = a.statuses.clone();
    let (b, _) = Recorder::idle("B");
    let mut bt = battle(vec![(Box::new(a), (400.0, 500.0, 0.0)), (Box::new(b), (100.0, 100.0, 0.0))]);
    bt.step();
    bt.step();
    let s = &statuses.borrow()[1];
    assert!(s.distance_remaining > 0.0 && s.distance_remaining < 100.0, "{}", s.distance_remaining);
    assert_eq!(s.time, 1);
    assert_eq!((s.battlefield_width, s.battlefield_height), (800.0, 600.0));
}
