//! Snapshot tests: a readable turn-by-turn trace of fixed battles. When a
//! change alters a battle, the snapshot diff shows the first turn that differs
//! and what changed there. Review with `cargo insta review` (or
//! `INSTA_UPDATE=always cargo test --test snapshots` to accept everything).

use robocode::BulletMode;
use robocode::physics::BattleConfig;
use robocode::runner::Battle;
use robocode::terrain::Terrain;
use robocode::{Battlefield, roster};

/// Pinned to arcade: these tests assume classic Robocode bullet speed.
fn arcade() -> BattleConfig {
    BattleConfig { bullet_mode: BulletMode::Arcade, ..BattleConfig::default() }
}

fn battle(names: &[&str], width: f64, height: f64, urban: bool) -> Battle {
    let robots =
        names.iter().map(|n| roster::built_in(n).expect("built-in")).collect();
    let config = BattleConfig {
        battlefield: Battlefield::new(width, height),
        ..arcade()
    };
    let mut b = Battle::new(
        config,
        &roster::spawns(names.len(), width, height),
        robots,
    );
    if urban {
        b.set_terrain(Terrain::urban(width, height));
    }
    b
}

/// One line per robot and turn, plus what happened that turn. Numbers are
/// rounded so the snapshot reads well and float noise doesn't matter.
fn trace(mut b: Battle, turns: u64) -> String {
    let mut out = String::new();
    for turn in 1..=turns {
        let events = b.step();
        for r in &b.state().robots {
            out.push_str(&format!(
                "t{turn:>3} {:<12} at ({:>6.1},{:>6.1}) heading {:>5.1} energy {:>6.2} gun {:.2}{}\n",
                b.name(r.id),
                r.position.x,
                r.position.y,
                r.heading,
                r.energy,
                r.gun_heat,
                if r.alive { "" } else { " (dead)" },
            ));
        }
        let mut what = Vec::new();
        if !events.bullet_fired.is_empty() {
            what.push(format!("fired {}", events.bullet_fired.len()));
        }
        if !events.bullet_hits.is_empty() {
            what.push(format!("hits {}", events.bullet_hits.len()));
        }
        if !events.bullet_hit_obstacles.is_empty() {
            what.push(format!(
                "absorbed by buildings {}",
                events.bullet_hit_obstacles.len()
            ));
        }
        if !events.robot_deaths.is_empty() {
            what.push(format!("deaths {:?}", events.robot_deaths));
        }
        if !what.is_empty() {
            out.push_str(&format!("      {}\n", what.join(", ")));
        }
    }
    out
}

#[test]
fn open_arena_trace() {
    insta::assert_snapshot!(trace(
        battle(&["SafeShooter", "Spinner"], 800.0, 600.0, false),
        60
    ));
}

#[test]
fn urban_trace() {
    insta::assert_snapshot!(trace(
        battle(&["SafeShooter", "Spinner"], 800.0, 600.0, true),
        60
    ));
}

#[test]
fn three_rounds_scores() {
    let mut b = battle(&["Spinner", "SittingDuck"], 800.0, 600.0, false);
    let results = b.run_rounds(3, 500);
    let mut out = String::new();
    for (i, name) in ["Spinner", "SittingDuck"].iter().enumerate() {
        let s = &results.scores[i];
        out.push_str(&format!(
            "{name}: wins {} survived {} bullet damage {:.2} total score {:.2} (survival {:.2}, last survivor {:.2}, bullet damage {:.2}, kill bonus {:.2}, ram {:.2})\n",
            results.wins[i],
            results.survived[i],
            results.bullet_damage[i],
            s.total(),
            s.survival,
            s.last_survivor_bonus,
            s.bullet_damage,
            s.bullet_kill_bonus,
            s.ram_damage,
        ));
    }
    insta::assert_snapshot!(out);
}
