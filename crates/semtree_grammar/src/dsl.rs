use crate::ir::{FieldDef, Grammar, GrammarError, Rule, RuleExpr};
use smol_str::SmolStr;

/// Parse a SemTree DSL grammar definition into the Grammar IR.
///
/// Example input:
/// ```text
/// language rust
///
/// keyword fn
/// keyword let
/// keyword struct
///
/// Function :=
///     "fn"
///     name: Identifier
///     Parameters
///     Block
///
/// Parameters :=
///     "(" (Parameter ("," Parameter)*)? ")"
///
/// Parameter :=
///     name: Identifier ":" type: TypeRef
/// ```
pub fn parse_semtree_dsl(input: &str) -> Result<Grammar, GrammarError> {
    let mut parser = DslParser::new(input);
    parser.parse()
}

struct DslParser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
}

impl<'a> DslParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            lines: input.lines().collect(),
            pos: 0,
        }
    }

    fn parse(&mut self) -> Result<Grammar, GrammarError> {
        let mut grammar = Grammar::new("unnamed");

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos].trim();

            if line.is_empty() || line.starts_with('#') {
                self.pos += 1;
                continue;
            }

            if let Some(name) = line.strip_prefix("language ") {
                grammar.name = name.trim().into();
                self.pos += 1;
            } else if let Some(kw) = line.strip_prefix("keyword ") {
                grammar.add_keyword(kw.trim());
                self.pos += 1;
            } else if line == "indent-sensitive" {
                grammar.indent_sensitive = true;
                self.pos += 1;
            } else if let Some(rest) = line.strip_prefix("token ") {
                self.parse_token_def(&mut grammar, rest.trim());
            } else if let Some(extra) = line.strip_prefix("extra ") {
                grammar.extras.push(extra.trim().into());
                self.pos += 1;
            } else if line.starts_with("indent ")
                || line.starts_with("linebreak ")
                || line.starts_with("space ")
            {
                self.parse_format_hint(&mut grammar);
            } else if line.contains(":=") {
                self.parse_rule(&mut grammar)?;
            } else {
                self.pos += 1;
            }
        }

        Ok(grammar)
    }

    fn parse_rule(&mut self, grammar: &mut Grammar) -> Result<(), GrammarError> {
        let line = self.lines[self.pos].trim();
        let parts: Vec<&str> = line.splitn(2, ":=").collect();
        let rule_name: SmolStr = parts[0].trim().into();

        self.pos += 1;

        let mut fields = Vec::new();

        // Check for inline body on the same line as :=
        let inline_body = parts.get(1).map(|s| s.trim()).unwrap_or("");

        // Collect all indented body lines into a single string, joining continuation
        // lines (those starting with `|`) with the previous line.
        let mut body_text = if inline_body.is_empty() {
            String::new()
        } else {
            inline_body.to_string()
        };
        while self.pos < self.lines.len() {
            let body_line = self.lines[self.pos];
            if body_line.is_empty() {
                self.pos += 1;
                break;
            }
            if !body_line.starts_with(' ') && !body_line.starts_with('\t') {
                break;
            }

            let body = body_line.trim();
            if body.is_empty() {
                self.pos += 1;
                break;
            }

            if !body_text.is_empty() {
                body_text.push(' ');
            }
            body_text.push_str(body);
            self.pos += 1;
        }

        let expr = if body_text.is_empty() {
            RuleExpr::Blank
        } else {
            self.parse_expr_line(&body_text, &mut fields)
        };

        let rule = Rule {
            name: rule_name.clone(),
            expr,
            fields,
        };

        if grammar.rules.contains_key(&rule_name) {
            return Err(GrammarError::DuplicateRule(rule_name));
        }
        if grammar.entry_rule.is_none() {
            grammar.entry_rule = Some(rule_name.clone());
        }
        grammar.add_rule(rule_name, rule);
        Ok(())
    }

    fn parse_expr_line(&self, line: &str, fields: &mut Vec<FieldDef>) -> RuleExpr {
        let tokens = tokenize_dsl_line(line);
        if tokens.is_empty() {
            return RuleExpr::Blank;
        }

        let mut exprs = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            let tok = &tokens[i];

            // field: Rule pattern
            if i + 2 < tokens.len() && tokens[i + 1] == ":" {
                let field_name: SmolStr = tok.as_str().into();
                let rule_token = tokens[i + 2].as_str();
                // Handle modifiers on the field's rule reference (e.g. Expression?, Statement*, Item+)
                let inner_expr = if rule_token.ends_with('?') && rule_token.len() > 1 {
                    let inner = &rule_token[..rule_token.len() - 1];
                    RuleExpr::Optional(Box::new(RuleExpr::RuleRef(inner.into())))
                } else if rule_token.ends_with('*') && rule_token.len() > 1 {
                    let inner = &rule_token[..rule_token.len() - 1];
                    RuleExpr::Repeat(Box::new(RuleExpr::RuleRef(inner.into())))
                } else if rule_token.ends_with('+') && rule_token.len() > 1 {
                    let inner = &rule_token[..rule_token.len() - 1];
                    RuleExpr::Repeat1(Box::new(RuleExpr::RuleRef(inner.into())))
                } else {
                    RuleExpr::RuleRef(rule_token.into())
                };
                fields.push(FieldDef {
                    name: field_name.clone(),
                    rule: match &inner_expr {
                        RuleExpr::Optional(b) | RuleExpr::Repeat(b) | RuleExpr::Repeat1(b) => {
                            if let RuleExpr::RuleRef(r) = b.as_ref() {
                                r.clone()
                            } else {
                                rule_token.into()
                            }
                        }
                        RuleExpr::RuleRef(r) => r.clone(),
                        _ => rule_token.into(),
                    },
                });
                exprs.push(RuleExpr::Field(
                    field_name,
                    Box::new(inner_expr),
                ));
                i += 3;
                continue;
            }

            if tok.starts_with('"') && tok.ends_with('"') && tok.len() >= 2 {
                let literal = &tok[1..tok.len() - 1];
                let lit_expr = RuleExpr::Literal(literal.into());
                // Check for postfix modifier on literal: ","? or ";"* etc.
                if i + 1 < tokens.len() {
                    match tokens[i + 1].as_str() {
                        "?" => {
                            exprs.push(RuleExpr::Optional(Box::new(lit_expr)));
                            i += 2;
                            continue;
                        }
                        "*" => {
                            exprs.push(RuleExpr::Repeat(Box::new(lit_expr)));
                            i += 2;
                            continue;
                        }
                        "+" => {
                            exprs.push(RuleExpr::Repeat1(Box::new(lit_expr)));
                            i += 2;
                            continue;
                        }
                        _ => {}
                    }
                }
                exprs.push(lit_expr);
            } else if tok.ends_with('*') && tok.len() > 1 {
                let inner = &tok[..tok.len() - 1];
                exprs.push(RuleExpr::Repeat(Box::new(RuleExpr::RuleRef(inner.into()))));
            } else if tok.ends_with('+') && tok.len() > 1 {
                let inner = &tok[..tok.len() - 1];
                exprs.push(RuleExpr::Repeat1(Box::new(RuleExpr::RuleRef(inner.into()))));
            } else if tok.ends_with('?') && tok.len() > 1 {
                let inner = &tok[..tok.len() - 1];
                exprs.push(RuleExpr::Optional(Box::new(RuleExpr::RuleRef(
                    inner.into(),
                ))));
            } else if tok == "|" {
                // Choice: collect remaining into a separate branch
                let left = if exprs.len() == 1 {
                    exprs.pop().unwrap()
                } else {
                    RuleExpr::Seq(std::mem::take(&mut exprs))
                };
                let rest_tokens = &tokens[i + 1..];
                let rest_line = rest_tokens.join(" ");
                let right = self.parse_expr_line(&rest_line, fields);
                // Flatten nested Choice to produce a flat list of alternatives.
                let mut alts = vec![left];
                match right {
                    RuleExpr::Choice(inner_alts) => alts.extend(inner_alts),
                    other => alts.push(other),
                }
                return RuleExpr::Choice(alts);
            } else {
                exprs.push(RuleExpr::RuleRef(tok.as_str().into()));
            }
            i += 1;
        }

        if exprs.len() == 1 {
            exprs.into_iter().next().unwrap()
        } else {
            RuleExpr::Seq(exprs)
        }
    }

    fn parse_format_hint(&mut self, grammar: &mut Grammar) {
        use crate::ir::FormatHint;
        let line = self.lines[self.pos].trim();
        if let Some(rule) = line.strip_prefix("indent ") {
            grammar
                .format_hints
                .push(FormatHint::Indent(rule.trim().into()));
        } else if let Some(rule) = line.strip_prefix("linebreak ") {
            grammar
                .format_hints
                .push(FormatHint::Linebreak(rule.trim().into()));
        } else if let Some(rest) = line.strip_prefix("space around ") {
            grammar.format_hints.push(FormatHint::SpaceAround(
                rest.trim().trim_matches('"').into(),
            ));
        } else if let Some(rest) = line.strip_prefix("space before ") {
            grammar.format_hints.push(FormatHint::SpaceBefore(
                rest.trim().trim_matches('"').into(),
            ));
        } else if let Some(rest) = line.strip_prefix("space after ") {
            grammar
                .format_hints
                .push(FormatHint::SpaceAfter(rest.trim().trim_matches('"').into()));
        }
        self.pos += 1;
    }

    fn parse_token_def(&mut self, grammar: &mut Grammar, rest: &str) {
        // token Name := /regex/  OR  token Name := "literal"
        let Some((name_part, pattern_part)) = rest.split_once(":=") else {
            self.pos += 1;
            return;
        };
        let name: SmolStr = name_part.trim().into();
        let pattern_part = pattern_part.trim();

        let (pattern, is_regex) = if let Some(inner) = pattern_part
            .strip_prefix('/')
            .and_then(|s| s.strip_suffix('/'))
        {
            (inner.into(), true)
        } else if let Some(lit) = pattern_part
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
        {
            (lit.into(), false)
        } else {
            (pattern_part.into(), false)
        };

        grammar.tokens.push(crate::ir::TokenDef {
            name,
            pattern,
            is_regex,
        });
        self.pos += 1;
    }
}

