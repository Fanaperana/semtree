use crate::ir::{Grammar, Rule, RuleExpr};
use rustc_hash::FxHashSet;
use smol_str::SmolStr;

/// Optimize a grammar by simplifying its rule expressions.
///
/// Optimizations applied:
/// - Inline small rules (single RuleRef or Literal body)
/// - Flatten nested Seq/Choice
/// - Collapse redundant Optional(Optional(x)) → Optional(x)
/// - Remove Blank from Seq
pub fn optimize(grammar: &Grammar) -> Grammar {
    let inline_candidates = find_inline_candidates(grammar);

    let mut optimized = Grammar::new(grammar.name.clone());
    optimized.keywords = grammar.keywords.clone();
    optimized.extras = grammar.extras.clone();
    optimized.format_hints = grammar.format_hints.clone();

    for (name, rule) in &grammar.rules {
        if inline_candidates.contains(name) {
            continue;
        }
        let expr = optimize_expr(&rule.expr, &inline_candidates, grammar);
        optimized.add_rule(
            name.clone(),
            Rule {
                name: rule.name.clone(),
                expr,
                fields: rule.fields.clone(),
            },
        );
    }

    optimized
}

/// Rules whose body is a single Literal or RuleRef can be inlined at use sites.
fn find_inline_candidates(grammar: &Grammar) -> FxHashSet<SmolStr> {
    let mut candidates = FxHashSet::default();
    // Don't inline the entry rule (first rule).
    let entry = grammar.rules.keys().next().cloned();

    for (name, rule) in &grammar.rules {
        if Some(name) == entry.as_ref() {
            continue;
        }
        match &rule.expr {
            RuleExpr::Literal(_) | RuleExpr::RuleRef(_) => {
                candidates.insert(name.clone());
            }
            _ => {}
        }
    }
    candidates
}

