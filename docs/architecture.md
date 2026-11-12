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
