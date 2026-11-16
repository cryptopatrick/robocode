# Walls Are Not Enough: An Urban Arena for RoboCode in Rust

Patrick Kothbauer · November 16, 2026 · *Simulation*

> **DRAFT.** Not yet published. The example output below is from `cargo run --example urban` at v0.5.0.

**TL;DR**

- Until now the arena's only obstacles were its four walls. Robots drove through everything else, and bullets never met anything.
- v0.5.0 adds an opt-in **urban arena**: a fixed map of buildings and roads. Buildings stop tanks at their faces and absorb bullets. Roads are for show.
- It's one flag on the command line (`--urban`), one call in the API (`set_terrain`), and a city view in the browser, with a 2D and a 3D mode.
- Robot code does not change. The new `BulletHitObstacle` event tells you when a shot hit a building, but you don't have to handle it.

## Why buildings?

RoboCode's arena is empty. That makes the first battles easy to read and the engine easy to check. It also makes a whole class of decisions disappear: where to hide, when a wall is a shield, whether a shot hits the enemy or the corner of a building. Those are the decisions that make a battle look like a fight.

So the rule we wanted was simple to state: a tank can't drive through a building, and a bullet that reaches a building is gone.

## What changed in the engine

Buildings are axis-aligned rectangles, and there are two rules.

- **Tanks.** A tank's movement is checked as a sweep from where it started to where it would end up. If the sweep meets a building's face, the tank stops at that face. There's no damage: walls hurt, buildings don't. Checking the sweep rather than the end point matters, because a fast tank would otherwise tunnel through a thin wall.
- **Bullets.** A bullet's path for the turn is checked the same way. If it crosses a building, the bullet is removed and a `BulletHitObstacle` event is raised. Nobody takes damage and nobody gets a bonus.

Roads are rectangles too, but they only appear on the map. They never block anything.

Buildings do not block radar. That was a deliberate choice for this version: line of sight would be the next rule to add, and it changes what a scanner can know.

```rust
use robocode::physics::BattleConfig;
use robocode::runner::Battle;
use robocode::terrain::Terrain;
use robocode::{Battlefield, roster};

let (width, height) = (800.0, 600.0);
let robots = vec![
    roster::built_in("SafeShooter").unwrap(),
    roster::built_in("Spinner").unwrap(),
];
let config = BattleConfig {
    battlefield: Battlefield::new(width, height),
    ..BattleConfig::default()
};
let mut battle = Battle::new(config, &roster::spawns(2, width, height), robots);
battle.set_terrain(Terrain::urban(width, height));
```

`Terrain::urban` is one fixed layout, so every urban battle is reproducible, like every other battle in the crate. It keeps only the buildings that fit the arena and leave the spawn corners free. We found this the hard way: the first version put a block past the bottom edge of the smaller arena. A test now checks every arena size the viewer offers.

## What it looks like

Running the example gives this on the default build:

```text
Buildings: 9
Turns: 280
Bullets absorbed by buildings: 26
SafeShooter: energy 79.0
Spinner: energy 0.0 (dead)
```

Twenty-six bullets met a building before they could reach anyone. Spinner, which fires in circles, is the one that lost. Whether that's a coincidence of the layout or a real tactic is a question for the next post.

On the command line, the same battle is one flag:

```text
cargo robocode battle hunter.rs --vs Spinner --urban
```

The browser viewer has an *Urban* checkbox on the new-battle form. A 2D view draws the map from above. A 3D view, built with three.js and loaded only when you ask for it, shows the same battle in perspective, with tanks rotating their hulls, turrets and radars.

## What the recording carries

A battle's recording gained an optional `terrain` entry in its header, with the buildings and roads as rectangles. Recordings without buildings are byte-for-byte what they were before, so older viewers and old files still work. The bullet events only appear in battles where a bullet was actually absorbed.

## What it does not do yet

- **Radar is not blocked.** A scanner can see through a building.
- **Absorbed bullets are not drawn.** The event is recorded and counted, but neither view shows a spark or a puff at the wall.
- **The map is fixed.** You can't yet pass your own layout. That would mean deciding how a layout is written down and checked, which we'd rather do carefully.
- **Robots can't react.** The event API has no handler for "my bullet hit a building". It is informational for now.

## What it did not change

The Verus proofs still cover the proved strategy, not the physics. The urban map doesn't touch them. The mutation checks still kill all fifteen planted bugs. The golden battles, the ones with empty arenas, produce exactly the recordings they did before.

## Lessons

The change was small to describe and fiddly to get right. Two things stood out. First, the spawn corners need a rule of their own: a building that is valid on paper can trap a robot at the start, so the map is filtered against the spawn points, not just the arena. Second, a property test found the first real problem in our own test harness, a dead robot that had moved in the turn it died. The engine was fine. The test was checking one turn too early.

The engine and the viewer are on master, and version 0.5.0 is on crates.io. The changelog has the full list.
