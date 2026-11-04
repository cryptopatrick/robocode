# RoboVerus Prototype Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A standalone `robocode` crate with a strategy layer (`Observation → decide → Action`), radar scanning, sample robots and a headless battle, plus an optional `verus` feature and the first three RoboVerus guide chapters, which prove that `SafeShooter` never fires unsafely.

**Architecture:** The existing `f64` physics engine stays faithful to Java. Strategies see and act in integer fixed-point units (`src/units.rs`) in every build, so the `decide` that Verus proves is the code that fights. Verus specs attach to ordinary Rust functions only when the `verus` feature is on; with it off, no Verus crate is compiled.

**Tech Stack:** Rust 2024, Verus (version pinned by Task 2), mdBook, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-11-04-roboverus-prototype-design.md`

## Global Constraints

- Java reference: `_wb/latest_version_robocode-main` (1.12.0) is the primary source of truth; `_wb/robocode-VER_1_10_1` is secondary. Physics are identical between them. The 1.10.1 copy also carries a local `parity/ParitySnapshotLogger` that is not upstream.
- `cargo build` / `cargo test` with no features must compile **zero** Verus crates (check with `cargo tree -e normal`).
- `decide` for `SafeShooter` is written **once**. If Task 2 shows this cannot coexist with the previous line, **stop and ask the user**. Do not create a twin, and do not make `vstd` unconditional.
- No product code until Task 1 (PRD update) is approved by the user.
- Fixed-point units: energy, gun heat and power in **milli-units (`u32`, ×1000)**; angles in **decidegrees (`i32`, ×10)**, absolute angles in `0..3600`; distances and positions in **whole pixels (`i32`)**; velocity in **decipixels per turn (`i32`)**.
- Conversion from `f64` rounds **in the safe direction**: energy rounds **down** (`floor`) and gun heat rounds **up** (`ceil`), so a strategy never believes it can fire when the engine would refuse.
- The trade-off must be documented in the `lib.rs` crate docs and in the guide: the `f64` physics matches Java; the fixed-point model is what gets proved; the two are not the same code. Do **not** edit `README.md`, `LICENSE`, `COPYING` or `CONTRIBUTING.md`, which have uncommitted user edits.
- One `set_fire` produces at most one bullet (Java 1.11.0 fix #89).
- Battles are deterministic. Sample robots use no unseeded randomness.
- Commits: no `Co-Authored-By` or other Claude attribution (CLAUDE.md). Work on branch `roboverus-prototype`.
- Ignore `_wb/` except as Java reference material.

## Review Focus

1. **Angle wrap-around.** Gun at 359.5° with the target at 0.5° counts as aimed (difference 1°), not off by 359°. Tested in Task 3 (`angle_diff`) and Task 7.
2. **Energy at or below the power.** With energy exactly equal to the chosen power, or below `MIN_BULLET_POWER`, or 0 (disabled), `SafeShooter` does not fire, or fires only when `energy ≥ p` holds exactly. Tested in Task 7.
3. **Float residue in gun heat.** After cooling, the engine can leave `gun_heat` at around 1e-16, which the engine treats as hot. The observation must report it as non-zero, so the strategy waits instead of issuing a fire the engine silently drops. Tested in Task 3 (`GunHeat::from_f64`).
4. **Degenerate scan geometry.** Two robots at distance 0 or exactly at the 1200 px scan radius, or a radar that did not move this turn: no panic, no NaN bearing, a deterministic result. Tested in Task 5.
5. **End of battle.** The last robot alive wins. If everyone dies on the same turn, the winner is `None`. Dead robots are never asked to `decide` and are never reported as scan targets. Tested in Task 6.

---

### Task 1: PRD update (approval gate)

**Files:**
- Modify: `_dev/PRD.md` (append a new top-level section; leave the existing WASM text untouched)

- [ ] **Step 1: Draft a section `## RoboVerus Track`** covering: the purpose (learning Verus via RoboCode, following the Verus guide); the standalone crate plus optional `verus` feature; the strategy layer (`Observation`/`Action`/`decide`); fixed-point units and the documented trade-off; the prototype scope and non-goals (spec §2); the new Java reference `_wb/latest_version_robocode-main` (1.12.0); and a statement that the WASM/browser track continues unchanged alongside it.
- [ ] **Step 2: Show the user the diff and stop.** Continue only on explicit approval, folding in any changes they ask for.
- [ ] **Step 3: Commit**

