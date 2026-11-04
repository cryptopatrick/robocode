# Battle Viewer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A browser tab where you pick robots, run a battle and watch it like the classic RoboCode battle window. It has playback controls, a robot panel, results and saved recordings, plus a RoboVerus twist: each robot's per-turn decision and SafeShooter's safety checklist.

**Architecture:** The Rust engine is compiled to WebAssembly (new crate `wasm/`, `robocode-wasm`). It runs a whole battle in the browser and returns a JSON **recording**: every turn's snapshot plus each robot's observation and action. A React 18 + TypeScript + Tailwind app (`web/`) replays the recording on a `<canvas>`. Because the recording is complete, seeking, stepping back, replay, and save/open all come for free. There is no server.

**Tech Stack:** Rust + `wasm-bindgen` (built with the installed `wasm-pack`), Vite, React 18.3, TypeScript, Tailwind CSS 3.4, Vitest.

**Spec:** No separate design doc. The user skipped the questions ("proceed"); the requirements are the Java battle window analysed in conversation (§ Design below) plus `_dev/PRD.md`'s browser track (Canvas rendering, battle viewer, replay files).

## Design (decided by the planner; the user opted out of questions)

Features carried over from the Java battle window (`_wb/latest_version_robocode-main/robocode.ui`):

| Java | Here |
|---|---|
| Arena with robots (body/gun/radar), bullets, explosions, scan arcs, name + energy labels, "Disabled" at 0 energy | Canvas arena; scan arc is the sweep from the previous turn's radar heading to this turn's |
| View toggles | Toggles: names, energy, scan arcs, hit effects |
| Pause, Next Turn, Stop, Restart, Replay, speed slider, turn/round display | Play/Pause, step ±1, Restart, speed select, seek slider, "Turn n / N" |
| Robot buttons + robot details (energy, heat, velocity, headings) | Robot panel with energy bars; selecting a robot shows its details |
| Robot "Paint" debug drawing | **Decision panel**: that turn's `Observation` → `Action`, and SafeShooter's four firing conditions as ✓/✗ |
| Results / rankings | Results card at the last turn: winner, turns, final energy (the engine has no scoring) |
| Save/Open record, Replay | Download recording `.json` / open a `.json` file |
| New Battle dialog | Panel: choose 2–4 robots, arena size, max turns, Run |

Not carried over: scoring and rounds (the engine has neither), robot console output, Kill Robot, team colours, sound, the source editor.

**Visual style:** an original look. Do **not** copy Robocode's images or artwork. Use a dark slate arena with a faint 50 px grid. Each robot gets a palette colour: a rounded-square body with track marks, a gun barrel line and a small radar dish. Bullets are dots sized by power. Hits are expanding rings fading over 8 turns, deaths a larger burst.

## Global Constraints

- `robocode` keeps **zero** non-optional dependencies. `wasm-bindgen` lives only in `wasm/`. `cargo tree -p robocode -e normal` must still list no other crate.
- Frontend: React 18 + TypeScript + Tailwind; **no inline styles** (CLAUDE.md). Dynamic sizes such as energy bars use SVG attributes (`<rect width={…}>`) or canvas drawing, never `style=`.
- Pin exact npm versions in `package.json` (no `^`/`~`), all released at least two weeks ago, and commit `package-lock.json`.
- Generated WASM output goes to `web/src/pkg/` and is git-ignored. `npm run wasm` regenerates it.
- Recording format: `"format": "roboverus-recording"`, `"version": 1` (exact shape in Task 2).
- Robots available in the browser: `SafeShooter`, `Spinner`, `SittingDuck`. 2–4 per battle. Arena 400–5000 px per side, default 800×600. Max turns 1–20000, default 5000.
- Spawn positions for robot i (0-based), all heading 0°: `(100,100)`, `(w−100,h−100)`, `(w−100,100)`, `(100,h−100)`.
- Don't edit `README.md`, `LICENSE`, `COPYING`, `CONTRIBUTING.md`, or the user's `Cargo.toml` license line (uncommitted user edits). Stage `Cargo.toml` hunk-only.
- Commits: no Claude attribution. Branch: `battle-viewer`.

