# 3D Scene Export (.glb) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `cargo robocode` writes a battle recording as an animated glTF 2.0 binary (`.glb`) that Unity (glTFast), Godot, Blender and three.js open. This includes real-city battles such as GinzaBattle02.

**Architecture:** A new `glb` module in the `cli` crate turns recording JSON into bytes in pure stages:
- `replay::parse`: JSON → typed `Replay`.
- `geometry`: meshes for boxes, spheres, extruded footprints and road strips.
- `scene::build`: `Replay` → nodes, meshes and animation tracks in glTF space, split into `scene/robots.rs` and `scene/map.rs`.
- `writer::write_glb`: `Scene` → GLB bytes.

The CLI gets `battle --glb <file>` and `export <recording> --glb <file>`. The engine and the recording format do not change.

**Tech Stack:** Rust (edition 2024, MSRV 1.85), clap, serde + serde_json (new in `cli`), `gltf` crate as a dev-dependency only, for round-trip tests.

**Spec:** `_dev/PRD.md` section **5.7 3D Scene Export (opt-in)** (revised 2026-11-25). `_dev/` is git-ignored, so executors read it from the working tree. The recording format, including the real-city map fields, is in `docs/viewer.md` "Recording format (version 2)". The Ginza brief is in `_wb/unity/UNITY-EXPORT-GINZA.md`, but its scale section and check 3 are superseded by the PRD.

## Global Constraints