fn optimize_expr(
    expr: &RuleExpr,
    inline_candidates: &FxHashSet<SmolStr>,
    grammar: &Grammar,
) -> RuleExpr {
    match expr {
        RuleExpr::RuleRef(name) => {
            if inline_candidates.contains(name)
                && let Some(rule) = grammar.rules.get(name.as_str())
            {
                return optimize_expr(&rule.expr, inline_candidates, grammar);
            }
            RuleExpr::RuleRef(name.clone())
        }

        RuleExpr::Seq(exprs) => {
            let mut flat = Vec::new();
            for e in exprs {
                let opt = optimize_expr(e, inline_candidates, grammar);
                match opt {
                    RuleExpr::Blank => {}                       // remove Blank from Seq
                    RuleExpr::Seq(inner) => flat.extend(inner), // flatten nested Seq
                    other => flat.push(other),
                }
            }
            match flat.len() {
                0 => RuleExpr::Blank,
                1 => flat.into_iter().next().unwrap(),
                _ => RuleExpr::Seq(flat),
            }
        }

        RuleExpr::Choice(exprs) => {
            let mut flat = Vec::new();
            for e in exprs {
                let opt = optimize_expr(e, inline_candidates, grammar);
                match opt {
                    RuleExpr::Choice(inner) => flat.extend(inner), // flatten nested Choice
                    other => flat.push(other),
                }
            }
            match flat.len() {
                0 => RuleExpr::Blank,
                1 => flat.into_iter().next().unwrap(),
                _ => RuleExpr::Choice(flat),
            }
        }

        RuleExpr::Optional(inner) => {
            let opt = optimize_expr(inner, inline_candidates, grammar);
            match opt {
                RuleExpr::Optional(_) => opt, // Optional(Optional(x)) → Optional(x)
                other => RuleExpr::Optional(Box::new(other)),
            }
        }

        RuleExpr::Repeat(inner) => {
            RuleExpr::Repeat(Box::new(optimize_expr(inner, inline_candidates, grammar)))
        }
        RuleExpr::Repeat1(inner) => {
            RuleExpr::Repeat1(Box::new(optimize_expr(inner, inline_candidates, grammar)))
        }
        RuleExpr::Token(inner) => {
            RuleExpr::Token(Box::new(optimize_expr(inner, inline_candidates, grammar)))
        }
        RuleExpr::Prec(p, inner) => RuleExpr::Prec(
            *p,
            Box::new(optimize_expr(inner, inline_candidates, grammar)),
        ),
        RuleExpr::PrecLeft(p, inner) => RuleExpr::PrecLeft(
            *p,
            Box::new(optimize_expr(inner, inline_candidates, grammar)),
        ),
        RuleExpr::PrecRight(p, inner) => RuleExpr::PrecRight(
            *p,
            Box::new(optimize_expr(inner, inline_candidates, grammar)),
        ),
        RuleExpr::Field(name, inner) => RuleExpr::Field(
            name.clone(),
            Box::new(optimize_expr(inner, inline_candidates, grammar)),
        ),

        RuleExpr::Literal(_) | RuleExpr::Blank => expr.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Rule;

    fn make_grammar(rules: Vec<(&str, RuleExpr)>) -> Grammar {
        let mut g = Grammar::new("test");
        for (name, expr) in rules {
            g.add_rule(
                name,
                Rule {
                    name: name.into(),
                    expr,
                    fields: vec![],
                },
            );
        }
        g
    }

    #[test]
    fn inlines_single_literal_rule() {
        let g = make_grammar(vec![
            (
                "A",
                RuleExpr::Seq(vec![
                    RuleExpr::RuleRef("B".into()),
                    RuleExpr::Literal("x".into()),
                ]),
            ),
            ("B", RuleExpr::Literal("y".into())),
        ]);
        let opt = optimize(&g);
        assert!(!opt.rules.contains_key("B"), "B should be inlined away");
        let a = &opt.rules["A"];
        assert_eq!(
            a.expr,
            RuleExpr::Seq(vec![
                RuleExpr::Literal("y".into()),
                RuleExpr::Literal("x".into()),
            ])
        );
    }

    #[test]
    fn inlines_single_ruleref_rule() {
        let g = make_grammar(vec![
            ("A", RuleExpr::RuleRef("Alias".into())),
            ("Alias", RuleExpr::RuleRef("Target".into())),
            ("Target", RuleExpr::Literal("t".into())),
        ]);
        let opt = optimize(&g);
        // A is entry, kept. Alias and Target are inline candidates.
        assert!(opt.rules.contains_key("A"));
        // A's body should resolve through the alias chain.
        assert_eq!(opt.rules["A"].expr, RuleExpr::Literal("t".into()));
    }

    #[test]
    fn flattens_nested_seq() {
        let g = make_grammar(vec![(
            "A",
            RuleExpr::Seq(vec![
                RuleExpr::Literal("a".into()),
                RuleExpr::Seq(vec![
                    RuleExpr::Literal("b".into()),
                    RuleExpr::Literal("c".into()),
                ]),
            ]),
        )]);
        let opt = optimize(&g);
        assert_eq!(
            opt.rules["A"].expr,
            RuleExpr::Seq(vec![
                RuleExpr::Literal("a".into()),
                RuleExpr::Literal("b".into()),
                RuleExpr::Literal("c".into()),
            ])
        );
    }

    #[test]
    fn flattens_nested_choice() {
        let g = make_grammar(vec![(
            "A",
            RuleExpr::Choice(vec![
                RuleExpr::Literal("a".into()),
                RuleExpr::Choice(vec![
                    RuleExpr::Literal("b".into()),
                    RuleExpr::Literal("c".into()),
                ]),
            ]),
        )]);
        let opt = optimize(&g);
        assert_eq!(
            opt.rules["A"].expr,
            RuleExpr::Choice(vec![
                RuleExpr::Literal("a".into()),
                RuleExpr::Literal("b".into()),
                RuleExpr::Literal("c".into()),
            ])
        );
    }

    #[test]
    fn collapses_double_optional() {
        let g = make_grammar(vec![(
            "A",
            RuleExpr::Optional(Box::new(RuleExpr::Optional(Box::new(RuleExpr::Literal(
                "x".into(),
            ))))),
        )]);
        let opt = optimize(&g);
        assert_eq!(
            opt.rules["A"].expr,
            RuleExpr::Optional(Box::new(RuleExpr::Literal("x".into())))
        );
    }

    #[test]
    fn removes_blank_from_seq() {
        let g = make_grammar(vec![(
            "A",
            RuleExpr::Seq(vec![
                RuleExpr::Literal("a".into()),
                RuleExpr::Blank,
                RuleExpr::Literal("b".into()),
            ]),
        )]);
        let opt = optimize(&g);
        assert_eq!(
            opt.rules["A"].expr,
            RuleExpr::Seq(vec![
                RuleExpr::Literal("a".into()),
                RuleExpr::Literal("b".into()),
            ])
        );
    }

    #[test]
    fn seq_of_one_unwraps() {
        let g = make_grammar(vec![(
            "A",
            RuleExpr::Seq(vec![RuleExpr::Literal("a".into())]),
        )]);
        let opt = optimize(&g);
        assert_eq!(opt.rules["A"].expr, RuleExpr::Literal("a".into()));
    }

    #[test]
    fn preserves_entry_rule() {
        // Even if entry is a single literal, it should not be removed.
        let g = make_grammar(vec![
            ("Entry", RuleExpr::Literal("e".into())),
            ("User", RuleExpr::RuleRef("Entry".into())),
        ]);
        let opt = optimize(&g);
        assert!(opt.rules.contains_key("Entry"), "entry rule must be kept");
    }
}
