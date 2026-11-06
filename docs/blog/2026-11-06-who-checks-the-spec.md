# Who Checks the Spec? Mutation-Testing Verus Proofs in RoboVerus

Patrick Kothbauer · November 6, 2026 · *Formal Methods*

> "The plan was always supposed to be the contract, and now the agent actually has to keep it."
> — A. Lavaee and M. Philipose, on PDD Kit [1]

**TL;DR**

- Yesterday's post [2] ended with a confession: our "proved safe" robot could shoot itself dead, because the *spec* was wrong. A proof only rules out bugs relative to its spec.
- Microsoft Research's *Proof-Driven Development* kit (PDD Kit) [1] has a practical answer to "who checks the spec?" I applied five of its practices to RoboVerus.
- The most useful one is the **mutation check**: plant one bug per `ensures` clause and require Verus [3, 4] to reject it, naming that clause. On the first run, **3 of 12 mutants survived**.
- Two survivors were real spec gaps. One of them meant **a robot that never fires satisfied our firing spec**. The third was a classic trap: an *equivalent mutant* that changed nothing.
- After the fixes: 15 of 15 mutants killed, a new proof that the robot never drives into a wall it's close to, an explicit list of every trusted assumption with the test that guards it, and six tests deleted because proofs replaced them.

## Where we left off

RoboVerus is a Rust take on Robocode [5], built for learning Verus. Its showcase robot, `SafeShooter`, comes with a proof:

$$
\forall o.\;\; \mathrm{WF}(o) \;\wedge\; \mathit{decide}(o).\mathit{fire} = \mathrm{Some}(p)
\;\Longrightarrow\; \mathrm{Safe}(o, p),
$$

where $\mathrm{Safe}$ requires a legal power, energy left over after the shot, a cool gun, a fresh target and an aimed gun. Verus checks it in seconds.

The previous post's lesson was that the first version of $\mathrm{Safe}$ said $\mathit{energy} \ge p$, which lets the robot fire its last energy and die. Verus proved the wrong property perfectly. A human reviewer found the problem, by running a battle.

That doesn't scale. So the question for this post: **how do you test a specification?**

## What PDD Kit does

PDD Kit [1] is a research prototype in which agents write code *and* Verus proofs for a web app, and humans review the intent rather than the diff. Several of its practices don't depend on agents at all:

| PDD Kit practice | What it means |
|---|---|
| **Mutation check** | Plant one bug per `ensures` clause; Verus must reject each one on the clause it targets. "A surviving mutant means the clause constrains nothing." |
| **Decision cards with both readings** | For "only with the right password", ask whether the right password must *always* work: $\Rightarrow$ or $\Leftrightarrow$? |
| **Contracts on each piece** | Decompose the logic, and give every piece its own contract. |
| **An explicit trusted boundary** | Everything outside the proof gets an *assumed* contract, accepted on purpose, with a planned test. |
| **Prune redundant tests** | Delete tests that only re-check proved behaviour; keep the tests guarding the trusted boundary. |

All five apply directly to a robot.

## The mutation check

The idea comes from mutation testing [6, 7], pointed at specs instead of test suites. Call a spec $S$ with clauses $C_1, \dots, C_k$. For each clause $C_i$ I write a mutant $m_i$, a small edit to the code that a correct $C_i$ should forbid.

**Definition 1 (killed).** Mutant $m_i$ is *killed by* $C_i$ when Verus rejects $m_i$ *and* names $C_i$ among the failed postconditions. Otherwise it *survives*.

The second condition matters. If Verus fails for some other reason, say an arithmetic overflow, the clause I meant to test hasn't been tested.

The harness is about 150 lines of Rust with no dependencies. Each mutant is four lines of text:

```text
id = never-fires
file = src/robots/safe_shooter.rs
find = fire = Some(p);
replace = let _ = p;
expect = ready(*obs)
```

It verifies an unmutated copy first, then applies each mutant, runs `cargo verus verify`, and checks for the `expect` text in the failure.

```
   mutants.txt ──► apply one edit ──► cargo verus verify ──► failed?  ──no──► SURVIVED
                       ▲                                       │yes
                       │                                       ▼
                   restore ◄── killed ◄──yes── names the clause? ──no──► MISSED
```

*Figure 1. One mutant, one clause. A survivor means a weak clause, or a mutant that changed nothing.*

### First run

| Mutant | Result |
|---|---|
| `gun-heat-too-low`, `gun-heat-too-high` | killed |
| `damage-zero-at-min-power`, `damage-over-cap`, `hit-bonus-off-by-one` | killed |
| `angle-diff-wrong-wrap`, `aimed-tolerance-too-wide`, `fires-with-hot-gun` | killed |
| `closest-of-nothing` | killed |
| **`never-fires`** | **survived** |
| **`closest-picks-last-of-tie`** | **survived** |
| **`radar-turn-over-limit`** | **survived** |

*Table 1. 9 of 12 killed on the first run (38 seconds).*

Each survivor taught something different.

## Finding 1: the robot that never fires

The `never-fires` mutant deletes the only line that ever sets `fire`. SafeShooter then never shoots. And it still satisfies the firing spec:

$$
\varphi_1 \;\equiv\; \mathit{fire} = \mathrm{Some}(p) \Rightarrow \mathrm{Safe}(o, p)
$$

holds vacuously when $\mathit{fire}$ is always $\mathrm{None}$. Model checking has a name for this: *vacuity* [8, 9], a property that holds for a reason that has nothing to do with the behaviour you cared about.

This is exactly PDD Kit's decision card. "SafeShooter fires when it's safe" has two readings: *only when* ($\Rightarrow$) or *whenever* ($\Leftrightarrow$). I'd written the first and meant both. Rather than pick silently, I asked the person who owns the robot, and the answer was "if and only if". So:

**Definition 2 (ready).** $\mathrm{Ready}(o)$ holds when a target was seen this turn, the gun is cool and aimed, and $o.\mathit{energy} > 1000$, i.e. more than the smallest shot SafeShooter uses.

$$
\varphi_2 \;\equiv\; \mathit{fire} \neq \mathrm{None} \;\Longleftrightarrow\; \mathrm{Ready}(o).
$$

**Proposition 1.** For every well-formed $o$, `decide` satisfies both $\varphi_1$ and $\varphi_2$.

*Proof sketch.* ($\Leftarrow$) If $\mathrm{Ready}(o)$, the power is $3000$ when the target is close and $o.\mathit{energy} > 3000$, and $1000$ otherwise. In both cases $o.\mathit{energy} > p$, so the firing guard passes. ($\Rightarrow$) Firing implies $\mathrm{Safe}(o, p)$ by $\varphi_1$, and $p \ge 1000$, so $o.\mathit{energy} > 1000$. The rest of $\mathrm{Ready}$ is part of $\mathrm{Safe}$. Verus proves this with no hints. ∎

`never-fires` is now killed. Together the two clauses say what I meant all along: safe *and* not useless.

## Finding 2: the docs promised more than the spec

`closest_target` picks the nearest scanned robot. Its doc comment said "the first one on ties", and a unit test checked exactly that. The spec only said "*a* nearest one":

$$
\forall j.\; d_i \le d_j .
$$

The mutant changed `<` to `<=`, which picks the *last* of a tie, and it survived. So the promise was tested, but not proved. The fix adds a clause, and the loop invariant needed one more conjunct to carry it:

$$
\forall j < i.\;\; d_j > d_i,
\qquad
\text{invariant: } \forall j < b.\;\; d_j > d_b .
$$

## Finding 3: the mutant that changed nothing

`radar-turn-over-limit` loosened a clamp from $450$ to $451$, to check the clause that the radar turns at most $45^\circ$. It survived, and the spec was fine.

The radar turn is $2\,\delta(\cdot,\cdot)$, where $\delta$ is the angle difference. That is **always even**, so the value $451$ the mutant needed can never occur. The mutant changed the text of the program but not its behaviour: an *equivalent mutant*, the oldest headache in mutation testing [6, 7]. Deciding equivalence in general is undecidable.

I fixed the mutant, not the spec (it now allows $452$), and left a comment saying why. The general lesson: a survivor is a question, not a verdict. Either the clause is weak or the mutant is a no-op, and only a human can tell which.

## Contracts on each piece: never drive into a wall

`decide` already had pieces (`is_aimed`, `radar_turn`), but `movement` had no contract. That left out a property from my very first notes on the project: *the robot never drives towards a wall it's close to.*

**Definition 3.** $\mathrm{IntoWall}(o, h)$ holds when moving in direction $h$ heads into a wall the robot is already within $80$ px of. For example: near the left wall and $h > 180^\circ$.

$$
\mathit{ahead} > 0 \Rightarrow \neg\,\mathrm{IntoWall}(o, o.\mathit{heading}),
\qquad
\mathit{ahead} < 0 \Rightarrow \neg\,\mathrm{IntoWall}(o, \overline{o.\mathit{heading}}),
$$

where $\overline{h}$ is the reverse heading. The executable wall check is proved equal to $\mathrm{IntoWall}$, `movement` carries the two clauses, and `decide` passes them up. Three new mutants, three kills.

## The trusted boundary, written down

Every proof rests on assumptions. Ours were scattered through prose. PDD Kit's discipline is to list them, accept each one deliberately, and guard each with a test. That became `docs/verification-receipt.md`:

| # | Assumed (not proved) | Guarded by |
|---|---|---|
| A1 | Converting degrees always lands in $[0, 3600)$ | a test over extreme inputs: NaN, $\pm\infty$, $\pm 10^{300}$ |
| A2 | The runner only builds well-formed observations | a `debug_assert!` on every turn, plus a test over full battles |
| A3 | "Gun heat is 0" in the observation ⇔ the engine accepts the shot | a turn-by-turn test over five shot powers |
| A4 | Observed energy never exceeds the engine's | a rounding test |
| A5 | A target's bearing points at the target | a radar test |
| A6 | $\mathit{energy} > p$ keeps the robot alive after firing | a battle test at exactly 1.0 energy |

*Table 2. The trusted boundary of SafeShooter's proof.*

Two of these got stronger along the way.

