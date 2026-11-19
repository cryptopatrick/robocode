# Five tanks for the 2D and 3D viewers: design

**Status:** approved, 2026-11-18.
**Repo:** robocode, `web/` (the battle viewer), plus one optional field in the recording header.
**Sources:**
- `_wb/tankmodels.md`: the 3D source, a procedural low-poly Three.js collection of the five tanks.
- `_wb/tanks_sprites02.png`: the 2D art, supplied by the user. For each tank it has a top-down hull layer
  (with tracks) and a separate turret layer (with gun) on a transparent background, all pointing up.
- `_wb/tanks.png`: the same five tanks assembled, as a look reference. Mockups from the design
conversation are in `.superpowers/brainstorm/98353-1794957904/content/tanks-topdown-v2.html`.
**When:** after robocode 0.6.0 ships. It doesn't touch the physics engine (only the recording layer and wasm entry points) and doesn't block 0.6.0.

## Intent

Today every robot is drawn as the same tank:
- in 2D, a 36 px rounded square in its team colour, a steel line for the gun and an arc for the radar;
- in 3D, a box hull and a box turret.

The viewers should instead show five recognisable vehicles, in both 2D and 3D:

1. **Leopard 2A8:** arrowhead wedge turret, side skirts, olive with dark blotches.
2. **M1A2 Abrams SEPv3:** wide hull, big flat slab turret with a rear bustle rack, desert tan.
3. **K2 Black Panther:** sleek cut-corner nose, compact turret, longest gun, green and sand camouflage.
4. **Leclerc XLR:** boxy, low square turret, bolt-on armour blocks, green/brown/grey three-tone.
5. **Merkava (inspired):** long sloped nose (engine in front), turret set far back (superseded — see Amendments), rear hatch, pale sand.

These are stylised interpretations, not engineering replicas, and the Merkava is labelled "inspired".

## Goals

- Each tank is recognisable at battle size (the engine's 36 px body), in 2D and in 3D.
- Each tank keeps its own paint (camouflage). **The team colour is shown on the muzzle**: the tip of the
  gun barrel is painted in the robot's team colour (`ROBOT_COLORS` in `draw.ts`), in 2D and in 3D.
- The person setting up a battle picks a tank per robot. The choice is saved in the recording, so a
  replay looks the same as the live battle.
- Hull, turret (gun) and radar still turn independently, exactly as the recording says.

## Non-goals

- No national flags or country labels anywhere on the tanks.
- No change to the engine, the physics, the robot API (`Strategy`, `Robot`) or collision sizes. The tank
  type is decoration only: every tank fights identically.
- No gun elevation (the engine has none), no suspension animation and no new tank types beyond the five.
- No change to user-supplied sprite images (`sprites.ts`). If a user loads their own body, gun and radar
  images, those still win over the built-in tanks.

## Anti-goals (failure even if it works)

- **A tank whose drawing misleads about where it is.** The engine's body is a circle of radius 18 px.
  The hull may run past it lengthwise, but the drawn centre must be the robot's position. A bullet that hits
  must visibly land on the hull, not in empty space beside it.
- **Robots that can't be told apart.** If two robots pick the same tank, the muzzle colour and the name
  label must still tell them apart.
- **The 2D and 3D versions of a tank disagreeing**, for example a turret set back in 3D but centred in 2D.

## Constraints

- **2D size.** Each tank keeps its own proportions from the supplied art, scaled so that its width
  (tracks included) is 28 px, which fits inside the 36 px circle. Its length follows from those
  proportions, about 44–50 px, so nose and tail run past the circle. Barrel reach is measured from the
  robot's centre (not the pivot): about 20–28 px beyond the centre, with the K2's longest (superseded — see Amendments). The definitions
  table holds the scaled numbers. (Builder's call: measure them once from the art and write them in as literals; the 3D
  models are scaled to match.)
- **Bullets still leave from the robot's centre**, as the engine has them. A long barrel means a new bullet
  appears a little behind the muzzle. This is accepted; it is how today's viewer works too.
- **Heading convention.** A heading of 0° points up (−y). Turret and radar rotate about the robot's centre,
  except where a tank's turret pivot is offset along the hull (the Merkava's is set back). The pivot offset
  is part of each tank's definition, and both viewers use it.
