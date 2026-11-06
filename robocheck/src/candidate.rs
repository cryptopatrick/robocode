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

use proc_macro2::{Delimiter, TokenStream, TokenTree};

/// Identifiers a candidate may not use anywhere (as tokens, so spacing and
/// line breaks can't hide them): ways to skip or weaken verification, to make
/// the verified and the battled builds differ (`cfg`), or to reach outside the
/// strategy.
const FORBIDDEN_IDENTS: [&str; 33] = [
    "cfg", "cfg_attr", "verifier", "assume", "admit", "assume_specification", "external", "external_body",
    "exec_allows_no_decreases_clause", "broadcast", "uninterp", "proof", "unsafe", "extern", "include",
    "include_str", "include_bytes", "env", "option_env", "asm", "global_asm", "macro_rules", "std", "core",
    "alloc", "crate", "mod", "use", "impl", "trait", "static", "union", "debug_assertions",
];

/// Keywords that may be followed by `!(…)` without being a macro call.
const NOT_MACROS: [&str; 6] = ["if", "while", "match", "return", "in", "else"];

fn text(t: &TokenTree) -> String {
    t.to_string()
}

/// Checks one attribute body (the tokens inside `#[…]`): it must be exactly
/// `cfg_attr(feature = "verus", verus_verify)` or
/// `cfg_attr(feature = "verus", verus_spec(…))`. Returns the `verus_spec`
/// arguments to scan, or a problem.
fn check_attribute(body: &TokenStream) -> Result<Option<TokenStream>, String> {
    let shown = body.to_string();
    let bad = || format!("attribute not allowed: #[{shown}]");
    let tokens: Vec<TokenTree> = body.clone().into_iter().collect();
    let [TokenTree::Ident(head), TokenTree::Group(args)] = tokens.as_slice() else { return Err(bad()) };
    if head != "cfg_attr" || args.delimiter() != Delimiter::Parenthesis {
        return Err(bad());
    }
    let inner: Vec<TokenTree> = args.stream().into_iter().collect();
    let prefix: Vec<String> = inner.iter().take(4).map(text).collect();
    if prefix != ["feature", "=", "\"verus\"", ","] {
        return Err(bad());
    }
    match &inner[4..] {
        [TokenTree::Ident(i)] if i == "verus_verify" => Ok(None),
        [TokenTree::Ident(i), TokenTree::Group(g)] if i == "verus_spec" && g.delimiter() == Delimiter::Parenthesis => {
            Ok(Some(g.stream()))
        }
        _ => Err(bad()),
    }
}

/// Scans tokens recursively for forbidden identifiers, attributes and macros.
fn scan(ts: TokenStream, problems: &mut Vec<String>) {
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            TokenTree::Punct(p) if p.as_char() == '#' => match tokens.get(i + 1) {
                Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Bracket => {
                    match check_attribute(&g.stream()) {
                        Ok(Some(spec)) => scan(spec, problems),
                        Ok(None) => {}
                        Err(e) => problems.push(e),
                    }
                    i += 2;
                    continue;
                }
                _ => problems.push("only `#[cfg_attr(feature = \"verus\", …)]` attributes are allowed".into()),
            },
            TokenTree::Ident(id) => {
                let name = id.to_string();
                if FORBIDDEN_IDENTS.contains(&name.as_str()) {
                    problems.push(format!("`{name}` is not allowed"));
                }
                if name == "fn" && matches!(tokens.get(i + 1), Some(TokenTree::Ident(n)) if n == "decide") {
                    problems.push("the candidate may not define `decide` itself; the template does".into());
                }
                let bang = matches!(tokens.get(i + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!');
                let call = matches!(tokens.get(i + 2), Some(TokenTree::Group(_)));
                if bang && call && !NOT_MACROS.contains(&name.as_str()) {
                    problems.push(format!("macro `{name}!` is not allowed"));
                }
            }
            TokenTree::Group(g) => scan(g.stream(), problems),
            _ => {}
        }
        i += 1;
    }
}

/// Checks a candidate before anything is compiled: the helpers must parse as
/// plain `fn` items and the body as one block; tokens may not use forbidden
/// identifiers, macros, or any attribute but the two Verus ones. Works on
/// tokens, so whitespace, comments and string contents can't hide anything.
/// Not a sandbox. Returns every problem found.
pub fn validate(c: &Candidate) -> Result<(), Vec<String>> {
    let mut problems = Vec::new();
    match syn::parse_str::<syn::File>(&c.helpers) {
        Ok(file) => {
            if !file.attrs.is_empty() {
                problems.push("inner attributes (`#![…]`) are not allowed".into());
            }
            for item in &file.items {
                if !matches!(item, syn::Item::Fn(_)) {
                    problems.push("helpers may only be `fn` items".into());
                }
            }
        }
        Err(e) => problems.push(format!("helpers are not valid Rust items: {e}")),
    }
    let body = format!("{{\n{}\n}}", c.body);
    if let Err(e) = syn::parse_str::<syn::Block>(&body) {
        problems.push(format!("the body is not a valid block: {e}"));
    }
    for src in [&c.helpers, &body] {
        match src.parse::<TokenStream>() {
            Ok(ts) => scan(ts, &mut problems),
            Err(e) => problems.push(format!("cannot tokenize: {e}")),
        }
    }
    problems.dedup();
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

    /// Review findings 1 and 2: spacing, aliases and cfg must not get past validation.
    #[test]
    fn rejects_spelling_variants_and_cfg() {
        for bad in [
            "#[cfg_attr(feature = \"verus\", verus_spec(ensures false), verifier :: exec_allows_no_decreases_clause)]\nfn s() {}",
            "# [verifier::external_body]\nfn s() {}",
            "#[cfg_attr(feature = \"verus\", verus_spec(), cfg(any()))]\nfn s() {}",
            "#[cfg_attr(feature = \"verus\", verus_verify, inline)]\nfn s() {}",
            "fn s() { assume (false); }",
            "fn s() { let _ = env !(\"HOME\"); }",
            "fn s() { let _ = std :: fs :: read(\"x\"); }",
            "use std::{fs, process};",
            "fn s() -> bool { cfg!(debug_assertions) }",
            "fn s() { if cfg!(feature = \"verus\") {} }",
            "fn s() { let s = \"//\"; std::process::exit(0); }",
            "const MAX_POWER: u32 = 5;",
            "fn s() { println!(\"x\"); }",
        ] {
            let c = parse(&wrap(bad)).unwrap();
            assert!(validate(&c).is_err(), "must be refused: {bad}");
        }
        let c = parse("// @@helpers\n// @@body\n    if cfg!(feature = \"verus\") { Action::default() } else { Action::default() }").unwrap();
        assert!(validate(&c).is_err(), "cfg! in the body");
    }

    #[test]
    fn negation_and_comparisons_are_not_macros() {
        let c = parse(&wrap("fn s(a: bool, b: i32) -> bool { if !(a && b != 0) { !a } else { a } }")).unwrap();
        assert_eq!(validate(&c), Ok(()));
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
        for name in [
            "safe_shooter", "energy_positive", "never_fires", "wall_blind", "type_error", "hangs",
            "cheat_assume", "cheat_cfg", "cheat_nodecreases",
        ] {
            let src = std::fs::read_to_string(dir.join(format!("{name}.rs"))).unwrap();
            let c = parse(&src).unwrap_or_else(|e| panic!("{name}: {e:?}"));
            assert_eq!(validate(&c).is_ok(), !name.starts_with("cheat_"), "{name}: {:?}", validate(&c));
        }
    }
}
