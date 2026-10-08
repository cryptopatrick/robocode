# RoboCode Port Parity Verification Plan

This plan tracks how we keep the Rust/WebAssembly engine aligned with the upstream RoboCode 1.10.1 Java implementation located under `_wb/robocode-VER_1_10_1`.

## 1. Goals
- **Turn-by-turn parity:** Robot pose, velocity, headings, energy, gun heat, bullet lists, and queued commands must match between engines (≤1e-6 scalar tolerance, ≤0.01 px positional tolerance).
- **Event alignment:** Event order and payloads (`ScannedRobot`, `HitWall`, `HitRobot`, `BulletHit(Bullet/Missed)`, `Death`, etc.) must be identical across logs.
- **Score/physics fidelity:** Round scores and physics-derived stats (ram damage, bullet damage, skipped turns) should stay in lockstep.
- **Performance sanity:** WASM builds should maintain ≥50 turns/sec headless throughput while staying within ~5% of the JVM engine when rendering is disabled.

## 2. JVM Instrumentation
1. Inject a `BattleSnapshotSink` inside `Battle.runTurn()` / `RobotPeer.performMove()` so every turn records:
   - Battle metadata (ID, RNG seed, rules, battlefield size)
   - Robot state (pose, headings, velocities, gun/radar turn queues, energy, gun heat, pending commands, event queue)
   - Bullet state (id, owner, pose, heading, power, alive flag)
   - Event stream for that turn (hits, walls, deaths, custom events)
2. Emit newline-delimited JSON that mirrors our Rust snapshot schema.
3. Surface the RNG seed and source version inside the JSON header so we can cross-check reproducibility.

## 3. Rust Instrumentation & CLI
1. `BattleState::snapshot()` already exposes the canonical state; `TurnSnapshot` serializes it with a tiny JSON writer (no extra deps).
2. The `parity` CLI now supports three workflows:
   - `parity --battle-id demo --turns 64 --output tmp/demo.log` → emit Rust snapshots.
   - `parity --expected ref/demo.log --actual tmp/demo.log` → diff two logs and stop on the first mismatch, printing helpful context.
   - `parity --manifest manifests/sample.parity` (new) → iterate over multiple battle entries, generate logs, and optionally compare them automatically.
3. Next steps include tolerance-aware numeric comparisons once JVM logs are available.

## 4. Manifest Format (Consumed by `parity --manifest`)
Each manifest file is a series of blocks separated by blank lines. Lines are simple `key = value` pairs; `#` starts a comment. Supported keys:
- `battle_id` (required)
- `turns` (defaults to 32 if omitted)
- `output` (path for the Rust-generated log; defaults to `<battle_id>.log`)
- `expected` (path to JVM log to compare against; optional)

Example:
```
# Run one battle and compare with JVM log
battle_id = sample-melee
turns = 128
output = snapshots/sample-melee.rust.log
expected = snapshots/sample-melee.jvm.log

# Generate a Rust-only snapshot
battle_id = headless-benchmark
turns = 500
output = snapshots/headless.log
```
When `expected` is present, the CLI generates the Rust log (creating directories as needed) and immediately compares it with the JVM log, stopping at the first mismatch.

### JVM Logger Toggle
- Enable JVM snapshots by launching Robocode with `-Drobocode.snapshotLog=/path/to/log.jsonl`. Each turn will be written as newline-delimited JSON using the schema consumed by the Rust CLI.
- Optionally override the battle identifier with `-Drobocode.snapshotBattleId=some-id`. If omitted, the default `java-battle` is used.
- The logger is dormant unless `robocode.snapshotLog` is provided, so normal gameplay is unaffected.

## 5. Automation Workflow
1. Define a golden corpus manifest referencing key battles (1v1, melee, team, sentry) and check it into the repo.
2. CI pipeline steps:
   - `./gradlew :robocode.battle:runSnapshotHarness --args @manifest` runs the JVM snapshot sink and produces the expected logs.
   - `cargo run --bin parity -- --manifest manifests/golden.parity` recreates the Rust snapshots and ensures every entry matches.
3. Developers regenerate JVM logs via `scripts/gen_snapshots.sh` before pushing physics changes.

## 6. Supplemental Checks
- Expand Rust integration tests (e.g., wall glancing, simultaneous hits, max-turn edge cases).
- Add replay-based regression tests once `.battle` playback is supported.
- Track turns/sec via Criterion or custom headless benchmarks to guard performance regressions.