- **The palette lives in the tank definitions.** Arena, grid, labels and beam keep the existing
  ink-blue identity. The camouflage colours are the only new colours, and they are confined to the tanks.
- **Dead robots:** same shape, greyed out at 55% opacity.
  - 2D: draw the art through a canvas `grayscale(1)` filter, with no team-colour muzzle.
  - 3D: swap every material for `WRECK` and keep today's flattening (`scale.y`).
- **Web rules (CLAUDE.md):** React 18 + TypeScript, Tailwind for any new UI, no inline styles.

## Approaches considered

1. **One measurements table, drawn twice.** A TS port of `tankmodels.md`'s `TANKS` table feeds both a 3D
   builder and a code-drawn 2D top view. It keeps 2D and 3D in step automatically, but code-drawn
   top views at 36 px take a lot of tuning to read well.
2. **Render the 3D models from above into 2D images.** The two views match exactly, but this adds a
   build step, gives blurry results at 36 px and makes muzzle tinting harder.
3. **Dedicated top-down art for 2D, procedural models for 3D (chosen).** This gives the best 2D look per
   tank, at the cost of keeping two designs in step by review. The user supplied the art
   (`tanks_sprites02.png`), so no drawing is needed, only cutting out and cleaning.

**Picked: 3**, by the user. To keep the two designs in step, both read the shared facts (hull length and
width, turret pivot offset, barrel length, base colours) from one small definitions table. The art and
the meshes each follow that table.

## The design

### Shared tank definitions

`web/src/tanks/defs.ts` exports a `TankId` union (`'leopard2a8' | 'abrams' | 'k2' | 'leclerc' |
'merkava'`) and one record per tank. Each record holds:
- the display name;
- 2D size (hull length and width in px);
- turret pivot offset along the hull (px, + forward);
- barrel length (px from the pivot);
- base, secondary and armour colours.

It also exports `DEFAULT_TANKS: TankId[]`, the slot order `leopard2a8, abrams, k2, leclerc, merkava`.
Robot *i* defaults to `DEFAULT_TANKS[i % 5]`.

### 2D: the supplied art

- **Cutting out.** From `_wb/tanks_sprites02.png` the builder cuts each tank's **hull** (top row) and
  **turret** (middle row) into separate PNGs under `web/src/assets/tanks/`, named `<id>-hull.png` and
  `<id>-turret.png`. The app never reads `_wb/`.
  - The bottom rows (turrets pre-rotated to fixed angles) are not used. The viewer rotates the turret
    to any angle.
- **Cleaning.** The sheet has coloured fringes on some edges: red around the Abrams turret, yellow-green on
  others. The builder removes the fringes by clearing semi-transparent edge pixels of those colours, or by
  removing any pixel that isn't close to the art's own palette. The cleaned PNGs are what gets committed.
  This is a one-off step done as a committed script under `web/scripts/`, so it can be re-run if the user
  exports a new sheet.
- **Pivot.** Each turret turns about the centre of the turret ring drawn on its hull. The PNGs are cropped
  so that the ring centre is a recorded offset; that offset goes in the definitions table.
- **Size.** Each hull is drawn 28 px wide, with its length following the art (about 50 px). The turret is
  scaled by the same factor.
- The radar stays a separate small dish drawn by code on the turret roof, at the turret pivot. It rotates by `radar_heading`.
- **Drawing order:** hull rotated by `heading`, then turret rotated by `gun_heading` about the pivot, then
  radar, then the team-colour muzzle tip. The muzzle tip is a short cap at the barrel's end, filled with the
  robot's team colour and drawn by code, so the PNGs need no recolouring.
