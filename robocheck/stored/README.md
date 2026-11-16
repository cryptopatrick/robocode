# Stored starter-rule runs

One real `robocheck rules` run per subset of the starter rules S1–S3, made by `robocheck/scripts/robocheck_stored.sh`
and kept exactly as they came out: no run was regenerated to get a better result. The RoboCheck page shows these
instantly, and falls back to them when the live service is down or out of budget. `manifest.json` records when,
with which model, robocode and Verus, and what the 8 runs cost.

Each folder holds `run.json` (what the page shows), `evidence.json`, the AI's `candidate.rs`, the assembled
`strategy.rs`, the Verus logs and the two battle recordings, gzipped (`recording-open.json.gz`,
`recording-urban.json.gz`). The per-turn decision logs are not kept; their result is in `run.json`.

## As they came out (2026-11-17, claude-sonnet-5-5, robocode 0.5.0; $0.28 for all 8)

Made with the prompt that asks for a robot that keeps only the rules in force and is otherwise as aggressive as it
can be, so a deleted rule shows up in battle.

| Run | Outcome | Deleted starter rules in battle |
|---|---|---|
| `s1-s2-s3` | proved | none deleted |
| `s1-s2` | proved | S3 broken in both battles |
| `s1-s3` | proved | S2 broken in both battles |
| `s2-s3` | proved | S1 never broken |
| `s1` | unknown | S1 not proved: the AI put the firing logic in a helper Verus could not use. S2 and S3 broken in both battles |
| `s2` | proved | S3 broken in both battles; S1 never broken |
| `s3` | proved | S1 broken in the open arena, S2 in both |
| `none` | no rules in force | S2 and S3 broken in both battles; S1 never broken |

Regenerate all 8 together with `robocheck/scripts/robocheck_stored.sh --replace-all`.
