use crate::ir::{Grammar, GrammarError, RuleExpr};
use smol_str::SmolStr;

/// Validate a grammar IR for correctness.
pub fn validate_grammar(grammar: &Grammar) -> Vec<GrammarError> {
    let mut errors = Vec::new();

    for (name, rule) in &grammar.rules {
        validate_expr(&rule.expr, grammar, name, &mut errors);
    }

    errors
}

fn validate_expr(
    expr: &RuleExpr,
    grammar: &Grammar,
    context_rule: &SmolStr,
    errors: &mut Vec<GrammarError>,
) {
    match expr {
        RuleExpr::RuleRef(name) => {
            if !grammar.rules.contains_key(name.as_str())
                && !is_builtin(name)
            {
                errors.push(GrammarError::UndefinedRule(name.clone()));
            }
        }
        RuleExpr::Seq(exprs) | RuleExpr::Choice(exprs) => {
            for e in exprs {
                validate_expr(e, grammar, context_rule, errors);
            }
        }
        RuleExpr::Repeat(inner)
        | RuleExpr::Repeat1(inner)
        | RuleExpr::Optional(inner)
        | RuleExpr::Token(inner)
        | RuleExpr::Prec(_, inner)
        | RuleExpr::PrecLeft(_, inner)
        | RuleExpr::PrecRight(_, inner) => {
            validate_expr(inner, grammar, context_rule, errors);
        }
        RuleExpr::Field(_, inner) => {
            validate_expr(inner, grammar, context_rule, errors);
        }
        RuleExpr::Literal(_) | RuleExpr::Blank => {}
    }
}

fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "Identifier" | "Integer" | "Float" | "String" | "Char" | "Boolean"
    )
}
