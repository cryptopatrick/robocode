# Battle viewer

A browser page for running and watching battles, modelled on the classic RoboCode battle window, with its own look.

## Run it

```sh
rustup target add wasm32-unknown-unknown     # once
cargo install wasm-pack --version 0.13.1 --locked   # once
cd web
npm install
npm run wasm     # compiles the Rust engine to WebAssembly into web/src/pkg/
npm run dev      # open the printed http://localhost:5173 address
```

`npm run build` produces a static site in `web/dist/`; any static file server can host it. There's no backend: battles run in the browser tab.

## What's on the page

- **Urban** (New battle): tick it for the fixed city map: buildings block tanks and absorb bullets; roads are display only. Buildings block the radar (see `docs/architecture.md`), and the 2D view draws the beam as it is blocked. The 2D/3D buttons above the arena switch views; the 3D view (three.js, loaded when first chosen) shows the same battle. The Show toggles apply to the 2D view.
- **New battle** (top): pick 2–4 robots, the arena **ratio** (4:3 = 800 × 600, 2:1 = 1200 × 600, 16:9 = 1067 × 600, 1:1 = 600 × 600; all 600 high) and a turn limit, then *Run battle*. The battle runs **live**: a Web Worker advances it at the chosen speed while the page draws the newest turn. Pause and step act on the simulation itself; scrub back through history and use *Back to live* to return. Opened recordings and `?recording=` play back as before. See `docs/architecture.md`.
- **Arena**: robots (body, gun and radar), bullets, hit rings, death bursts, and each radar's sweep this turn. Labels show name and energy, *Disabled* at 0 energy, *Dead* once removed.
- **Playback**: Restart, step back, Play/Pause, step forward, speed (1–120 turns/s), a seek bar, and the current turn. Pressing Play on the last turn replays from the start.
- **Robots**: energy bars; click a robot for its energy, gun heat, velocity, headings and position. For a robot that reports an intent (Tactician), the panel also has a **Maneuver** line such as `Attack · 14 turns`: the current maneuver and how many turns it has run. The same robot's arena label gets a maneuver line, such as `ATTACK · normal`. Robots without intent show neither.
- **Decision this turn**: what the selected robot observed (energy, gun heat, target) and what it decided, in readable units. Beneath it are the five conditions of SafeShooter's verified `safe_to_fire` spec with ✓/✗. When the robot fired, these show why the shot was safe. When it didn't, they show whether a 1.0 shot would have been.
- **Results**: winner, turns and final energy, shown on the last turn.
- **Show**: toggles for names, energy, scan arcs and hit effects.
- **Save recording / Open recording**: download the battle as JSON, or open one saved earlier.
- **Robot images…**: optionally pick your own images. File names containing `body`, `gun` or `radar` replace those parts for every robot. Images should point up. *Reset images* goes back to the built-in look. No images are bundled.

## Recording format (version 2)

```json
{ "format": "roboverus-recording", "version": 2,
  "robots": [ { "id": 0, "name": "SafeShooter" } ],
  "battlefield": { "width": 800, "height": 600 },
  "turns": [ /* turn snapshots, index 0 = spawn state */ ],
  "decisions": [ /* per turn, per robot: Decision or null */ ],
  "outcome": { "winner": 0, "turns": 385, "energies": [94.0, -3.5] } }
```

- `turns[k]` is `TurnSnapshot::to_json_string()` from the engine (pixels and degrees).
- `decisions[k][i]` is what robot `i` observed and decided to produce turn `k`, in the fixed-point units of `robocode::units`: milli-units for energy, heat and power, decidegrees for angles. It is `null` at turn 0 and once the robot is dead.
- A decision may carry an optional `intent`, `{"maneuver":"attack","risk":"normal"}`, added in version 2. Maneuvers are `search`, `patrol`, `defensive`, `evade` and `attack`; risks are `cautious`, `normal` and `aggressive`. It is omitted for robots that report no intent and for dead robots (whose decision is `null`). Viewers read version 1 recordings, which have none.
- Version 2 adds `beam` to each robot object in `turns[k].robots`, only in battles that have buildings (`terrain` in the header). It is the polygon the radar swept this turn and could see, as `[[x,y],...]` in pixels rounded to 0.1, in angle order around the robot. Typically 5 to 8 vertices. It is written on every turn for every living robot. On the turn a robot dies its frame has `alive: false` and may still carry a non-empty beam; after that the beam is `[]`. The viewer ignores the beam of a dead robot. `beam` is optional on each robot: it is absent in battles without buildings. Open-arena recordings have no `beam` key and are identical to version 1 apart from `"version"`. Viewers read versions 1 and 2.

The 2D view draws a recorded beam as a filled polygon in the robot's color, with the same faint fill as the old scan wedge, exactly as recorded (no clipping or reshaping). A recording without `beam` (version 1, or no buildings) still gets the 1200 px wedge, and an empty beam draws nothing. Shadows behind buildings are left undrawn. The 3D view does not draw the beam in 0.6.0.
- `tanks` is an optional header field, a list of tank ids (`leopard2a8`, `abrams`, `k2`, `leclerc`, `merkava`), one per robot, as in `"tanks":["k2","abrams"]`. It is written only when tanks were picked; without it, or for an unknown id, the viewer uses the default tank for that robot's slot. The engine never reads it, so it is decoration only.
- Robots spawn at `(100,100)`, `(w−100,h−100)`, `(w−100,100)`, `(100,h−100)`, heading 0°.

## Not carried over from the Java game

Scoring and multiple rounds (the engine has neither yet), robot console output, Kill Robot, team colours, sound and the source editor.