- The export reads the recording and nothing else. No changes to `src/` (the engine) or to the recording format.
- Same recording → same bytes. Robots sorted by id, bullets by id, map items in recording order, and JSON through `serde_json::Value` (BTreeMap-ordered). No timestamps, and no version number in `asset.generator` (`"cargo-robocode"`).
- **Scale:** `m_per_px = 1 / px_per_m`, where `px_per_m = terrain.px_per_m` if present, else **4**.
- **Axes:** arena `x` → glTF `+X`, arena `y` → glTF **`+Z`** (recordings have `y` growing south). The arena's north-west corner `(0,0)` → glTF `(0,0,0)`. Y up.
- **Heading** `h` degrees (0 = north, clockwise) → rotation of `−h` about `+Y`. Every robot part looks along local `−Z`.
- One clip named `battle`. One key per recorded turn at `k / 30` s (`TURNS_PER_SECOND = 30`). Translation and rotation are LINEAR. Bullet and tracer visibility is a STEP scale track (full size ↔ 0).
- **Sizes**, with `D = 2 · radius · m_per_px` (the robot's diameter in metres, 9 m for Ginza):
  - body `D × 0.28D × D` with its centre at y `0.14D`
  - `gun` pivot at the robot origin, with `gun/barrel` `0.1D × 0.1D × 0.65D` at local `[0, 0.33D, −0.325D]`
  - `radar` `0.28D × 0.08D × 0.1D` at local `[0, 0.4D, 0]`
  - bullet sphere radius `0.08D` at y `0.33D`, and tracer cross-section `0.05D × 0.05D` at y `0.33D`

  Fixed metres:
  - ground 0.05 thick with its top at 0
  - walls 1.0 thick, 2.0 high, outside each edge
  - road strips with their top at 0.03, crossings with their top at 0.04
  - signal pole 0.3 × 5.0 × 0.3, entrance marker 3 × 2 × 3
  - building fallback height **7.0**
- **Node names:**
  - robots: `robot{id}`, `robot{id}/body`, `robot{id}/gun`, `robot{id}/gun/barrel`, `robot{id}/radar`
  - bullets: `bullet{id}`, for tracked spheres and tracers alike
  - arena: `ground`, `wall/north|south|east|west`
  - map: `building{i}`, `road{i}`, `crossing{i}`, `signal{i}`, `entrance{i}`. `i` is the index in the recording's own list, so gaps appear where items were left out.
- **Colours** (sRGB hex → linear `baseColorFactor`):
  - robots by slot: `#e0a526`, `#3fb6c9`, `#ef5d79`, `#9c7cf4`
  - ground `#0f1e2e`, walls `#e9eef2`, buildings `#4a5866`, roads `#1e2b3a`
  - crossings `#e9eef2`, signals `#c9d6e2`, entrances `#3fb6c9`, bullets and tracers `#fff6c8`
- New dependencies are pinned with `=x.y.z`, like the rest of `cli/Cargo.toml`.
- Commits: conventional messages, no co-author trailers.

## Ginza facts (from `cli/tests/fixtures/ginzabattle02.json`, checked 2026-11-25)

- 2001 turns (0..=2000), 1200×600 px, `px_per_m` 4, robots Candidate, Circler, Tactician and SittingDuck (ids 0–3), `winner: null`.
- Robot 3 has `alive:false` from turn 65 at ≈(290, 490). Robot 0 is dead from turn 209 at ≈(1128, 499). At turn 2000, robot 1 is at ≈(525, 447) and robot 2 at ≈(715, 263).
- 40 `bullet_fired` events (ids 0–39). 19 ids appear in snapshots (1–2 frames each). 21 never do, and all 21 have a `bullet_hits` or `bullet_hit_obstacles` event on their firing turn.
- Terrain counts: 45 `buildings`, 45 `heights`, 77 `shapes`, 198 `road_lines`, 39 `crossings`, 8 `signals`, 16 `entrances`, 27 `labels`.
  - Of the `road_lines`, 166 enter the arena. Of the point items, 34 crossings, 7 signals and 16 entrances lie inside it.
  - Shape rings are open (first point not repeated) and all 77 have a positive shoelace area in px.
  - Road line points span x −522..2482 and y −1632..2779.

## Review Focus

1. A heading that wraps (359° → 1°) turns 2°, not 358°. Task 2 `continuous_takes_short_way`, Task 4 `rotation_keys_stay_continuous`.
2. Same-turn bullets that never appear in a snapshot still show up as tracers. Task 4 `same_turn_hit_becomes_tracer`, Task 7 `ginza_bullets`.
3. Non-convex footprints (L-shapes) triangulate with roofs facing up. Task 3 `l_shape_triangulates`, `roof_faces_up`, `all_ginza_shapes_triangulate`.
4. Roads that run far outside the arena are clipped, and lines that never enter it are dropped. Task 3 `strip_is_clipped_to_arena`, Task 5 `road_outside_arena_dropped`.
5. A version-1 recording (no `bullets`, `terrain`, `tanks` or events) and bad input (not JSON, wrong `format`, zero turns): the first exports and the second fails clearly with exit code 2 and no file written. Task 1 `parses_version_1_fixture` and `rejects_*`, Task 7 `export_rejects_non_recording`.

---

### Task 1: Recording reader

**Files:**
- Modify: `cli/Cargo.toml` (add `serde` with `derive` and `serde_json`, both pinned to current releases)
- Create: `cli/src/lib.rs` (`pub mod glb;`). `main.rs` uses `cargo_robocode::glb`, so integration tests can call it.
- Create: `cli/src/glb/mod.rs` (`pub mod replay;` for now)
- Create: `cli/src/glb/replay.rs`
- Fixture already in place: `cli/tests/fixtures/ginzabattle02.json` (4.3 MB, unchanged copy of `_wb/unity/ginzabattle02.recording.json`)

**Interfaces (all derive `Debug, Clone, PartialEq`):**
```rust
pub struct Replay {
    pub width: f64, pub height: f64,
    pub robots: Vec<RobotInfo>,            // sorted by id
    pub bullet_mode: String,               // "arcade" when missing
    pub tanks: Option<Vec<String>>,
    pub winner: Option<String>,            // outcome.winner index → name; null → None
    pub map: Map,                          // Map::default() when no terrain
    pub turns: Vec<Turn>,                  // index = turn, 0 = spawn
}
pub struct RobotInfo { pub id: usize, pub name: String }
pub struct Turn { pub robots: Vec<RobotFrame>, pub bullets: Vec<BulletFrame>, pub events: Events }
pub struct RobotFrame { pub id: usize, pub x: f64, pub y: f64, pub heading: f64, pub gun_heading: f64,
                        pub radar_heading: f64, pub alive: bool, pub radius: f64 }
pub struct BulletFrame { pub id: u64, pub x: f64, pub y: f64, pub alive: bool }
#[derive(Default)] pub struct Events {
    pub fired: Vec<Fired>,                 // bullet_fired {shooter, bullet_id}
    pub hits: Vec<Hit>,                    // bullet_hits {victim, bullet_id}
    pub obstacle_hits: Vec<ObstacleHit>,   // bullet_hit_obstacles {bullet_id, position}
}
pub struct Fired { pub shooter: usize, pub bullet_id: u64 }
pub struct Hit { pub victim: usize, pub bullet_id: u64 }
pub struct ObstacleHit { pub bullet_id: u64, pub x: f64, pub y: f64 }
#[derive(Default)] pub struct Map {
    pub px_per_m: Option<f64>,
    pub buildings: Vec<[f64; 4]>, pub heights: Vec<f64>,
    pub shapes: Vec<Shape2d>,              // { ring: Vec<[f64;2]>, h: f64, id: String }
    pub roads: Vec<[f64; 4]>,
    pub road_lines: Vec<RoadLine>,         // { class: String, width: f64, points: Vec<[f64;2]> }
    pub crossings: Vec<Crossing>,          // { x, y, angle, len }
    pub signals: Vec<[f64; 2]>,
    pub entrances: Vec<Entrance>,          // { x, y, name }
    pub labels: Vec<Label>,                // { text, x, y, h }
    pub name: Option<String>, pub attribution: Option<String>,
}
pub fn parse(json: &str) -> Result<Replay, String>
```

- [ ] **Step 1: Write failing tests** (`#[cfg(test)]` in `replay.rs`)
  - `parses_version_1_fixture`: `include_str!("../../../tests/fixtures/open_arena_0_5_0.json")`. Assert `bullet_mode == "arcade"`, `map == Map::default()`, `turns.len() > 1`, every turn's `events == Events::default()` or parses, and turn 0 robot 0 at (100, 100) with heading 0.
  - `parses_ginza`: `include_str!("../../tests/fixtures/ginzabattle02.json")`. Check every count in "Ginza facts" (turns, robots, winner `None`, terrain counts, `px_per_m == Some(4.0)`, 40 `fired` events summed over turns), plus `turns[65].robots[3].alive == false`.
  - `parses_obstacle_hit_position`: a 2-turn hand-written recording with a `bullet_hit_obstacles` event at `{x:10,y:20}` → `ObstacleHit { bullet_id, x: 10., y: 20. }`.
  - `rejects_non_json`, `rejects_wrong_format` (error contains `not a robocode recording`), `rejects_zero_turns` (error contains `no turns`).
- [ ] **Step 2:** `cargo test -p cargo-robocode glb::replay` → FAIL (does not compile).
- [ ] **Step 3: Implement `parse`** with private `#[derive(Deserialize)]` mirror structs that ignore unknown fields (`decisions`, `beam`, `control`, other events). Missing fields default. Accept `version` 1 and 2.
- [ ] **Step 4:** run → PASS.
- [ ] **Step 5: Commit** `feat(cli): read battle recordings for glb export` (include the Ginza fixture).

### Task 2: Space conversions

**Files:** Create `cli/src/glb/space.rs`

**Interfaces:**
```rust
pub const DEFAULT_PX_PER_M: f64 = 4.0;
pub const TURNS_PER_SECOND: f64 = 30.0;
#[derive(Clone, Copy)] pub struct Space { pub m_per_px: f64 }
impl Space {
    pub fn new(px_per_m: Option<f64>) -> Space                 // None → 4
    pub fn point(&self, x_px: f64, y_px: f64, up_m: f64) -> [f32; 3]   // [x·s, up, +y·s]
    pub fn len(&self, px: f64) -> f64
}
pub fn yaw(heading_deg: f64) -> [f32; 4]                        // [x,y,z,w], −heading about +Y
pub fn yaw_toward(dx_px: f64, dy_px: f64) -> [f32; 4]           // rotation whose −Z points along (dx, dy) on screen
pub fn continuous(prev: [f32; 4], next: [f32; 4]) -> [f32; 4]   // negate next if dot < 0
pub fn turn_time(k: usize) -> f32
pub fn srgb_hex(hex: &str) -> [f32; 4]
```

- [ ] **Step 1: Write failing tests**
  - `point_maps_axes`: `Space::new(Some(4.)).point(100., 200., 0.5) == [25.0, 0.5, 50.0]`, and `Space::new(None).m_per_px == 0.25`.
  - `heading_matches_recorded_motion`: from the Ginza check, heading 42.5° moves (+3.38, −3.69) px. Rotating local `[0,0,−1]` by `yaw(42.5)` gives a vector parallel to `point(3.38, −3.69, 0)` normalised, within 1e-2. Also `yaw(90.)` → `≈[1,0,0]` and `yaw(180.)` → `≈[0,0,1]`.
  - `yaw_toward_east`: `yaw_toward(1., 0.)` ≈ `yaw(90.)`.
  - `continuous_takes_short_way`: `dot(yaw(359.), continuous(yaw(359.), yaw(1.))) > 0.99`.
  - `turn_time_is_thirtieths`, and `srgb_hex_converts` (`#ffffff` → 1s, `#808080` red ≈ 0.2159).
- [ ] **Step 2:** run → FAIL.
- [ ] **Step 3: Implement.** sRGB to linear uses the standard piecewise curve.
- [ ] **Step 4:** `cargo test -p cargo-robocode glb::space` → PASS.
- [ ] **Step 5: Commit** `feat(cli): glb units, axes and heading conversions`

### Task 3: Geometry

**Files:** Create `cli/src/glb/geometry.rs`

**Interfaces:**
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh { pub positions: Vec<[f32; 3]>, pub normals: Vec<[f32; 3]>, pub indices: Vec<u32> }
pub fn unit_box() -> Mesh                       // 1 m edges, centred, 24 vertices
pub fn unit_sphere() -> Mesh                    // radius 1, 12 segments × 8 rings
pub fn triangulate(ring: &[[f64; 2]]) -> Vec<[usize; 3]>   // ear clipping; open ring, either winding
pub fn extrude(ring_m: &[[f64; 2]], height_m: f64) -> Mesh // ring in glTF (X, Z) metres; walls + roof, no floor
pub fn clip_segment(a: [f64; 2], b: [f64; 2], w: f64, h: f64) -> Option<([f64; 2], [f64; 2])> // to [0,w]×[0,h]
pub fn strip(points_px: &[[f64; 2]], width_px: f64, arena: (f64, f64), space: &Space, top_m: f64) -> Option<Mesh>
    // one flat quad per clipped segment; None if no segment enters the arena
```
Every mesh's triangles are wound counter-clockwise seen from outside, which glTF requires for front faces.

- [ ] **Step 1: Write failing tests**
  - `box_and_sphere_are_closed`: index counts are 36 and a multiple of 3, all indices < vertex count, and normals are unit length.
  - `l_shape_triangulates`: the ring `[[0,0],[2,0],[2,1],[1,1],[1,2],[0,2]]` gives 4 triangles whose areas sum to 3.0, in both windings.
  - `roof_faces_up`: for `extrude` of a square, every triangle with all `y == h` has a cross-product normal with `y > 0`. Every wall triangle's normal points away from the centroid.
  - `all_ginza_shapes_triangulate`: for each of the 77 Ginza rings, `triangulate` returns `n − 2` triangles whose areas sum to the ring's |shoelace area| within 1e-6 relative.
  - `strip_is_clipped_to_arena`: the line `[[−100,50],[100,50]]` with width 10 in a 50×50 arena gives positions only with X in `[0, 50·s]`. A line wholly outside → `None`.
  - `ginza_road_count`: exactly 166 of the 198 road lines give `Some`.
- [ ] **Step 2:** run → FAIL.
- [ ] **Step 3: Implement.** Use Liang–Barsky for `clip_segment`, and give each road segment its own rectangle (overlap at joints is fine). `extrude` triangulates once and reverses the roof winding when needed so its normal is +Y. Walls get flat normals.
- [ ] **Step 4:** `cargo test -p cargo-robocode glb::geometry` → PASS.
- [ ] **Step 5: Commit** `feat(cli): glb geometry for boxes, footprints and road strips`

### Task 4: Scene types, robots and bullets

**Files:** Create `cli/src/glb/scene/mod.rs` and `cli/src/glb/scene/robots.rs`

**Interfaces (all derive `Debug, Clone, PartialEq`):**
```rust
pub enum MeshRef { Box, Sphere, Own(usize) }      // Own indexes Scene::meshes
pub struct Node { pub name: String, pub mesh: Option<(MeshRef, usize)>,   // (mesh, material index)
                  pub translation: [f32; 3], pub rotation: [f32; 4], pub scale: [f32; 3],
                  pub children: Vec<usize>, pub extras: Option<serde_json::Value> }
pub enum Path { Translation, Rotation, Scale }
pub enum Interp { Linear, Step }
pub struct Track { pub node: usize, pub path: Path, pub interp: Interp,
                   pub times: Vec<f32>, pub values: Vec<f32> }     // 3 or 4 floats per key
pub struct Scene { pub nodes: Vec<Node>, pub roots: Vec<usize>, pub meshes: Vec<Mesh>,
                   pub materials: Vec<(String, [f32; 4])>, pub tracks: Vec<Track>,
                   pub extras: serde_json::Value }
impl Scene { pub fn material(&mut self, name: &str, hex: &str) -> usize; pub fn add(&mut self, n: Node, parent: Option<usize>) -> usize }
pub fn build(replay: &Replay) -> Scene               // robots + bullets now; Task 5 adds map
pub(crate) fn add_robots(scene: &mut Scene, replay: &Replay, space: &Space)
pub(crate) fn add_bullets(scene: &mut Scene, replay: &Replay, space: &Space)
```
A node with a non-uniform `scale` is always a leaf, so scale never reaches animated children: `robot{id}` (animated) → `body` (scaled leaf), `gun` (animated rotation) → `gun/barrel` (scaled leaf), and `radar` (animated rotation, scaled leaf).

- [ ] **Step 1: Write failing tests** with a `replay(turns)` helper (800×600, robots A and B, radius 18, no map), plus `find(name)` and `track(node, Path)`.
  - `robot_track_follows_recording`: 3 turns → the `robot0` translation keys are at `[0, 1/30, 2/30]`, and key 1 is `space.point(x1, y1, 0.)`.
  - `body_is_robot_diameter`: the `robot0/body` scale is `[9.0, 2.52, 9.0]` at the default scale.
  - `gun_rotation_is_relative`: heading 90 with gun_heading 120 → the gun key ≈ `yaw(30.)`.
  - `rotation_keys_stay_continuous`: headings 359 → 1 → 3 give a positive dot product between consecutive keys.
  - `dead_robot_holds_last_pose`: robot 1 is alive on turns 0–1, has `alive:false` on turn 2 and is absent on turns 3–4. Keys 2–4 all equal turn 2's pose.
  - `tracked_bullet_scale_keys`: bullet 7 is alive on turn 2 only, of 5 turns. Its STEP scale times are `[0, 2/30, 3/30]`, with values 0, full (`0.08D` each axis), 0.
  - `bullet_alive_at_end_keeps_scale`: alive on turns 3–4 of 5 → times `[0, 3/30]`, with no key past `4/30`.
  - `same_turn_hit_becomes_tracer`: turn 2 has `fired {shooter 0, bullet 9}` and `hits {victim 1, bullet 9}`, and bullet 9 is in no snapshot. Node `bullet9` exists, translated to the midpoint between robot 0 and robot 1 on turn 2 at y `0.33D`, rotated `yaw_toward(robot1 − robot0)`. Its STEP scale shows `[0.05D, 0.05D, distance]` from `2/30` to `3/30` and 0 otherwise.
  - `obstacle_hit_tracer_ends_at_event_position`: the same, with `obstacle_hits` at (300, 40).
  - `scene_extras`: `battlefield`, `bullets`, `tanks` (null when absent), `robots` (names), `winner` (null), `px_per_m` (4). `robot0.extras == {"name":"A","slot":0}`.
  - `same_replay_same_scene`: `build(&r) == build(&r)`.
- [ ] **Step 2:** run → FAIL.
- [ ] **Step 3: Implement.** Tracked bullets: the ids of alive frames go into a `BTreeMap<u64, Vec<(turn, x, y)>>`, and the translation keys cover only first..=last alive turn. Tracers: ids in `fired` that never appear in a snapshot, matched with a hit or obstacle hit on the same turn. A fired id with no impact that turn gets no node. A robot's pose on a turn it is missing from is the previous pose.
- [ ] **Step 4:** `cargo test -p cargo-robocode glb::scene` → PASS.
- [ ] **Step 5: Commit** `feat(cli): glb scene with robots, bullets and tracers`

### Task 5: Arena and map

**Files:** Create `cli/src/glb/scene/map.rs`, and call it from `build`.

**Interfaces:** `pub(crate) fn add_arena_and_map(scene: &mut Scene, replay: &Replay, space: &Space)`. It also writes `map_name`, `attribution` and `labels` (`[{text, position: [x,y,z] m, h}]`) into `scene.extras` when present.

- [ ] **Step 1: Write failing tests**
  - `ground_and_walls`: an 800×600 replay with no map → `ground` and the four walls. `ground` covers `[0, 200] × [0, 150]` m with its top at y 0. No `building*` or `road*` nodes.
  - `rect_buildings_use_heights`: `buildings [[10,20,30,40]]` with `heights [12.0]` → `building0` box of size `[7.5, 12.0, 10.0]` at `point(25., 40., 6.0)`. Without `heights` the height is 7.0.
  - `shapes_replace_rects`: a map with 2 rects and 1 shape → exactly one building node, `building0`, using an `Own` mesh whose max y equals the shape's `h`.
  - `road_outside_arena_dropped`: lines 0 and 2 inside and line 1 outside → nodes `road0` and `road2` only.
  - `crossing_pose`: crossing `{x:100, y:100, angle:0, len:40}` → `crossing0` at `point(100,100,0.04 − half thickness)`, length `len · s` across the road (`angle` is the road's direction in radians, so an east-running road gives a north–south zebra, ±Z). Its width is `0.3 · len · s` (stripes are drawn as one pale rectangle in this version).
  - `point_items_outside_dropped`: a signal at (−5, 10) gets no node.
  - `ginza_map`: building nodes 77, road 166, crossing 34, signal 7, entrance 16. `extras.attribution` equals the recording's string, and `extras.labels.len() == 27`.
- [ ] **Step 2:** run → FAIL.
- [ ] **Step 3: Implement.** Shape rings are converted to metres with `space` before `extrude`. All geometry for one map item goes in one `Own` mesh at an identity transform.
- [ ] **Step 4:** run → PASS.
- [ ] **Step 5: Commit** `feat(cli): glb arena, buildings, roads and street furniture`

### Task 6: GLB writer and `export`

**Files:**
- Create: `cli/src/glb/writer.rs`
- Modify: `cli/src/glb/mod.rs` (add `pub fn export(recording_json: &str) -> Result<Vec<u8>, String>` = parse → build → write)
- Modify: `cli/Cargo.toml` (`[dev-dependencies] gltf`, pinned current)
- Create: `cli/tests/glb.rs`, plus `cli/tests/fixtures/open_arena.glb` (golden, generated)

**Interfaces:** `pub fn write_glb(scene: &Scene) -> Vec<u8>`

- [ ] **Step 1: Write failing tests in `cli/tests/glb.rs`**
  - `exports_version_1_fixture`: `gltf::Gltf::from_slice` validates the file. It has 1 animation named `battle`, and the input accessor of `robot0`'s translation has `count == turns` and `max == [(turns−1)/30]`.
  - `positions_round_trip`: with `gltf::import_slice`, the `robot0` translation output at key 5 equals `point()` of the fixture's turn 5 within 1e-4.
  - `golden_bytes`: `export(open_arena) == include_bytes!("fixtures/open_arena.glb")`. `UPDATE_GOLDEN=1` rewrites the file and passes.
  - `ginza_is_deterministic`: `export(ginza)` twice gives equal bytes, and `from_slice` validates.
- [ ] **Step 2:** `cargo test -p cargo-robocode --test glb` → FAIL.
- [ ] **Step 3: Implement `write_glb`**
  - Header: magic `0x46546C67`, version 2, total length. JSON chunk `0x4E4F534A` padded with spaces, BIN chunk `0x004E4942` padded with zeros, both to 4 bytes, little-endian.
  - `Box` and `Sphere` geometry is written once each. Every `(MeshRef, material)` pair used gets one glTF mesh that reuses the accessors.
  - Indices are `u32`. POSITION and animation-input accessors carry `min`/`max`.
  - Materials are `pbrMetallicRoughness` with `metallicFactor 0` and `roughnessFactor 0.9`.
  - Scene, node and root `extras` are passed through.
- [ ] **Step 4:** `UPDATE_GOLDEN=1 cargo test -p cargo-robocode --test glb`, then run again without it → PASS.
- [ ] **Step 5: Commit** `feat(cli): write glb files` (include the golden).

### Task 7: CLI, Ginza acceptance and docs

**Files:**
- Modify: `cli/src/main.rs` (`Battle` gets `--glb <FILE>` with `conflicts_with = "live"`. New `Export { recording: PathBuf, #[arg(long)] glb: PathBuf }`.)
- Modify: `docs/cargo-robocode.md` (a "3D export" section: both commands, glTFast for Unity, scale and axes, what is not exported, and that viewers must show `extras.attribution`)
- Modify: `docs/viewer.md` (one line under `terrain`: `cargo robocode export --glb` reads all map fields)
- Test: `cli/tests/cli.rs`, `cli/tests/battle.rs`, `cli/tests/glb.rs`

**Behaviour:**
- Both commands print `Saved the 3D scene to <path>` on stderr.
- In `battle`, `save_only` becomes `no_open && (output.is_some() || glb.is_some()) && !live`.
- Errors use `Failure::usage` (exit code 2): `could not read <path>: …`, `<path> is not a robocode recording: …` or `could not write <path>: …`. Nothing is written on error.

- [ ] **Step 1: Write failing tests**
  - `cli.rs` `export_writes_glb`: export the open-arena fixture → success, the file starts with `glTF`, and stderr contains `Saved the 3D scene`.
  - `cli.rs` `export_rejects_non_recording`: `{}` → exit code 2, `not a robocode recording`, no output file.
  - `cli.rs` `glb_conflicts_with_live`: `battle x.rs --live --glb a.glb` → non-zero exit (clap).
  - `battle.rs` `battle_writes_glb`: following `battle_writes_recording`, run with `--no-open --glb out.glb` → the file validates with `gltf`.
  - `glb.rs` `ginza_acceptance`: import the Ginza export with `gltf::import_slice` and check:
    - (a) nodes `robot0..=robot3`, with extras names Candidate, Circler, Tactician, SittingDuck
    - (b) `robot3` translation keys 65..=2000 all equal key 65 ≈ `point(290, 490, 0)` ±1 m, and `robot0` keys 209..=2000 ≈ `point(1128, 499, 0)` ±1 m
    - (c) the last keys: `robot1` ≈ `point(525, 447, 0)` and `robot2` ≈ `point(715, 263, 0)` ±1 m
    - (d) 40 `bullet*` nodes, and every bullet's last scale key is 0 or lies at or before `2000/30`
    - (e) node counts: building 77, road 166, crossing 34, signal 7, entrance 16
    - (f) the clip's max input time is `2000/30`
    - (g) the `robot0/body` scale z is 9.0
    - (h) scene `extras.winner` is null and `extras.attribution` is present
- [ ] **Step 2:** `cargo test -p cargo-robocode` → the new tests FAIL.
- [ ] **Step 3: Implement** the flags, the subcommand and the docs.
- [ ] **Step 4:** `cargo test -p cargo-robocode` → PASS. `cargo clippy -p cargo-robocode --all-targets -- -D warnings` → clean.
- [ ] **Step 5: Manual release check.** Run `cargo robocode export cli/tests/fixtures/ginzabattle02.json --glb ginza.glb` and import the file into Unity with `com.unity.cloud.gltfast`. Confirm that:
  - the Ginza buildings stand at their varied heights and the streets line up between them
  - the four tanks drive along the streets and face where they drive
  - SittingDuck stops at turn 65 (≈2.2 s) and Candidate at turn 209 (≈7 s)
  - tracers flash

  Note the Unity and glTFast versions in the commit body.
- [ ] **Step 6: Commit** `feat(cli): battle --glb and export subcommand`
