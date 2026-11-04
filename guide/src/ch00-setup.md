# Setup

*Mirrors Verus guide: "Getting Started".*

## Install Verus

RoboVerus is pinned to one Verus release; `docs/verus-integration.md` in the repository has the exact version. On macOS with Apple silicon:

```sh
V=0.2026.11.02.8bf68ee
mkdir -p ~/.verus && cd ~/.verus
gh release download release/$V -R verus-lang/verus -p "verus-$V-arm64-macos.zip"
unzip verus-$V-arm64-macos.zip
export PATH="$HOME/.verus/verus-arm64-macos:$PATH"   # add this to your shell profile
rustup toolchain install 1.99.1
verus --version
```

On Linux use `verus-$V-x86-linux.zip` and `verus-x86-linux/`. If macOS refuses to run the binaries, run `./verus-arm64-macos/macos_allow_gatekeeper.sh` once.

## Your first verified RoboCode function

From the repository root, verify the crate:

```sh
cargo verus verify --features verus
```

You should see a line like:

```text
verification results:: 21 verified, 0 errors
```

One of the functions Verus just checked is in `src/units.rs`:

```rust
#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires p.0 <= 3000,
    ensures r.0 >= 1000 && r.0 <= 1600,
))]
pub fn gun_heat_for_power(p: Power) -> GunHeat {
    GunHeat(1000 + p.0 / 5)
}
```

It says that for any legal bullet power, firing heats the gun by between 1.0 and 1.6, and Verus proved it for **every** possible input, not just a few test values.

### Why the `cfg_attr`?

The crate must work for people who have never heard of Verus. Every Verus annotation is wrapped in `cfg_attr(feature = "verus", …)`, so with a plain `cargo build` it disappears and the function is ordinary Rust. In the exercises you'll write specs in the Verus guide's own `verus! { … }` syntax instead. It means the same thing.

## See a failure

Change `r.0 >= 1000` to `r.0 >= 1001` and verify again:

```text
error: postcondition not satisfied
verification results:: 20 verified, 1 errors
```

Verus found that `p = 0` breaks the claim. Change it back.

## The exercises

```sh
cargo build -p roboverus-exercises            # always builds
cargo verus verify -p roboverus-exercises     # fails until you solve the TODOs
```
