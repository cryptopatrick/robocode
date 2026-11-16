//! Two robots fight on the urban map: buildings stop tanks at their faces and
//! absorb bullets. Run with `cargo run --example urban`.

use robocode::physics::BattleConfig;
use robocode::runner::Battle;
use robocode::terrain::Terrain;
use robocode::{Battlefield, roster};

fn main() {
    let (width, height) = (800.0, 600.0);
    let robots = vec![
        roster::built_in("SafeShooter").expect("built-in robot"),
        roster::built_in("Spinner").expect("built-in robot"),
    ];
    let config = BattleConfig {
        battlefield: Battlefield::new(width, height),
        ..BattleConfig::default()
    };
    let mut battle =
        Battle::new(config, &roster::spawns(2, width, height), robots);
    battle.set_terrain(Terrain::urban(width, height));

    // Play turn by turn, so the buildings' effect on bullets can be counted.
    let mut absorbed = 0;
    let mut turns = 0;
    while turns < 5000
        && battle.state().robots.iter().filter(|r| r.alive).count() > 1
    {
        absorbed += battle.step().bullet_hit_obstacles.len();
        turns += 1;
    }

    println!("Buildings: {}", battle.state().terrain.buildings.len());
    println!("Turns: {turns}");
    println!("Bullets absorbed by buildings: {absorbed}");
    for r in &battle.state().robots {
        println!(
            "{}: energy {:.1}{}",
            battle.name(r.id),
            r.energy,
            if r.alive { "" } else { " (dead)" }
        );
    }
}
