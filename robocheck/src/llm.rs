//! The Claude client: one Messages API call per AI step, over raw HTTPS
//! (there is no official Rust SDK), with refusal fallback and a cost figure
//! taken from the reply's own token usage.

use std::time::Duration;

use serde_json::{Value, json};

/// The model every AI step uses.
pub const MODEL: &str = "claude-sonnet-5-5";
const URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2026-10-21";
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
const TIMEOUT: Duration = Duration::from_secs(120);

/// USD per million tokens: (model, input, output, cache read). Unknown models
/// are priced at the dearest row, so nothing is ever counted as free.
const PRICES: [(&str, f64, f64, f64); 2] = [
    ("claude-sonnet-5-5", 2.0, 10.0, 0.10),
    ("claude-sonnet-5", 2.0, 10.0, 0.20),
];

/// One request: a system prompt, one user message, and optionally a JSON
/// schema the reply must follow.
#[derive(Debug, Clone, PartialEq)]
pub struct LlmRequest {
    pub system: String,
    pub user: String,
    pub schema: Option<Value>,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
}

/// The reply's text (all text blocks joined), its usage, the model that
/// produced it, and what it cost.
#[derive(Debug, Clone, PartialEq)]
pub struct LlmReply {
    pub text: String,
    pub usage: Usage,
    pub served_by: String,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LlmError {
    NoKey,
    Http(u16, String),
    Network(String),
    /// The model declined (`stop_reason: refusal`), with its category. Still billed.
    Refused {
        category: Option<String>,
        cost_usd: f64,
    },
    /// The reply hit `max_tokens`. Still billed.
    Truncated {
        cost_usd: f64,
    },
    BadReply(String),
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::NoKey => f.write_str("ANTHROPIC_API_KEY is not set"),
            LlmError::Http(code, body) => {
                write!(f, "Claude API returned HTTP {code}: {body}")
            }
            LlmError::Network(e) => {
                write!(f, "could not reach the Claude API: {e}")
            }
            LlmError::Refused { category, .. } => {
                write!(
                    f,
                    "the model declined the request ({})",
                    category.as_deref().unwrap_or("no category")
                )
            }
            LlmError::Truncated { .. } => {
                f.write_str("the reply was cut off at max_tokens")
            }
            LlmError::BadReply(e) => {
                write!(f, "unexpected reply from the Claude API: {e}")
            }
        }
    }
}

impl LlmError {
    /// What the failed call was billed (refusals and truncations are; the rest never reached the model).
    pub fn cost_usd(&self) -> f64 {
        match self {
            LlmError::Refused { cost_usd, .. }
            | LlmError::Truncated { cost_usd } => *cost_usd,
            _ => 0.0,
        }
    }
}

/// Anything that answers an [`LlmRequest`]: the real client or a test double.
pub trait Llm {
    fn complete(&mut self, req: &LlmRequest) -> Result<LlmReply, LlmError>;
}

/// USD for `u` tokens on `model`.
pub fn cost_usd(model: &str, u: &Usage) -> f64 {
    let dearest = PRICES
        .iter()
        .copied()
        .fold((0.0_f64, 0.0_f64, 0.0_f64), |a, (_, i, o, c)| {
            (a.0.max(i), a.1.max(o), a.2.max(c))
        });
    let (i, o, c) = PRICES
        .iter()
        .find(|(m, ..)| *m == model)
        .map(|(_, i, o, c)| (*i, *o, *c))
        .unwrap_or(dearest);
    (u.input_tokens as f64 * i
        + u.output_tokens as f64 * o
        + u.cache_read_input_tokens as f64 * c)
        / 1e6
}

/// The Messages API request body for `req`.
pub fn request_body(model: &str, req: &LlmRequest) -> Value {
    let mut output_config = json!({ "effort": "medium" });
    if let Some(schema) = &req.schema {
        output_config["format"] =
            json!({ "type": "json_schema", "schema": schema });
    }
    json!({
        "model": model,
        "max_tokens": req.max_tokens,
        "system": req.system,
        "messages": [{ "role": "user", "content": req.user }],
        "output_config": output_config,
        "fallbacks": "default",
    })
}

/// The request headers (the key included; never logged).
pub fn request_headers(key: &str) -> Vec<(&'static str, String)> {
    vec![
        ("x-api-key", key.to_string()),
        ("anthropic-version", API_VERSION.to_string()),
        ("anthropic-beta", FALLBACK_BETA.to_string()),
        ("content-type", "application/json".to_string()),
    ]
}

