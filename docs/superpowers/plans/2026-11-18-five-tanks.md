# Five Tanks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the single drawn tank with five recognisable tanks (Leopard 2A8, M1A2 Abrams, K2, Leclerc XLR, Merkava-inspired) in both viewers. Each robot gets the tank picked for it, and the team colour shows on the muzzle.

**Architecture:**
- One TS definitions table (`web/src/tanks/defs.ts`) holds each tank's sizes, pivot and colours.
- 2D draws the user-supplied art, cut from `_wb/tanks_sprites02.png` into hull and turret PNGs by a committed script.
- 3D builds procedural meshes ported from `_wb/tankmodels.md`, scaled to the same table.
- The tank choice travels as an optional `tanks` field in the recording header, written by `src/recording.rs`. The physics engine never reads it.

**Tech Stack:** React 18, TypeScript, Vite, Vitest, three.js, Tailwind. Rust (recording header and wasm entry points). Node script with `pngjs` for the one-off art cut.

**Spec:** `docs/superpowers/specs/2026-11-18-five-tanks-design.md`

## Global Constraints

- **When to run:** start after robocode 0.6.0 is merged. Branch from the 0.6.0 result.
- **Tank ids:** exactly `leopard2a8`, `abrams`, `k2`, `leclerc`, `merkava`. Default slot order is the same; robot *i* gets `DEFAULT_TANKS[i % 5]`.
- **No national flags** or country labels anywhere.
- **No changes** to physics, the robot API (`Strategy`, `Robot`) or collision sizes. The tank type is decoration only.
- **2D size:** hull width (tracks included) is 28 px. Length follows the art's proportions (about 50 px). The turret is scaled by the same factor.
- **Team colour:** the team colour (`ROBOT_COLORS` in `web/src/draw.ts`, by robot index) appears only on the muzzle tip, in 2D and 3D.
- **Dead tanks:** same shape at 55% opacity. In 2D, draw through a `grayscale(1)` canvas filter with no team-colour muzzle. In 3D, use `WRECK` for every material and keep today's `scale.y` flattening.
- **Bullets** still leave from the robot's centre. Do not move them.
- **User sprites:** user-loaded sprites (`sprites.body`) still win in 2D. 3D always shows the built-in tanks.
- **Recording format:** stays at version 2. `tanks` is an optional header field. If it is missing or an id is unknown, fall back to the slot default.
- **Viewer:** the app never reads `_wb/` at runtime.
- **Web code:** Tailwind only, no inline styles in TSX.
- **Commits:** conventional prefixes, no co-author or contributor trailers.

## Review Focus

1. A `tanks` list that is shorter or longer than the robot list must not crash. Missing entries get slot defaults and extra entries are ignored. Task 4 adds `tank_for_handles_short_and_long_lists`.
2. When the tank images haven't loaded yet (first frame, or jsdom in tests), the plain tank must be drawn without throwing. Task 3 adds `falls_back_to_plain_tank_before_images_load`.
3. When two robots pick the same tank, both must be drawn with their own muzzle colour. Task 3 adds `same_tank_twice_keeps_both_muzzle_colours`.
4. The live-stream header (`header_json`) must carry `tanks`, so a live battle looks the same as its replay. Task 4 adds `live_header_carries_tanks`.
5. Removing a robot row in New Battle must remove that row's tank, and the remaining rows keep their own choices. Task 5 adds `removing_a_robot_keeps_other_tanks`.

---

### Task 1: Tank definitions table

**Files:**
- Create: `web/src/tanks/defs.ts`
- Test: `web/src/tanks/defs.test.ts`

**Interfaces:**
- Produces:
  - `export type TankId = 'leopard2a8' | 'abrams' | 'k2' | 'leclerc' | 'merkava'`
  - `export const TANK_IDS: TankId[]`
  - `export const DEFAULT_TANKS: TankId[]` (same order as `TANK_IDS`)
  - `export interface TankDef { id: TankId; name: string; hullLength: number; hullWidth: number; pivotY: number; barrelReach: number; colors: { base: number; secondary: number; armor: number } }`
  - `export const TANKS: Record<TankId, TankDef>`
  - `export function defaultTank(i: number): TankId`
  - `export function isTankId(s: string): s is TankId`
