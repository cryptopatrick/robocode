# Fire, but Verify: Proving a Robot Tank Safe with Verus

Patrick Kothbauer · November 5, 2026 · *Formal Methods*

> "Program testing can be used to show the presence of bugs, but never to show their absence!"
> — Edsger W. Dijkstra [1]

**TL;DR**

- I ported the classic programming game Robocode [2] to Rust, and used it as a playground for learning **Verus** [3, 4], a verifier for Rust built on the Z3 SMT solver [5].
- Robots that should be *proved* are written as a single function, $\mathit{decide}: \mathit{Observation} \to \mathit{Action}$, over whole-number units. The function Verus proves is the same function that fights in battles.
- The main result: Verus proves that the sample robot `SafeShooter` **never fires unsafely**, for every possible observation, not for a few test cases.
- The most instructive moment wasn't a proof failing. It was a proof *succeeding* about the wrong property: the "safe" robot could shoot itself dead.
- Next: a verified interpreter for an online Arena. Verus proves the interpreter once, for **every** robot profile anyone will ever upload, so the server never runs or checks untrusted code.

## Why a tank game?

Robocode has been teaching people to program since 2001 [2]. You write the brain of a small tank. It scans with a radar, turns its gun and fires bullets, and an engine runs the battle. The rules are small and precise: bullet power is between 0.1 and 3.0, firing heats the gun by $1 + p/5$, and the gun can only fire when its heat is zero.

That precision is what makes it good for formal verification. The rules read like a specification already. And a robot's job — look, decide, act, every turn — has exactly the shape verifiers like: a pure function from what you see to what you do.

I wanted a way to learn Verus that wasn't another proof that `min(x, y) <= x`. So the project became two things:

1. A Rust crate, `robocode`, that runs battles like the original Java game, with no dependencies at all.
2. **RoboVerus**: a guide that follows the official Verus tutorial chapter by chapter, but every example is about robots.

## Verus in one paragraph

Verus [3] lets you write specifications and proofs *in Rust syntax*, next to ordinary Rust code. Every function can carry a precondition (`requires`) and a postcondition (`ensures`). Verus turns each function into verification conditions, logical formulas that hold exactly when the code meets its spec, and hands them to Z3 [5]. This is Floyd–Hoare reasoning [6, 7], automated in the style of Dafny [8] and Boogie [9]. For a function $f$ with precondition $P$ and postcondition $Q$, Verus checks the Hoare triple

$$
\{\,P\,\}\; f \;\{\,Q\,\}
\quad\text{i.e.}\quad
\forall x.\; P(x) \Rightarrow Q\big(x, f(x)\big).
$$

Three things about Verus mattered most here:

- **Callers only see the spec.** When code calls `decide`, Verus doesn't look inside it; it only knows the `ensures`. So a spec is an interface, and a weak spec makes every caller's proof fail.
- **Ghost code is erased.** Specs and proofs disappear when the code is compiled, so verification costs nothing at run time.
- **Integers are easy and floats are hard.** Z3 reasons about linear integer arithmetic very well. IEEE-754 floating point [10] is a different world.

That last point shaped everything else.

## The architecture: two kinds of numbers

The original game's physics uses `double`. If I want battles to follow the original, the engine has to use `f64`. If I want proofs, the robots have to use integers. So the crate has two layers with a deliberate boundary between them:

```
 ┌─────────────────────────────┐        ┌──────────────────────────────┐
 │  physics engine   (f64)     │        │  strategy        (integers)  │
 │  follows the Java original  │        │  proved by Verus             │
 │                             │        │                              │
 │  RobotState, bullets,       │ observe│  Observation ──► decide ──►  │
 │  radar scans, walls         ├───────►│                      Action  │
 │                             │◄───────┤                              │
 │                             │ apply  │  same code in battle and     │
 └─────────────────────────────┘        │  in the proof                │
          not verified                  └──────────────────────────────┘
                 ▲                                  ▲
                 └──────── the trust boundary ──────┘
                   (energy rounds down, heat rounds up)
```

*Figure 1. The `f64` engine follows the Java game. The integer strategy layer is what Verus proves. The runner converts between them.*

The integer units are chosen so that every legal value in the game's rules is exact:

