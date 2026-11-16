# Stored starter-rule runs

One real `robocheck rules` run per subset of the starter rules S1–S3, made by `robocheck/scripts/robocheck_stored.sh`
and kept exactly as they came out: no run was regenerated to get a better result. The RoboCheck page shows these
instantly, and falls back to them when the live service is down or out of budget. `manifest.json` records when,
with which model, robocode and Verus, and what the 8 runs cost.

Each folder holds `run.json` (what the page shows), `evidence.json`, the AI's `candidate.rs`, the assembled
`strategy.rs`, the Verus logs and the two battle recordings, gzipped (`recording-open.json.gz`,
`recording-urban.json.gz`). The per-turn decision logs are not kept; their result is in `run.json`.

## As they came out (2026-11-16, claude-sonnet-5-5, robocode 0.5.0; $0.40 for all 8)

Made with the prompt that asks for a robot that keeps only the rules in force and is otherwise as aggressive as it
can be, so a deleted rule shows up in battle.

| Run | Outcome | Notes |
|---|---|---|
| `s1-s2-s3` | proved | all three rules proved |
| `s1-s2` | unknown | Verus could neither prove nor reject S1 and S2; S3 deleted and broken in both battles (from turn 2) |
| `s1-s3` | proved | S2 deleted and broken in both battles (from turn 0) |
| `s2-s3` | proved | S1 deleted; never broken |
| `s1` | proved | S2 and S3 deleted; both broken in both battles |
| `s2` | proved | S1 and S3 deleted; never broken |
| `s3` | refused | the AI returned an empty body for `decide`, so the robot was refused before it ran |
| `none` | generate failed | the AI's reply was cut off at the 16,000-token limit ($0.17 of the $0.40) |

Regenerate all 8 together with `robocheck/scripts/robocheck_stored.sh --replace-all`.
