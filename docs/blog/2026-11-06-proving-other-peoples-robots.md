# Proving Other People's Robots: What Verus Says About Real Robocode Code

Patrick Kothbauer · November 6, 2026 · *Formal Methods*

> "Program testing can be used to show the presence of bugs, but never to show their absence!"
> — Edsger W. Dijkstra [1]

**TL;DR**

- Two earlier posts proved properties about a robot *I* wrote [2, 3]. This one asks a harder question: what does formal verification say about robots *other people* wrote, without verification in mind?
- I ported five Java `AdvancedRobot`s from public Robocode archives [4] to Rust, kept them local, and ran a monitored 1v1 tournament against our own robots.
- A runtime monitor checked each robot against the properties Verus proves for `SafeShooter`. **Every human-written robot broke at least one**: firing with a hot gun up to 93% of the time, firing its last energy, or driving into walls hundreds of times.
- I then pulled the interesting decision code out of each robot into small integer models and asked Verus [5, 6]. It rejected all four originals, for reasons you can read straight off the Java. The smallest fixes that verify are tiny: an energy guard, a precondition, `>=` instead of `>`, and an explicit `abs`.
- Enforcing the safest property at run time, "never fire your last energy", **helped one robot (+4 wins), hurt another (−5), and cost our proved robot 8 wins without changing a line of its code.** Safe and good are different specifications.

## Why other people's robots?

Proving your own code has a built-in bias: you write the code knowing the spec is coming. Real code isn't written that way. Robocode's community has written robots for twenty-five years [4], mostly by trial and error, which makes it an honest test of what a verifier notices that tests and tournaments don't.

I collected eight archives of robots from public repositories. Most have no license, so the ports stay on my machine and this post quotes only short snippets for commentary. I kept to robots written against `AdvancedRobot`, which maps onto our Rust Robot API, and left out anything using blocking calls, custom events, data files or team messaging.

That left five:

| Robot | Lines (Java) | Style |
|---|---|---|
| `NarrowBeam` | 47 | locks its radar, turns toward the enemy, fires 3.0 when aligned |
| `EfficientShooter` | 67 | spirals, tracks one enemy, power from distance |
| `ImprovedOscillator` | 92 | oscillating radar, drives toward the enemy |
| `PredictiveShooter` | 109 | aims at where the enemy *will* be |
| `SuperRamFire` | 109 | circles and rams, circular targeting |

*Table 1. The ported robots.*

## Porting is where the first bugs were

Before any proof, faithful porting forced three engine changes and taught three lessons.

1. **Java's gun and radar turn with the body.** Our engine had them independent, which suits the proofs, so we added a `TurretMode::Java` that copies Java's algorithm. That includes its three `setAdjust…` flags and the quirk that `setAdjustRadarForGunTurn(x)` silently sets a second flag too.
2. **`setMaxVelocity` takes the absolute value.** My first version clamped negative speeds to 0. Java uses $\min(|v|, 8)$ and ignores NaN, so mine would have stopped `SuperRamFire` every time it turned left.
3. **Java builds a new robot object every round.** Only `static` fields survive. My first ports kept their "first turn" setup flag between rounds, so in round 2 three of them never switched their radar on and sat out 5,000 turns. The fix: reset per-round state in `on_round_ended`, keeping the Java statics.

None of these is a verification result. All three are what the Dijkstra quote warns about: the bugs you find are the ones your tests happen to reach.

## The safety monitor

Our proved robot, `SafeShooter`, comes with a Verus spec [3]. I turned its clauses into a runtime monitor that wraps any robot and checks what it actually commands, turn by turn:

| Property (from `SafeShooter`'s spec) | Monitor counts |
|---|---|
| The gun is cool when firing | fire commands while the gun is hot (the engine ignores them) |
| Energy left over after the shot | shots whose power reaches the robot's energy |
| A legal power, $0.1 \le p \le 3.0$ | fire commands outside that range |
| Never drive into a wall | wall collisions |
| Never hold fire when ready | turns with a cool gun, a robot scanned this turn, the gun within 3° of it, and energy $> 1.0$, but no shot |

*Table 2. Spec clauses as runtime checks.*

**A monitor bug, caught by a proof.** On the first run, `SafeShooter`, proved to fire *if and only if* it's ready, showed 33 turns of holding fire when ready. Either Verus was wrong or the monitor was. Logging those turns settled it: all 33 had exactly $0.875$ energy. The spec's $\mathrm{Ready}$ requires more than $1.0$, the smallest shot `SafeShooter` uses, while my monitor had used $0.1$. The proof was right and the measuring tool was wrong. After aligning the monitor with the spec, `SafeShooter` scored $0/1541$, as it must.

## Results: everyone breaks something

Twenty-eight 1v1 matches of 10 rounds each, on an 800 × 600 arena:

| Robot | Wins | Walls | Fire cmds | Hot gun | Last-energy shots | Held fire / ready |
|---|---:|---:|---:|---:|---:|---:|
| SuperRamFire | 59 | 450 | 11,540 | 10,783 | 5 | 0 / 191 |
| Circler *(ours, Robot API)* | 52 | 341 | 16,991 | 15,735 | 6 | 0 / 1,193 |
| EfficientShooter | 45 | 1,408 | 1,518 | 0 | 1 | 10 / 645 |
| ImprovedOscillator | 36 | 391 | 13,759 | 12,804 | 6 | 0 / 678 |
| PredictiveShooter | 35 | 1,295 | 1,929 | 0 | 12 | 114 / 234 |
| **SafeShooter** *(ours, Verus-proved)* | 31 | **0** | 2,600 | **0** | **0** | **0 / 1,541** |
| NarrowBeam | 12 | 0 | 481 | 408 | 0 | 0 / 41 |
| Spinner *(ours, unverified)* | 10 | 963 | 5,790 | 3,543 | 20 | 0 / 15 |

*Table 3. Standings and monitor totals across all matches. No robot ever commanded an illegal power.*

What stands out:

- **Only the proved robot breaks nothing**, which is what a correct proof plus a correct monitor should give. It also finishes sixth. The proofs say nothing about winning.
- **Firing with a hot gun is the norm.** `SuperRamFire` asks to fire on every scan, and 93% of those commands are ignored. That's harmless in Robocode, because the engine drops them, but it's a clear case of code whose *intent* differs from its effect. A spec would force the author to decide.
- **Last-energy shots happen everywhere, including in our own samples.** `Spinner` and `Circler` have no energy checks either.
- **"Held fire when ready" is the wrong property for some robots.** `PredictiveShooter` holds fire on 114 of 234 "ready" turns because it aims at where the enemy *will be*, not where it is. `SafeShooter`'s notion of "aimed" doesn't describe its intent. A spec encodes one robot's strategy, and applying it to another can be meaningless.

## What Verus says about the code itself

The monitor shows *behaviour in these battles*. Verus shows *what can happen at all*. For each robot I extracted the interesting decision into a small integer model (milli-energy, pixels, degrees), wrote the spec it should meet, and verified it. Each `original_*` keeps the Java logic; each `fixed_*` is the smallest change that verifies.

### 1. NarrowBeam fires its last energy

```java
if (Math.abs(getTurnRemaining()) < 10) {
    ...
    setFire(3.0);
}
```

```rust
pub fn original_narrow_beam_fire(energy: u32, aligned: bool) -> (r: Option<u32>)
    ensures r matches Some(p) ==> energy > p,
{ if aligned { Some(3000) } else { None } }
```

Verus: **postcondition not satisfied** (any energy $\le 3000$). The fix adds `&& energy > 3000` to the guard.

### 2. EfficientShooter divides by distance

```java
setFire(Math.min(400 / enemy.getDistance(), 3));
```

With the spec "a legal power", $100 \le r \le 3000$, Verus reports **two** problems: a **possible division by zero**, and a power below $0.1$ once the distance exceeds 4000 px. In Java the first gives `Infinity`, which `min` then quietly turns into 3.0, so the code works by accident. The fix is a precondition, `requires 1 <= distance <= 4000`, which on an 800 × 600 arena always holds. Verus also needed one nonlinear hint to connect $400\,000 / d \ge 100$ with $d \le 4000$:

```rust
assert(p >= 100) by (nonlinear_arith) requires p == 400_000u32 / distance, 1u32 <= distance <= 4000u32;
```

### 3. PredictiveShooter forgets the axes

```java
if (xo > 0 && yo > 0) { bearing = arcSin; }
else if (xo < 0 && yo > 0) { bearing = 360 + arcSin; }
else if (xo > 0 && yo < 0) { bearing = 180 - arcSin; }
else if (xo < 0 && yo < 0) { bearing = 180 - arcSin; }
```

Every comparison is strict. A target exactly due east, west or south falls through all four branches, and the bearing stays $0$: north. I modelled the case analysis as a function returning which branch fires, with the spec

$$
(x_o \ne 0 \;\vee\; y_o \ne 0) \;\Longrightarrow\; r \ne 0 .
$$

Verus: **postcondition not satisfied**, at $x_o = 0$. The fix turns four `>`/`<` into `>=`/`<=`. In practice the bug almost never fires, since exact zero differences are rare with floating-point positions, and the monitor never caught it in a battle. That's exactly why it's a good example. Tests sample; proofs cover.

### 4. SuperRamFire leans on Java's forgiveness

```java
setMaxVelocity(400/getTurnRemaining());
```

When the turn is finished this divides by zero, giving $+\infty$. When the robot turns left it goes negative. Java's `setMaxVelocity` takes $\min(|v|, 8)$, so both come out right. The intent ("a speed in $[0, 8]$") is nowhere in the robot's code. Verus rejects the integer model twice (division by zero, out-of-range result), and the verified version spells out the `abs` and the zero case.

### 5. A loop that terminates for a non-obvious reason

`SuperRamFire`'s circular targeting is a loop whose exit condition depends on a point that moves every iteration:

```java
while ((++deltaTime) * BULLET_SPEED < Point2D.Double.distance(getX(), getY(), predictedX, predictedY)) {
    ... predictedX = Math.max(Math.min(predictedX, getBattleFieldWidth() - 18), 18); ...
}
```

Why does it stop? Because the predicted point is clamped inside the field, the distance is bounded while `deltaTime * 11` grows. In the model, Verus needs exactly that argument written down as an invariant and a ranking function:

$$
\text{invariant } d_0 \le d \le F, \qquad \text{decreases } F + 11 - 11t .
$$

My first attempt used $F - 11t$, and Verus rejected it: on the last iteration that measure goes negative. Ranking functions are fussy, and that's their point.

| Kernel | Original | Fixed |
|---|---|---|
| NarrowBeam firing | ✗ fires last energy | ✓ |
| EfficientShooter power | ✗ division by zero, illegal power | ✓ (with precondition) |
| PredictiveShooter bearing cases | ✗ axes uncovered | ✓ |
| SuperRamFire speed cap | ✗ division by zero, out of range | ✓ |
| SuperRamFire targeting loop | — | ✓ terminates |

*Table 4. Verus on the extracted kernels: 4 originals rejected (6 errors), 5 fixed versions plus the loop verified.*

## Does enforcing a proved property help?

The cheapest way to apply a verified property to code you don't own is to enforce it at run time, wrapping the robot in a *shield* [7, 8] that drops forbidden actions. This one drops any shot that would spend the robot's last energy, which is the `fixed_narrow_beam_fire` rule applied to everyone, and passes every other command through. The same tournament, shielded:

| Robot | Wins (plain) | Wins (shielded) | Shots blocked |
|---|---:|---:|---:|
| SuperRamFire | 59 | 58 | 54 |
| Circler | 52 | 52 | 179 |
| EfficientShooter | 45 | 45 | 1 |
| PredictiveShooter | 35 | **39** | 18,638 |
| ImprovedOscillator | 36 | 36 | 201 |
| SafeShooter *(not shielded)* | 31 | **23** | — |
| Spinner | 10 | 10 | 3,035 |
| NarrowBeam | 12 | **7** | 27 |

*Table 5. Runtime enforcement of "never fire your last energy". "Blocked" counts commands. A nearly-dead robot asks to fire every turn, often for thousands of turns of a battle that runs to the turn limit.*

Three outcomes, none of them "safety is free":

- **PredictiveShooter wins 4 more.** It stops killing itself with its final shots.
- **NarrowBeam wins 5 fewer.** Sometimes its last, desperate shot was the one that finished the opponent first. "Safe" and "winning" are different specs.
- **SafeShooter loses 8 wins without changing a line.** Some of its wins came from opponents shooting themselves dead. When they stopped, SafeShooter's record got worse. A proof about your own robot says nothing about the world it runs in.

One engine detail sharpens this. In our engine a robot at 0 energy is removed; classic Robocode only *disables* it until it's hit again. Firing your last energy is therefore deadlier here than in the original game, and the table would look different under Java's rule.

## What we learned

1. **Verus reads intent off code.** Each rejected kernel pointed at a line where the author relied on something unstated: Java's `abs`, `Infinity` behaving, positions never landing on an axis.
2. **Faithful porting is its own verification problem.** Turret coupling, `setMaxVelocity`'s `abs`, round-by-round object lifetimes: each was a small spec about Java's semantics that I got wrong first.
3. **Check your monitors against your proofs.** The proof caught a bug in the monitor, not the other way round.
4. **Properties belong to strategies.** "Hold fire only when not aimed" is right for SafeShooter and meaningless for a predictive shooter. There is no universal "safe robot".
5. **Enforcement changes the game for everyone.** A shield helped one robot, hurt another, and changed the record of a robot it never touched.

## What this does not show

1. **The kernels are models, not the robots.** Each is a few lines of integer logic extracted by hand from `f64` Java code. Verus proves the models, not the ports.
2. **The ports may differ from the originals.** Despite matching Java's turret, velocity and round semantics, our engine moves robots and bullets before checking collisions (Java interleaves per robot), uses fixed spawn corners rather than random ones, and removes robots at 0 energy.
3. **Five robots, one arena, 10 rounds per match** is enough to find violations, but not to rank strategies with any confidence. Round-to-round variance is high.
4. **The monitor checks one robot's notion of safety.** Its "held fire" column is meaningless for predictive shooters.
5. **Nothing here is about the originals' authors' skill.** These robots weren't written to be verified, and most work well. That's the point: correct enough to win is not the same as specified.

## Reproducing

The engine changes (Java turret mode, collisions, rounds, `setMaxVelocity`) are in [github.com/cryptopatrick/robocode](https://github.com/cryptopatrick/robocode). The ports themselves stay local, because most originals have no license. To try it with robots you are allowed to port:

```sh
cargo robocode new mybot                              # write the port
cargo robocode battle mybot.rs --vs SafeShooter --rounds 10
```

Write the kernels you care about as `verus!` functions with the spec they *should* meet, and run `cargo verus verify`. Expect the first failures to be in your specs.

## References

1. E. W. Dijkstra, "Notes on Structured Programming", EWD249, 1970.
2. P. Kothbauer, "Fire, but Verify: Proving a Robot Tank Safe with Verus", cryptopatrick.com, November 2026.
3. P. Kothbauer, "Who Checks the Spec? Mutation-Testing Verus Proofs in RoboVerus", cryptopatrick.com, November 2026.
4. M. A. Nelson and Robocode contributors, *Robocode*, https://robocode.sourceforge.io/, 2001–2026; and the RoboWiki community, https://robowiki.net/.
5. A. Lattuada, T. Hance, C. Cho, M. Brun, I. Subasinghe, Y. Zhou, J. Howell, B. Parno, C. Hawblitzel, "Verus: Verifying Rust Programs using Linear Ghost Types", OOPSLA 2023.
6. A. Lattuada et al., "Verus: A Practical Foundation for Systems Verification", SOSP 2024.
7. F. B. Schneider, "Enforceable Security Policies", *ACM Transactions on Information and System Security* 3(1), 2000.
8. R. Bloem, B. Könighofer, R. Könighofer, C. Wang, "Shield Synthesis: Runtime Enforcement for Reactive Systems", TACAS 2015.

## Cite this article

Patrick Kothbauer, "Proving Other People's Robots: What Verus Says About Real Robocode Code", cryptopatrick.com, November 6, 2026.

```bibtex
@misc{kothbauer2026otherrobots,
  author       = {Patrick Kothbauer},
  title        = {Proving Other People's Robots: What Verus Says About Real Robocode Code},
  year         = {2026},
  month        = nov,
  howpublished = {\url{https://cryptopatrick.com/research/proving-other-peoples-robots/}},
  note         = {Accessed: 2026-11-06}
}
```
