# Roadmap

Ideas agreed for later, with the decisions already made about them.

## RoboCode Arena (later)

An online arena where users pit their robots against other users' robots.

- Each arena accepts **up to 8 robots** and opens in **timed slots**.
- **The arena runs no user code.** It accepts only robot **profiles**: JSON in a small, fixed robot language (states with prioritised condition → action rules, with capped size).
- **What "Verus verified" means here:** a single trusted interpreter runs every profile, and Verus proves it once for *every* well-formed profile. It never panics or overflows, each turn finishes in bounded time, it never fires unsafely (fire rules only take effect when `safe_to_fire` holds), and it is deterministic. The arena only validates the JSON against the schema and limits. No uploaded proof is checked on the server: that would mean running a Rust-based prover on untrusted input, and a client-side "verified" flag can be forged.
- **Locally**, `cargo robocode` checks profiles and battles them with the same interpreter, so local battles match the arena. Learners may prove extra properties of their own profile locally; the arena doesn't need to trust those.
- **Trade-off accepted:** arena robots can do only what the profile language expresses. Full Rust robots stay local.
- **Order:** local `cargo robocode` tool first, then the profile language and verified interpreter, then the Arena website.
