# RoboVerus — Initial Prototype Design

Date: 2026-11-04
Status: Draft, awaiting approval

## 1. Intent

RoboVerus is a way to **learn Verus through RoboCode**. It follows the structure
of the official Verus guide (`_wb/verus_guide.pdf`) chapter by chapter, but every
example and exercise is about RoboCode robots and rules instead of abstract
integers and arrays. The original Rocq-oriented thinking is in `_wb/roboverus.md`;
this design carries its ideas over to Verus.

There are two deliverables:

1. **The `robocode` crate**: a standalone Rust port of classic RoboCode
   (source of truth: `_wb/latest_version_robocode-main`, Robocode 1.12.0; `_wb/robocode-VER_1_10_1` is a secondary reference with identical physics). Anyone can use it with plain
   `cargo`, with no knowledge of or dependency on Verus.
2. **The RoboVerus guide**: lessons and exercises that use the crate's optional
   `verus` feature to specify and prove properties of robots and rules.

The user is the first learner. The motivation is learning formal verification
in a setting that is fun and concrete.

## 2. Goals, non-goals, anti-goals

**Goals (prototype)**
- The crate stays standalone. With the `verus` feature off, it pulls in **no Verus
  crates** and behaves like an ordinary Rust library.
- With the `verus` feature on, Verus can verify the specs and proofs that ship
  with the crate, and learners can write their own against it.
- Robots are written as **strategies**: a pure decision function from an
  observation to an action. That shape is useful without Verus, and it is what
  the lessons prove things about.
- A running example robot, **`SafeShooter`**, whose central proof is the first
  milestone from `_wb/roboverus.md`:
  if `decide(o).fire == Some(p)`, then the energy is enough for `p`, the gun is cool,
  a target is visible and the gun is aimed at it (see §5.3 for what "aimed" means).
- A headless battle where `SafeShooter` fights a simple sample robot using the
  existing engine. The outcome is printed to the terminal.
- The first lessons of the guide, mirroring the opening chapters of the Verus guide.

