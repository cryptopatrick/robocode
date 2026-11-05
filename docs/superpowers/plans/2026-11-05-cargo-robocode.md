# `cargo robocode` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A local command-line tool: `cargo robocode new | list | battle | verify`. It compiles a developer's `robot.rs` with their own `cargo`, runs a battle, and opens the existing viewer on a local page with the battle loaded.

**Architecture:**
- **The library gains three modules.** `robocode::robot!` names a file's robot. `robocode::roster` holds the built-in robots and spawn table. `robocode::recording` is the recording v1 writer, moved here from `robocode-wasm`.
- **A new crate `cli/`** (package `cargo-robocode`, binary `cargo-robocode`) works in a reusable build folder. For `battle` it generates a program that `include!`s the robot files and writes a recording. It then serves the embedded `web/dist` and the recording with Axum on `127.0.0.1`, and opens the browser.
- **The viewer** learns `?recording=<url>`.

**Tech Stack:** Rust, Axum + Tokio (CLI only), clap, React/TypeScript viewer (existing).

**Spec:** `docs/superpowers/specs/2026-11-05-cargo-robocode-design.md`

## Global Constraints

- `robocode` keeps zero non-optional dependencies. `cargo tree -p robocode -e normal` lists only `robocode`. CLI dependencies are pinned to exact versions released at least two weeks ago.
- Robot files may use only `std` and `robocode`.
- **The recording format stays at v1**, and the browser engine's output doesn't change: the `robocode-wasm` record tests pass unchanged, and a fingerprint of SafeShooter vs Spinner's recording is pinned before the move.
- Golden battles are unchanged: SafeShooter vs Spinner gives 386 turns and `77.0/-1.0`, and vs SittingDuck gives 364 turns and `147.0/0.0`.
- Exit codes: 0 for success, 1 for a robot that fails to compile or verify, 2 for usage or setup errors (including robocode source not found).
- The local server binds `127.0.0.1` only.
- Don't edit `README.md`, `LICENSE`, `COPYING` or `CONTRIBUTING.md`, or the user's `Cargo.toml` license line; stage `Cargo.toml` hunk-only. Commits carry no Claude attribution. Work on branch `local-cli`.

## Review Focus

1. **Robot paths with spaces or relative paths.** `include!` needs an absolute, escaped path. Tested in Task 4.
2. **Two opponents from the same file, or two built-ins with the same name.** Module names must stay unique; display names get " (2)". Tested in Task 4.
3. **The first run with no cache folder, and a second run that reuses it.** Both work, and the second doesn't rebuild the library from scratch. Tested in Task 4.
4. **A port already in use, or a browser that fails to open.** The tool picks a free port (bind to `:0`). If the browser fails to open, it still prints the URL and keeps serving. Tested in Task 5.
5. **`verify` on a robot with no specs.** It reports `0 specs checked in your robot` and exits 0, without claiming a proof. Tested in Task 6.

---

### Task 1: Library `roster`, `recording` and `robot!`

**Files:**
- Create: `src/roster.rs`, `src/recording.rs`
- Modify: `src/lib.rs` (modules plus the `robot!` macro), `wasm/src/lib.rs` (delegate to the library)
- Test: `tests/library_cli_support.rs`

**Interfaces:**
- `roster`:
  - `pub const BUILT_IN: [&str; 4] = ["SafeShooter", "Circler", "Spinner", "SittingDuck"];`
  - `pub fn built_in(name: &str) -> Option<Box<dyn Robot>>`
  - `pub fn spawns(n: usize, width: f64, height: f64) -> Vec<SpawnSpec>` (the corner table, heading 0, n ≤ 4)
- `recording`:
  - `pub fn record(battle: Battle, max_turns: u64) -> String` writes the v1 JSON, with robot names from `battle.name(i)` and `battlefield` from the config.
  - `pub fn validate_setup(n: usize, width: f64, height: f64, max_turns: u64) -> Result<(), String>` returns the exact error strings currently in `wasm/src/lib.rs`.
