# Introduction

RoboVerus teaches [Verus](https://github.com/verus-lang/verus) through RoboCode, the classic game where you program a tank and send it into battle.

It follows the structure of the official [Verus guide](https://verus-lang.github.io/verus/guide/). Where the Verus guide proves facts about `min(x, y)` or about arrays, RoboVerus proves facts about robots. For example:

- the gun heat produced by a shot always stays between 1.0 and 1.6;
- `SafeShooter` never fires unless it has the energy, a cool gun and an aimed gun;
- a loop over scanned enemies really finds the weakest one.

## What you work with

- **The `robocode` crate**: a Rust port of RoboCode. It works as an ordinary Rust library. Its optional `verus` feature switches on the specs and proofs.
- **Robots as strategies**: a robot is a function `decide(&Observation) -> Action`. Each turn, it sees its energy, gun heat, headings and the last enemy it scanned, then decides what to fire, how to move and how to turn.
- **`SafeShooter`**: the running example robot, in `src/robots/safe_shooter.rs`. Its `decide` function is proved safe by Verus, and the very same function fights in battles:

```sh
cargo run --example battle -- spinner
```

## How each chapter works

Each chapter names the Verus guide section it mirrors, explains the concept on RoboCode, and ends with exercises.

- Exercises live in `exercises/src/chNN.rs`. Each `// TODO` marks something to write.
- An exercise is solved when `cargo verus verify -p roboverus-exercises` reports no errors for it.
- Reference solutions are in `solutions/src/chNN.rs`. Try first!

This guide assumes you know some Rust. You don't need to know Verus or RoboCode.
