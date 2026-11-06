//! The candidate file: helper functions and the body of `decide`, as an AI
//! would generate them. The spec'd signature of `decide` is never part of it.

/// A parsed candidate: the helper functions and the body of `decide`.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub helpers: String,
    pub body: String,
    /// The whole file, as given.
    pub source: String,
}

const HELPERS: &str = "// @@helpers";
const BODY: &str = "// @@body";

/// Splits a candidate file at its `// @@helpers` and `// @@body` marker lines.
pub fn parse(src: &str) -> Result<Candidate, Vec<String>> {
    let lines: Vec<&str> = src.lines().collect();
    let find = |m: &str| lines.iter().position(|l| l.trim() == m);
    let Some(b) = find(BODY) else {
        return Err(vec![format!("missing the `{BODY}` marker line")]);
    };
    let h = find(HELPERS);
    let start = match h {
        Some(h) if h < b => h + 1,
        Some(_) => return Err(vec![format!("`{HELPERS}` must come before `{BODY}`")]),
        None => 0,
    };
    if let Some(h) = h
        && lines[..h].iter().any(|l| !(l.trim().is_empty() || l.trim_start().starts_with("//")))
    {
        return Err(vec![format!("code before the `{HELPERS}` marker")]);
    }
    let body = lines[b + 1..].join("\n");
    if body.trim().is_empty() {
        return Err(vec!["the body of `decide` is empty".into()]);
    }
    Ok(Candidate { helpers: lines[start..b].join("\n"), body, source: src.to_string() })
}

/// Removes `//` line comments and `/* */` block comments.
fn strip_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match (c, it.peek()) {
            ('/', Some('/')) => {
                for c in it.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                it.next();
                let mut prev = ' ';
                for c in it.by_ref() {
                    if prev == '*' && c == '/' {
                        break;
                    }
                    prev = c;
                }
                out.push(' ');
            }
            _ => out.push(c),
        }
    }
    out
}

/// Text that may not appear anywhere in a candidate (outside comments): ways
/// to skip or weaken verification, or to reach outside the strategy.
const FORBIDDEN: [&str; 14] = [
    "assume(", "admit(", "external", "verifier::", "unsafe", "include", "env!", "std::fs", "std::net",
    "std::process", "extern", "asm!", "macro_rules", "#![",
];

/// The only attributes a candidate may use.
const ALLOWED_ATTRIBUTES: [&str; 2] =
    ["#[cfg_attr(feature = \"verus\", verus_verify)]", "#[cfg_attr(feature = \"verus\", verus_spec("];

/// Checks a candidate for constructs that could weaken verification or escape
/// the strategy. A denylist, not a sandbox. Returns every problem found.
pub fn validate(c: &Candidate) -> Result<(), Vec<String>> {
    let code = strip_comments(&format!("{}\n{}", c.helpers, c.body));
    let mut problems: Vec<String> =
        FORBIDDEN.iter().filter(|f| code.contains(*f)).map(|f| format!("`{f}` is not allowed")).collect();
    for (i, _) in code.match_indices("#[") {
        if !ALLOWED_ATTRIBUTES.iter().any(|a| code[i..].starts_with(a)) {
            let attr: String = code[i..].chars().take_while(|&c| c != '\n').collect();
            problems.push(format!("attribute not allowed: {attr}"));
        }
    }
    let words: Vec<&str> =
        code.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty()).collect();
    if words.contains(&"mod") {
        problems.push("`mod` is not allowed".into());
    }
    if words.windows(2).any(|w| w == ["fn", "decide"]) {
        problems.push("the candidate may not define `decide` itself; the template does".into());
    }
    if problems.is_empty() { Ok(()) } else { Err(problems) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrap(helpers: &str) -> String {
        format!("// @@helpers\n{helpers}\n// @@body\nAction::default()")
    }

    #[test]
    fn splits_helpers_and_body() {
        let c = parse("// @@helpers\nfn h() {}\n// @@body\nAction::default()").unwrap();
        assert!(c.helpers.contains("fn h"));
        assert_eq!(c.body.trim(), "Action::default()");
    }

    #[test]
    fn code_before_helpers_is_an_error() {
        assert!(parse("fn x() {}\n// @@helpers\n// @@body\nAction::default()").unwrap_err()[0].contains("before"));
        assert!(parse("// a header comment\n// @@helpers\n// @@body\nAction::default()").is_ok());
    }

    #[test]
    fn missing_markers_is_an_error() {
        assert!(parse("Action::default()").unwrap_err()[0].contains("@@body"));
    }

    #[test]
    fn rejects_escape_hatches() {
        for bad in [
            "assume(false);",
            "admit();",
            "#[verifier::external_body]",
            "#[verifier::exec_allows_no_decreases_clause]",
            "unsafe { }",
            "include_str!(\"x\")",
            "std::fs::read(\"x\")",
            "std::process::Command",
            "std::net::TcpStream",
            "extern \"C\" {}",
            "mod m;",
            "#[path = \"x\"]",
            "macro_rules! m {}",
            "pub fn decide(obs: &Observation) -> Action",
            "#![allow(x)]",
        ] {
            let c = parse(&wrap(bad)).unwrap();
            assert!(validate(&c).is_err(), "{bad} must be refused");
        }
    }

    #[test]
    fn allows_verus_attributes_on_helpers() {
        let c = parse(&wrap(
            "#[cfg_attr(feature = \"verus\", verus_verify)]\n#[cfg_attr(feature = \"verus\", verus_spec(r => ensures r > 0))]\nfn h() -> i32 { 1 }",
        ))
        .unwrap();
        assert_eq!(validate(&c), Ok(()));
    }

    #[test]
    fn comments_are_ignored() {
        let c = parse(&wrap("// assume(false) is not allowed\n/* unsafe */ fn h() {}")).unwrap();
        assert_eq!(validate(&c), Ok(()));
    }

    #[test]
    fn fixtures_validate_as_expected() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
        for name in ["safe_shooter", "energy_positive", "never_fires", "wall_blind", "type_error", "cheat_assume"] {
            let src = std::fs::read_to_string(dir.join(format!("{name}.rs"))).unwrap();
            let c = parse(&src).unwrap_or_else(|e| panic!("{name}: {e:?}"));
            assert_eq!(validate(&c).is_ok(), name != "cheat_assume", "{name}: {:?}", validate(&c));
        }
    }
}
