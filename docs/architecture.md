# How a battle runs

The engine advances a battle **one turn at a time**. On every turn, each robot decides from what it sees *at that moment*, then physics, collisions, firing and damage are computed. Nothing is scripted or interpolated. What differs between the two ways of watching is **when** those turns are computed relative to what you see.

| | Replay | Live |
|---|---|---|
| **When turns are computed** | all of them, before the first frame is shown | while you watch, paced to a target rate |
| **What the screen shows** | a recorded turn, chosen by the playback controls | the newest turn (or one you scrubbed back to) |
| **Pause / step** | pause playback | pause the *simulation* |
| **Used by** | `?recording=…`, *Open recording*, `cargo robocode battle` (default), `recording::record` | *Run battle* in the viewer, `cargo robocode battle --live` |
| **Category** | precomputed simulation (C) | real-time simulation with decoupled rendering (B), like Java Robocode |

**Both produce the same battle.** For the same robots and setup, a live battle's frames, and the recording it saves, are identical byte for byte to the precomputed ones (`tests/live_session.rs`, `wasm/tests/record.rs`, `cli/tests/live.rs`). Pausing, stepping and changing speed change *when* turns happen, never *what* happens.

## One turn

`runner::Battle::step` (`src/runner.rs`):

1. For each living robot (in a fixed order): `on_skipped_turn` if it missed turns, then last turn's events (`on_scanned_robot`, `on_hit_by_bullet`, …), then `run`. Its commands are applied.
2. `BattleState::step` (`src/physics/engine.rs`):
   1. cool guns;
   2. turn bodies, guns and radars;
   3. move robots and check walls;
   4. robot-to-robot collisions;
   5. radar scans;
   6. firing;
   7. bullets (bullet-on-bullet, then hits);
   8. deaths.
3. Engine events become next turn's API events. Scores are updated.

## The live path

```
 simulation clock                                   render clock (requestAnimationFrame)
 ┌───────────────────────────────────────┐ frames  ┌──────────────────────────────────┐
 │ Pacer: how many turns are due now?    ├───────► │ buffer → append to the recording │
 │ LiveSession::step() × n               │         │ draw the newest turn             │
 │ measured turns/s                      │ ◄────── │ pause / step / speed / scrub     │
 └───────────────────────────────────────┘ controls└──────────────────────────────────┘
  browser: Web Worker (src/engine.worker.ts)          browser main thread (useLiveBattle)
  CLI: the robot program (native, own process) ── stdout NDJSON ──► cargo-robocode (Axum) ── SSE /live ──►
```

- **Pacing:** `live::Pacer` (`src/live.rs`) does the pacing, like Java's `BaseBattle.synchronizeTPS`. Given the time, it says how many turns are due for the target turns per second (1–120 in the viewer, 1–1000 or "max" in the CLI). If the simulation falls behind, for example during a slow robot turn or a stalled browser tab, it runs at most 5 turns at once and then carries on at the normal rate, with no time-warp burst. It reports the speed it actually achieves, like Java's `calculateTPS`. The Pacer has no clock of its own: callers pass the time in, which keeps it deterministic and testable.
- **Simulation:** `live::LiveSession` wraps a `Battle` and a `recording::RecordingBuilder`. `step()` advances one turn and returns its `Frame`.
- **Browser:** a Web Worker steps the wasm engine (`LiveBattle`, `LivePacer`) every 4 ms and posts the frames. The page buffers them and, once per animation frame, appends them to the recording and draws. The simulation never waits for rendering. At "Max" it runs hundreds of turns per second while the page still redraws at the display's rate.
- **CLI:** `cargo robocode battle bot.rs --live [--tps N|max]` runs your robot natively in its own process. That process prints one JSON line per turn and reads `pause`, `resume`, `step` and `tps N` commands on stdin. The tool's server keeps every line streamed, so a late viewer gets the whole battle; streams them as Server-Sent Events on `/live`; and accepts commands on `POST /control`. `/battle.json` answers 409 until the battle ends, then serves the finished recording.