- The existing user-sprite path keeps priority. If `sprites.body` is loaded, it is drawn instead, in 2D
  only. 3D always uses the built-in tanks.
- **Loading:** the PNGs are preloaded once into `HTMLImageElement`s before the first frame, keeping
  `drawArena` synchronous. Until they've loaded, and in tests where jsdom doesn't decode images, the
  viewer draws today's plain tank.

### 3D: procedural models

- Port `tankmodels.md`'s factory to TypeScript in `web/src/tanks/models3d.ts`, keeping its low-poly,
  flat-shaded style and its per-tank differences (Merkava rear compartment and set-back turret (superseded — see Amendments), armour
  blocks, APS sensors, optics).
- **Changes from the file:**
  - Scale it so the hull length and width match the 2D definitions. The 3D world is in battlefield px
    (`scene.ts`).
  - Use the scene's axes. The file uses X forward; the scene's tank pose (`tankPose`) uses heading about
    the vertical axis. Match `tankPose`.
  - Drop gun elevation.
  - Animate the track links and road wheels from the distance the robot has driven up to the shown turn.
    That distance is the sum of the recorded `velocity` over the turns so far, precomputed once per
    recording. The animation is then deterministic and survives scrubbing, and a robot that stops has
    stopped tracks. No wall-clock animation loop is added.
  - Paint the muzzle ring in the team colour.
  - Keep a small radar dish on the turret roof, turning by `radar_heading`.
- `ArenaScene3D.tsx` builds `createTank(tankId, teamColour)` per robot instead of today's box tank.
  Dead tanks are greyed.

### Picking a tank and recording it

- The New Battle form gets a tank picker per robot. It shows the five names and defaults to the slot
  order, and two robots may pick the same tank.
- **Recording:** the header gains an optional `"tanks": ["leopard2a8", "abrams", ...]`, one entry per robot.
  It stays version 2, because the field is optional and older viewers ignore it.
  - A recording without `tanks` uses the slot defaults.
  - An unknown id falls back to that slot's default.
- **How it gets there:** `wasm::BattleSpec` gains `tanks: Vec<String>`. `run_battle` and
  `LiveBattle::create` take it as an extra comma-separated argument, the same way robots are passed.
  `recording::header_json` and `RecordingBuilder` write it, so the live-stream header carries it too.
  The physics engine never reads it. The CLI passes nothing, so CLI recordings use the slot defaults.
- **The picker follows the robot's row.** Removing a robot removes its tank choice, and the rows below it
  keep theirs. With at most 4 robots the Merkava is never a default; it is always available to pick.

## Acceptance

- A dev-only gallery page shows all five tanks in 2D at 1:1 and at 4× size, and in 3D. The user approves the
  look there before release.
- Unit tests cover the definitions table, the default slot order, the header field round-trip (Rust and
  TS), the unknown-id fallback, and the accumulated-distance precompute.

## What's left to the builder

- The exact fringe-cleaning method, judged by eye on the dark arena.
- How the PNGs are bundled and loaded (Vite asset imports are fine).
- Track-link counts, mesh detail level and materials in 3D, within the low-poly style and frame budget.
- Where the tank picker sits in the New Battle form (Tailwind, matching the existing form).

## Calls I made (check these)

- **Shared definitions table.** Both views read one table for sizes, pivot and colours, even though the
  2D art is supplied separately. This is how the anti-goal "2D and 3D disagree" is kept.
- **Picker and defaults.** Tanks are picked in the New Battle form and saved as an optional header field;
  slot defaults apply otherwise.
- **Two robots may share a tank.** The muzzle colour and the label tell them apart.
- **Radar.** The radar stays a small dish on the turret roof in both views, because the source models
  have no radar part.
- **Release.** This ships after 0.6.0, as its own release.

## Amendments (during implementation)

- The supplied 2D art measured hull lengths of about 45-47 px, gun reach of about 28-29 px for every tank, and turret pivots near the hull centre, the Merkava included. The art is the authority for geometry, so the 3D models follow these numbers rather than the earlier seed proportions.
