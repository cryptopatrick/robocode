# Is your spec strong enough?

*Goes beyond the Verus guide. Based on the "mutation check" from Microsoft Research's Proof-Driven Development (PDD Kit, 2026).*

Verus answers one question: *does the code meet the spec?* It can't tell you whether the spec says enough. A spec that says too little verifies just as happily, and then the proof means less than you think.

## A spec that verifies a useless robot

SafeShooter's first spec had one clause:

```rust
ensures r.fire matches Some(p) ==> safe_to_fire(*obs, p),
```

*If* it fires, the shot is safe. Now replace the firing line with nothing, so SafeShooter **never** fires. The spec still holds, because a robot that never fires never fires unsafely.

The fix was a second clause, the other *reading* of "fires when it's safe":

```rust
ensures
    r.fire matches Some(p) ==> safe_to_fire(*obs, p),   // only when safe
    r.fire is Some <==> ready(*obs),                    // and always when ready
```

Whenever you write `A ==> B`, ask whether you also mean `B ==> A`. Often you do, and leaving it out lets a do-nothing implementation through.

## Mutation checks

How do you find weak clauses systematically? **Plant a bug that one clause should catch, and check that Verus rejects it.** Such a planted bug is a *mutant*. If a mutant verifies, the clause it targeted constrains nothing, at least for that kind of bug.

RoboVerus keeps one mutant per `ensures` clause in `mutants/mutants.txt`:

```text
id = never-fires
file = src/robots/safe_shooter.rs
find = fire = Some(p);
replace = let _ = p;
expect = ready(*obs)
```

and checks them all with:

```sh
cargo run -p verus-mutants
```

Each mutant must make Verus fail, and the failure must name the targeted clause (`expect`).

What the first run found:

| Mutant | Result | What it meant |
|---|---|---|
| `never-fires` | survived | The do-nothing robot above. Fixed with the second reading. |
| `closest-picks-last-of-tie` | survived | The spec said "*a* nearest target", but the docs promised "the *first* on ties". Fixed by adding that clause. |
| `radar-turn-over-limit` | survived | **The mutant's fault, not the spec's.** The radar turn is `2 × angle_diff`, always even, so the value the mutant needed (451) could never occur. That's an *equivalent mutant*; fixed by planting a real bug (452). |

So a surviving mutant has two possible meanings: the spec is too weak, or the mutant changed nothing. Telling them apart is part of the job.

## Exercises: `exercises/src/ch04.rs`

1. **`clamp_power`** has the spec `100 <= r <= 3000`, which "always return 100" also meets. Strengthen it until `check_clamp`'s assertions verify. (Hint: say what happens inside, below and above the range.)
2. **`should_fire`** promises only one direction. Add the other reading so `check_should_fire` verifies.
