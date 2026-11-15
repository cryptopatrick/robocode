# Improvements from Jiff

Ideas taken from [Jiff](https://github.com/BurntSushi/jiff), a mature Rust
datetime crate, when comparing its practices with ours. Each item says where it
stands. Items marked *decision* need a choice from the maintainer before work.

| # | Improvement | Status |
|---|---|---|
| 1 | Run the test suite in release mode too (a `testrelease` profile with debug assertions off), so results don't depend on the build profile | done on `inajiff` (186 tests pass under it) |
| 2 | Enforce `cargo fmt --check` in CI; format the code to the existing `rustfmt.toml` | done on `inajiff`; the mutant for `fires-with-hot-gun` was retargeted because formatting split its line |
| 3 | Build the docs in CI with `-D rustdoc::broken_intra_doc_links` | done on `inajiff`; two links to Verus-only spec functions became code spans |
| 4 | Run `cargo package` in CI, so publish-file mistakes are caught before a release | done on `inajiff` |
| 5 | Test on macOS as well as Linux | done on `inajiff` (a macOS job runs the non-Verus tests) |
| 6 | Property tests for simulator invariants (positions stay on the arena, energy only falls except through bonuses, no NaN, same inputs give the same frames), with a seeded generator | *decision*: a quickcheck dev-dependency, or a hand-rolled generator with no new dependency |
| 7 | Snapshot tests that show the first differing turn when a battle changes | *decision*: insta as a dev-dependency |
| 8 | Fuzz targets for untrusted input, starting with RoboCheck's candidate checker | not started |
| 9 | Declare a minimum supported Rust version (`rust-version`) and test it in CI | *decision*: which version to promise |
| 10 | Generated files are regenerated in CI and must match what's committed (the lock-template test does this for one file) | partly done; extend to any new generated file |
| 11 | A design FAQ (`DESIGN.md`) explaining the choices, with opinions marked as opinions | not started |
| 12 | A public AI and contribution policy, as Jiff has | *decision* |
| 13 | Changelog notes generated from commit messages (git-cliff) | *decision* |
| 14 | Benchmarks run as tests in CI, so they don't rot | not started (no benchmarks yet) |

Not needed: Miri (we have no `unsafe` code of our own) and a no_std core split.