fn tokenize_dsl_line(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = line.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }

        if c == '"' {
            let mut s = String::new();
            s.push(chars.next().unwrap());
            while let Some(&ch) = chars.peek() {
                s.push(chars.next().unwrap());
                if ch == '"' && s.len() > 1 {
                    break;
                }
            }
            tokens.push(s);
        } else if c == ':' || c == '|' || c == '(' || c == ')' {
            tokens.push(chars.next().unwrap().to_string());
        } else {
            let mut s = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_whitespace() || ch == ':' || ch == '|' || ch == '(' || ch == ')' {
                    break;
                }
                s.push(chars.next().unwrap());
            }
            tokens.push(s);
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_grammar() {
        let input = r#"
language rust

keyword fn
keyword let
keyword struct

Function :=
    "fn"
    name: Identifier
    Parameters
    Block

Parameters :=
    "(" ")"
"#;
        let grammar = parse_semtree_dsl(input).unwrap();
        assert_eq!(grammar.name.as_str(), "rust");
        assert_eq!(grammar.keywords.len(), 3);
        assert!(grammar.rules.contains_key("Function"));
        assert!(grammar.rules.contains_key("Parameters"));

        let func = &grammar.rules["Function"];
        assert_eq!(func.fields.len(), 1);
        assert_eq!(func.fields[0].name.as_str(), "name");
    }

    #[test]
    fn parse_format_hints() {
        let input = r#"
language test

indent Block
linebreak Function
space around "+"

Rule :=
    "x"
"#;
        let grammar = parse_semtree_dsl(input).unwrap();
        assert_eq!(grammar.format_hints.len(), 3);
    }

    #[test]
    fn literal_optional_modifier() {
        let input = r#"
language test

Rule :=
    "{" Item* ","? "}"
"#;
        let grammar = parse_semtree_dsl(input).unwrap();
        let rule = grammar.rules.get("Rule").unwrap();
        match &rule.expr {
            RuleExpr::Seq(parts) => {
                // Should have 4 parts: "{", Item*, ","?, "}"
                assert_eq!(parts.len(), 4, "expected 4 seq parts, got {:?}", parts);
                assert!(
                    matches!(&parts[2], RuleExpr::Optional(inner)
                        if matches!(inner.as_ref(), RuleExpr::Literal(s) if s == ",")),
                    "expected Optional(Literal(\",\")), got: {:?}",
                    parts[2]
                );
            }
            other => panic!("expected Seq, got: {other:?}"),
        }
    }

    #[test]
    fn choice_is_flat() {
        let input = r#"
language test

Rule :=
    A | B | C | D
"#;
        let grammar = parse_semtree_dsl(input).unwrap();
        let rule = grammar.rules.get("Rule").unwrap();
        match &rule.expr {
            RuleExpr::Choice(alts) => {
                assert_eq!(alts.len(), 4, "should be flat 4-way choice, got {:?}", alts);
                for alt in alts {
                    assert!(matches!(alt, RuleExpr::RuleRef(_)), "all alts should be RuleRef");
                }
            }
            other => panic!("expected Choice, got: {other:?}"),
        }
    }

    #[test]
    fn multi_line_choice_is_flat() {
        let input = r#"
language test

Rule :=
    A | B | C
    | D | E
"#;
        let grammar = parse_semtree_dsl(input).unwrap();
        let rule = grammar.rules.get("Rule").unwrap();
        match &rule.expr {
            RuleExpr::Choice(alts) => {
                assert_eq!(alts.len(), 5, "should be flat 5-way choice, got {:?}", alts);
            }
            other => panic!("expected Choice, got: {other:?}"),
        }
    }
}