fn usage_of(v: &Value) -> Usage {
    let n = |k: &str| v.get(k).and_then(Value::as_u64).unwrap_or(0);
    Usage {
        input_tokens: n("input_tokens"),
        output_tokens: n("output_tokens"),
        cache_read_input_tokens: n("cache_read_input_tokens"),
    }
}

/// Reads a Messages API response.
pub fn parse_reply(v: &Value) -> Result<LlmReply, LlmError> {
    let content = v
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| LlmError::BadReply("no content array".into()))?;
    let served_by =
        v.get("model").and_then(Value::as_str).unwrap_or(MODEL).to_string();
    let top = v.get("usage").cloned().unwrap_or(Value::Null);
    let usage = usage_of(&top);
    let cost = match top.get("iterations").and_then(Value::as_array) {
        Some(its) if !its.is_empty() => its
            .iter()
            .map(|it| {
                cost_usd(
                    it.get("model")
                        .and_then(Value::as_str)
                        .unwrap_or(&served_by),
                    &usage_of(it),
                )
            })
            .sum(),
        _ => cost_usd(&served_by, &usage),
    };
    match v.get("stop_reason").and_then(Value::as_str) {
        Some("refusal") => {
            let category = v
                .pointer("/stop_details/category")
                .and_then(Value::as_str)
                .map(str::to_string);
            return Err(LlmError::Refused { category, cost_usd: cost });
        }
        Some("max_tokens") => {
            return Err(LlmError::Truncated { cost_usd: cost });
        }
        _ => {}
    }
    let text: String = content
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|b| b.get("text").and_then(Value::as_str))
        .collect();
    Ok(LlmReply { text, usage, served_by, cost_usd: cost })
}

/// The real client. Its `Debug` never shows the key.
pub struct ClaudeClient {
    key: String,
    agent: ureq::Agent,
}

impl std::fmt::Debug for ClaudeClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeClient")
            .field("model", &MODEL)
            .field("key", &"<redacted>")
            .finish()
    }
}

impl ClaudeClient {
    /// From `ANTHROPIC_API_KEY`.
    pub fn from_env() -> Result<Self, LlmError> {
        Self::from_key(std::env::var("ANTHROPIC_API_KEY").ok())
    }

    pub fn from_key(key: Option<String>) -> Result<Self, LlmError> {
        match key {
            Some(k) if !k.trim().is_empty() => Ok(Self::with_key(k.trim())),
            _ => Err(LlmError::NoKey),
        }
    }

    pub fn with_key(key: &str) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .http_status_as_error(false)
            .build()
            .into();
        ClaudeClient { key: key.to_string(), agent }
    }

    /// One HTTP attempt: `Ok(Err(wait))` means "retry after `wait`".
    fn attempt(
        &self,
        body: &Value,
    ) -> Result<Result<Value, Duration>, LlmError> {
        let mut req = self.agent.post(URL);
        for (k, v) in request_headers(&self.key) {
            req = req.header(k, &v);
        }
        let mut resp = match req.send_json(body) {
            Ok(r) => r,
            Err(e) if is_transient(&e) => {
                return Ok(Err(Duration::from_secs(2)));
            }
            Err(e) => return Err(LlmError::Network(e.to_string())),
        };
        let status = resp.status().as_u16();
        let wait = resp
            .headers()
            .get("retry-after")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map_or(Duration::from_secs(2), |s| {
                Duration::from_secs(s.min(30))
            });
        let text = resp
            .body_mut()
            .read_to_string()
            .map_err(|e| LlmError::Network(e.to_string()))?;
        if status == 429 || status >= 500 {
            return Ok(Err(wait));
        }
        if status != 200 {
            return Err(LlmError::Http(
                status,
                text.chars().take(500).collect(),
            ));
        }
        serde_json::from_str(&text)
            .map(Ok)
            .map_err(|e| LlmError::BadReply(e.to_string()))
    }
}

fn is_transient(e: &ureq::Error) -> bool {
    matches!(
        e,
        ureq::Error::Io(_)
            | ureq::Error::Timeout(_)
            | ureq::Error::ConnectionFailed
            | ureq::Error::HostNotFound
    )
}

