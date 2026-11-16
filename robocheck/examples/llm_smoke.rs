//! One real call to the Claude API through robocheck's client, to check the
//! request shape, the JSON-schema reply and the cost figure. Needs
//! `ANTHROPIC_API_KEY`; costs well under one cent.
//!
//!     cargo run -p robocheck --example llm_smoke

use robocheck::llm::{ClaudeClient, Llm, LlmRequest};
use serde_json::json;

fn main() {
    let mut client = match ClaudeClient::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let req = LlmRequest {
        system: "Answer in JSON only.".into(),
        user: "Is 7 a prime number?".into(),
        schema: Some(json!({
            "type": "object",
            "properties": { "prime": { "type": "boolean" } },
            "required": ["prime"],
            "additionalProperties": false
        })),
        max_tokens: 16000,
    };
    match client.complete(&req) {
        Ok(r) => {
            println!("reply: {}", r.text);
            println!("served by: {}", r.served_by);
            println!("usage: {} in, {} out", r.usage.input_tokens, r.usage.output_tokens);
            println!("cost: ${:.5}", r.cost_usd);
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