- `robot!`:
  - `robocode::robot!(T)` expands to `pub fn __robocode_robot() -> Box<dyn robocode::api::Robot> { Box::new(<T as Default>::default()) }`.
  - `robocode::robot!(strategy T)` wraps the type with `robocode::runner::boxed`.

- [ ] **Step 1: Pin the fingerprint before the move.** Add a test `recording_fingerprint_is_unchanged` in `wasm/tests/record.rs` that hashes `record_battle(SafeShooter vs Spinner)` with a small FNV-1a function in the test, and asserts the length and the hash. Run it once with a placeholder to capture the values, then hard-code them. Expected: PASS on today's code.
- [ ] **Step 2: Write failing tests** in `tests/library_cli_support.rs`:
  - `roster_lists_and_builds` (every name in `BUILT_IN` builds, and `built_in("Nope")` is `None`);
  - `spawns_use_corners` (n=4 at 800×600 → (100,100), (700,500), (700,100), (100,500));
  - `record_matches_v1_shape` (format/version/robots, `turns.len() == outcome.turns + 1`);
  - `robot_macro_builds_api_robot` and `robot_macro_wraps_strategy` (call `__robocode_robot()` from a test module that invokes the macro).
- [ ] **Step 3: Run `cargo test --test library_cli_support`.** Expected: FAIL.
- [ ] **Step 4: Implement**, moving the logic out of `wasm/src/lib.rs`. `record_battle` keeps its signature and calls `validate_setup`, `roster` and `recording::record`.
- [ ] **Step 5: Run `cargo test --workspace --exclude roboverus-exercises --exclude roboverus-solutions`.** Expected: PASS, including the fingerprint and golden tests.
- [ ] **Step 6: Commit** with the message `feat: shared roster, recording writer and robot! macro in the library`.

### Task 2: The viewer loads `?recording=`

**Files:**
- Create: `web/src/loadFromUrl.ts`
- Modify: `web/src/App.tsx`
- Test: `web/src/loadFromUrl.test.ts`

**Interfaces:**
- `recordingUrl(search: string): string | null` returns the `recording` query value, or null.
- `loadRecording(url: string, fetchFn = fetch): Promise<Recording>` fetches and calls `parseRecording`. On a non-OK HTTP status it throws `Error("Could not load <url>: HTTP <status>")`.
- `App`: if `recordingUrl(location.search)` is set, load it at startup, start playing, and skip the default in-browser battle. Errors go to the error box. The engine still loads, for New Battle.

- [ ] **Step 1: Write failing tests**: `reads_recording_param`, `no_param_is_null`, `loads_and_parses` (a stub fetch returning a minimal v1 recording), `http_error_message` (status 404).
- [ ] **Step 2: Run `npm test`.** Expected: FAIL.
- [ ] **Step 3: Implement it and wire it into `App`.**
- [ ] **Step 4: Run `npm test && npm run build`.** Expected: PASS.
- [ ] **Step 5: Commit** with the message `feat: viewer loads a recording from ?recording=`.

### Task 3: The CLI crate: `list`, `new`, argument parsing, exit codes

**Files:**
- Create: `cli/Cargo.toml` (package `cargo-robocode`, `[[bin]] name = "cargo-robocode"`), `cli/build.rs`, `cli/src/main.rs`, `cli/src/template.rs`
- Modify: `Cargo.toml` (add `"cli"` to workspace members; hunk-only)
- Test: `cli/tests/cli.rs` (runs `env!("CARGO_BIN_EXE_cargo-robocode")`)

**Interfaces:**
- **Arguments:** the tool accepts being called as `cargo-robocode robocode <cmd>`, which is how `cargo robocode` invokes it, and also as `cargo-robocode <cmd>`.
- **Subcommands:** `list`, `new <name>`, `battle …`, `verify <file>`.
- **`build.rs`:**
  - sets `ROBOCODE_SOURCE` to the workspace root's absolute path;
  - generates `OUT_DIR/viewer_assets.rs`: a static list of `(path, bytes)` for every file in `web/dist`;
  - if `web/dist/index.html` is missing, panics with "run `npm run wasm && npm run build` in web/ first".