## Time budgets (optional)

`Battle::set_time_budget(Some(d))` gives each robot at most `d` per turn, covering its handlers plus `run`, like Java's CPU constant. A turn that takes *k* whole budgets makes the robot skip its next *k* turns: no events, no `run`, while its pending commands keep executing. That mirrors a Java robot whose thread is still busy. The robot then gets `on_skipped_turn`, and after 30 skipped turns in a row it's disabled. Budgets are **off by default**, which keeps tests, proofs and pinned battles deterministic. They have no effect in the browser build, which has no clock.

## Compared with Java Robocode

| | Java Robocode | here |
|---|---|---|
| Simulation loop | battle thread, `BaseBattle.runRound` | `LiveSession::step`, driven by a Web Worker or the robot program |
| Pacing | `synchronizeTPS` sleeps to the desired TPS; unpaced when minimised or at max | `Pacer`, with the same idea and a catch-up limit |
| Rendering | `TurnEndedEvent` snapshots drawn on the GUI thread | frames drawn on `requestAnimationFrame` |
| Controls | pause and single step act on the running battle | the same in live mode; replay mode scrubs a recording |
| Robot execution | a thread per robot; slow robots skip turns | direct calls; optional time budgets skip turns, but **can't interrupt** a robot that never returns |
| Robot order each turn | random | fixed |
| Turn order | bullets first, then each robot moves *and* is collision-checked in turn | all robots move, then collisions, firing, bullets |
| Hit and scan shapes | 36×36 bounding boxes | circles |
| Energy 0 | the robot is disabled | the robot is removed |
| Spawns, randomness | random positions, seeded RNG | fixed corners rotated per round; deterministic |
| Not here | teams, sentry robots, robot console, `onPaint`, custom events, data files, blocking calls | |

## Is it real-time? (checked 2026-11-12)

Answers to the eight questions from the original analysis, for the **live** path:

1. **Execution:** the battle is computed incrementally, one `LiveSession::step` → `Battle::step` per turn, while it's on screen. In live mode nothing is calculated ahead. Replay mode is unchanged.
2. **Real-time behaviour:** the state keeps advancing while you watch. The viewer draws the newest computed turn, not a replay (`useLiveBattle`, `follow`). A headless check confirmed the turn count rises while `finished` is false, then stops on pause.
3. **Timing:** the tick rate is the target turns per second, held by `Pacer::due` against a monotonic clock: `performance.now()` in the worker, `Instant` in the CLI. Measured at 31 turns/s against a target of 30, in both the browser and the CLI.
4. **Robot decisions:** robots decide every turn, from that turn's events and `Status`. Nothing is planned ahead.
5. **Physics:** movement, turning, collisions, bullets, firing and damage are all computed each turn in `BattleState::step`.
6. **Rendering:** rendering is decoupled. The simulation (Worker or child process) never waits for the renderer. `requestAnimationFrame` draws whatever is newest, and frames that arrive between paints are coalesced. You can scrub into history while the simulation runs on.
7. **Performance:** speeds run from 1 to 120 turns/s in the viewer, and from 1 to 1000 or "max" in the CLI. A simulation that can't keep up runs as fast as it can, at most 5 catch-up turns at once, without a time-warp burst (`tests/live_pacer.rs`), and the measured turns/s shows the shortfall. Optional time budgets let a slow *robot* skip turns, as in Java.
8. **Compared with Java:** see the table above. The remaining differences are engine semantics (robot and turn order, bounding shapes, energy 0, spawns) and the lack of preemption, not timing.

**Classification:** the live path is **B, real-time simulation with decoupled rendering**, the same category as Java Robocode. The replay path (`?recording=`, *Open recording*, `cargo robocode battle` without `--live`) is deliberately still **C**. Live and replay produce identical battles.
