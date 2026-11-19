#!/usr/bin/env bash
# Makes RoboCheck's stored starter-rule runs: one real `robocheck rules` run per
# subset of S1–S3 (8 runs), kept as they came out, into robocheck/stored/.
#   robocheck/scripts/robocheck_stored.sh               # refuses if stored runs exist
#   robocheck/scripts/robocheck_stored.sh --replace-all # regenerates all 8 together
# Needs ANTHROPIC_API_KEY and Verus. Costs real money (each run makes 3–4 Claude calls).
# Runs are never regenerated one at a time, so no combination is re-rolled until it looks better.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
dest="$root/robocheck/stored"
combos=(s1-s2-s3 s1-s2 s1-s3 s2-s3 s1 s2 s3 none)

if [ -e "$dest/manifest.json" ] && [ "${1:-}" != "--replace-all" ]; then
  echo "stored runs already exist in $dest; use --replace-all to regenerate all 8" >&2
  exit 2
fi
[ -n "${ANTHROPIC_API_KEY:-}" ] || { echo "ANTHROPIC_API_KEY is not set" >&2; exit 2; }

cd "$root"
cargo build -q -p robocheck
bin="$root/target/debug/robocheck"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

for combo in "${combos[@]}"; do
  ids="$(echo "$combo" | tr 'a-z-' 'A-Z ')"
  [ "$combo" = none ] && ids=""
  python3 - "$root/robocheck/starter_rules.json" "$stage/$combo.rules.json" $ids <<'PY'
import json, sys
starters = json.load(open(sys.argv[1]))["rules"]
keep = sys.argv[3:]
json.dump({"rules": [{"id": r["id"], "text": r["text"]} for r in starters if r["id"] in keep]}, open(sys.argv[2], "w"))
PY
  echo "== $combo"
  "$bin" rules "$stage/$combo.rules.json" --stored --out "$stage/raw/$combo"
  mkdir -p "$stage/out/$combo"
  for f in run.json evidence.json candidate.rs strategy.rs build.log; do
    [ -e "$stage/raw/$combo/$f" ] && cp "$stage/raw/$combo/$f" "$stage/out/$combo/"
  done
  cp "$stage/raw/$combo"/verus-*.log "$stage/out/$combo/" 2>/dev/null || true
  for rec in "$stage/raw/$combo"/recording-*.json; do
    [ -e "$rec" ] && gzip -9 -c "$rec" > "$stage/out/$combo/$(basename "$rec").gz"
  done
done

python3 - "$stage/out" <<'PY'
import json, sys, os, datetime
out = sys.argv[1]
first = json.load(open(os.path.join(out, "s1-s2-s3", "run.json")))
ev = json.load(open(os.path.join(out, "s1-s2-s3", "evidence.json")))
combos = ["s1-s2-s3", "s1-s2", "s1-s3", "s2-s3", "s1", "s2", "s3", "none"]
manifest = {
    "created_at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "model": first["model"],
    "robocode": first["robocode"],
    "verus": ev["toolchain"]["verus"],
    "cost_usd": round(sum(json.load(open(os.path.join(out, c, "run.json")))["cost_usd"] for c in combos), 4),
    "runs": {c: f"{c}/run.json" for c in combos},
}
json.dump(manifest, open(os.path.join(out, "manifest.json"), "w"), indent=2)
print(f"total cost: ${manifest['cost_usd']}")
PY
# Keep the files this script does not make (README, the stored cheat refusals).
for keep in README.md cheats.json; do
  cp "$dest/$keep" "$stage/out/" 2>/dev/null || true
done
rm -rf "$dest"
mv "$stage/out" "$dest"
echo "stored runs written to $dest"
