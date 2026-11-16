# Design decisions

This document explains why `robocode` is built the way it is. It is written as
a FAQ. Where a point is a judgment call rather than a fact about the code, it
says so, and reasonable people may disagree with it.

For how a battle runs, see [`docs/architecture.md`](docs/architecture.md). For
the robot API, see [`docs/robot-api.md`](docs/robot-api.md).

## Why port RoboCode to Rust at all?

Two reasons. The first is practice: RoboCode is a good, small, well-known
simulation to practice formal verification on, so the Verus guide in
`guide/` could use it as its running example. The second is to make the
simulation's rules explicit enough to state as specifications. A Java
simulation with 25 years of accumulated behaviour is hard to specify; a Rust
port that is deliberately smaller is easier to.

## Why not port RoboCode exactly?

Some behaviour was changed on purpose, and each change is listed in
`docs/architecture.md`. Examples: the order in which robots act each turn is
fixed (Java shuffles it), a robot with zero energy is removed (Java disables
it until it is hit again), and hit and scan checks use circles. The goal is a
faithful model of the rules that matters to robot writers, not a bit-for-bit
copy of the Java engine. (Judgment: a deterministic engine is worth more here
than matching every Java quirk.)

## Why does the default battle have no randomness?

Every battle is reproducible: the same robots, arena and turn limit always
give the same recording. Spawn points are fixed corners, rotated per round.
This makes the golden tests, the fingerprints, the snapshots and the property
tests possible. Java uses a seeded random generator for spawns; we don't.

## Why are time budgets off by default?

Java robots run on their own threads, so a slow robot falls behind in real
time and skips turns. A Rust robot is a direct call. A time budget can
reproduce that behaviour (`Battle::set_time_budget`), but it makes results
depend on the machine's speed. So it is opt-in, and the default stays
deterministic. Even with a budget, nothing can interrupt a robot that never
returns (see below).

## Why can't a stuck robot be interrupted?

Java can stop a thread. Rust can't safely interrupt a running function call,
so an infinite loop in a robot hangs the battle. This is a real limit of the
approach, not something we plan to hide. Robots run in the user's own process
(`cargo robocode battle`), so a hang blocks that run, not a shared server.

## Why do Verus specifications sit next to the code?

The specifications are attributes on the ordinary Rust functions
(`verus_spec`), and they only exist when the `verus` feature is on. A plain
build compiles to plain Rust with no Verus crates at all, and `cargo tree`
shows no dependency on them by default. The alternative, separate specification
files that mirror the code, would drift out of step with it.

## Why are there two robot styles?

A `Robot` (the event-handler API, `docs/robot-api.md`) is how someone writes a
robot that behaves like a Java `AdvancedRobot`. A `Strategy` is a single
`decide(&Observation) -> Action` over fixed-point numbers. Verus can prove a
`Strategy` (SafeShooter is the example), because its inputs and outputs are
plain values. The event API is for porting existing robots, and the strategy
is for robots we want to prove things about. (Judgment: this split keeps the
proof-friendly core small without making the everyday API harder to use.)

## Why fixed-point units?

The strategy layer and the `units` module store energy, gun heat and bullet
power as whole thousandths, and angles as tenths of a degree. Integer
arithmetic makes those rules exact enough to state for Verus, and it avoids
float rounding differences in the proved code. The physics engine itself uses
`f64` throughout and is not proved; the boundary between the two is where
values are rounded, in a stated direction (energy down, gun heat up).

## Why circles for hits and scans?

A bullet hits a robot when its centre is within the robot's radius plus the
bullet's radius. A robot is scanned when it is within 1200 px. These are
simple to state and to prove. Java uses bounding boxes for both, and that
difference is listed in `docs/architecture.md`. Robot-to-robot collisions do
use the 36 × 36 box, as Java does, because "at fault" depends on the box edges.

## Why is the live simulation separate from the replay?

A battle can be computed all at once and then played back (the replay), or it
can be advanced turn by turn while it is watched (live). Both use the same
engine and produce the same turns, which is checked by tests. The live path
exists so that a viewer can watch a battle as it happens, the way Java
Robocode does, and so that pausing, stepping and changing speed act on the
simulation itself. The replay is kept because it's simpler and needs no pacing.

## Why does the browser only battle built-in robots?

The browser runs the engine as WebAssembly, and the robots it knows are the
ones compiled into it. Running a user's Rust robot in the browser would mean
compiling it there, which the tool deliberately avoids (see the next
question). Writing and running your own robot is a local job.

## Why compile robots locally instead of on a server?

The local-first decision (`cargo robocode`) keeps user code on the user's
machine. The compiler runs locally, there is no upload, and there is no
shared server to protect against untrusted code. A hosted arena was considered
and deferred: it would only accept robots as Verus-verified data files, never
as code that runs on the server. (Judgment: this is the safer default for a
tool meant for learning.)

## Why does the urban map look the way it does?

The urban map is one fixed layout, so every battle on it is reproducible.
Buildings are axis-aligned rectangles: they stop tanks at their faces and
absorb bullets on their path. They don't block radar, which keeps the first
version simple. Roads are for display only. Buildings are kept only where
they fit the arena and leave the spawn corners free, so every robot can start
and move.

## Why is the Verus proof about SafeShooter?

SafeShooter is the sample whose decisions are simple enough to state. The
proofs say it fires only when the shot is safe, never drives into a nearby
wall, and that a helper function is well formed. They don't say anything about
the whole battle, and they don't cover the event API. The mutation checks
(`cargo run -p verus-mutants`) confirm that each `ensures` clause rejects a
planted bug, which is the evidence that the specifications have teeth.

## Why EPL-1.0?

The crate is licensed under the Eclipse Public License 1.0, the licence of the
original RoboCode, which it ports. (Judgment: following the original's licence
is the simplest way to be clear about a port's rights.)