**Non-goals (prototype)**
- A graphical battle viewer, a browser/WASM build, a backend, a database or a frontend.
  (These remain in the PRD's longer-term scope; see §7.)
- Full parity with the Java engine. The existing `parity` harness work continues
  separately.
- Proving anything about the `f64` physics engine.
- Covering the later Verus guide chapters (concurrency, ghost/tracked state, and so on).

**Anti-goals (failure even if it "works")**
- The core crate is shaped around Verus: Verus types, macros or idioms leak into
  the non-`verus` API, or plain users pay compile time or dependencies for it.
- Proofs are about a toy that has nothing to do with the code that actually runs
  in battles.
- Lessons that teach RoboCode trivia instead of Verus concepts. Every lesson must
  map to a concept in the Verus guide.

## 3. Constraints

- Rust, edition 2024 (existing `Cargo.toml`). Crate name `robocode`, license AGPL-3.0-only.
- Follow `CLAUDE.md`. Its stack rules (Axum, utoipa, Postgres/SQLx, React,
  Tailwind) apply to backend/frontend work, and the prototype has none.
  GitHub Actions is used for CI.
- `CLAUDE.md` requires following `_dev/PRD.md`. The PRD does not mention
  Verus or RoboVerus yet, so it must be updated as part of this work (see §7).
- The existing code in `src/` (geometry, rules, physics engine, snapshots,
  parity CLI) is kept and built on, not rewritten.
- Do not touch the user's uncommitted changes to `LICENSE`, `COPYING`,
  `CONTRIBUTING.md`, `README.md` and `Cargo.toml` beyond what this work needs.

## 4. Decisions made with the user

1. **Standalone crate + `verus` feature.** The implementation is generic enough
   not to be tied to Verus.
2. **Numbers.** Verus reasons well about integers and poorly about floats. The
   `f64` engine stays faithful to Java. The verified world uses integer or
   fixed-point quantities. **This trade-off must be written down in the crate
   docs and the guide**: the `f64` physics matches Java, the integer model is
   what gets proved, and the two are not the same code.
3. **Focus on robot strategies**, with some lessons about the game rules
   themselves where they fit the early integer chapters.
4. **Robot shape:** event handling gathers observations → `decide(Observation) -> Action`
   → an executor issues `set_fire`, `set_ahead` and so on. This is the three-layer
   structure from `_wb/roboverus.md`.
5. **Lessons follow the Verus guide's order**, starting with single-decision
   proofs and moving to multi-turn invariants later.

## 5. Design

### 5.1 Crate layout (additions to existing `src/`)

```
src/
  lib.rs
  geometry.rs, rules.rs, physics/        (existing f64 engine; unchanged except as needed)
  units.rs        fixed-point integer quantities shared by strategies (new)
  strategy/       Observation, Action, Strategy trait (new)
  runner.rs       connects strategies to the engine: build observation → decide → apply (new)
  robots/         sample strategies: SafeShooter, SittingDuck, Spinner (new)
  verified/       only with feature "verus": spec fns, integer rules model, proofs (new)
guide/            RoboVerus guide (mdBook) (new)
exercises/        lesson exercise files with holes to fill in (new)
```

### 5.2 Fixed-point boundary (*my call*)

`Observation` and `Action` use **integer fixed-point units in every build**, for
example energy and gun heat in tenths, distance in whole pixels and angles in
tenths of a degree. The runner converts from the `f64` engine into these units
and back. The consequence is that **`decide` is the same code whether proved or
run**: the proofs about `SafeShooter` are about the function that actually
fights. The only gap is quantization at the boundary and the `f64` physics,
which is the documented trade-off from §4.2. The exact units are left to the
builder. They must keep `MIN_BULLET_POWER` (0.1) and `GUN_COOLING_RATE` (0.1)
exactly representable.

### 5.3 Strategy API (sketch, builder may refine)

```rust
pub struct Observation { energy, gun_heat, gun_heading, heading, position,
                         battlefield, target: Option<TargetInfo>, /* walls, etc. */ }
// One Action per turn, carrying several optional commands, like classic
// RoboCode's set_* calls (move, turn, aim, scan and fire in the same turn).
pub struct Action { fire: Option<Power>, ahead: Option<Distance>, turn_body: Option<Angle>,
                    turn_gun: Option<Angle>, turn_radar: Option<Angle> }
pub trait Strategy { fn decide(&mut self, obs: &Observation) -> Action; }
```

*My call:* an action is a bundle of optional commands, not a single enum
variant, so SafeShooter can track, steer and shoot in the same turn.

*My call, "aimed":* `TargetInfo` includes the **absolute bearing** to the target,
computed by the runner in `f64` and rounded into fixed-point units. "Aimed"
means `|gun_heading − target bearing| ≤ AIM_TOLERANCE`, using angle-wrapped
integer arithmetic. The trigonometry stays outside the proof. That keeps
chapter 2 about `requires`/`ensures`. Proving geometry on integers (for example,
that a computed bearing points at the target) is a later chapter.

Whether `decide` takes `&mut self` (stateful robots) or is a pure `fn` plus an
explicit state value is left to the builder. A pure form is preferred if it
keeps the proofs simpler.

### 5.4 Verus integration mechanism (spike first)

The exact mechanism is **open and must be settled by a spike before anything
else is built**. The spike proves the round trip:

- `cargo build` / `cargo test` with no features: compiles with zero Verus crates.
- With feature `verus`: Verus verifies a spec on a function in `src/`, and the
  same function compiles and runs normally.

Candidates are Verus's attribute-style specs (`verus_spec` on ordinary Rust
functions) behind `cfg_attr(feature = "verus", …)`, or putting verifiable code in
`verus!` blocks gated by the feature, with a plain-Rust twin otherwise. Prefer the
option that keeps `decide` written once.

