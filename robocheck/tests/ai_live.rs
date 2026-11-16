//! Live checks against the Claude API (cost about a cent each). Run with
//! `cargo test -p robocheck --test ai_live -- --ignored --nocapture` (needs ANTHROPIC_API_KEY).

use robocheck::ai::{AiCost, RuleOutcome, formalise};
use robocheck::llm::ClaudeClient;
use robocheck::rulespec::{render_verus, starter_rules};

#[test]
#[ignore = "calls the Claude API"]
fn starter_rules_formalise_live() {
    let mut llm = ClaudeClient::from_env().expect("ANTHROPIC_API_KEY");
    let mut cost = AiCost::default();
    let out = formalise(&mut llm, &starter_rules(), &mut cost)
        .expect("the API accepts the schema and replies");
    for o in &out {
        match o {
            RuleOutcome::Formalised { id, clauses } => {
                for c in clauses {
                    println!("{id}: {}", render_verus(c));
                }
            }
            RuleOutcome::NotFormalised { id, kind, reason } => {
                println!("{id}: not formalised ({kind:?}): {reason}")
            }
        }
    }
    println!("cost: ${:.4} over {} call(s)", cost.usd, cost.calls);
    assert!(
        out.iter().all(|o| matches!(o, RuleOutcome::Formalised { .. })),
        "every starter rule should formalise"
    );
}