| Quantity | Type | Unit | Example |
|---|---|---|---|
| energy, gun heat, bullet power | `u32` | milli-units (×1000) | `Power(1500)` is 1.5 |
| angles | `i32` | decidegrees (×10), absolute in $[0, 3600)$ | `Angle(900)` is 90° |
| positions, distances | `i32` | whole pixels | |

Milli-units make the smallest bullet (0.1) and the gun's cooling rate (0.1 per turn) exact. The gun heat rule becomes integer arithmetic:

$$
\mathit{heat}(p) = 1000 + \left\lfloor p / 5 \right\rfloor
\qquad (0 \le p \le 3000),
$$

and Verus proves its first, warm-up property without help: $1000 \le \mathit{heat}(p) \le 1600$.

The important design decision: **robots use these integer units in every build, not just when verifying.** Proving a model and then hoping the real code matches it is the classic gap in formal methods. Here the `decide` that Verus proves *is* the `decide` the battle calls, so the gap shrinks to the conversion at the boundary. That conversion rounds in the safe direction: energy rounds down, gun heat rounds up.

## What *counts* as safe?

Here is the specification at the heart of the project, as Verus spec functions. These are pure mathematical definitions that exist only for verification.

**Definition 1 (well-formed observation).** An observation $o$ is well-formed, $\mathrm{WF}(o)$, when its body, gun and radar headings are in $[0, 3600)$, its battlefield is at most $100\,000$ pixels per side, and any target's bearing is in $[0, 3600)$.

The runner only ever produces well-formed observations, and since a later fix `Battle::new` refuses battlefields beyond the proof's limit.

**Definition 2 (signed angle difference).** For angles $a, b \in [0, 3600)$,

$$
\delta(a, b) =
\begin{cases}
a - b - 3600 & \text{if } a - b > 1800,\\
a - b + 3600 & \text{if } a - b \le -1800,\\
a - b & \text{otherwise.}
\end{cases}
$$

In words: the shortest signed turn from $b$ to $a$. Verus proves that the executable `angle_diff` equals $\delta$, that $-1800 < \delta(a,b) \le 1800$, and that $b + \delta(a,b) \equiv a \pmod{3600}$. I wrote the congruence as a three-way disjunction, not with `%`, because SMT solvers handle nonlinear modulo poorly and the disjunction is equivalent on this range.

**Definition 3 (safe to fire).** Firing power $p$ under observation $o$ is safe, $\mathrm{Safe}(o, p)$, when

$$
\begin{aligned}
&100 \le p \le 3000 && \text{(a legal bullet)}\\
\wedge\;& o.\mathit{energy} > p && \text{(energy left over afterwards)}\\
\wedge\;& o.\mathit{gunHeat} = 0 && \text{(the gun is cool)}\\
\wedge\;& o.\mathit{target} = \mathrm{Some}(t) \wedge t.\mathit{age} = 0 && \text{(a target seen this turn)}\\
\wedge\;& |\delta(t.\mathit{bearing},\, o.\mathit{gunHeading})| \le 30 && \text{(aimed within } 3.0^\circ\text{)}
\end{aligned}
$$

In Verus it reads almost the same:

```rust
pub open spec fn safe_to_fire(obs: Observation, p: Power) -> bool {
    &&& MIN_POWER.0 <= p.0 <= MAX_POWER.0
    &&& obs.energy.0 > p.0
    &&& obs.gun_heat.0 == 0
    &&& obs.target matches Some(t) && t.age == 0
    &&& aimed(obs)
}
```

**Proposition 1 (SafeShooter never fires unsafely).**

$$
\forall o.\;\; \mathrm{WF}(o) \;\wedge\; \mathit{decide}(o).\mathit{fire} = \mathrm{Some}(p)
\;\Longrightarrow\; \mathrm{Safe}(o, p).
$$

*Proof.* Attached to `decide` as `requires well_formed(*obs)` and `ensures r.fire matches Some(p) ==> safe_to_fire(*obs, p)`. `decide` sets `fire` in exactly one place, guarded by `gun_heat == 0 && age == 0 && is_aimed(obs) && energy > p`. The power `p` is one of two constants, $1000$ or $3000$, both in range. `is_aimed` has its own proved postcondition, which says it returns exactly $\mathit{aimed}(o)$, so at the call site Verus knows the guard implies every conjunct of Definition 3. On every other path `fire` is `None` and the implication holds trivially. Verus discharges the verification conditions with Z3 and needs no proof hints. ∎

