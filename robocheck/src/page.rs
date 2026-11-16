//! `run.json`: everything the RoboCheck page shows about one run, in one file.

use serde::Serialize;

use crate::ai::NotFormalisedKind;
use crate::verify::FailedAt;

/// The page's one input (format `robocheck-run`, version 1).
#[derive(Debug, Clone, Serialize)]
pub struct RunPage {
    pub format: &'static str,
    pub version: u32,
    pub run_id: String,
    pub created_at: String,
    /// `stored` (precomputed starter-rule runs) or `live`.
    pub source: String,
    pub model: &'static str,
    pub robocode: String,
    pub robocheck_arena: &'static str,
    pub cost_usd: f64,
    /// `proved`, `rejected`, `unknown`, `timed_out`, `build_failed`, `refused`,
    /// `formalise_failed`, `generate_failed` or `no_rules_in_force`.
    pub outcome: &'static str,
    /// Why the run stopped early, for the failure outcomes.
    pub outcome_note: Option<String>,
    pub rules: Vec<RuleEntry>,
    /// Starter rules the visitor deleted, checked in the battles with their reference formalisation.
    pub deleted_starters: Vec<DeletedStarter>,
    pub code: Code,
    pub explanation: Option<ExplanationEntry>,
    pub battles: Battles,
    /// Every battle ran the exact `strategy.rs` that was verified.
    pub same_artifact: bool,
    pub evidence: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleEntry {
    pub id: String,
    pub text: String,
    /// `proved`, `rejected`, `unknown`, `not_formalised` or `not_checked`.
    pub status: &'static str,
    pub read_back: Option<String>,
    /// The rule's Verus clauses, as rendered into the template.
    pub clauses: Vec<String>,
    pub not_formalised: Option<NotFormalised>,
    pub failed_at: Option<FailedAt>,
    /// The AI's reading of why the code can break a rejected rule.
    pub why: Option<String>,
    pub violations: ArenaViolations,
    /// `yes` (a battle broke it), `rejected_not_triggered`, or `n/a`.
    pub triggered: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct NotFormalised {
    #[serde(flatten)]
    pub kind: NotFormalisedKind,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ArenaViolations {
    pub open: Option<Broken>,
    pub urban: Option<Broken>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Broken {
    pub first_turn: u32,
    pub turns: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeletedStarter {
    pub id: String,
    pub text: String,
    pub violations: ArenaViolations,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Code {
    /// The assembled `strategy.rs` (trusted header, spec, the AI's robot).
    pub strategy_rs: Option<String>,
    /// The AI's candidate file as it came back.
    pub candidate: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExplanationEntry {
    pub headline: String,
    pub summary: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Battles {
    pub open: Option<BattleEntry>,
    pub urban: Option<BattleEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleEntry {
    /// `ok`, `failed` or `timed_out`.
    pub status: &'static str,
    pub recording: Option<String>,
    pub decisions: Option<String>,
    pub winner: Option<String>,
    pub turns: Option<u64>,
    pub error: Option<String>,
}
