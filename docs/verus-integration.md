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
| Verify the crate | `cargo verus verify --features verus` |

A successful verification ends with a line like `verification results:: N verified, 0 errors` for the `robocode` crate.

## Mechanism

- `vstd` is an **optional** dependency, enabled only by the `verus` feature.
- Specs attach to **ordinary Rust items** with attributes that exist only when the feature is on:

```rust
#[cfg(feature = "verus")]
use vstd::prelude::*;

#[cfg_attr(feature = "verus", verus_verify)]          // on structs/enums used in specs, too
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires power.0 <= 3000,
    ensures r.0 >= 1000 && r.0 <= 1600
))]
pub fn gun_heat_for_power(power: Milli) -> Milli { Milli(1000 + power.0 / 5) }
```

- Without the feature, the attributes vanish and the function is plain Rust. With it, Verus verifies the **same** function body. Functions are written once.
- Every type used by a verified function must also carry `#[cfg_attr(feature = "verus", verus_verify)]`, otherwise Verus reports it as "ignored".
- Spec-only helpers (`spec fn`) and proofs live in `#[cfg(feature = "verus")]` items inside `verus! { … }`, so plain builds never see them.
- Functions that use floats (unit conversions) are left without `verus_verify`, so Verus ignores them.
- `[package.metadata.verus] verify = true` tells `cargo verus` to verify this crate. Plain cargo ignores it.
