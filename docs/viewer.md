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

- **New battle** (top): pick 2–4 robots, the arena size and a turn limit, then *Run battle*. The battle runs **live**: a Web Worker advances it at the chosen speed while the page draws the newest turn. Pause and step act on the simulation itself; scrub back through history and use *Back to live* to return. Opened recordings and `?recording=` play back as before. See `docs/architecture.md`.
- **Arena**: robots (body, gun and radar), bullets, hit rings, death bursts, and each radar's sweep this turn. Labels show name and energy, *Disabled* at 0 energy, *Dead* once removed.
- **Playback**: Restart, step back, Play/Pause, step forward, speed (1–120 turns/s), a seek bar, and the current turn. Pressing Play on the last turn replays from the start.
- **Robots**: energy bars; click a robot for its energy, gun heat, velocity, headings and position.
- **Decision this turn**: what the selected robot observed (energy, gun heat, target) and what it decided, in readable units. Beneath it are the five conditions of SafeShooter's verified `safe_to_fire` spec with ✓/✗. When the robot fired, these show why the shot was safe. When it didn't, they show whether a 1.0 shot would have been.
- **Results**: winner, turns and final energy, shown on the last turn.
- **Show**: toggles for names, energy, scan arcs and hit effects.
- **Save recording / Open recording**: download the battle as JSON, or open one saved earlier.
- **Robot images…**: optionally pick your own images. File names containing `body`, `gun` or `radar` replace those parts for every robot. Images should point up. *Reset images* goes back to the built-in look. No images are bundled.

## Recording format (version 1)

```json
{ "format": "roboverus-recording", "version": 1,
  "robots": [ { "id": 0, "name": "SafeShooter" } ],
  "battlefield": { "width": 800, "height": 600 },
  "turns": [ /* turn snapshots, index 0 = spawn state */ ],
  "decisions": [ /* per turn, per robot: Decision or null */ ],
  "outcome": { "winner": 0, "turns": 385, "energies": [94.0, -3.5] } }
```

- `turns[k]` is `TurnSnapshot::to_json_string()` from the engine (pixels and degrees).
- `decisions[k][i]` is what robot `i` observed and decided to produce turn `k`, in the fixed-point units of `robocode::units`: milli-units for energy, heat and power, decidegrees for angles. It is `null` at turn 0 and once the robot is dead.
- Robots spawn at `(100,100)`, `(w−100,h−100)`, `(w−100,100)`, `(100,h−100)`, heading 0°.

## Not carried over from the Java game

Scoring and multiple rounds (the engine has neither yet), robot console output, Kill Robot, team colours, sound and the source editor.