## Review Focus

1. **Opening a bad file.** Opening a non-JSON file, a JSON file of another kind, or `version: 2` shows a readable error in the page and keeps the current battle. It never shows a blank screen. Tested in Task 3.
2. **Ends of playback.** Pressing step-forward on the last turn, step-back on turn 0, or Play at the end must not go out of range or loop silently. Play at the end restarts from 0, like the Java Replay button. Tested in Task 4.
3. **Robots that died.** A dead robot stays drawn as a faded wreck, labelled "Disabled" or "Dead", and has no decision. The decision panel says "no decision (dead)" instead of crashing on `null`. Tested in Tasks 2 and 6.
4. **Scan arc wrap-around.** A radar going 350° → 10° draws a 20° arc, not a 340° one. Tested in Task 4.
5. **Invalid battle setup.** One robot, five robots, an unknown name, or an arena that is too small gives a clear error from Rust, shown in the page. Tested in Task 2.

---

### Task 1: Runner records each robot's decision

**Files:**
- Modify: `src/runner.rs`
- Test: `tests/runner.rs`

**Interfaces:**
- Produces: `impl Battle { pub fn last_decision(&self, i: usize) -> Option<(Observation, Action)> }`. This is the observation and action robot `i` used in the most recent `step()`, or `None` if it was dead (or before the first step).

