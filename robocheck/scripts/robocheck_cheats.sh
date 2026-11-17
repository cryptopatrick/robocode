#!/usr/bin/env bash
# Writes robocheck/stored/cheats.json: robocheck's cheating fixtures and the reasons the candidate
# validator refuses them, for the RoboCheck page's "The Cheats" panel. Free, offline: refusal
# happens before anything compiles, so neither Verus nor the Claude API is needed.
#   robocheck/scripts/robocheck_cheats.sh
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
cargo build -q -p robocheck
bin="$root/target/debug/robocheck"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT
cheats=(cheat_assume.rs cheat_cfg.rs cheat_nodecreases.rs)
for f in "${cheats[@]}"; do
  "$bin" run "robocheck/fixtures/$f" --out "$out/$f" > /dev/null
done
python3 - "$root/robocheck" "$out" "${cheats[@]}" <<'PY'
import json, re, sys, os
crate, out, files = sys.argv[1], sys.argv[2], sys.argv[3:]
cheats = []
for f in files:
    lines = []
    for line in open(os.path.join(crate, "fixtures", f)):
        if not line.startswith("//") or line.startswith("// @@"):
            break
        lines.append(line[2:].strip())
        if "Expected:" in line:
            break
    what = " ".join(lines).split("Expected:")[0].strip()
    what = re.sub(r"^Review finding \d+:\s*", "", what)
    what = what[:1].upper() + what[1:]
    ev = json.load(open(os.path.join(out, f, "evidence.json")))
    assert not ev["validation"]["ok"], f"{f} was not refused"
    cheats.append({"file": f, "what": what, "problems": ev["validation"]["problems"]})
json.dump({"cheats": cheats}, open(os.path.join(crate, "stored", "cheats.json"), "w"), indent=2)
print(json.dumps(cheats, indent=2))
PY
