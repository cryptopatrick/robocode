# `cargo robocode`: Local Robot Tool Design

Date: 2026-11-05
Status: Draft, awaiting approval

## 1. Intent

Developers write their own robot in a single `robot.rs` and battle it on their own machine, with no server. This is the **local-first** direction the user chose over a web upload-and-compile service. RoboVerus learners already need Rust and Verus installed, and compiling untrusted code on a server is the riskiest, most expensive part of a web version. The battle opens in the existing browser viewer, served locally.

## 2. Goals, non-goals, anti-goals

**Goals**
- `cargo robocode new <name>` creates a robot file from a template.
- `cargo robocode battle <robot.rs> --vs <opponent> …` compiles the robot with the user's own `cargo`, runs the battle, saves a recording, and opens the viewer in the browser with that battle loaded.
- Opponents can be built-in robots (`SafeShooter`, `Circler`, `Spinner`, `SittingDuck`) or other `.rs` robot files.
- `cargo robocode verify <robot.rs>` runs Verus on a robot file, for RoboVerus learners writing proved strategy robots.
- Compile errors appear in the terminal exactly as `cargo` prints them.

**Non-goals (now)**
- Running *other people's* robots in a sandbox. Robots compile natively, because it's your own code on your own machine.
- Any web upload. **The RoboCode Arena is deferred**; see `docs/roadmap.md`.
- Scoring and rounds, and more than 4 robots per battle. Both are limited by the engine and viewer today.
- Publishing to crates.io (§6).