- Field meanings:
  - `hullWidth` is 28 for every tank.
  - `pivotY` is the px offset of the turret pivot from the robot centre along the hull, positive forward. For example, the Merkava's is negative.
  - `barrelReach` is the muzzle's distance from the robot centre in px.
- Task 2 fills `hullLength`, `pivotY` and `barrelReach` from the art. Task 1 seeds them from `tankmodels.md` proportions scaled to width 28, and Task 2 overwrites them with the measured numbers.

- [ ] **Step 1: Write the failing tests** in `defs.test.ts`:
  - `ids are the five tanks`: `TANK_IDS` equals `['leopard2a8','abrams','k2','leclerc','merkava']`.
  - `defaults follow slot order`: `defaultTank(0) === 'leopard2a8'`, `defaultTank(3) === 'leclerc'` and `defaultTank(5) === 'leopard2a8'`.
  - `every hull is 28 px wide and fits the art proportions`: for every tank, `hullWidth === 28` and `40 <= hullLength <= 56`.
  - `barrels reach 20 to 28 px past the centre, K2 longest`: every `barrelReach` is in [20, 28] and `TANKS.k2.barrelReach` is the maximum.
  - `merkava turret is set back`: `TANKS.merkava.pivotY < 0`.
  - `names have no country`: no `name` contains "Germany", "USA", "Korea", "France" or "Israel".
  - `isTankId rejects unknown`: `isTankId('t34') === false` and `isTankId('k2') === true`.
- [ ] **Step 2:** Run `cd web && npx vitest run src/tanks/defs.test.ts`. It should FAIL because the module is missing.
- [ ] **Step 3:** Implement `defs.ts`. Names: "Leopard 2A8", "M1A2 Abrams SEPv3", "K2 Black Panther", "Leclerc XLR", "Merkava (inspired)". Colours come from `tankmodels.md`'s `TANKS` (`color`, `secondary`, `armor`).
- [ ] **Step 4:** Run the same command. It should PASS.
- [ ] **Step 5:** Commit: `feat: definitions for the five tanks`.

### Task 2: Cut and clean the 2D art

**Files:**
- Create: `web/scripts/cut-tanks.mjs`
- Create: `web/src/assets/tanks/<id>-hull.png` and `<id>-turret.png` (10 files)
- Modify: `web/package.json` (devDependency `pngjs`, pinned to an exact version at least two weeks old; script `"cut-tanks": "node scripts/cut-tanks.mjs ../_wb/tanks_sprites02.png src/assets/tanks"`)
- Modify: `web/src/tanks/defs.ts` (measured `hullLength`, `pivotY`, `barrelReach`, and per-asset `pivot` pixel coordinates)
- Test: `web/src/tanks/assets.test.ts`

**Interfaces:**
- Consumes: `TANK_IDS` and `TANKS` from Task 1.
- Produces:
  - The 10 PNGs.
  - In `defs.ts`, `TANKS[id].art: { hull: { w: number; h: number; pivotX: number; pivotY: number }; turret: { w: number; h: number; pivotX: number; pivotY: number } }`. These are pixel sizes of each PNG and the pixel at which each rotates: the hull's turret-ring centre, and the turret's rotation centre.
  - `TANKS[id].scale`: the px-per-art-pixel factor that makes the hull 28 px wide.

