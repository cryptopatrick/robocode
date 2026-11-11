# Live Simulation (Category B): Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans (or subagent-driven-development). Steps use checkbox (`- [ ]`) syntax.

**Goal:** Move from **C**, where the battle is fully precomputed and then replayed, to **B**, as in Java Robocode. The simulation advances turn by turn while you watch, paced to a target turns-per-second (TPS), and rendering runs independently, always drawing the latest state.

**Today (from the analysis):**
- `Battle::step` (`src/runner.rs`) is already a true single-turn advance.
- `recording::record_with_summary` loops it to the end before anything is displayed. Both the browser (`wasm::run_battle`) and the CLI (`cargo robocode battle`) hand the viewer a finished recording.
- There are no clocks anywhere in the engine.

**Target, like Java's `BaseBattle.runRound` / `synchronizeTPS` / `TurnEndedEvent`:**
- A **simulation loop** that steps the battle and sleeps to hold the desired TPS. It reports measured TPS, and when it can't keep up it runs as fast as it can rather than "catching up" unboundedly.
- A **snapshot stream**: each finished turn emits a frame.
- **Rendering on its own clock**, drawing the newest frame it has. It never blocks the simulation and is never blocked by it.
- **Controls that act on the running simulation:** pause, resume, single step, and change TPS (including "max").
- **Optionally, Java's per-robot time budget**, where a robot that takes too long *skips turns*.

## Design

```
            ┌──────────── simulation clock ───────────┐        ┌──── render clock (rAF) ────┐
 controls → │ Pacer: how many steps are due now?       │ frames │ store: history + latest     │
 (pause,    │ LiveSession::step() × n  (Battle::step)  ├──────► │ draw latest (live) or       │
  step, TPS)│ measured TPS, time budgets/skipped turns │        │ a past turn (scrubbing)     │
            └──────────────────────────────────────────┘        └─────────────────────────────┘
   browser: in a Web Worker (wasm)                     browser main thread
   CLI:     in the generated program (native) ──NDJSON stdout──► cargo-robocode (Axum) ──SSE──► browser
```