```bash
git add _dev/PRD.md
git commit -m "docs: add RoboVerus track to PRD"
```

### Task 2: Verus integration spike (gate)

**Files:**
- Modify: `Cargo.toml` (add `[features] verus = [...]` and optional Verus dependencies)
- Create: `src/units.rs` containing **only** `pub struct Milli(pub u32);` and `pub fn gun_heat_for_power(power: Milli) -> Milli` (= `1000 + power/5`), with a Verus spec: `requires power.0 <= 3000, ensures result.0 >= 1000 && result.0 <= 1600`
- Create: `docs/verus-integration.md`
- Modify: `src/lib.rs` (`pub mod units;`)

**Interfaces:**
- Produces: the mechanism every later Verus task uses, recorded in `docs/verus-integration.md`: the exact Verus release, install commands for macOS and Linux, the verify command (for example `cargo verus verify --features verus`), and how a spec attaches to a plain function (for example `#[cfg_attr(feature = "verus", verus_spec(...))]`) and where `spec fn`s live (for example a `#[cfg(feature = "verus")]` module holding `verus! { … }`).

- [ ] **Step 1: Install Verus** from a pinned GitHub release (plus `cargo-verus` if that release uses it). Record the version.
- [ ] **Step 2: Try candidate mechanisms in order:** (a) attribute specs (`verus_spec` / `verus_verify`) on plain functions behind `cfg_attr(feature = "verus", …)`, with `vstd` and the macro crates optional; (b) code in `verus! {}` blocks with a feature-gated shim macro. Keep the first that meets all three checks below.
- [ ] **Step 3: Check that it builds with no Verus crates.** Run `cargo build && cargo tree -e normal | grep -Ei 'vstd|verus'`. Expected: the build succeeds and grep prints nothing.
- [ ] **Step 4: Check that Verus verifies the spec.** Run the recorded verify command. Expected: `verification results:: N verified, 0 errors` with N ≥ 1. Then change the `ensures` to `result.0 >= 1001` and confirm it **fails**, then revert.
- [ ] **Step 5: Check that the same function runs.** A unit test in `src/units.rs`, `gun_heat_for_power(Milli(3000)) == Milli(1600)`, passes under `cargo test`.
- [ ] **Step 6: If any check cannot pass, stop.** Write the findings in `docs/verus-integration.md` and bring them to the user.
- [ ] **Step 7: Commit**

```bash
git add Cargo.toml src/lib.rs src/units.rs docs/verus-integration.md
git commit -m "feat: verus feature spike with first verified function"
```

### Task 3: Fixed-point units

