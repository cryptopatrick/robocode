# RoboCheck

**The human writes the rules. Something else writes the robot. Verus checks the robot against the rules.**

`robocheck` takes a *candidate* strategy, the kind of code an AI assistant would generate, and:

1. **validates** it: refuses constructs that could weaken verification or reach outside the strategy;
2. **slots it into a trusted template** whose Verus spec on `decide` is human-written (the candidate cannot change it);
3. **compiles** it;
4. **verifies** it with Verus, one result per reference property;
5. **battles** the *same* code against a built-in robot;
6. writes **one evidence file** in which the verification and the battle carry the same implementation hash.

This is Phase 0: no AI is called. The candidates in `robocheck/fixtures/` stand in for AI output.

## Run it

Needs Verus (see [Verus integration](verus-integration.md)); `robocheck` looks in `ROBOCHECK_VERUS_DIR`, default `~/.verus/verus-arm64-macos`.

```sh
cargo run -p robocheck -- run robocheck/fixtures/energy_positive.rs
```

```
build: ok
SAFE-001  proved        Firing power is legal (0.1 to 3.0)
SAFE-002  rejected      Energy stays above zero after the shot
SAFE-003  proved        The gun is cool when firing
SAFE-004  proved        The target was scanned this turn
SAFE-005  proved        The gun is aimed within 3.0 degrees
SAFE-006  proved        Never drives toward a wall it is within 80 px of
SAFE-007  rejected      Fires if and only if it is ready
battle vs SittingDuck: Candidate wins after 364 turns; same implementation as verified: true
evidence: target/robocheck/runs/<run id>/evidence.json
```

Options: `--opponent` (`SafeShooter`, `Circler`, `Spinner`, `SittingDuck`; default `SittingDuck`), `--turns` (default 2000), `--out DIR`, `--timeout SECS` (per Verus run, default 300; a timeout kills Verus's whole process group), `--battle-timeout SECS` (default 120; a hung battle is recorded as `timed_out`). Exit code 0 when the pipeline completed, whatever the verdicts; 2 when the tool itself could not run.

## The candidate format

```rust
// optional header comments
// @@helpers
// helper functions; they may carry verus_verify / verus_spec attributes
// @@body
// the statements of `decide`'s body; `obs: &Observation` is in scope; it must evaluate to an `Action`
```

The template supplies the imports (`Observation`, `Action`, `Power`, `Angle`, `angle_diff`, SafeShooter's constants and, for specs, its spec functions) and the spec'd signature of `decide`.

Validation works on parsed Rust and tokens, so spacing, comments and string contents can't hide anything:

- the helpers must parse as plain `fn` items, and the body as one block (plain Rust: no Verus `proof` blocks);
- the only attributes allowed are exactly `#[cfg_attr(feature = "verus", verus_verify)]` and `#[cfg_attr(feature = "verus", verus_spec(...))]`;
- no `cfg` of any kind (`cfg!`, `#[cfg]`), so the verified build and the battle build can't diverge;
- no macros, and no `verifier`, `assume`, `admit`, `proof`, `unsafe`, `extern`, `std`/`core`, `use`, `mod`, `impl`, `include*`, `env`, its own `fn decide`, among others.

It is still not a sandbox: candidates are compiled and run on the machine running `robocheck`.

## The reference properties

SafeShooter's spec (`src/robots/safe_shooter.rs`), one `ensures` clause per property, for every well-formed observation:

| ID | Property |
|---|---|
| SAFE-001 | If it fires power p, 0.1 ≤ p ≤ 3.0 |
| SAFE-002 | If it fires power p, energy > p |
| SAFE-003 | If it fires, the gun is cool |
| SAFE-004 | If it fires, a target was scanned this turn |
| SAFE-005 | If it fires, the gun is within 3.0° of the target |
| SAFE-006 | It never drives toward a wall it is within 80 px of (two clauses) |
| SAFE-007 | It fires **if and only if** it is ready (scanned, cool, aimed, energy > 1.0) |

## How the verdicts are reached

A property is **proved** only from a Verus run with zero errors that actually verified the candidate: the candidate crate's cached results are cleared before every run (cargo would otherwise treat identical code as already verified and print `0 verified`), and a run counts only if Verus reports at least one item verified and no errors. Verus runs with `--multiple-errors 20`, so every failing clause is reported with its line. Each round, rejected clauses are replaced by `true` (line numbers kept) and the rest re-verified, until a run is clean. Any error that is not one of the reference clauses (an overflow, a helper's own `ensures`) leaves the unsettled properties **unknown**, never proved. Statuses: `proved`, `rejected`, `unknown`, `not_checked`, `build_failed`, `timed_out`.

## One implementation

The verified crate and the battle crate are written from the same `strategy.rs`. The battle binary embeds that file and prints its SHA-256; the evidence records it next to the verified hash as `same_artifact`. Each implementation gets its own crate and binary name (from its hash): candidate crates share a build cache, and with one shared name Cargo served one candidate's build for another.

## Fixtures

| Fixture | Expected |
|---|---|
| `safe_shooter.rs` | all 7 proved |
| `energy_positive.rs` (fires when energy > 0) | SAFE-002 and SAFE-007 rejected |
| `never_fires.rs` | SAFE-007 rejected |
| `wall_blind.rs` (always drives forward) | SAFE-006 rejected |
| `type_error.rs` | build_failed; no battle |
| `cheat_assume.rs` (`assume(false)`) | refused by validation; nothing runs |
| `cheat_cfg.rs` (safe under Verus, fires 3.0 every turn in battle via `cfg!`) | refused by validation |
| `cheat_nodecreases.rs` (`verifier :: exec_allows_no_decreases_clause` with spaces) | refused by validation |
| `hangs.rs` (`loop {}`) | verification unknown; battle `timed_out` |

Tests: `cargo test -p robocheck` (no Verus needed); `cargo test -p robocheck --test end_to_end -- --ignored --test-threads 1` (Verus).

## What this does not show

- **Per turn, integer strategy only.** As for SafeShooter: the `f64` engine is not verified, properties hold at the start of a turn.
- **SafeShooter's predicates are the reference for every candidate.** `ready`'s 1.0-energy threshold applies even to a robot that uses smaller shots.
- **No counterexample values.** Verus names the clause that failed, not an input that breaks it.
- **The validator is a denylist, not a sandbox.** Candidates are compiled and run on the machine running `robocheck`; isolation is a later phase.
- **The candidates are hand-written.** Nothing here measures what an AI generates.