**Anti-goals**
- A tool that needs network access to run a battle, other than `cargo` fetching dependencies the first time.
- Battles that play out differently from the same robots run through the library API **natively**. *(Designer's call:)* matching the browser engine's WebAssembly build exactly isn't promised. Floating-point maths can differ slightly between native code and WebAssembly. The future Arena runs profiles natively on its server, so this doesn't block it.
- A robot file format that forces boilerplate beyond one `robot!` line.

## 3. Constraints

- Rust. A local HTTP server counts as backend code, so it uses **Axum** (CLAUDE.md). There's no database, because nothing is persisted beyond files.
- The `robocode` library keeps **zero** non-optional dependencies. Axum, Tokio, browser-opening and CLI parsing live only in the new CLI crate.
- The viewer stays React + TypeScript + Tailwind, with no inline styles.
- Don't touch the user's uncommitted `README.md`, `LICENSE`, `COPYING`, `CONTRIBUTING.md` or `Cargo.toml` license line.
- Keep the Arena's future needs in mind: determinism, and recording format v1 stays compatible.

## 4. Design

### 4.1 Robot file

```rust
use robocode::api::*;

#[derive(Default)]
pub struct MyRobot { /* state */ }

impl Robot for MyRobot { /* name, run, handlers */ }

robocode::robot!(MyRobot);
```

- `robocode::robot!(T)` is a `macro_rules!` macro in the library. It expands to a known function, for example `pub fn __robocode_robot() -> Box<dyn robocode::api::Robot> { Box::new(<T as Default>::default()) }`. `T` must implement `Default` and `Robot`.
- It also works for strategies: `robocode::robot!(strategy MyStrategy)` wraps the type with `runner::boxed`.
- A file without exactly one `robot!` line fails to compile with a clear error (a missing function), and the tool explains the fix.
- *(Designer's call)* **Robot files may use only `std` and `robocode`.** No other crates for now; this is documented. It keeps builds offline after the first fetch and matches the Arena's no-dependencies direction. A way to declare extra crates can come later.

### 4.2 Compiling

- The tool keeps one reusable build project at `<cache>/robocode/build/` (platform cache dir), so cargo's incremental compilation makes later battles fast.
- For each battle it writes `src/main.rs`, including each robot file as a module (`mod robot_0 { include!("<absolute path>"); }`). The main program builds the battle, runs it, and writes the recording.
- The `robocode` dependency in that project is a **path dependency on the robocode checkout the tool was built from** (embedded at build time), overridable with `--robocode-path` or `ROBOCODE_PATH`. Once `robocode` is published it can switch to a version dependency (§6).
- It builds in release mode, for speed. Robot code runs in that generated binary, not inside the tool.
- *(Designer's call)* If the embedded robocode path no longer exists and no override is given, the tool exits 2 with: `robocode source not found at <path>; pass --robocode-path or set ROBOCODE_PATH`. There's no network fallback.

### 4.3 Recording and viewer

- *(Designer's call)* **One shared roster in the library:** the built-in robot names and constructors, plus the corner spawn table, move from `wasm/src/lib.rs` into the library as `robocode::roster`. The CLI's `list` and `--vs`, the CLI's generated program and the browser engine all use it, so they can't drift apart.
- The recording writer moves from `robocode-wasm` into the library, as `robocode::recording`, so the CLI-built binary and the browser engine produce identical recording v1 JSON from a `Battle`. `robocode-wasm` keeps its API and calls the shared writer.
- The viewer gains a URL parameter: `?recording=<url>` loads that recording at startup instead of running the default battle. Load errors show in the page's existing error box.
- `battle` starts an Axum server on `127.0.0.1` with a random free port. It serves the viewer's built files (embedded in the tool at build time from `web/dist`) and the battle at `/battle.json`, then opens `http://127.0.0.1:<port>/?recording=/battle.json`. It keeps serving until Ctrl+C.

### 4.4 Commands

```
cargo robocode new <name>                    # writes <name>.rs from the template (refuses to overwrite)
cargo robocode battle <robot.rs> [--vs <opponent>]... [--arena 800x600] [--turns 5000]
                      [--output battle.json] [--no-open]
cargo robocode verify <robot.rs>             # cargo verus verify on the generated project
cargo robocode list                          # built-in robot names
```

- **`--vs`:** a built-in name or a path to a `.rs` file. It's repeatable, for 2–4 robots in total. The default is `--vs SittingDuck`.
- **Spawns** use the viewer's corner table, in command-line order.
- **`--no-open`** prints the URL instead of opening a browser. With `--output` and `--no-open`, the tool writes the file and exits without serving.
- *(Designer's call)* **What `verify` checks:** it uses a separate generated *library* project with the robot file **copied** to `src/robot.rs`, not `include!`d, because Verus handles `include!` inside a module poorly. Only that crate is the verification target. The `robocode` dependency's own proofs are checked as part of the dependency build, and a failure there is reported as a tool or installation problem, not the user's fault. Exit 1 means the user's robot failed verification. A robot with no specs "verifies" trivially, and the tool says `0 specs checked in your robot`, so nobody mistakes that for a proof.
- **`verify`** needs `cargo verus` on `PATH`; otherwise it says so and links to `docs/verus-integration.md`. It verifies the generated project with `robocode/verus` enabled, so robots can use the attribute-style specs that `SafeShooter` uses.
- **Exit codes:** 0 for success, 1 for a robot that won't compile or fails verification, 2 for usage errors.

## 5. Testing

- **Library:** tests for `robot!`, and for `recording` producing the same JSON as before the move. The existing `robocode-wasm` record tests stay as they are.
- **CLI integration tests** (they run real `cargo` against a temp robot file):
  - battling a template robot against `SittingDuck` with `--output --no-open` writes a valid v1 recording, with the right names and a winner;
  - a robot with a compile error exits 1 and shows the compiler message;
  - a missing `robot!` line gives the explanatory error;
  - an unknown opponent exits 2;
  - `new` refuses to overwrite.
- **Viewer:** a unit test for the `?recording=` loader (success, and failure going to the error box), plus a headless-browser check that the served page loads the battle.
- **CI:** builds `web/dist` before the CLI crate; runs the CLI tests.

## 6. Left to the builder

- The CLI crate's name and layout (for example `cli/`, package `cargo-robocode`), the argument parser and the browser-opening crate, each pinned to a released version.
- The exact cache path and template contents (based on `examples/my_robot.rs`).
- How `web/dist` gets embedded (a `build.rs` that fails with "run `npm run build` in web/ first" if it's missing).
- **Publishing:** not in scope. Record in the docs that `cargo install --path cli` is the install method for now.
