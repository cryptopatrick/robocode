//! Robocode's scoring, per Java's RobotStatistics.

use robocode::api::{Commands, Robot, Status};
use robocode::physics::{BattleConfig, SpawnSpec};
use robocode::runner::{Battle, Score};

/// Fires `power` whenever the gun is cool (gun points up, at heading 0).
struct Gunner(f64);
impl Robot for Gunner {
    fn name(&self) -> &str { "Gunner" }
    fn run(&mut self, _s: &Status, c: &mut Commands) { c.set_fire(self.0); }
}

/// Drives forward every turn.
struct Rammer;
impl Robot for Rammer {
    fn name(&self) -> &str { "Rammer" }
    fn run(&mut self, _s: &Status, c: &mut Commands) { c.set_ahead(100.0); }
}

struct Still(&'static str);
impl Robot for Still {
    fn name(&self) -> &str { self.0 }
    fn run(&mut self, _s: &Status, _c: &mut Commands) {}
}

fn close(a: f64, b: f64) -> bool { (a - b).abs() < 1e-9 }

#[test]
fn bullets_score_capped_damage_kill_bonus_and_survival() {
    // Target (energy 20) sits straight above the Gunner. Two 3.0 shots: 16 + 4 (capped).
    let spawns = [SpawnSpec::new(0, 400.0, 500.0, 0.0), SpawnSpec::new(1, 400.0, 200.0, 0.0)];
    let mut b = Battle::new(BattleConfig::default(), &spawns, vec![Box::new(Gunner(3.0)), Box::new(Still("Target"))]);
    b.state_mut().robots[1].energy = 20.0;
    let outcome = b.run(500);
    assert_eq!(outcome.winner, Some(0));
    let s: Score = b.scores()[0];
    assert!(close(s.bullet_damage, 20.0), "{s:?}");
    assert!(close(s.bullet_kill_bonus, 0.2 * 20.0), "{s:?}");
    assert!(close(s.survival, 50.0), "{s:?}");
    assert!(close(s.last_survivor_bonus, 10.0), "{s:?}");
    assert!(close(s.total(), 20.0 + 4.0 + 50.0 + 10.0), "{s:?}");
    assert!(close(b.scores()[1].total(), 0.0));
}

#[test]
fn ramming_scores_bonus_per_hit_and_kill_bonus() {
    // Rammer (heading right) pushes into a victim with 1.0 energy: two hits of 0.6.
    let spawns = [SpawnSpec::new(0, 400.0, 300.0, 90.0), SpawnSpec::new(1, 440.0, 300.0, 0.0)];
    let mut b = Battle::new(BattleConfig::default(), &spawns, vec![Box::new(Rammer), Box::new(Still("Victim"))]);
    b.state_mut().robots[1].energy = 1.0;
    let outcome = b.run(200);
    assert_eq!(outcome.winner, Some(0));
    let s = b.scores()[0];
    assert!(close(s.ram_damage, 2.0 * 1.2), "{s:?}");
    assert!(close(s.ram_kill_bonus, 0.3 * 1.2), "{s:?}");
    assert!(close(s.survival, 50.0) && close(s.last_survivor_bonus, 10.0), "{s:?}");
}

#[test]
fn timeout_gives_no_last_survivor_bonus() {
    let spawns = [SpawnSpec::new(0, 100.0, 100.0, 0.0), SpawnSpec::new(1, 700.0, 500.0, 0.0)];
    let mut b = Battle::new(BattleConfig::default(), &spawns, vec![Box::new(Still("A")), Box::new(Still("B"))]);
    let outcome = b.run(50);
    assert_eq!(outcome.winner, None);
    assert_eq!(b.scores(), &[Score::default(), Score::default()]);
}

#[test]
fn rounds_add_up_scores() {
    let spawns = [SpawnSpec::new(0, 400.0, 500.0, 0.0), SpawnSpec::new(1, 400.0, 200.0, 0.0)];
    let robots: Vec<Box<dyn Robot>> = vec![Box::new(Gunner(3.0)), Box::new(Still("Target"))];
    let mut b = Battle::new(BattleConfig::default(), &spawns, robots);
    let r = b.run_rounds(2, 2000);
    assert_eq!(r.scores.len(), 2);
    // Every round the Gunner outlives the Target: 50 survival + 10 last-survivor each.
    let won: u32 = r.wins[0];
    assert!(close(r.scores[0].survival, 50.0 * won as f64), "{:?}", r.scores[0]);
    assert!(close(r.scores[0].last_survivor_bonus, 10.0 * won as f64));
    assert!(r.scores[0].total() > r.scores[1].total());
}