**If the spike shows these can't all hold at once** (no Verus crates without the
feature, and `decide` written once), **stop and bring the findings to the user**
before choosing a direction. Do not fall back to a hand-kept twin or an
unconditional Verus dependency on your own. *(User's decision.)* Verus is not installed on the dev
machine yet. Installing it is part of the spike, and the steps go into the guide's
setup chapter.

### 5.5 No classic event API in the prototype (*my call*)

The runner reads engine state and scan results directly to build each
`Observation`. A classic AdvancedRobot-style event API (`on_scanned_robot`,
`on_hit_wall`, …) is **not** built in the prototype. It stays on the PRD's
roadmap and can later sit beside the strategy layer.

### 5.6 Scanning

The engine has no radar scanning yet, and a strategy needs a `target` in its
observation. The prototype adds radar scanning to the engine, following the
Java original (`robocode.battle`'s scan arc between the previous and current
radar heading). It is enough for `ScannedRobot`-style information: bearing,
distance, energy, heading and velocity.

### 5.7 Sample robots and battle

- `SafeShooter`: scans, turns the gun toward the target, fires only when safe,
  and keeps away from walls.
- `SittingDuck` and a simple `Spinner` as opponents.
- `cargo run --example battle` runs SafeShooter against an opponent for N rounds
  headlessly and prints the winner, energy and turn count.

### 5.8 Guide and exercises (prototype scope)

An mdBook in `guide/`. Each chapter names the Verus guide section it mirrors.
Exercises in `exercises/` are Rust files with `// TODO` holes that learners fill
in and check with Verus. Reference solutions are kept separately so the
exercises can be checked in CI.

Prototype chapters:
0. **Setup**: install Verus, verify a first RoboCode spec.
1. **Specs on integers** (Verus: basic specs, `int`/`nat`, `spec fn`): specify
   fixed-point `bullet_damage`, `bullet_hit_bonus` and `gun_heat_for_power`, and
   prove simple facts (damage is positive, heat ≥ 1).
2. **`requires` / `ensures`** (Verus: pre/postconditions): the SafeShooter
   firing-safety proof (the first milestone).
3. **Loops and invariants** (Verus: loop invariants): pick the closest target
   from a list of scanned robots.

Later chapters (multi-turn invariants, opponents, "eventually attacks",
quantifiers over sequences) are out of prototype scope but should be listed in
the guide's table of contents as "coming".

### 5.9 Testing and CI

- Unit and integration tests for units conversion, scanning, runner and sample robots.
- A test that `SafeShooter` beats `SittingDuck`.
- GitHub Actions: `cargo test` (no features), plus a job that installs Verus
  and verifies the crate with `verus` enabled and the exercise solutions.

## 6. Left to the builder

- The exact fixed-point units and type names.
- The exact `Observation` / `Action` fields and the stateful-vs-pure `decide` choice.
- The Verus integration mechanism (via the spike in §5.4).
- mdBook config, chapter wording and the exercise file layout.
- How rounds and winners are reported in the battle example.

## 7. PRD update (gate)

`_dev/PRD.md` must be updated before building starts. The first step is to
**draft the PRD change and have the user approve it**. *(User's decision.)*
The draft adds RoboVerus as **its own track alongside** the existing browser/WASM
plan, without replacing it. It covers the `verus` feature, the strategy layer, the
fixed-point trade-off and this prototype's scope. No product code is written until
it is approved.

## 8. Calls made by the designer (check these)

- §5.2 Observations/actions use fixed-point integers in every build, so `decide` is identical when proved and when run.
- §5.3 An `Action` is a bundle of optional commands per turn, and the milestone is stated as `decide(o).fire == Some(p) ⇒ …`.
- §5.3 "Aimed" uses a bearing the runner computes in `f64` and passes in. No trigonometry inside proofs in the prototype.
- §5.5 No classic event API in the prototype. The runner reads engine state directly.
- §5.6 Radar scanning is added to the engine, following the Java original.
- §5.8 The guide is an mdBook in `guide/`, with exercises in `exercises/` and reference solutions checked in CI.