- **A1 is now visible to Verus.** The angle conversion is marked `external_body` with `ensures 0 <= r.0 < 3600`, so the assumption is an explicit, named contract instead of an unstated hope.
- **A2 turned from an assumption into a runtime check.** I wrote an executable `is_well_formed` and proved it equal to the spec `well_formed`, so the check can't drift from the precondition it guards. The runner now asserts it on every observation in debug builds.

## Deleting tests

With $\varphi_1$, $\varphi_2$ and the angle lemmas proved, six SafeShooter tests checked nothing a proof didn't already cover: "fires when ready", "never fires with a hot gun", "never fires without a target", "aimed across 0°", "not aimed outside tolerance" and "the energy boundary". I deleted them, and the receipt records which clause replaced each.

The tests that stayed test what the spec *leaves open*: which power SafeShooter picks, how its gun tracks a stale target, how its radar locks on. They also cover the trusted boundary, and whole battles. That split feels right. Proofs cover the promises, and tests cover the rest.

## A small trap

One afternoon an integration test couldn't find a function the library plainly exported. The cause: `cargo verus` and plain `cargo` share the `target/` folder, and the test had linked against a library build Verus had produced, with different settings. `cargo clean -p robocode` fixed it. If you mix the two tools in one workspace and something impossible happens, check that first.

## What we learned

1. **A proof is a claim about a spec, so test the spec.** Mutation checks are cheap (about 13 seconds for 15 mutants once the build is cached) and they found two real gaps that our tests and proofs had both let through.
2. **Ask both readings of every implication.** $A \Rightarrow B$ is usually half of what you mean, and the missing half is where do-nothing code hides.
3. **Survivors need a human.** One of three was the mutant's fault. Tools can flag survivors, but can't explain them.
4. **Write down what you trust.** Listing the assumptions turned one of them into a contract Verus knows about, and another into a check that runs on every turn.
5. **Proofs make some tests redundant, and that's fine.** Removing them makes it clearer what the remaining tests are for.

## What this does not show

1. **Mutation checks are only as good as the mutants.** One mutant per clause catches gross weakness, not every way a clause could be too permissive.
2. **"Named in the failure" is a text match**, so a clause whose text appears elsewhere could be credited wrongly.
3. **The trusted boundary is still tested, not proved.** A1–A6 hold in the tests, not in a theorem.
4. **Liveness over many turns is untouched.** $\varphi_2$ says SafeShooter fires *on a turn where it's ready*, not that it ever becomes ready.
5. **Everything here was done by hand** for a small codebase. PDD Kit's point is doing this with agents at scale, which I haven't tried.

## Reproducing

At [github.com/cryptopatrick/robocode](https://github.com/cryptopatrick/robocode), with Verus `0.2026.11.02.8bf68ee` and Rust `1.99.1`:

```sh
cargo verus verify --features verus     # the proofs
cargo run -p verus-mutants              # 15 mutants, each must be killed by its clause
cat docs/verification-receipt.md        # proved / tested / out of scope
```

To watch Finding 1 happen, delete the `r.fire is Some <==> ready(*obs)` clause from `decide` and rerun the mutants.

## References

1. A. Lavaee, M. Philipose, "Formally verifying agent-written full-stack web apps with PDD Kit", *Microsoft Research Blog*, November 2026.
2. P. Kothbauer, "Fire, but Verify: Proving a Robot Tank Safe with Verus", cryptopatrick.com, November 2026.
3. A. Lattuada, T. Hance, C. Cho, M. Brun, I. Subasinghe, Y. Zhou, J. Howell, B. Parno, C. Hawblitzel, "Verus: Verifying Rust Programs using Linear Ghost Types", OOPSLA 2023.
4. A. Lattuada et al., "Verus: A Practical Foundation for Systems Verification", SOSP 2024.
5. M. A. Nelson and Robocode contributors, *Robocode*, https://robocode.sourceforge.io/, 2001–2026.
6. R. A. DeMillo, R. J. Lipton, F. G. Sayward, "Hints on Test Data Selection: Help for the Practicing Programmer", *IEEE Computer* 11(4), 1978.
7. Y. Jia, M. Harman, "An Analysis and Survey of the Development of Mutation Testing", *IEEE Transactions on Software Engineering* 37(5), 2011.
8. I. Beer, S. Ben-David, C. Eisner, Y. Rodeh, "Efficient Detection of Vacuity in Temporal Model Checking", *Formal Methods in System Design* 18(2), 2001.
9. O. Kupferman, "Sanity Checks in Formal Verification", CONCUR 2006.

## Cite this article

Patrick Kothbauer, "Who Checks the Spec? Mutation-Testing Verus Proofs in RoboVerus", cryptopatrick.com, November 6, 2026.

```bibtex
@misc{kothbauer2026whochecks,
  author       = {Patrick Kothbauer},
  title        = {Who Checks the Spec? Mutation-Testing Verus Proofs in RoboVerus},
  year         = {2026},
  month        = nov,
  howpublished = {\url{https://cryptopatrick.com/research/who-checks-the-spec/}},
  note         = {Accessed: 2026-11-06}
}
```
