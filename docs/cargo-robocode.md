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
cargo robocode battle hunter.rs --output battle.json --no-open   # just save the recording
cargo robocode verify hunter.rs                        # run Verus on it
cargo robocode list                                    # built-in robots
```

- **`battle`** takes 2 to 4 robots in total. Opponents are built-in names or `.rs` files, and the same file may appear twice. Compile errors print as normal `cargo` errors, and the tool exits 1. The browser page serves only on `127.0.0.1`; press Ctrl+C to stop it. `--verbose` shows cargo's build output.
- The **first battle** compiles the robocode library, which takes a little while. Later battles reuse it. The cache lives in your platform's cache folder under `robocode/build` (or `ROBOCODE_BUILD_DIR`).
- **Exit codes:** 0 for success, 1 for a robot that won't compile, crashes or fails verification, 2 for usage or setup problems.

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