**Files:**
- Modify: `src/units.rs` (replace the spike's `Milli` with the types below; keep `gun_heat_for_power`)
- Test: `tests/units.rs`

**Interfaces:**
- Produces:
  - `pub struct Energy(pub u32)`, `pub struct GunHeat(pub u32)`, `pub struct Power(pub u32)` (all milli), `pub struct Angle(pub i32)` (decidegrees, absolute `0..3600`). All `Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord`.
  - `Energy::from_f64(f64) -> Energy` (floor, clamp to ≥0); `GunHeat::from_f64(f64) -> GunHeat` (ceil, clamp ≥0); `Power::to_f64(self) -> f64`; `Angle::from_degrees(f64) -> Angle` (round, normalize to `0..3600`); `Angle::to_degrees(self) -> f64`
  - `pub const MIN_POWER: Power = Power(100)`, `MAX_POWER: Power = Power(3000)`, `GUN_COOLING: GunHeat = GunHeat(100)`
  - `pub fn angle_diff(to: Angle, from: Angle) -> i32`: the signed shortest turn from `from` to `to`, in `(-1800, 1800]`
  - `pub fn bullet_damage(p: Power) -> Energy` (`4p + 2·max(p−1000, 0)`), `pub fn bullet_hit_bonus(p: Power) -> Energy` (`3p`), `pub fn gun_heat_for_power(p: Power) -> GunHeat` (`1000 + p/5`)
  - All functions are integer-only and Verus-verifiable (no floats) except the `from_f64`/`to_f64`/`*_degrees` conversions, which are marked external to Verus.

- [ ] **Step 1: Write failing tests**

```rust
#[test] fn energy_rounds_down() { assert_eq!(Energy::from_f64(99.9999), Energy(99_999)); assert_eq!(Energy::from_f64(-1.0), Energy(0)); }
#[test] fn gun_heat_residue_rounds_up() { assert_eq!(GunHeat::from_f64(1e-16), GunHeat(1)); assert_eq!(GunHeat::from_f64(0.0), GunHeat(0)); }
#[test] fn angle_normalizes() { assert_eq!(Angle::from_degrees(-0.5), Angle(3595)); assert_eq!(Angle::from_degrees(360.0), Angle(0)); }
#[test] fn angle_diff_wraps() { assert_eq!(angle_diff(Angle(5), Angle(3595)), 10); assert_eq!(angle_diff(Angle(3595), Angle(5)), -10); assert_eq!(angle_diff(Angle(1800), Angle(0)), 1800); }
#[test] fn rules_match_java_at_bounds() {
    assert_eq!(bullet_damage(Power(100)), Energy(400));    assert_eq!(bullet_damage(Power(3000)), Energy(16_000));
    assert_eq!(bullet_hit_bonus(Power(3000)), Energy(9000)); assert_eq!(gun_heat_for_power(Power(100)), GunHeat(1020));
}
#[test] fn fixed_rules_agree_with_f64_rules() { // every legal power in steps of 5 milli
    for p in (100..=3000).step_by(5) { let f = p as f64 / 1000.0;
        assert!((bullet_damage(Power(p)).0 as f64 / 1000.0 - robocode::rules::bullet_damage(f)).abs() < 1e-9);
        assert!((gun_heat_for_power(Power(p)).0 as f64 / 1000.0 - robocode::rules::gun_heat_for_power(f)).abs() < 1e-9); }
}
```

- [ ] **Step 2: Run `cargo test --test units`.** Expected: FAIL (types are missing).
- [ ] **Step 3: Implement the types and functions** in `src/units.rs` per the Interfaces block. Add Verus specs (via the Task 2 mechanism): `angle_diff` ensures `-1800 < r <= 1800` and `(from + r) ≡ to (mod 3600)`; `bullet_damage` requires `p ≤ 3000` and ensures `r > 0` whenever `p ≥ 100`; `gun_heat_for_power` keeps its spike spec.
- [ ] **Step 4: Run `cargo test --test units`** and then the Verus verify command. Expected: PASS, and `0 errors`.
- [ ] **Step 5: Commit** with the message `feat: fixed-point units and integer rules`.

### Task 4: Strategy API

**Files:**
- Create: `src/strategy/mod.rs`
- Modify: `src/lib.rs` (`pub mod strategy;`)
- Test: `tests/strategy_api.rs`

**Interfaces:**
- Consumes: Task 3 types.
- Produces:

```rust
pub struct TargetInfo { pub bearing: Angle, pub distance: i32, pub energy: Energy, pub heading: Angle, pub velocity: i32, pub age: u32 }
pub struct Observation { pub turn: u32, pub energy: Energy, pub gun_heat: GunHeat, pub heading: Angle, pub gun_heading: Angle,
    pub radar_heading: Angle, pub velocity: i32, pub x: i32, pub y: i32, pub field_width: i32, pub field_height: i32,
    pub target: Option<TargetInfo> }
#[derive(Default)] pub struct Action { pub fire: Option<Power>, pub ahead: Option<i32>, pub turn_body: Option<i32>,
    pub turn_gun: Option<i32>, pub turn_radar: Option<i32> }   // turns: relative decidegrees, +ve = right
pub trait Strategy { fn name(&self) -> &str; fn decide(&mut self, obs: &Observation) -> Action; }
```

`age` is the number of turns since the target was last scanned (0 = scanned this turn). `bearing` is absolute. All types derive `Clone, Debug, PartialEq`.

- [ ] **Step 1: Write a failing test** `action_default_is_idle`: `Action::default()` has every field `None`.
- [ ] **Step 2: Run `cargo test --test strategy_api`.** Expected: FAIL.
- [ ] **Step 3: Implement the types** per the Interfaces block.
- [ ] **Step 4: Run it.** Expected: PASS.
- [ ] **Step 5: Commit** with the message `feat: strategy observation/action API`.

### Task 5: Radar scanning in the engine

**Files:**
- Modify: `src/physics/robot.rs` (add `pub last_radar_heading: f64`)
- Modify: `src/physics/engine.rs` (scan after movement, inside `step`)
- Modify: `src/physics/events.rs` (add `ScannedRobot` and `TurnEvents.scans: Vec<ScannedRobot>`)
- Modify: `src/rules.rs` (add `pub const RADAR_SCAN_RADIUS: f64 = 1200.0;`)
- Modify: `src/physics/mod.rs` (re-export `ScannedRobot`)
- Test: `tests/scanning.rs`

**Interfaces:**
- Produces: `pub struct ScannedRobot { pub scanner: RobotId, pub target: RobotId, pub bearing: f64 /* absolute degrees */, pub distance: f64, pub energy: f64, pub heading: f64, pub velocity: f64 }`

**Semantics** (follow `RobotPeer.updateBoundingBox` and scan logic in `_wb/latest_version_robocode-main/robocode.battle/.../peer/RobotPeer.java`, simplified):
- At the start of `apply_turns`, set `last_radar_heading = radar_heading`.
- After movement, a living robot B is scanned by living robot A ≠ B if `distance(A,B) ≤ RADAR_SCAN_RADIUS` and B's angular extent `[bearing − w, bearing + w]` overlaps the arc swept from `last_radar_heading` to `radar_heading`. Here `w = asin(min(1, radius/distance))` in degrees, and `w = 180` when `distance < radius`.
- `bearing` comes from `geometry::angle_to_point` (`atan2(0,0) = 0`, so it is never NaN). Scans are ordered by `(scanner, target)` id.

- [ ] **Step 1: Write failing tests**

```rust
#[test] fn radar_sweep_finds_robot_in_arc()        // A at (400,300), radar 0°→45° sweep; B at bearing 20°, 200px → one scan, bearing≈20, distance≈200
#[test] fn robot_outside_arc_not_scanned()         // B at bearing 90° → no scan
#[test] fn robot_beyond_radius_not_scanned()       // B at 1201px on bearing 20° → none; at 1199px → one
#[test] fn stationary_radar_still_sees_robot_on_line() // radar not turned, B straight ahead at 0° → one scan
#[test] fn overlapping_robots_scan_without_nan()   // A and B at same point → scan.bearing.is_finite() && distance == 0
#[test] fn dead_robots_neither_scan_nor_are_scanned()
```

Use a `BattleConfig` field of 2000×2000 for the radius tests.

- [ ] **Step 2: Run `cargo test --test scanning`.** Expected: FAIL.
- [ ] **Step 3: Implement** per the Semantics above.
- [ ] **Step 4: Run `cargo test`.** Expected: all tests pass, including the existing `tests/physics.rs`.
- [ ] **Step 5: Commit** with the message `feat: radar scanning`.

### Task 6: Runner (engine ↔ strategies)

**Files:**
- Create: `src/runner.rs`
- Modify: `src/lib.rs` (`pub mod runner;`)
- Test: `tests/runner.rs`

**Interfaces:**
- Consumes: `BattleState`, `ScannedRobot`, Task 3 and Task 4 types.
- Produces:

```rust
pub fn observe(robot: &RobotState, field: Battlefield, turn: u64, last_scan: Option<(&ScannedRobot, u32 /*age*/)>) -> Observation
pub fn apply(robot: &mut RobotState, action: &Action)
pub struct Battle { /* BattleState + Vec<Box<dyn Strategy>> + last scan per robot */ }
impl Battle {
    pub fn new(config: BattleConfig, spawns: &[SpawnSpec], strategies: Vec<Box<dyn Strategy>>) -> Battle  // panics if lengths differ
    pub fn step(&mut self) -> TurnEvents
    pub fn run(&mut self, max_turns: u64) -> BattleOutcome
    pub fn state(&self) -> &BattleState
}
pub struct BattleOutcome { pub winner: Option<RobotId>, pub turns: u64, pub energies: Vec<f64>, pub wall_hits: Vec<u32> }
```

- `step`: for each living robot, build the observation (last scan of the *nearest* target seen by that robot, with its age) → `decide` → `apply` → engine `step` → store scans.
- `apply` converts to `f64` and calls `set_fire`, `set_ahead`, `set_turn_right`, `set_turn_gun_right` and `set_turn_radar_right` for each `Some` field. `None` leaves the pending command as it is.
- `run` stops when ≤1 robot is alive or at `max_turns`. The winner is the sole survivor, otherwise `None`. A timeout with several survivors means `winner = None`.

- [ ] **Step 1: Write failing tests**

```rust
#[test] fn observe_converts_units()       // energy 99.9996 → Energy(99_999); heading 359.96 → Angle(0); gun_heat 1e-16 → GunHeat(1)
#[test] fn apply_sets_controls()          // Action{fire:Some(Power(1500)), turn_gun:Some(-150),..} → control.fire_power==Some(1.5), gun_turn_remaining==-15.0
#[test] fn one_fire_one_bullet()          // strategy returning fire once then idle → exactly 1 bullet over 30 turns (Java #89)
#[test] fn last_survivor_wins()           // robot with energy 0.1 vs. healthy shooter → winner == Some(shooter)
#[test] fn simultaneous_death_has_no_winner()
#[test] fn dead_robots_are_not_asked_to_decide() // counting strategy: decide count stops after death
```

- [ ] **Step 2: Run `cargo test --test runner`.** Expected: FAIL.
- [ ] **Step 3: Implement** per the Interfaces block.
- [ ] **Step 4: Run `cargo test`.** Expected: PASS.
- [ ] **Step 5: Commit** with the message `feat: battle runner connecting strategies to engine`.

### Task 7: Sample robots and the SafeShooter proof

**Files:**
- Create: `src/robots/mod.rs`, `src/robots/sitting_duck.rs`, `src/robots/spinner.rs`, `src/robots/safe_shooter.rs`
- Create: `src/strategy/targeting.rs` (`closest_target`, for chapter 3)
- Modify: `src/lib.rs`, `src/strategy/mod.rs`
- Create: `examples/battle.rs`
- Test: `tests/robots.rs`

**Interfaces:**
- Produces:
  - `pub struct SittingDuck;`: always idle.
  - `pub struct Spinner;`: `ahead 100`, `turn_body 100`, `turn_radar 450` and `turn_gun 0` every turn; fires `Power(1000)` when the target `age == 0`.
  - `pub fn safe_shooter::decide(obs: &Observation) -> Action` (pure) and `pub struct SafeShooter;` whose `Strategy::decide` calls it.
  - `pub const AIM_TOLERANCE: i32 = 30;` (3.0°) and `pub fn safe_shooter::is_aimed(obs: &Observation) -> bool` (`target` present and `|angle_diff(target.bearing, gun_heading)| ≤ AIM_TOLERANCE`).
  - `pub fn targeting::closest_target(targets: &[TargetInfo]) -> Option<usize>`: index of the minimum `distance`, first on ties; `None` if empty.

**SafeShooter behaviour:** radar `turn_radar = Some(450)` every turn. If a target is present with `age ≤ 1`: `turn_gun = clamp(angle_diff(bearing, gun_heading), ±200)`. Fire `Power(3000)` if distance < 150, else `Power(1000)`, **only if** `gun_heat == 0`, `age == 0`, `is_aimed`, and `energy ≥ p`. Movement avoids walls; the algorithm is the builder's choice, but it must produce zero wall hits in the battle test below.

**Verus specs (via the Task 2 mechanism):**
- `safe_shooter::decide` ensures: `r.fire == Some(p) ⇒ obs.energy.0 ≥ p.0 ∧ obs.gun_heat.0 == 0 ∧ obs.target.is_some() ∧ obs.target.unwrap().age == 0 ∧ |angle_diff(target.bearing, obs.gun_heading)| ≤ AIM_TOLERANCE`.
- `closest_target` ensures: `Some(i) ⇒ i < len ∧ ∀j<len. targets[i].distance ≤ targets[j].distance`, and `None ⇔ len == 0`. Proved with a loop invariant.

- [ ] **Step 1: Write failing tests**

```rust
#[test] fn fires_when_safe()              // energy 50_000, heat 0, target age 0 at bearing == gun_heading, dist 300 → fire == Some(Power(1000))
#[test] fn never_fires_when_hot()         // heat GunHeat(1) → fire None
#[test] fn never_fires_without_target()   // target None → fire None
#[test] fn never_fires_on_stale_target()  // age 1 → fire None (but turn_gun is Some)
#[test] fn aimed_across_zero()            // gun Angle(3595), bearing Angle(5) → fires
#[test] fn not_aimed_outside_tolerance()  // diff 31 → fire None
#[test] fn energy_boundary()              // energy == Energy(1000), dist 300 → fires Power(1000); energy Energy(999) → None; Energy(0) → None
#[test] fn close_range_power_needs_energy() // dist 100, energy 2_000 → fire None or a power p with p.0 ≤ 2000
#[test] fn closest_target_picks_min_first_on_ties() // distances [300,100,100] → Some(1); [] → None
#[test] fn safe_shooter_beats_sitting_duck() // Battle 800×600, spawns (100,100,0°) vs (700,500,0°), run(5000) → winner == Some(0) and wall_hits[0] == 0
#[test] fn battles_are_deterministic()    // same SafeShooter vs Spinner battle twice → identical BattleOutcome
```

- [ ] **Step 2: Run `cargo test --test robots`.** Expected: FAIL.
- [ ] **Step 3: Implement** the robots, `is_aimed`, `closest_target` and `examples/battle.rs`. The example takes the opponent name `sitting_duck|spinner` as its argument, runs one battle and prints `winner=<name|none> turns=<n> energy=<a>/<b>`.
- [ ] **Step 4: Run `cargo test` and `cargo run --example battle -- spinner`.** Expected: tests pass and one result line is printed.
- [ ] **Step 5: Add the Verus specs and run the verify command.** Expected: `0 errors`. Then sanity-check that removing the `gun_heat == 0` condition from `decide` makes verification **fail**, and revert.
- [ ] **Step 6: Commit** with the message `feat: sample robots and verified SafeShooter`.

### Task 8: Guide and exercises

**Files:**
- Create: `guide/book.toml`, `guide/src/SUMMARY.md`, `guide/src/introduction.md`, `guide/src/trade-off.md`, `guide/src/ch00-setup.md`, `guide/src/ch01-integer-specs.md`, `guide/src/ch02-requires-ensures.md`, `guide/src/ch03-loop-invariants.md`, `guide/src/coming.md`
- Create: `exercises/Cargo.toml` (package `roboverus-exercises`, depends on `robocode` with `features = ["verus"]`), `exercises/src/lib.rs`, `exercises/src/ch01.rs`, `exercises/src/ch02.rs`, `exercises/src/ch03.rs`
- Create: `solutions/` (same layout, package `roboverus-solutions`)
- Modify: `Cargo.toml` (`[workspace] members = [".", "exercises", "solutions"]`)
- Modify: `src/lib.rs` crate docs (add the fixed-point trade-off paragraph and a link to the guide)

**Content:**
- Each chapter opens with "Mirrors Verus guide: <section name>", teaches that concept on RoboCode, and ends with its exercises.
- ch00: install and verify, using the commands from `docs/verus-integration.md`.
- ch01: specify and prove `bullet_damage > 0`, `gun_heat ≥ 1000` and `bullet_hit_bonus ≤ 9000`.
- ch02: write the SafeShooter firing-safety `ensures` (the learner writes the spec; `decide` is provided).
- ch03: write the loop invariant for `closest_target`.
- `trade-off.md` states the `f64` vs fixed-point trade-off and the safe-direction rounding.
- `coming.md` lists: multi-turn invariants, opponents, "eventually attacks", quantifiers over sequences, and integer geometry for bearings.
- Exercise files compile with plain `cargo build -p roboverus-exercises` but fail verification at each `// TODO`. Solutions verify.

- [ ] **Step 1: Write the solutions first and verify them** with `cargo verus verify -p roboverus-solutions` (or the recorded equivalent). Expected: `0 errors`.
- [ ] **Step 2: Derive the exercises from the solutions** by replacing specs or invariants with `// TODO` holes. Run `cargo build -p roboverus-exercises`; expected: it succeeds. Verifying the exercises should fail at the TODOs.
- [ ] **Step 3: Write the chapters** and run `mdbook build guide`. Expected: success with no broken links.
- [ ] **Step 4: Commit** with the message `docs: RoboVerus guide chapters 0-3 with exercises`.

### Task 9: CI

**Files:**
- Create: `.github/workflows/ci.yml`

Jobs on push and PR:
1. **test**: `cargo test --workspace --exclude roboverus-exercises --exclude roboverus-solutions`, `cargo clippy -- -D warnings`, and the no-Verus check `! cargo tree -p robocode -e normal | grep -Ei 'vstd|verus'`.
2. **verus**: install the pinned Verus from `docs/verus-integration.md`, verify `robocode` with `--features verus`, verify `roboverus-solutions`, and `cargo build -p roboverus-exercises`.
3. **guide**: install mdBook and run `mdbook build guide`.

- [ ] **Step 1: Write the workflow.**
- [ ] **Step 2: Run the job commands locally** (`act` is optional). Expected: each passes.
- [ ] **Step 3: Commit** with the message `ci: tests, verus verification, guide build`.
