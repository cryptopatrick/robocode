# Stored starter-rule runs

One real `robocheck rules` run per subset of the starter rules S1–S3, made by `robocheck/scripts/robocheck_stored.sh`
and kept exactly as they came out: no run was regenerated to get a better result. The RoboCheck page shows these
instantly, and falls back to them when the live service is down or out of budget. `manifest.json` records when,
with which model, robocode and Verus, and what the 8 runs cost.

Each folder holds `run.json` (what the page shows), `evidence.json`, the AI's `candidate.rs`, the assembled
`strategy.rs`, the Verus logs and the two battle recordings, gzipped (`recording-open.json.gz`,
`recording-urban.json.gz`). The per-turn decision logs are not kept; their result is in `run.json`.

## As they came out (2026-11-19, claude-sonnet-5-5, robocode 0.7.0, realistic bullets; $0.26 for all 8)

Made with the prompt that asks for a robot that keeps only the rules in force and is otherwise as aggressive as it
can be, so a deleted rule shows up in battle.

| Run | Outcome | Deleted starter rules in battle |
|---|---|---|
| `s1-s2-s3` | unknown | none deleted. No rule proved: the AI put steering in a helper (`clamp_turn`) Verus could not use |
| `s1-s2` | proved | S3 broken in both battles |
| `s1-s3` | proved | S2 broken in both battles |
| `s2-s3` | proved | S1 never broken |
| `s1` | unknown | S1 not proved: the code calls a helper (`clamp_to`) Verus could not use. S2 and S3 broken in both battles |
| `s2` | proved | S1 and S3 never broken |
| `s3` | proved | S1 broken once in the urban battle, S2 in both |
| `none` | no rules in force | nothing broken: the AI's robot does nothing at all |

The previous set (2026-11-17, robocode 0.5.0, arcade bullets, $0.28) is in git history.

Regenerate all 8 together with `robocheck/scripts/robocheck_stored.sh --replace-all`.
