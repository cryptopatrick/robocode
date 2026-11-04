# Floats, integers and what gets proved

RoboCode's physics, in the original Java and in this crate's engine, use floating-point numbers (`f64`). Verus reasons very well about integers and very poorly about floats. RoboVerus handles that with one clear split.

| | Uses | Matches Java? | Proved by Verus? |
|---|---|---|---|
| Physics engine (`robocode::physics`) | `f64` | yes | no |
| Strategies (`Observation`, `Action`, `decide`) | integers, see below | via conversion | yes |

**The `f64` physics matches Java. The fixed-point model is what gets proved. The two are not the same code.**

## The units

| Quantity | Type | Unit | Example |
|---|---|---|---|
| energy, gun heat, bullet power | `Energy`, `GunHeat`, `Power` (`u32`) | milli-units (×1000) | `Power(1500)` = 1.5 |
| angles | `Angle` (`i32`) | decidegrees (×10), `0..3600` | `Angle(900)` = 90° |
| positions, distances | `i32` | whole pixels | |
| velocity | `i32` | decipixels per turn | `80` = 8 px/turn |

Milli-units make every legal bullet power (0.1 to 3.0) and the gun cooling rate (0.1 per turn) exact.

## What the proofs do cover

Strategies use these integer units in **every** build, not just when verifying. So the `decide` function Verus proves is exactly the function that runs in battles. The gap lies at the boundary only: the runner converts the engine's floats into an `Observation`, and converts the `Action` back.

That conversion rounds **in the safe direction**:

- **energy rounds down**, so a robot never thinks it has more energy than it does;
- **gun heat rounds up**, so a gun with float residue like `0.0000000000000001` reads as still hot, which is how the engine treats it.

So a proof like "SafeShooter only fires when the gun heat is 0" stays true about the real engine, not just about the model.