**Decisions (designer's calls; user confirmed 2026-11-11, including time budgets opt-in):**
1. **Both paths go live.** The browser runs the wasm engine in a **Web Worker**, so the simulation never runs on the rendering thread. The CLI runs the robot program **natively in its own process**, streaming frames to the tool's Axum server, which forwards them to the browser as **Server-Sent Events**. Controls go back via `POST /control`. User code still never runs inside the server process.
2. **Replay stays.** Live sessions keep the full frame history, so scrubbing backwards, "Save recording" and loading `?recording=` all still work. Scrubbing to an old turn shows history while the simulation keeps running; a "Back to live" button returns to the newest turn.
3. **The recording format stays v1.** A live stream is the v1 header followed by v1 turn frames, so a saved live battle is a normal recording.
4. **Determinism is kept.** For the same robots and setup, the frames a live session produces are **identical** to `record()`'s. Pacing changes *when* turns happen, never *what* happens. This is the plan's central test.
5. **Time budgets are opt-in.** The default is off, keeping headless runs, the proofs, tests and the pinned battles deterministic. When on, the runner measures each robot's handlers plus `run` per turn. Over budget means the robot skips its next turn(s), and it gets `on_skipped_turn` like Java. There's **no preemption**: Rust can't interrupt a running call, so an infinite loop still hangs. Recorded as a known limit.
6. **Pacing logic is pure and testable.** A `Pacer` takes timestamps as input and returns how many steps are due. The worker and the native program read the clock and call it.

## Global Constraints

- `robocode` keeps zero dependencies. Clocks (`std::time::Instant`) are used only in the native live loop and the time-budget measurement, both outside the default deterministic path.
- The default behaviour is unchanged:
  - the golden battles, the recording fingerprint, Verus (`cargo verify`) and the 15 mutants all stay green;
  - `cargo robocode battle` without `--live` behaves as today;
  - the viewer without `?live=` behaves as today.
- React 18 + TypeScript + Tailwind, no inline styles; Axum for the server.
- Commits carry no Claude attribution. Branch `live-simulation`.

## Tasks

### Task 1: Per-turn frames and an incremental recording

- **Files:** `src/recording.rs`, `tests/recording_frames.rs`.
- **API:**
  - `pub fn header_json(battle: &Battle) -> String`: the v1 `format`/`version`/`robots`/`battlefield` prefix.
  - `pub fn frame_json(battle: &Battle, events: &TurnEvents) -> String`: one turn snapshot plus that turn's decisions.
  - `pub struct RecordingBuilder` with `push_frame` and `finish(outcome) -> String`.
  - `record_with_summary` is rebuilt on top of these.
- **Tests:**
  - `frames_rebuild_the_same_recording` (the recording fingerprint is unchanged);
  - `header_is_v1`.

### Task 2: The Pacer (pure TPS logic)

- **Files:** `src/live.rs` (new), `tests/live_pacer.rs`.
- **API:** `pub struct Pacer { tps: Option<u32> /* None = max */, max_catch_up: u32, … }` with:
  - `due(now_ns: u64) -> u32`: steps due at this moment;
  - `pause()`, `resume(now_ns)`, `request_step()`, `set_tps(Option<u32>, now_ns)`;
  - `measured_tps() -> f64`, measured over a sliding 0.5 s window like Java's `calculateTPS`.
- **Tests (fake timestamps):**
  - 30 TPS yields 30 steps per simulated second;
  - a 1 s stall yields at most `max_catch_up` steps, then the normal rate resumes ("can't keep up → run as fast as possible, no time-warp");
  - paused yields 0, and `request_step` yields exactly 1;
  - a speed change takes effect at once;
  - `None` (max) is limited only by `max_catch_up` per call.

### Task 3: Time budgets and skipped turns (opt-in)

- **Files:** `src/runner.rs`, `src/api.rs`, `tests/time_budget.rs`.
- **API:**
  - `Battle::set_time_budget(Option<Duration>)`, `None` by default;
  - `SkippedTurnEvent { turn }` and `Robot::on_skipped_turn`;
  - skipped turns are counted in `BattleResults`.
- **Rule:**
  - if handlers plus `run` exceed the budget, the robot's commands that turn still apply, but it **skips its next turn** (no events, no `run`, pending commands keep executing in the engine), and it gets `on_skipped_turn` afterwards;
  - after 30 consecutive skips the robot is disabled, as in Java.
- **Tests:**
  - a robot sleeping 5 ms with a 1 ms budget skips turns;
  - with no budget it never skips;
  - the golden battles are unchanged (no budget by default).

### Task 4: `LiveSession` (engine side)

- **Files:** `src/live.rs`, `tests/live_session.rs`.
- **API:** `pub struct LiveSession` wrapping a `Battle` and a `RecordingBuilder`, with:
  - `step() -> Option<String>`: advances one turn and returns its frame, or `None` once finished;
  - `header()`, `finished()`, `outcome()`, `recording()` (everything so far, as v1 JSON).
- **Tests:**
  - **`live_frames_equal_precomputed_frames`**: for SafeShooter vs Spinner, the frames from stepping a `LiveSession` turn by turn equal `record()`'s, byte for byte;
  - it finishes on the same turn as `run()`.

### Task 5: The browser goes live (Web Worker and viewer live mode)

- **Files:** `wasm/src/lib.rs` (a `#[wasm_bindgen] pub struct LiveBattle` exposing `new`, `header`, `step`, `finished`), `web/src/engine.worker.ts`, `web/src/live.ts` (frame store and controls), `web/src/App.tsx`, `web/src/components/Controls.tsx`, plus tests.
- **Worker loop:** a `setTimeout` loop using the Pacer rule (ported to TS, or exposed from wasm). It calls `step()` the due number of times and posts the frames in batches, plus measured TPS. Messages in: `pause`, `resume`, `step`, `tps`.
- **Main thread:**
  - `requestAnimationFrame` draws the **latest** frame received (the live head) unless the user has scrubbed into history;
  - the controls send messages to the worker;
  - a "LIVE ●" indicator, measured TPS next to target TPS (like Java's title bar), and "Back to live";
  - New Battle starts a live session, while Open recording and `?recording=` stay replay.
- **Tests:**
  - vitest for the frame store (append, follow live head, scrub away and back, history kept);
  - a headless-Chromium check that the turn number increases *while* the battle is still unfinished (`finished` false), and that pausing stops it.

### Task 6: The CLI goes live (`cargo robocode battle --live`)

- **Files:** `cli/src/battle.rs` (generated program gains a live mode), `cli/src/serve.rs`, `cli/tests/live.rs`.
- **Generated program `--live`:** it prints the header and then one frame per line (NDJSON) on stdout, paced by the Pacer with `Instant`. It reads control lines from stdin (`pause`, `resume`, `step`, `tps N`, `tps max`).
- **Server:**
  - `GET /live` streams the child's frames as SSE;
  - `POST /control` writes control lines to the child's stdin;
  - `GET /battle.json` serves the recording so far;
  - the browser opens `/?live=/live`.
- **Tests:**
  - the first frames arrive over SSE *before* the battle has finished (the turn count is still rising after it's first read);
  - `pause` stops the turn count rising and `resume` restarts it;
  - when the battle ends, the stream closes and `/battle.json` is a complete v1 recording identical to a non-live run.

### Task 7: Docs, CI, and the C → B check

- **`docs/architecture.md`:** the replay and live paths, pacing, decoupled rendering, time budgets, and an updated comparison with Java (thread-per-robot, random robot order, turn order, energy-0 handling, and so on).
- **CI:** the new tests run in the existing jobs.
- **A final check against the original seven questions**, with evidence: the live path is B, the replay path is still available, and the remaining differences from Java are listed.

## Known limits (stated up front)

- **No preemption of robot code.** A robot stuck in an infinite loop hangs the live loop, the same as today. Java can interrupt threads; Rust can't.
- **Robot execution order stays fixed**, as do the turn order and the other engine differences listed in the analysis. B is about *timing*, not full Java parity.
