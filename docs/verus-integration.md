# Verus integration

How the `robocode` crate works with [Verus](https://github.com/verus-lang/verus) without depending on it.

## Pinned version

| Item | Value |
|---|---|
| Verus release | `release/0.2026.11.02.8bf68ee` |
| `vstd` crate | `=0.0.0-2026-11-01-0251` (optional dependency) |
| Rust toolchain Verus needs | `1.99.1` (`rustup toolchain install 1.99.1`) |

Change all three together when upgrading.

## Install

macOS (Apple silicon):

```sh
V=0.2026.11.02.8bf68ee
mkdir -p ~/.verus && cd ~/.verus
gh release download release/$V -R verus-lang/verus -p "verus-$V-arm64-macos.zip"
unzip verus-$V-arm64-macos.zip            # creates verus-arm64-macos/
./verus-arm64-macos/macos_allow_gatekeeper.sh   # only if macOS blocks the binaries
export PATH="$HOME/.verus/verus-arm64-macos:$PATH"
rustup toolchain install 1.99.1
verus --version
```

Linux (x86-64): the same, with `verus-$V-x86-linux.zip` and the `verus-x86-linux/` directory. No gatekeeper step is needed.

Without `gh`, download the zip from
`https://github.com/verus-lang/verus/releases/tag/release/0.2026.11.02.8bf68ee`.

## Commands

| What | Command |
|---|---|
| Plain build/test (no Verus crates) | `cargo build` / `cargo test` |
| Check no Verus crate is compiled | `cargo tree -e normal \| grep -Ei 'vstd\|verus'` prints nothing |
| Feature on, plain cargo (must build) | `cargo build --features verus` |
| Verify the crate | `cargo verify` (alias for `cargo verus verify --features verus --target-dir target/verus`) |
| Verify the guide's solutions | `cargo verify-solutions` |
| Exercises (build; verification fails at each `TODO` until solved) | `cargo build -p roboverus-exercises` |

A successful verification ends with a line like `verification results:: N verified, 0 errors` for the `robocode` crate.

## Mechanism

- `vstd` is an **optional** dependency, enabled only by the `verus` feature.
- Specs attach to **ordinary Rust items** with attributes that exist only when the feature is on:

```rust
#[cfg(feature = "verus")]
use vstd::prelude::*;

#[cfg_attr(feature = "verus", verus_verify)]          // on structs/enums used in specs, too
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires p.0 <= 3000,
    ensures r.0 >= 1000 && r.0 <= 1600
))]
pub fn gun_heat_for_power(p: Power) -> GunHeat { GunHeat(1000 + p.0 / 5) }
```

- Without the feature, the attributes vanish and the function is plain Rust. With it, Verus verifies the **same** function body. Functions are written once.
- Every type used by a verified function must also carry `#[cfg_attr(feature = "verus", verus_verify)]`, otherwise Verus reports it as "ignored".
- Spec-only helpers (`spec fn`) and proofs live in `#[cfg(feature = "verus")]` items inside `verus! { … }`, so plain builds never see them.
- **Loops** take their `invariant` / `decreases` in a `verus_spec` attribute on the loop itself:

```rust
#[cfg_attr(verus_only, verus_spec(
    invariant 1 <= i <= targets@.len(),
    decreases targets@.len() - i
))]
while i < targets.len() { /* ... */ }
```

  Attributes on loops need the unstable `proc_macro_hygiene` feature, so `src/lib.rs` has
  `#![cfg_attr(verus_keep_ghost, feature(proc_macro_hygiene))]`. Verus sets `verus_keep_ghost`
  itself, so plain stable builds never see this line. The loop attribute is gated on `verus_only`
  (also set by Verus) rather than on the feature, so `cargo build --features verus` still works
  with plain cargo.
- **Importing spec functions** (`spec fn`, e.g. `well_formed`) needs `#[cfg(verus_only)]` on the
  `use`, because spec functions are erased outside Verus.
- Constants used in specs carry `#[cfg_attr(feature = "verus", verus_verify)]` too.
- Functions that use floats (unit conversions) are left without `verus_verify`, so Verus ignores them.
- `[package.metadata.verus] verify = true` tells `cargo verus` to verify this crate. Plain cargo ignores it.

## Keep Verus's build separate

`cargo verus` and plain `cargo` must not share `target/`: a library built by Verus can shadow the plain build, so a test suddenly can't see an item that plainly exists. The aliases in `.cargo/config.toml` (`cargo verify`, `cargo verify-solutions`, `cargo verify-exercises`) build into `target/verus`. If it happens anyway, `cargo clean -p robocode` fixes it.