A note on what the proof is about. It isn't about a model of `decide`; it is about the Rust function that runs in `cargo run --example battle`. The specs sit in attributes on ordinary Rust:

```rust
#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires well_formed(*obs),
    ensures r.fire matches Some(p) ==> safe_to_fire(*obs, p),
))]
pub fn decide(obs: &Observation) -> Action { /* … */ }
```

Without the `verus` feature the attributes vanish and the crate compiles with no Verus crates at all. With it, Verus verifies the same function body.

### A loop, and an invariant

The guide's third chapter needs a loop: pick the closest of several scanned robots. Verus can't run a loop to see what it does, so the loop carries an invariant:

$$
I(i, b) \;\equiv\; 1 \le i \le n \;\wedge\; b < i \;\wedge\;
\forall j.\; 0 \le j < i \Rightarrow d_b \le d_j .
$$

It holds on entry ($i = 1, b = 0$), each iteration preserves it, and on exit $i = n$ turns it into the postcondition: $d_b \le d_j$ for every $j$. The `decreases n - i` clause proves termination. It's induction, written as a loop annotation.

## Results

| Item | Count |
|---|---|
| Items Verus verifies in the crate | 25 |
| Items verified in the guide's reference solutions | 9 |
| Exercise holes (`// TODO`) that fail until a learner fills them | 8 |
| Rust tests (library and tools) | 107 |
| Verus job time in CI | ~1.5 minutes |

Everything here verifies with plain `requires`, `ensures` and loop invariants. No manual proof blocks, no triggers written by hand. That's a deliberate choice for a teaching project: the early chapters should feel like writing good types, not like writing proofs.

## The one real bug

The most useful moment of the project was a proof that went through.

The first version of Definition 3 said $o.\mathit{energy} \ge p$: enough energy to pay for the shot. Verus proved Proposition 1 with it, and the tests passed. Then an independent code review ran a battle in which SafeShooter had exactly 1.0 energy and fired a 1.0 shot:

```
turn 1:  fired = [1.0]   alive = false   energy = 0   deaths = [0]     (robot 0 = SafeShooter)
```

*Figure 2. A "proved safe" robot shooting itself dead.*

The engine removes robots whose energy reaches zero. The proof was correct, and the property was wrong. The fix was one character in the spec ($\ge$ to $>$), one in the code, and a new test that fires at exactly-equal energy and checks the robot survives.

The same review found two more spec gaps of the same kind:

1. **Bullet power wasn't constrained.** Nothing in Definition 3 said the power had to be legal. Today's code only uses legal constants, but a learner editing it could slip an illegal power past the proof. The first conjunct was added afterwards.
2. **The proof covers the start of the turn only.** The engine moves robots *before* it fires, and hitting a wall costs energy. A robot that saw 2.0 energy, fired 1.0 and crashed at full speed in the same turn could still die. The docs claimed more than that, so they were corrected to say exactly what is proved.

None of these was a code bug Verus missed. Each was a question about the specification, and Verus can't answer those. Which is the whole point of the Dijkstra quote, turned around: a proof shows the absence of bugs *relative to the spec*, and nothing more.

## Does the checker catch wrong code?

A proof that can't fail tells you nothing, so every important spec got a deliberate break, to check that Verus notices:

| Change made on purpose | Verus says |
|---|---|
| Remove `gun_heat == 0` from `decide`'s firing guard | `postcondition not satisfied` |
| Fire an illegal `Power(50)` | `postcondition not satisfied` |
| Remove the `energy > p` check | `postcondition not satisfied` |
| Claim `heat(p) >= 1001` | `postcondition not satisfied` (fails at $p < 5$) |
| Weaken the closest-target loop invariant | verification fails |

Every break was caught, and reverted afterwards.

## Making Verus optional

The crate has to be useful to people who have never heard of Verus. So:

- `vstd`, Verus's standard library, is an **optional** dependency behind a `verus` feature.
- Specs live in `cfg_attr(feature = "verus", …)` attributes, so they disappear from normal builds.
- Spec-only items, which Verus erases, are imported only under `cfg(verus_only)`, a flag Verus sets itself, so plain `cargo build --features verus` still compiles.
- CI checks that a default build compiles **zero** Verus crates.

The same trick lets developers write proved robots in a single file: specs gated on Verus's own `verus_keep_ghost` flag compile normally for battles and get checked by `cargo robocode verify`.

## Where Verus goes next

Everything above is about one decision, on one turn. The roadmap is about widening what is quantified over.

### 1. The Arena: verify the interpreter, not each upload

The next big addition is an online Arena: up to 8 robots per battle, in timed slots, from different people. The hard requirement is that **the server runs no user code**.

The obvious design is to let people upload a proof with their robot. That doesn't work. A "verified ✓" flag computed on the user's machine can be forged. And checking an uploaded proof on the server means running Verus, a tool built on the Rust compiler, on untrusted input. That is precisely the attack surface we wanted to avoid. Proof-carrying code [11] solves a similar problem with a tiny trusted proof checker. Verus isn't tiny.

So the plan inverts it. Robots in the Arena are **profiles**: JSON in a small, fixed rule language (states, each with prioritised "if this, then that" rules, with capped size). One interpreter runs every profile, and Verus proves it once, for all of them:

$$
\forall P.\;\; \mathrm{WF}_{\mathrm{profile}}(P) \;\Longrightarrow\;
\forall o.\;\; \mathrm{WF}(o) \Rightarrow
\Big(
\mathit{terminates}\big(\mathit{interp}(P, o)\big)
\;\wedge\;
\mathit{interp}(P,o).\mathit{fire} = \mathrm{Some}(p) \Rightarrow \mathrm{Safe}(o, p)
\Big).
$$

The quantifier over $P$ is the point. The server's only job per upload is checking $\mathrm{WF}_{\mathrm{profile}}(P)$ against a schema: a decidable, cheap test that runs no code and no prover. The proved interpreter is the same idea that makes verified compilers like CompCert [12] worth having: verify the tool once, and every input it accepts inherits the guarantee.

```
  user's machine                          Arena server
 ┌──────────────────────┐              ┌──────────────────────────────┐
 │ profile.json         │   upload     │ 1. schema + size check       │
 │ cargo robocode       ├─────────────►│    (no code, no prover)      │
 │   profile check      │              │ 2. verified interpreter      │
 │   battle (same       │              │    runs all 8 profiles       │
 │   interpreter)       │              │ 3. recording → viewer        │
 └──────────────────────┘              └──────────────────────────────┘
          ▲                                          ▲
          └───── one interpreter, proved once ───────┘
```

*Figure 3. The planned Arena. The interpreter carries the proof, and uploads carry only data.*

The trade-off is expressiveness: Arena robots can do only what the profile language can say. Full Rust robots stay on your own machine. Learners can still prove extra properties of their *own* profile locally ("never drives into a wall"). The Arena doesn't need to trust those proofs; they're for learning.

### 2. Multi-turn invariants

A single-turn property can't say "SafeShooter never dies by its own hand over a whole battle". That needs a model of state over time: a transition relation $s \to s'$ and an inductive invariant $I$:

$$
\mathit{Init}(s) \Rightarrow I(s),
\qquad
I(s) \wedge s \to s' \Rightarrow I(s').
$$

This is also where the "start of the turn" gap from earlier closes. Once movement is part of the model, wall damage within a turn is covered.

### 3. Liveness: "eventually attacks"

Safety says nothing bad happens. A robot that never fires is perfectly safe, and perfectly useless. The guide's later chapters will state progress properties in the temporal-logic style of Pnueli [13], roughly $\square\,(\mathit{targetVisible} \Rightarrow \lozenge\, \mathit{fires})$, and show how Verus handles them with ranking functions and `decreases` clauses.

### 4. Moving the trust boundary

Today the runner computes a target's bearing with floating-point `atan2` and hands it to `decide`, so the trigonometry sits outside the proof. Integer geometry for bearings would pull it inside. Further out is a fixed-point model of the engine's own rules (gun heat, damage, energy bookkeeping) proved equivalent to the integer functions the guide already verifies.