The script works on the sheet (1774 × 887). The sheet has five columns, one per tank, in id order. In each column, row 1 is the hull and row 2 is the turret. The rows below (pre-rotated turrets) are ignored. The script:
1. Finds each sprite as a connected component of pixels with alpha > 0. It takes the largest component in each column's row band, so the separate pre-rotated rows are never picked.
2. Crops each sprite to its bounding box plus 2 px of padding.
3. Cleans the fringes. It clears every pixel with alpha < 255 whose colour is far from the sprite's own palette. "Far" means it is red-dominant (r > g + 60 and r > b + 60) or yellow-green-dominant (g > r + 20 and g > b + 60) **and** it lies on the sprite's outer edge (within 3 px of a fully transparent pixel).
4. Writes the PNGs and prints, per sprite, its size and its pivot. The hull pivot is the centre of the turret-ring circle, found as the centroid of the ring area or measured by hand. The turret pivot is the centre of the turret body, excluding the barrel. Print the numbers so they can be pasted into `defs.ts`.

- [ ] **Step 1: Write the failing tests** in `assets.test.ts`. It reads the PNGs with `pngjs` in node.
  - `all ten images exist`: for every id, both files load.
  - `hulls point up and are taller than wide`: hull `h > w * 1.5`.
  - `no red fringe on the Abrams turret`: count the pixels in `abrams-turret.png` with alpha < 255 and red-dominant colour within 3 px of transparency. The count must be 0.
  - `no yellow-green fringe`: the same check over all ten files with the yellow-green rule, count 0.
  - `art metadata matches the files`: `TANKS[id].art.hull.w/h` equal the PNG's size, and so do the turret's.
  - `scale makes hulls 28 px wide`: `TANKS[id].art.hull.w * TANKS[id].scale` is within 0.5 of 28.
- [ ] **Step 2:** Run `npx vitest run src/tanks/assets.test.ts`. It should FAIL because the files are missing.
- [ ] **Step 3:** Write the script, run `npm run cut-tanks`, and paste the printed numbers into `defs.ts`. Set `hullLength = art.hull.h * scale`, and derive `pivotY` and `barrelReach` from the pivots and the turret's top edge. Check each PNG by eye on a dark background (`#0f1e2e`) for leftover fringes, and tighten the rule if any remain.
- [ ] **Step 4:** Run `npx vitest run src/tanks` (assets and defs). It should PASS.
- [ ] **Step 5:** Commit the script, the PNGs and `defs.ts`: `feat: cut the five tanks from the sprite sheet`.

### Task 3: Draw the tanks in 2D

**Files:**
- Create: `web/src/tanks/images.ts`
- Modify: `web/src/draw.ts` (`drawRobot` and `drawArena` signature)
- Modify: `web/src/components/ArenaCanvas.tsx` (preload images, pass them in)
- Test: `web/src/draw.test.ts`

**Interfaces:**
- Consumes: `TANKS`, `TankId` and `defaultTank` (Tasks 1–2), and `tankFor(rec, i)` (Task 4). If Task 4 isn't merged yet, use `defaultTank(i)` and switch to `tankFor` in Task 4.
- Produces:
  - `export type TankImages = Partial<Record<TankId, { hull: HTMLImageElement; turret: HTMLImageElement }>>`
  - `export function loadTankImages(): Promise<TankImages>` (Vite asset imports)
  - `drawArena(ctx, rec, turn, view, sprites = {}, tanks: TankImages = {})`, which adds a last parameter with a default.