impl Llm for ClaudeClient {
    fn complete(&mut self, req: &LlmRequest) -> Result<LlmReply, LlmError> {
        let body = request_body(MODEL, req);
        match self.attempt(&body)? {
            Ok(v) => parse_reply(&v),
            Err(wait) => {
                std::thread::sleep(wait);
                match self.attempt(&body)? {
                    Ok(v) => parse_reply(&v),
                    Err(_) => Err(LlmError::Http(
                        429,
                        "still unavailable after one retry".into(),
                    )),
                }
            }
        }
    }
}

/// A test double: answers with queued replies (text, or an error) and records
/// every request. Each text reply costs nothing.
#[derive(Debug, Default)]
pub struct ScriptedLlm {
    replies: std::collections::VecDeque<Result<String, LlmError>>,
    pub requests: Vec<LlmRequest>,
}

impl ScriptedLlm {
    pub fn new(replies: Vec<Result<String, LlmError>>) -> Self {
        ScriptedLlm { replies: replies.into(), requests: Vec::new() }
    }
}

impl Llm for ScriptedLlm {
    fn complete(&mut self, req: &LlmRequest) -> Result<LlmReply, LlmError> {
        self.requests.push(req.clone());
        let next = self.replies.pop_front().unwrap_or_else(|| {
            Err(LlmError::BadReply("no scripted reply left".into()))
        });
        next.map(|text| LlmReply {
            text,
            usage: Usage::default(),
            served_by: MODEL.to_string(),
            cost_usd: 0.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request() -> LlmRequest {
        LlmRequest {
            system: "You formalise rules.".into(),
            user: "Rule S1: never fire a shot you can't afford.".into(),
            schema: Some(
                json!({"type": "object", "properties": {"ok": {"type": "boolean"}}, "required": ["ok"], "additionalProperties": false}),
            ),
            max_tokens: 16000,
        }
    }

    #[test]
    fn builds_request_body() {
        assert_eq!(
            request_body(MODEL, &request()),
            json!({
                "model": "claude-sonnet-5-5",
                "max_tokens": 16000,
                "system": "You formalise rules.",
                "messages": [{"role": "user", "content": "Rule S1: never fire a shot you can't afford."}],
                "output_config": {
                    "effort": "medium",
                    "format": {"type": "json_schema", "schema": {"type": "object", "properties": {"ok": {"type": "boolean"}}, "required": ["ok"], "additionalProperties": false}}
                },
                "fallbacks": "default"
            })
        );
        let plain = LlmRequest { schema: None, ..request() };
        assert_eq!(
            request_body(MODEL, &plain)["output_config"],
            json!({"effort": "medium"})
        );
        let headers = request_headers("sk-test");
        for (k, v) in [
            ("x-api-key", "sk-test"),
            ("anthropic-version", "2026-10-21"),
            ("anthropic-beta", "server-side-fallback-2026-07-01"),
            ("content-type", "application/json"),
        ] {
            assert!(
                headers.contains(&(k, v.to_string())),
                "missing header {k}"
            );
        }
    }

    #[test]
    fn parses_text_and_usage() {
        let reply = parse_reply(&json!({
            "model": "claude-sonnet-5-5",
            "stop_reason": "end_turn",
            "content": [
                {"type": "thinking", "thinking": ""},
                {"type": "text", "text": "{\"ok\":"},
                {"type": "text", "text": "true}"}
            ],
            "usage": {"input_tokens": 1200, "output_tokens": 300, "cache_read_input_tokens": 0}
        }))
        .unwrap();
        assert_eq!(reply.text, "{\"ok\":true}");
        assert_eq!(
            reply.usage,
            Usage {
                input_tokens: 1200,
                output_tokens: 300,
                cache_read_input_tokens: 0
            }
        );
        assert_eq!(reply.served_by, "claude-sonnet-5-5");
        assert!(
            (reply.cost_usd - (1200.0 * 2.0 + 300.0 * 10.0) / 1e6).abs()
                < 1e-12
        );
    }

    #[test]
    fn refusal_is_an_error() {
        let refused = json!({
            "model": "claude-sonnet-5-5", "stop_reason": "refusal",
            "stop_details": {"type": "refusal", "category": "cyber", "explanation": "…"},
            "content": [], "usage": {"input_tokens": 10, "output_tokens": 0}
        });
        assert!(
            matches!(parse_reply(&refused).unwrap_err(), LlmError::Refused { category: Some(c), .. } if c == "cyber")
        );
        let truncated = json!({
            "model": "claude-sonnet-5-5", "stop_reason": "max_tokens",
            "content": [{"type": "text", "text": "{\"ok\":"}], "usage": {"input_tokens": 10, "output_tokens": 16000}
        });
        assert!(matches!(
            parse_reply(&truncated).unwrap_err(),
            LlmError::Truncated { .. }
        ));
        assert!(matches!(
            parse_reply(&json!({"content": "nope"})),
            Err(LlmError::BadReply(_))
        ));
    }

    #[test]
    fn refused_and_truncated_replies_still_cost() {
        let refused = json!({
            "model": "claude-sonnet-5-5", "stop_reason": "refusal", "stop_details": {"category": "cyber"},
            "content": [], "usage": {"input_tokens": 1000, "output_tokens": 0}
        });
        assert!(
            (parse_reply(&refused).unwrap_err().cost_usd() - 0.002).abs()
                < 1e-12
        );
        let truncated = json!({
            "model": "claude-sonnet-5-5", "stop_reason": "max_tokens",
            "content": [], "usage": {"input_tokens": 0, "output_tokens": 16000}
        });
        assert!(
            (parse_reply(&truncated).unwrap_err().cost_usd() - 0.16).abs()
                < 1e-12
        );
        assert_eq!(LlmError::NoKey.cost_usd(), 0.0);
    }

    #[test]
    fn cost_of_one_million_each_is_12_dollars() {
        let u = Usage {
            input_tokens: 1_000_000,
            output_tokens: 1_000_000,
            cache_read_input_tokens: 0,
        };
        assert!((cost_usd("claude-sonnet-5-5", &u) - 12.0).abs() < 1e-9);
        let cached = Usage {
            input_tokens: 0,
            output_tokens: 0,
            cache_read_input_tokens: 1_000_000,
        };
        assert!((cost_usd("claude-sonnet-5-5", &cached) - 0.10).abs() < 1e-9);
    }

    #[test]
    fn cost_sums_fallback_iterations() {
        let reply = parse_reply(&json!({
            "model": "claude-sonnet-5", "stop_reason": "end_turn",
            "content": [{"type": "fallback", "from": {"model": "claude-sonnet-5-5"}, "to": {"model": "claude-sonnet-5"}},
                        {"type": "text", "text": "ok"}],
            "usage": {"input_tokens": 2000, "output_tokens": 100,
                "iterations": [
                    {"type": "message", "model": "claude-sonnet-5-5", "input_tokens": 1000, "output_tokens": 40},
                    {"type": "fallback_message", "model": "claude-sonnet-5", "input_tokens": 1000, "output_tokens": 60}
                ]}
        }))
        .unwrap();
        let want =
            (1000.0 * 2.0 + 40.0 * 10.0 + 1000.0 * 2.0 + 60.0 * 10.0) / 1e6;
        assert!(
            (reply.cost_usd - want).abs() < 1e-12,
            "{} vs {want}",
            reply.cost_usd
        );
        assert_eq!(reply.served_by, "claude-sonnet-5");
        assert_eq!(reply.text, "ok");
    }

    #[test]
    fn unknown_model_is_priced_at_the_dearest_known_rate() {
        // A model missing from the table must never be counted as free against the budget.
        let u = Usage {
            input_tokens: 1_000_000,
            output_tokens: 0,
            cache_read_input_tokens: 0,
        };
        assert!(
            cost_usd("claude-something-new", &u)
                >= cost_usd("claude-sonnet-5-5", &u)
        );
    }

    #[test]
    fn key_never_in_debug_output() {
        let client = ClaudeClient::with_key("sk-ant-secret-123");
        assert!(!format!("{client:?}").contains("sk-ant-secret-123"));
    }

    #[test]
    fn missing_key_is_no_key() {
        assert!(matches!(ClaudeClient::from_key(None), Err(LlmError::NoKey)));
        assert!(matches!(
            ClaudeClient::from_key(Some(String::new())),
            Err(LlmError::NoKey)
        ));
    }

    #[test]
    fn scripted_llm_replays_and_records() {
        let mut llm = ScriptedLlm::new(vec![
            Ok("first".into()),
            Err(LlmError::Truncated { cost_usd: 0.0 }),
        ]);
        assert_eq!(llm.complete(&request()).unwrap().text, "first");
        assert_eq!(
            llm.complete(&request()).unwrap_err(),
            LlmError::Truncated { cost_usd: 0.0 }
        );
        assert_eq!(llm.requests.len(), 2);
        assert_eq!(llm.requests[0].user, request().user);
    }
}
