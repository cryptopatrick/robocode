# Stored starter-rule runs

One real `robocheck rules` run per subset of the starter rules S1–S3, made by `robocheck/scripts/robocheck_stored.sh`
and kept exactly as they came out: no run was regenerated to get a better result. The RoboCheck page shows these
instantly, and falls back to them when the live service is down or out of budget. `manifest.json` records when,
with which model, robocode and Verus, and what the 8 runs cost.

Each folder holds `run.json` (what the page shows), `evidence.json`, the AI's `candidate.rs`, the assembled
`strategy.rs`, the Verus logs and the two battle recordings, gzipped (`recording-open.json.gz`,
`recording-urban.json.gz`). The per-turn decision logs are not kept; their result is in `run.json`.

## As they came out (2026-11-16, claude-sonnet-5-5, robocode 0.5.0; $0.28 for all 8)

| Run | Outcome | Notes |
|---|---|---|
| `s1-s2-s3` | proved | all three rules proved |
| `s1-s2` | proved | S3 deleted; the robot never broke it in either battle |
| `s1-s3` | proved | S2 deleted; never broken |
| `s2-s3` | proved | S1 deleted; never broken |
| `s1` | proved | S2 and S3 deleted; never broken |
| `s2` | unknown | Verus could not rule out an arithmetic overflow in the AI's robot, so S2 is not proved (and not rejected) |
| `s3` | proved | S1 and S2 deleted; never broken |
| `none` | no rules in force | the robot still fought; it never broke any deleted starter rule |

Regenerate all 8 together with `robocheck/scripts/robocheck_stored.sh --replace-all`.
