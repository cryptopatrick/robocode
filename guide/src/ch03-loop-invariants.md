# Loops and invariants

*Mirrors Verus guide: "Loops and invariants".*

Verus can't run a loop to see what it does; there could be any number of iterations. Instead, you give a **loop invariant**: a fact that is true before the loop starts and stays true after every iteration. After the loop, Verus knows the invariant holds and the loop condition is false, and nothing else.

## Example: the closest target

`src/strategy/targeting.rs` finds the nearest scanned robot:

```rust
let mut best: usize = 0;
let mut i: usize = 1;
while i < targets.len()
    invariant
        1 <= i <= targets@.len(),
        best < i,
        forall|j: int| 0 <= j < i ==> targets@[best as int].distance <= targets@[j].distance,
    decreases targets@.len() - i,
{
    if targets[i].distance < targets[best].distance { best = i; }
    i += 1;
}
```

- `targets@` is the slice viewed as a mathematical sequence.
- The `forall` line is the heart of it: *`best` is at least as close as every target seen so far.* When the loop ends, `i == targets@.len()`, so `best` is at least as close as **every** target, which is exactly the `ensures`.
- `best < i` and `1 <= i` keep the indexing in bounds.
- `decreases` proves the loop terminates: `targets@.len() - i` shrinks every iteration and can't go below zero.

(In the crate itself the invariant is written in a `verus_spec` attribute on the loop, so plain builds ignore it. In the exercises you use the Verus guide's syntax shown above.)

## Exercise: `exercises/src/ch03.rs`

**`weakest_target`** returns the index of the target with the least energy, a good one to shoot at. The `ensures` is written for you. Add the loop invariant that makes it verify.