- [ ] **Step 1: Write failing tests** `decisions_are_recorded` (after one `step()`, the scripted robot's `last_decision(0)` is `Some((obs, action))` with `action == ` its scripted action and `obs.turn == 0`) and `dead_robot_has_no_decision` (reuse the wall-death setup from `dead_robots_are_not_asked_to_decide`: after it dies, `last_decision(0) == None`). Also check `last_decision(1)` before any step is `None`.
- [ ] **Step 2: Run `cargo test --test runner`.** Expected: FAIL (no method).
- [ ] **Step 3: Implement.** Store `Vec<Option<(Observation, Action)>>`, reset to `None` for every robot at the start of `step()`, and fill it for each robot that decides.
- [ ] **Step 4: Run `cargo test`.** Expected: PASS.
- [ ] **Step 5: Commit** with the message `feat: runner records each robot's decision`.

### Task 2: `robocode-wasm` crate: recording a battle

**Files:**
- Create: `wasm/Cargo.toml` (package `robocode-wasm`, `crate-type = ["cdylib", "rlib"]`, deps `robocode = { path = ".." }`, `wasm-bindgen = "0.2"`, dev-dep `serde_json = "1"`)
- Create: `wasm/src/lib.rs`
- Modify: `Cargo.toml` (add `"wasm"` to workspace members; hunk-only staging)
- Test: `wasm/tests/record.rs`

**Interfaces:**
- Produces (Rust):
  - `pub const ROBOT_NAMES: [&str; 3] = ["SafeShooter", "Spinner", "SittingDuck"];`
  - `pub struct BattleSpec { pub robots: Vec<String>, pub width: f64, pub height: f64, pub max_turns: u64 }`
  - `pub fn record_battle(spec: &BattleSpec) -> Result<String, String>`
- Produces (JS, via `#[wasm_bindgen]`): `robot_names(): string` (JSON array) and `run_battle(robots_csv: string, width: number, height: number, max_turns: number): string`, which throws a JS `Error` with the Rust message on `Err`.
- Recording JSON (version 1):

```json
{ "format": "roboverus-recording", "version": 1,
  "robots": [ { "id": 0, "name": "SafeShooter" }, … ],
  "battlefield": { "width": 800, "height": 600 },
  "turns": [ <TurnSnapshot::to_json_string()>, … ],           // index 0 = spawn state before any step
  "decisions": [ [ <Decision|null per robot> ], … ],           // same length as turns; decisions[k] = what produced turn k (decisions[0] all null)
  "outcome": { "winner": 0 | null, "turns": 385, "energies": [94.0, -3.5] } }
```

  `Decision` = `{ "energy": u32, "gun_heat": u32, "gun_heading": i32, "target": null | { "bearing": i32, "distance": i32, "age": u32 }, "fire": u32|null, "ahead": i32|null, "turn_body": i32|null, "turn_gun": i32|null, "turn_radar": i32|null }`, in the fixed-point units of `robocode::units` (milli-units, decidegrees, pixels).

- Errors (exact strings, tested): `"need 2 to 4 robots, got N"`, `"unknown robot: NAME"`, `"arena must be 400..=5000 px per side"`, `"max_turns must be 1..=20000"`.

- [ ] **Step 1: Write failing tests** in `wasm/tests/record.rs` (parse the output with `serde_json::Value`):
  - `records_safe_shooter_vs_spinner`: format/version correct; `turns.len() == decisions.len() == outcome.turns + 1`; `decisions[0]` all null; `outcome.winner == 0`; `turns[0]` robot positions match the spawn table.
  - `dead_robot_decisions_are_null`: after the turn where robot 1 dies, `decisions[k][1]` is null.
  - `rejects_bad_specs`: 1 robot, 5 robots, `"Nope"`, a 300 px arena and `max_turns = 0` each return the exact error string.
  - `is_deterministic`: two identical specs give identical strings.
- [ ] **Step 2: Run `cargo test -p robocode-wasm`.** Expected: FAIL.
- [ ] **Step 3: Implement** `record_battle` using `Battle`, `Battle::last_decision` and `BattleState::snapshot`, building the JSON by hand (robocode has no serde; names are fixed ASCII). Add the two `#[wasm_bindgen]` wrappers.
- [ ] **Step 4: Run `cargo test -p robocode-wasm`, then `wasm-pack build wasm --target web --out-dir ../web/src/pkg`.** Expected: tests pass and the `.wasm` builds. Also run `cargo tree -p robocode -e normal` and confirm only `robocode` is listed.
- [ ] **Step 5: Commit** with the message `feat: robocode-wasm records battles for the browser`.

### Task 3: Web app scaffold and recording parser

**Files:**
- Create: `web/package.json`, `web/package-lock.json`, `web/index.html`, `web/vite.config.ts`, `web/tsconfig.json`, `web/tailwind.config.js`, `web/postcss.config.js`, `web/src/main.tsx`, `web/src/index.css`, `web/src/App.tsx` (placeholder title "RoboVerus Arena"), `web/src/recording.ts`
- Modify: `.gitignore` (`web/node_modules/`, `web/dist/`, `web/src/pkg/`)
- Test: `web/src/recording.test.ts`

**Interfaces:**
- Produces: TypeScript types `Recording`, `TurnSnapshot`, `RobotSnapshot`, `BulletSnapshot`, `Decision`, mirroring Task 2's JSON. Also `parseRecording(text: string): Recording`, which throws `Error` with the message `"Not a JSON file"`, `"Not a RoboVerus recording"` or `"Unsupported recording version N"`.
- npm scripts: `wasm` (`wasm-pack build ../wasm --target web --out-dir ../web/src/pkg`), `dev`, `build` (`tsc -b && vite build`), `test` (`vitest run`).

- [ ] **Step 1: Scaffold.** Use exact versions: `react`/`react-dom` 18.3.1, `vite`, `@vitejs/plugin-react`, `typescript`, `tailwindcss` 3.4.x, `postcss`, `autoprefixer`, `vitest`, `jsdom`, `@testing-library/react`. Run `npm install`.
- [ ] **Step 2: Write failing tests** `parses_valid_recording` (a minimal hand-written v1 object round-trips), `rejects_non_json`, `rejects_other_json` (`{}`), `rejects_future_version` (`version: 2` → `"Unsupported recording version 2"`).
- [ ] **Step 3: Run `npm test`.** Expected: FAIL.
- [ ] **Step 4: Implement `parseRecording`.**
- [ ] **Step 5: Run `npm test && npm run build`.** Expected: PASS, and `dist/` builds.
- [ ] **Step 6: Commit** with the message `feat: web viewer scaffold and recording parser`.

### Task 4: Playback state and geometry

**Files:**
- Create: `web/src/playback.ts`, `web/src/geometry.ts`
- Test: `web/src/playback.test.ts`, `web/src/geometry.test.ts`

**Interfaces:**
- Produces:

```ts
export const SPEEDS = [1, 2, 5, 10, 20, 30, 60, 120] as const;   // turns per second
export interface Playback { turn: number; last: number; playing: boolean; speed: number }
export type PlaybackAction = { type: 'play' } | { type: 'pause' } | { type: 'toggle' } | { type: 'step'; by: 1 | -1 }
  | { type: 'seek'; turn: number } | { type: 'restart' } | { type: 'speed'; speed: number } | { type: 'tick' } | { type: 'load'; last: number };
export function playbackReducer(s: Playback, a: PlaybackAction): Playback;
export function headingToCanvas(deg: number): number;             // radians for ctx.rotate; 0° (up) → −π/2
export function scanArc(prevDeg: number, curDeg: number): { start: number; sweep: number }; // canvas radians, |sweep| ≤ π
```

**Rules:** `step` and `seek` clamp to `0..=last` and pause playback. `tick` advances by 1 when playing and pauses on reaching `last`. `play` at `last` restarts from 0. `load` resets to turn 0, paused.

- [ ] **Step 1: Write failing tests**: `step_clamps_at_ends`, `tick_stops_at_last`, `play_at_end_restarts`, `seek_clamps`, `load_resets`, `heading_zero_points_up` (`headingToCanvas(0) ≈ -π/2`, `headingToCanvas(90) ≈ 0`), `scan_arc_wraps` (`scanArc(350, 10).sweep ≈ +20° in radians`, `scanArc(10, 350).sweep ≈ −20°`), `scan_arc_zero_when_still`.
- [ ] **Step 2: Run `npm test`.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run `npm test`.** Expected: PASS.
- [ ] **Step 5: Commit** with the message `feat: playback state and arena geometry`.

### Task 5: Arena canvas, controls and New Battle panel

**Files:**
- Create: `web/src/wasm.ts` (loads `./pkg`, exposes `robotNames(): string[]` and `runBattle(robots, width, height, maxTurns): Recording`, which parses with `parseRecording`), `web/src/draw.ts`, `web/src/components/ArenaCanvas.tsx`, `web/src/components/Controls.tsx`, `web/src/components/NewBattle.tsx`
- Modify: `web/src/App.tsx`
- Test: `web/src/draw.test.ts`, `web/src/components/Controls.test.tsx`

**Interfaces:**
- `drawArena(ctx: CanvasRenderingContext2D, rec: Recording, turn: number, view: ViewOptions): void` with `ViewOptions = { names: boolean; energy: boolean; scanArcs: boolean; effects: boolean }`. It draws per the Design visual style. A robot's label reads `"Name"` and energy to 1 decimal, `"Disabled"` when alive at energy ≤ 0, and `"Dead"` when not alive. Hit rings come from `events.bullet_hits` in the previous 8 turns.
- `robotLabel(r: RobotSnapshot, name: string): string`: a pure function, exported for tests.
- `Controls` props: `{ playback: Playback; dispatch: (a: PlaybackAction) => void }`. It renders Restart, Step back, Play/Pause, Step forward, a speed `<select>` from `SPEEDS`, a seek range input, and "Turn n / N".
- `App` holds the recording and playback (`useReducer`). A `requestAnimationFrame` loop dispatches `tick` at `speed` turns per second. On first load it runs SafeShooter vs Spinner at 800×600.

- [ ] **Step 1: Write failing tests**: `robotLabel` (alive at 94.04 → `"SafeShooter 94.0"`, alive at 0 → `"SafeShooter Disabled"`, dead → `"SafeShooter Dead"`); `Controls` renders "Turn 0 / 10", clicking Play dispatches `{type:'toggle'}`, and step-forward is disabled at `last`.
- [ ] **Step 2: Run `npm test`.** Expected: FAIL.
- [ ] **Step 3: Implement** the components and drawing. `NewBattle` lets you pick 2–4 robots from `robotNames()` (the same robot may be picked twice), plus width/height selects (400×400, 800×600, 1000×1000) and max turns, then Run. Errors thrown by `runBattle` show in a red message box, and the current battle stays.
- [ ] **Step 4: Run `npm run wasm && npm test && npm run build`.** Expected: PASS. Then run `npm run dev` and load the page (curl or a browser): the HTML is served and the console shows no errors.
- [ ] **Step 5: Commit** with the message `feat: arena canvas, playback controls and new battle panel`.

### Task 6: Robot panel, decision panel, results, view toggles, save/open

**Files:**
- Create: `web/src/components/RobotPanel.tsx`, `web/src/components/DecisionPanel.tsx`, `web/src/components/Results.tsx`, `web/src/components/ViewToggles.tsx`, `web/src/components/RecordingFile.tsx`, `web/src/safety.ts`
- Modify: `web/src/App.tsx`
- Test: `web/src/safety.test.ts`, `web/src/components/DecisionPanel.test.tsx`

**Interfaces:**
- `safetyChecks(d: Decision, power: number): { label: string; ok: boolean }[]` returns exactly these four, in order, mirroring the Verus spec `safe_to_fire`:
  - `"energy > power"` (`d.energy > power`)
  - `"gun is cool"` (`d.gun_heat === 0`)
  - `"target seen this turn"` (`d.target?.age === 0`)
  - `"aimed within 3.0°"` (`|angleDiff(d.target.bearing, d.gun_heading)| ≤ 30`, using the same wrap rule as `units::angle_diff`)

  It is shown for every robot when its decision has `fire !== null`; otherwise the panel shows the checks against `power = 1000` with the heading "Would a 1.0 shot be safe?".
- `RobotPanel`: one row per robot with a colour swatch, name, an SVG energy bar (width ∝ energy, 0–100 and beyond, capped visually) and status. Clicking selects the robot. The selected robot shows energy, gun heat, velocity, body/gun/radar headings and position from the snapshot.
- `DecisionPanel` props `{ decision: Decision | null; dead: boolean }`. It shows the action fields in human units (power 1.0, turns in degrees). When `decision` is null it shows `"no decision (dead)"` if `dead`, otherwise `"no decision yet"`.
- `Results` appears when `turn === last`: the winner name (or "No winner"), turns, and final energies.
- `RecordingFile`: "Save recording" downloads `roboverus-<names>-<turns>.json` via a Blob URL. "Open recording" uses a file input and `parseRecording`; errors go to the same message box as Task 5.
- `ViewToggles`: four checkboxes bound to `ViewOptions`.

- [ ] **Step 1: Write failing tests**:
  - `safety_all_ok` and `safety_hot_gun` (`gun_heat: 1` → the second check is false);
  - `safety_wraps_angle` (gun 3595, bearing 5 → aimed ok);
  - `safety_no_target` (target null → the last two are false);
  - `DecisionPanel` shows `"no decision (dead)"` for `{decision: null, dead: true}`, and `"Fire 1.0"` for a decision with `fire: 1000`.
- [ ] **Step 2: Run `npm test`.** Expected: FAIL.
- [ ] **Step 3: Implement** and wire everything into `App`. Layout: a header bar with the New Battle panel; arena on the left with controls under it; on the right, Robot panel, Decision panel for the selected robot, View toggles, Results, Save/Open.
- [ ] **Step 4: Run `npm test && npm run build`.** Expected: PASS.
- [ ] **Step 5: Commit** with the message `feat: robot and decision panels, results, view toggles, save/open recordings`.

### Task 7: CI and docs

**Files:**
- Modify: `.github/workflows/ci.yml` (new job `web`)
- Create: `docs/viewer.md`

- [ ] **Step 1: Add the `web` job.** Steps: checkout; Rust stable with target `wasm32-unknown-unknown`; install `wasm-pack` (`curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh` or `cargo install wasm-pack --locked`); Node 22; `cd web && npm ci && npm run wasm && npm test && npm run build`. Also add `cargo test -p robocode-wasm` to the `test` job. That job's existing `cargo test --workspace` already covers it, so confirm rather than duplicate.
- [ ] **Step 2: Write `docs/viewer.md`**: how to run it (`cd web && npm install && npm run wasm && npm run dev`), what each panel does, the recording format, and what is not carried over from Java.
- [ ] **Step 3: Run every `web` job command locally.** Expected: all pass.
- [ ] **Step 4: Commit** with the message `ci: build and test the battle viewer`.