- **`new <name>`:** writes `<name>.rs` (the template: `examples/my_robot.rs`'s robot, plus `robocode::robot!(…)`). If the file exists it exits 2 with `<name>.rs already exists`.

- [ ] **Step 1: Write failing tests**:
  - `list_prints_built_ins` (stdout has the 4 names);
  - `new_writes_template` (temp dir; the file contains `robocode::robot!(`);
  - `new_refuses_overwrite` (exit 2);
  - `works_as_cargo_subcommand` (`cargo-robocode robocode list` behaves like `list`);
  - `unknown_command_exits_2`.
- [ ] **Step 2: Run `cargo test -p cargo-robocode`.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run it.** Expected: PASS.
- [ ] **Step 5: Commit** with the message `feat: cargo robocode list and new`.

### Task 4: `battle`: generate, compile, run, write the recording

**Files:**
- Create: `cli/src/build_project.rs`, `cli/src/battle.rs`
- Modify: `cli/src/main.rs`
- Test: `cli/tests/battle.rs`

**Interfaces:**
- **Cache:** the build folder is `<cache dir>/robocode/build` (from the `dirs` crate), overridable with `ROBOCODE_BUILD_DIR` for tests.
- **Generated project:**
  - `Cargo.toml` depends on `robocode = { path = "<source>" }`, where the source is `--robocode-path`, then `ROBOCODE_PATH`, then the build-time `ROBOCODE_SOURCE`. A missing source exits 2 with `robocode source not found at <path>; pass --robocode-path or set ROBOCODE_PATH`.
  - `src/main.rs` has one `mod robot_<k> { include!(<absolute path as a Rust string literal via {:?}>); }` per distinct robot file. `main` builds the robot list in command-line order (built-ins via `roster::built_in`), spawns via `roster::spawns`, and prints `recording::record(...)` to stdout.
- **Running it:** `cargo run --release --quiet --manifest-path <build>/Cargo.toml`. Compile errors are passed through to stderr, then the tool exits 1.
- **Missing `robot!`:** if the compiler output contains `__robocode_robot`, the tool adds the hint `add robocode::robot!(YourRobot); to <file>`.
- **Options:**
  - `--vs` (repeatable, default `SittingDuck`), with 2–4 robots in total;
  - `--arena WxH` (default `800x600`);
  - `--turns N` (default 5000), validated with `recording::validate_setup`;
  - `--output FILE`;
  - `--no-open`.
- **With `--output` and `--no-open`:** the tool writes the file, prints `<winner> wins after N turns` (or `No winner after N turns`), and exits 0. Serving is Task 5.

- [ ] **Step 1: Write failing tests.** Each uses a temp `ROBOCODE_BUILD_DIR` shared within the test file, to keep it fast:
  - `battle_writes_recording` (template robot vs `SittingDuck` → a valid v1 file, `robots[0].name == "MyRobot"`, and a winner);
  - `path_with_spaces_and_relative` (robot in `my robots/bot.rs`, given as a relative path);
  - `two_copies_of_same_file_and_names` (`bot.rs --vs bot.rs` → names `MyRobot`, `MyRobot (2)`);
  - `compile_error_exits_1_with_message` (`fn broken(` → exit 1, and stderr contains `error`);
  - `missing_robot_macro_hint`;
  - `unknown_opponent_exits_2`;
  - `missing_source_exits_2` (`--robocode-path /nonexistent`);
  - `second_run_reuses_cache` (the second battle finishes faster than the first, or the stderr has no `Compiling robocode`).
- [ ] **Step 2: Run `cargo test -p cargo-robocode --test battle`.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run it.** Expected: PASS.
- [ ] **Step 5: Commit** with the message `feat: cargo robocode battle compiles robots and writes recordings`.

### Task 5: Serve the viewer and open the browser

**Files:**
- Create: `cli/src/serve.rs`
- Modify: `cli/src/battle.rs`
- Test: `cli/tests/serve.rs`

**Interfaces:**
- `serve(recording: String) -> !` binds `127.0.0.1:0` (a free port) and prints `Watch the battle: http://127.0.0.1:<port>/?recording=/battle.json   (Ctrl+C to stop)`. It routes `/battle.json` (`application/json`), `/` (index.html), and every embedded asset with its content type (`.js`, `.css`, `.wasm` as `application/wasm`, `.html`); anything else is 404.
- Without `--no-open`, the tool tries `webbrowser::open`. If that fails, it prints `Could not open a browser; open the URL above.` and keeps serving.
- With `--no-open` and no `--output`, it serves without opening a browser.

- [ ] **Step 1: Write a failing test** `serves_viewer_and_recording`: spawn `battle bot.rs --no-open`, read the URL line from stdout, then GET `/battle.json` (valid v1), `/` (contains `<div id="root">`) and one `.wasm` asset (`application/wasm`), then kill the process. Use a plain `std::net::TcpStream` HTTP/1.0 request, with no client crate.
- [ ] **Step 2: Run it.** Expected: FAIL.
- [ ] **Step 3: Implement it** with Axum + Tokio.
- [ ] **Step 4: Run it, then check with headless Chromium** (the Playwright install in the scratchpad) that the served page plays the battle and the console shows no errors.
- [ ] **Step 5: Commit** with the message `feat: cargo robocode serves the viewer with the battle loaded`.

### Task 6: `verify`, CI and docs

**Files:**
- Create: `cli/src/verify.rs`, `docs/cargo-robocode.md`
- Modify: `cli/src/main.rs`, `.github/workflows/ci.yml`
- Test: `cli/tests/verify.rs` (skipped with a printed notice if `cargo verus` isn't on PATH)

**Interfaces:**
- `verify <file>` generates `<build>/verify/` (a library crate):
  - `Cargo.toml` with `robocode = { path, features = ["verus"] }`, `vstd` (same pin as robocode), `[package.metadata.verus] verify = true`, and the lints table from `exercises/Cargo.toml`;
  - `src/lib.rs` = `#![cfg_attr(verus_keep_ghost, feature(proc_macro_hygiene))] mod robot;`;
  - `src/robot.rs` = a **copy** of the file.
- It runs `cargo verus verify` there.
  - If verification fails in that crate, it exits 1 and passes the output through.
  - If it fails in a dependency, it exits 2 with "robocode's own proofs failed; check your robocode/Verus installation".
  - On success it prints `Verified: N specs checked in your robot`. N is the number of functions carrying `verus_spec` or `requires`/`ensures` in the file; 0 gives `0 specs checked in your robot (nothing to prove yet)`.
- No `cargo verus` gives exit 2, pointing to `docs/verus-integration.md`.
- **CI:** the `web` job also builds the CLI after `npm run build`, then runs `cargo test -p cargo-robocode`. The `test` job excludes `cargo-robocode`, since it needs `web/dist`.
- **`docs/cargo-robocode.md`:** install (`cargo install --path cli`), the commands, the robot file rules (`std` + `robocode` only, `robot!`), and the Arena pointer to `docs/roadmap.md`.

- [ ] **Step 1: Write failing tests**:
  - `verify_counts_zero_specs` (template robot → exit 0, "0 specs checked");
  - `verify_proved_strategy` (a small strategy file with one `verus_spec` ensures that holds → exit 0, "1 specs checked");
  - `verify_failing_spec_exits_1` (an ensures that's false).
- [ ] **Step 2: Run them.** Expected: FAIL.
- [ ] **Step 3: Implement the command, the CI change and the docs.**
- [ ] **Step 4: Run the full verification:**
  - workspace tests and golden results;
  - `cargo test -p cargo-robocode`;
  - clippy on the workspace and on `-p robocode`;
  - Verus on the crate and solutions, and the exercises failing only at their TODOs;
  - `web` tests and build;
  - `cargo install --path cli --root <scratch>`, then a real `cargo robocode battle` smoke run.

  Expected: all green.
- [ ] **Step 5: Commit** with the message `feat: cargo robocode verify; CI and docs`.
