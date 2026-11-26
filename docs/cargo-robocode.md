# `cargo robocode`

Write, battle and verify robots on your own machine. Nothing is uploaded: your robot compiles with your own `cargo`, and the battle opens in your browser from a local page.

## Install

```sh
cd web && npm install && npm run wasm && npm run build && cd ..   # the tool embeds the battle viewer
cargo install --path cli
```

The tool compiles robots against the robocode checkout it was built from. If you move that checkout, pass `--robocode-path <dir>` or set `ROBOCODE_PATH`.

## Commands

```sh
cargo robocode new hunter                              # writes hunter.rs from the template
cargo robocode battle hunter.rs                        # vs SittingDuck, opens the browser
cargo robocode battle hunter.rs --vs Circler --vs ../friends/bot.rs
cargo robocode battle hunter.rs --arena 1000x1000 --turns 3000
cargo robocode battle hunter.rs --vs Circler --rounds 10       # each round's winner, then totals
cargo robocode battle hunter.rs --vs Circler --live            # runs while you watch (30 turns/s)
cargo robocode battle hunter.rs --live --tps max               # live, as fast as possible
cargo robocode battle hunter.rs --urban                         # fight on the city map (buildings block tanks and bullets)
cargo robocode battle hunter.rs --bullets arcade                # classic Robocode bullet speed (default: realistic)
cargo robocode battle hunter.rs --output battle.json --no-open   # just save the recording
cargo robocode battle hunter.rs --no-open --glb battle.glb      # save the battle as a 3D scene
cargo robocode export battle.json --glb battle.glb             # turn a saved recording into a 3D scene
cargo robocode verify hunter.rs                        # run Verus on it
cargo robocode list                                    # built-in robots
```

- **`battle`** takes 2 to 4 robots in total. Opponents are built-in names or `.rs` files, and the same file may appear twice. Compile errors print as normal `cargo` errors, and the tool exits 1. The browser page serves only on `127.0.0.1`; press Ctrl+C to stop it. `--verbose` shows cargo's build output.
- The **first battle** compiles the robocode library, which takes a little while. Later battles reuse it. The cache lives in your platform's cache folder under `robocode/build` (or `ROBOCODE_BUILD_DIR`).
- **Exit codes:** 0 for success, 1 for a robot that won't compile, crashes or fails verification, 2 for usage or setup problems.

## 3D export

`--glb` (on `battle`) and `export` write the battle as a glTF 2.0 binary (`.glb`): the arena, the map, the tanks and one animation clip, `battle`, with a keyframe every turn at 30 turns per second. It opens in Unity through the free glTFast package (`com.unity.cloud.gltfast`), and in Godot, Blender, Unreal and three.js. `export` reads any saved recording, including real-city maps such as the Ginza recordings from robocheck. `--glb` cannot be combined with `--live`.

- **Scale and axes:** metres, Y up. The scale is the map's `px_per_m`, or 4 px per metre without one, so a robot (36 px) is 9 m across and a tank's hull 7 m wide. Arena `x` is `+X` and arena `y` (south) is `+Z`, with the arena's north-west corner at the origin.
- **What is in it:** ground and walls. Buildings are extruded footprints at their measured heights when the map has them, otherwise boxes, with two materials: `building` for walls and `roof`. The file also holds road strips clipped to the arena, crossings, traffic lights and subway entrances. Each robot drives its tank from the recording's `tanks` header (or its slot's default), the same five low-poly models as the viewer's 3D view, at the viewer's sizes. The tank's own colours are materials such as `leopard2a8/base`; the slot colour is only on the muzzle cap and the radar dish. A dead tank is squashed to 40% of its height from the turn it dies. Tracks and wheels do not move. Bullets are small spheres, and a bullet that is fired and hits on the same turn is a one-turn tracer. Bullets appear and disappear through their scale, because glTF cannot animate visibility.
- **Textures:** the file has none, but every mesh has texture coordinates in metres (`TEXCOORD_0`): faces that point up or down use (X, Z), walls use the distance along the wall and the height. A facade, asphalt or zebra texture therefore tiles at a real-world scale, set by the material's tiling.
- **Names and metadata:** nodes have stable names (`robot0`, `robot0/hull`, `robot0/gun`, `robot0/gun/barrel`, `robot0/radar`, `bullet17`, `building3`, `road12`), so a Unity script can find them and swap in its own models. Each robot's `extras` hold its name, slot and tank. The scene's `extras` hold the robot names, the winner, the scale, the bullet mode and, for a map, its name, labels and `attribution`. **A viewer that shows a map must show its attribution.**
- **Not exported yet:** sounds, explosions and hit sparks, the radar beam, label text, textures, moving tracks, wreck colours and rounds after the first.

## Robot files

- **One `.rs` file.** It may use `std` and `robocode` only; no other crates for now.
- Define a type that implements `robocode::api::Robot` (see `docs/robot-api.md`) and `Default`. Name it once at the end:

  ```rust
  robocode::robot!(Hunter);            // a Robot API robot
  robocode::robot!(strategy MyShooter); // a fixed-point Strategy (RoboVerus)
  ```

- **Proofs:** robots verified by `cargo robocode verify` use Verus attribute specs gated on `verus_keep_ghost`, so the same file still compiles for battles without Verus:

  ```rust
  #[cfg(verus_keep_ghost)]
  use vstd::prelude::*;

  #[cfg_attr(verus_keep_ghost, verus_verify)]
  #[cfg_attr(verus_keep_ghost, verus_spec(r =>
      ensures r.fire matches Some(p) ==> p.0 <= 3000,
  ))]
  pub fn decide(obs: &Observation) -> Action { /* … */ }
  ```

  `verify` needs `cargo verus` on `PATH` (see `docs/verus-integration.md`). It reports how many specs it checked, and "0 specs checked" means there was nothing to prove, not that the robot is proved safe.

## Not yet

- **Running other people's robots in a sandbox.** Robot files compile natively, so only battle code you trust.
- **Sharing robots online.** The planned RoboCode Arena accepts Verus-verified robot profiles instead of code; see `docs/roadmap.md`.
