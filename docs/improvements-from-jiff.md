# Improvements from Jiff

Ideas taken from [Jiff](https://github.com/BurntSushi/jiff), a mature Rust
datetime crate, when comparing its practices with ours. Each item says where it
stands. Items marked *open* are not done yet.

| # | Improvement | Status |
|---|---|---|
| 1 | Run the test suite in release mode too (a `testrelease` profile with debug assertions off), so results don't depend on the build profile | done on `inajiff` (186 tests pass under it) |
| 2 | Enforce `cargo fmt --check` in CI; format the code to the existing `rustfmt.toml` | done on `inajiff`; the mutant for `fires-with-hot-gun` was retargeted because formatting split its line |
| 3 | Build the docs in CI with `-D rustdoc::broken_intra_doc_links` | done on `inajiff`; two links to Verus-only spec functions became code spans |
| 4 | Run `cargo package` in CI, so publish-file mistakes are caught before a release | done on `inajiff` |
| 5 | Test on macOS as well as Linux | done on `inajiff` (a macOS job runs the non-Verus tests) |
| 6 | Property tests for simulator invariants (positions stay on the arena, energy only falls except through bonuses, no NaN, same inputs give the same frames), with a seeded generator | done: `tests/properties.rs` (quickcheck 1.1.0, pinned); 1,500 random battles pass locally |
| 7 | Snapshot tests that show the first differing turn when a battle changes | done: `tests/snapshots.rs` (insta 1.49.0, pinned); turn-by-turn traces and a three-round scores snapshot |
| 8 | Fuzz targets for untrusted input, starting with RoboCheck's candidate checker | open |
| 9 | Declare a minimum supported Rust version (`rust-version`) and test it in CI | done: 1.85, as Jiff; a CI job tests it; let-chains rewritten to work on 1.85 |
| 10 | Generated files are regenerated in CI and must match what's committed (the lock-template test does this for one file) | partly done; open for other generated files |
| 11 | A design FAQ (`DESIGN.md`) explaining the choices, with opinions marked as opinions | done: `DESIGN.md` |
| 12 | A public AI and contribution policy, as Jiff has | done: `AI_POLICY.md` (adapted from Jiff's, with attribution) and `CONTRIBUTING.md` |
| 13 | Changelog notes generated from commit messages (git-cliff) | done: `cliff.toml`, `scripts/changelog`, and a release recipe in `CONTRIBUTING.md` |
| 14 | Benchmarks run as tests in CI, so they don't rot | done: `benches/battle.rs` (turns per second; no new dependency); CI runs it in short form |

Not needed: Miri (we have no `unsafe` code of our own) and a no_std core split.
