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

So a proof like "SafeShooter only fires when the gun heat is 0" stays true about the engine **as it was at the start of the turn**, not just about the model.

## What the proofs don't cover

A strategy decides once, at the start of a turn. The engine then runs the whole turn: turning, **movement**, scanning, firing, bullets. Anything that changes the robot's state between the observation and the shot is outside the proof:

- **Gun heat is fine**: the engine only cools the gun during the turn, so a gun that was cool stays cool.
- **Energy is not fully covered**: hitting a wall during movement costs energy *before* the engine fires. A robot that saw 2.0 energy, fired 1.0 and drove into a wall at full speed could end the turn dead. SafeShooter's wall avoidance makes this unlikely, but the proof doesn't promise it. Closing this gap means modelling movement too, which is a job for a later chapter on multi-turn invariants.