## What this does not show

1. **The physics engine is not verified.** It uses `f64`, and reasoning about floating point is notoriously treacherous [10]. Proofs are about the integer strategy layer.
2. **The conversion at the boundary is tested, not proved.** For example, "the observation says the gun is cool exactly when the engine would fire" is checked turn by turn for five shot powers in a test. It is not a theorem.
3. **Bearings come from trusted float code.** "Aimed" means aimed at the bearing the runner computed.
4. **The proofs cover the start of a turn.** Energy lost to a wall later in the same turn is outside Proposition 1.
5. **The trusted base is large.** Verus, Z3 and rustc are all trusted, as with any SMT-based verifier.
6. **A proof is only as good as its spec.** The self-kill story above is the evidence. Specs got better through review and adversarial testing, not through verification.
7. **Robot API robots are not verified at all.** The everyday API uses `f64` like the original game. Only fixed-point strategy robots can be proved.

## Reproducing the numbers

Everything is at [github.com/cryptopatrick/robocode](https://github.com/cryptopatrick/robocode). Tested with Verus `0.2026.11.02.8bf68ee` and Rust `1.99.1` on macOS (arm64) and Ubuntu in CI.

```sh
# install Verus (see docs/verus-integration.md), then:
cargo verus verify --features verus          # 25 verified, 0 errors
cargo verus verify -p roboverus-solutions    # 9 verified, 0 errors
cargo verus verify -p roboverus-exercises    # fails at the 8 TODOs, as intended
cargo test --workspace --exclude roboverus-exercises --exclude roboverus-solutions
cargo run --example battle -- spinner        # watch the proved robot fight
```

To break the main proof yourself, delete `obs.gun_heat.0 == 0 &&` from the firing guard in `src/robots/safe_shooter.rs` and verify again.

## References

1. E. W. Dijkstra, "Notes on Structured Programming", EWD249, 1970.
2. M. A. Nelson and Robocode contributors, *Robocode*, https://robocode.sourceforge.io/, 2001–2026.
3. A. Lattuada, T. Hance, C. Cho, M. Brun, I. Subasinghe, Y. Zhou, J. Howell, B. Parno, C. Hawblitzel, "Verus: Verifying Rust Programs using Linear Ghost Types", OOPSLA 2023.
4. A. Lattuada et al., "Verus: A Practical Foundation for Systems Verification", SOSP 2024.
5. L. de Moura, N. Bjørner, "Z3: An Efficient SMT Solver", TACAS 2008.
6. R. W. Floyd, "Assigning Meanings to Programs", *Proceedings of Symposia in Applied Mathematics* 19, 1967.
7. C. A. R. Hoare, "An Axiomatic Basis for Computer Programming", *Communications of the ACM* 12(10), 1969.
8. K. R. M. Leino, "Dafny: An Automatic Program Verifier for Functional Correctness", LPAR-16, 2010.
9. M. Barnett, B.-Y. E. Chang, R. DeLine, B. Jacobs, K. R. M. Leino, "Boogie: A Modular Reusable Verifier for Object-Oriented Programs", FMCO 2005.
10. D. Monniaux, "The Pitfalls of Verifying Floating-Point Computations", *ACM TOPLAS* 30(3), 2008.
11. G. C. Necula, "Proof-Carrying Code", POPL 1997.
12. X. Leroy, "Formal Verification of a Realistic Compiler", *Communications of the ACM* 52(7), 2009.
13. A. Pnueli, "The Temporal Logic of Programs", FOCS 1977.

## Cite this article

Patrick Kothbauer, "Fire, but Verify: Proving a Robot Tank Safe with Verus", cryptopatrick.com, November 5, 2026.

```bibtex
@misc{kothbauer2026fireverify,
  author       = {Patrick Kothbauer},
  title        = {Fire, but Verify: Proving a Robot Tank Safe with Verus},
  year         = {2026},
  month        = nov,
  howpublished = {\url{https://cryptopatrick.com/research/fire-but-verify/}},
  note         = {Accessed: 2026-11-05}
}
```