Drawing order for a living robot whose tank images are loaded and with no `sprites.body`:
1. The hull image, rotated by `heading` about its pivot and scaled by `TANKS[id].scale`.
2. The turret image, rotated by `gun_heading` about the turret pivot placed at `pivotY` along the hull.
3. The radar dish (today's arc), at the turret pivot, rotated by `radar_heading`.
4. The muzzle cap: a 4 × 3 px rect at `barrelReach` along `gun_heading`, filled with the team colour.

A dead robot gets `ctx.filter = 'grayscale(1)'` and `globalAlpha` 0.55, and no muzzle cap. If images are missing, today's plain tank is drawn.

- [ ] **Step 1: Write the failing tests** in `draw.test.ts`, using the existing mock-context style and fake `HTMLImageElement`-shaped objects with `naturalWidth` and `naturalHeight`:
  - `draws hull then turret for the picked tank`: `drawImage` is called twice for the robot, first with the hull image and then with the turret image.
  - `muzzle cap uses the team colour`: a `fillRect` happens after the turret with `fillStyle === ROBOT_COLORS[i]`.
  - `same_tank_twice_keeps_both_muzzle_colours`: two robots on `k2` give muzzle fills `ROBOT_COLORS[0]` and `ROBOT_COLORS[1]`.
  - `dead tank is grey with no muzzle colour`: the dead robot sets `filter` to `'grayscale(1)'` and has no team-colour fill.
  - `falls_back_to_plain_tank_before_images_load`: with `tanks = {}`, the call doesn't throw, `roundRect` is called (the plain body) and `drawImage` isn't.
  - `user sprites still win`: with `sprites.body` set, the hull drawn is the sprite, not the tank art.
- [ ] **Step 2:** Run `npx vitest run src/draw.test.ts`. It should FAIL.
- [ ] **Step 3:** Implement. `ArenaCanvas` calls `loadTankImages()` once on mount, keeps the result in state and redraws when it arrives.
- [ ] **Step 4:** Run `npm test`, which should PASS, and `npm run build`, which should succeed.
- [ ] **Step 5:** Commit: `feat: the 2D viewer draws five tanks`.

### Task 4: The `tanks` header field

**Files:**
- Modify: `src/runner.rs` (`Battle::set_tanks(&mut self, tanks: Vec<String>)` and `Battle::tanks(&self) -> &[String]`)
- Modify: `src/recording.rs` (`robots_and_field` writes `,"tanks":[...]` when the list is non-empty)
- Modify: `wasm/src/lib.rs`:
  - `BattleSpec.tanks: Vec<String>`;
  - `run_battle(robots_csv, width, height, max_turns, urban, tanks_csv: &str)`;
  - `LiveBattle::new(..., tanks_csv: &str)` and `LiveBattle::create`;
  - an empty string means no tanks.
- Modify: `web/src/wasm.ts`, `web/src/engine.worker.ts` and `web/src/useLiveBattle.ts` (pass `tanks: TankId[]`)
- Modify: `web/src/recording.ts` (`Recording.tanks?: string[]`, plus `export function tankFor(rec: Recording, i: number): TankId` in `web/src/tanks/defs.ts`)
- Test: `tests/recording_frames.rs`, `wasm/tests/record.rs`, `web/src/tanks/defs.test.ts`
- Modify: `docs/viewer.md` (the format section gets `tanks`)

**Interfaces:**
- Consumes: `TankId`, `isTankId` and `defaultTank` (Task 1).
- Produces:
  - `tankFor(rec, i)`: if `rec.tanks?.[i]` passes `isTankId`, return it; otherwise return `defaultTank(i)`.
  - The JSON header field `"tanks":["k2","abrams"]`.

- [ ] **Step 1: Write the failing tests:**
  - Rust `tanks_in_header`: after `battle.set_tanks(vec!["k2".into(), "abrams".into()])`, the recorded JSON contains `"tanks":["k2","abrams"]` and `"version":2`.
  - Rust `no_tanks_no_field`: without `set_tanks`, the JSON doesn't contain `"tanks"`. The open-arena 0.5.0 fixture comparison still passes.
  - Rust `live_header_carries_tanks`: `recording::header_json(&battle)` contains the same `"tanks"` field.
  - wasm `run_battle_passes_tanks`: `run_battle("SafeShooter,Spinner", 800.0, 600.0, 10, false, "merkava,k2")` contains `"tanks":["merkava","k2"]`. Passing `""` gives no `tanks` field.
  - TS `tank_for_handles_short_and_long_lists`: with `tanks: ['k2']` and 3 robots, the results are `k2`, `abrams` and `k2`. That is index 0 from the list, then indexes 1 and 2 from `defaultTank`. With 5 entries and 2 robots, the extras are ignored.
  - TS `tank_for_unknown_id_falls_back`: `tanks: ['t34']` gives `defaultTank(0)`.
- [ ] **Step 2:** Run `cargo test --test recording_frames`, `cargo test -p robocode-wasm` and `cd web && npx vitest run src/tanks`. All should FAIL.
- [ ] **Step 3:** Implement. Escape the strings the same way robot names are escaped. Then rebuild the wasm package with `npm run wasm` and update every TS caller of `run_battle` and `LiveBattle`.
- [ ] **Step 4:** Run `cargo test --workspace` and `cd web && npm test`. Both should PASS.
- [ ] **Step 5:** Commit: `feat: recordings say which tank each robot drives`.

### Task 5: Tank picker in New Battle

**Files:**
- Modify: `web/src/components/NewBattle.tsx` (state `tanks: TankId[]`, kept in step with `robots`; a `<select>` per row; `BattleSetup.tanks: TankId[]`)
- Modify: whatever calls `onRun` (`App.tsx`) so it passes `tanks` to `runBattle` and the worker
- Test: create `web/src/components/NewBattle.test.tsx` if none exists

**Interfaces:**
- Consumes: `TANK_IDS`, `TANKS[id].name` and `defaultTank` (Task 1). `runBattle(..., tanks)` and the worker message `tanks` (Task 4).
- Produces: `BattleSetup` gains `tanks: TankId[]`.

- [ ] **Step 1: Write the failing tests:**
  - `each robot row has a tank picker with slot defaults`: the two default rows show "Leopard 2A8" and "M1A2 Abrams SEPv3".
  - `picked tanks reach onRun`: pick K2 in row 1 and run. `onRun` receives `tanks: ['leopard2a8','k2']`.
  - `removing_a_robot_keeps_other_tanks`: with 3 rows (leopard, k2, merkava), removing row 1 gives `['leopard2a8','merkava']`.
  - `adding a robot gets the next slot default`: with 2 rows, adding a third gives `k2`.
- [ ] **Step 2:** Run `npx vitest run src/components/NewBattle.test.tsx`. It should FAIL.
- [ ] **Step 3:** Implement with Tailwind classes that match the existing form's `field` style.
- [ ] **Step 4:** Run `npm test`, which should PASS, and `npm run build`.
- [ ] **Step 5:** Commit: `feat: pick a tank for each robot`.

### Task 6: Procedural 3D tanks

**Files:**
- Create: `web/src/tanks/models3d.ts`
- Test: `web/src/tanks/models3d.test.ts`

**Interfaces:**
- Consumes: `TANKS` and `TankId` (Tasks 1–2).
- Produces:
  - `export interface Tank3D { root: THREE.Group; turret: THREE.Group; radar: THREE.Object3D; muzzle: THREE.Mesh; setTrackDistance(d: number): void; setDead(dead: boolean): void }`
  - `export function createTank3D(id: TankId, teamColor: number): Tank3D`
- Root conventions: the root's local +z is the tank's nose (see `headingRotation` in `scene.ts`), y is up, and the units are battlefield px. The root is centred on the robot, and the turret group's pivot is at `pivotY` along +z.

This is a port of `_wb/tankmodels.md`'s `createTank`, with these changes:
- Scale it so the hull's footprint is `hullWidth` × `hullLength`.
- Map the file's X-forward/Z-width frame to the scene's Z-forward/X-width frame.
- Drop `gunSpeed` and elevation.
- Make `muzzle` the muzzle ring, with material colour `teamColor`.
- Add a small radar dish on the turret roof, at the pivot.
- Replace `update(dt)` with `setTrackDistance(d)`. It places tread pads at `(i/count)·perimeter + d` and spins the wheels by `d / radius`.
- `setDead(true)` swaps every material to `WRECK` (`0x4a5866`) and sets `root.scale.y = 0.4`.
- Keep the file's per-tank features: Merkava rear compartment and hatch, armour blocks, angular panels, APS and optics. Keep flat shading.

- [ ] **Step 1: Write the failing tests.** They use three.js in node, with no renderer.
  - `footprint matches the definitions`: for every id, `new THREE.Box3().setFromObject(root)` has an x-size within 10% of 28 (tracks included) and a z-size within 10% of `hullLength`, excluding the barrel. Measure with the turret group hidden.
  - `muzzle is team-coloured`: `createTank3D('k2', 0xef5d79).muzzle.material.color.getHex() === 0xef5d79`.
  - `track distance moves pads deterministically`: calling `setTrackDistance(10)` twice gives identical pad positions, and `setTrackDistance(10)` differs from `setTrackDistance(0)`.
  - `dead tank is wreck-grey and flattened`: after `setDead(true)`, every mesh material colour is `0x4a5866` and `root.scale.y === 0.4`.
  - `no gun elevation`: `Tank3D` has no elevation member. Type-level: `'gunSpeed' in tank === false`.
- [ ] **Step 2:** Run `npx vitest run src/tanks/models3d.test.ts`. It should FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Same command. It should PASS.
- [ ] **Step 5:** Commit: `feat: procedural 3D models of the five tanks`.

### Task 7: Five tanks in the 3D view

**Files:**
- Modify: `web/src/scene.ts` (add `export function drivenDistances(rec: Recording): number[][]`, indexed `[turn][robot]`)
- Modify: `web/src/components/ArenaScene3D.tsx` (build `createTank3D(tankFor(rec, i), tankColor(i))` instead of the box tank; on each turn call `setTrackDistance(dist[turn][i])`, `setDead(!r.alive)`, and rotate the turret and radar as today)
- Modify: `web/src/scene.test.ts`

**Interfaces:**
- Consumes: `createTank3D` (Task 6), `tankFor` (Task 4), and the existing `tankPose`.

`drivenDistances[t][i]` is the sum of `turns[k].robots[i].velocity` for k = 1..t. Velocities are signed, so a tank that reverses runs its tracks backwards. `drivenDistances[0]` is all zeros.

- [ ] **Step 1: Write the failing tests** in `scene.test.ts`:
  - `driven distance sums velocity`: with velocities 0, 8, 8 and −2 over turns 0–3, robot 0's distances are `[0, 8, 16, 14]`.
  - `driven distance is per robot`: two robots get independent sums.
- [ ] **Step 2:** Run `npx vitest run src/scene.test.ts`. It should FAIL.
- [ ] **Step 3:** Implement `drivenDistances`. Compute it once per recording in a `useMemo` in `ArenaScene3D`, and wire up the tanks.
- [ ] **Step 4:** Run `npm test`, which should PASS, and `npm run build`.
- [ ] **Step 5:** Commit: `feat: the 3D view shows the five tanks`.

### Task 8: Gallery, docs and the look check

**Files:**
- Create: `web/gallery.html` and `web/src/gallery.tsx`. The gallery is a dev-only page. It is not added to the production build inputs in `vite.config.ts`.
- Modify: `docs/viewer.md` (the tanks, the picker, and the muzzle colour), `README.md` (one line)

**Interfaces:**
- Consumes everything above.

The gallery shows each tank in 2D at 1:1 and at 4×, on the arena background, with its turret turned 30°. It also shows a dead copy, and each tank in 3D on a small turntable. It uses `drawArena`'s tank drawing (Task 3) and `createTank3D` (Task 6), and needs no extra Tailwind beyond layout.

- [ ] **Step 1:** Run `cd web && npm run build`. It should succeed. Then `ls dist` must not contain `gallery.html`.
- [ ] **Step 2:** Start the dev server (`npm run dev`) and open `/gallery.html`. Check by eye that every tank is recognisable at 1:1, shows no fringes, and that its turret turns about the turret ring. Screenshot it with the run skill if one is available.
- [ ] **Step 3:** Write the docs.
- [ ] **Step 4:** Commit: `docs: the five tanks`.
- [ ] **Step 5:** **Stop for the user's look approval.** Report the gallery URL and screenshots, and wait for them before finishing the branch.
