use crate::ir::{Grammar, GrammarError, RuleExpr};
use rustc_hash::FxHashSet;
use smol_str::SmolStr;

/// Validate a grammar IR for correctness.
///
/// Checks performed:
/// - Undefined rule references
/// - Cycle detection (rules forming infinite reference loops)
/// - Unreachable rules (never referenced from the entry rule)
/// - Empty alternatives (Choice containing a Blank)
pub fn validate_grammar(grammar: &Grammar) -> Vec<GrammarError> {
    let mut errors = Vec::new();

    // Undefined rule references
    for (name, rule) in &grammar.rules {
        check_undefined_refs(&rule.expr, grammar, name, &mut errors);
    }

    // Empty alternatives
    for (name, rule) in &grammar.rules {
        check_empty_alternatives(&rule.expr, name, &mut errors);
    }

    // Cycle detection
    detect_cycles(grammar, &mut errors);

    // Unreachable rules
    detect_unreachable(grammar, &mut errors);

    errors
}

fn check_undefined_refs(
    expr: &RuleExpr,
    grammar: &Grammar,
    _context_rule: &SmolStr,
    errors: &mut Vec<GrammarError>,
) {
    match expr {
        RuleExpr::RuleRef(name) => {
            if !grammar.rules.contains_key(name.as_str()) && !is_builtin(name) {
                errors.push(GrammarError::UndefinedRule(name.clone()));
            }
        }
        RuleExpr::Seq(exprs) | RuleExpr::Choice(exprs) => {
            for e in exprs {
                check_undefined_refs(e, grammar, _context_rule, errors);
            }
        }
        RuleExpr::Repeat(inner)
        | RuleExpr::Repeat1(inner)
        | RuleExpr::Optional(inner)
        | RuleExpr::Token(inner)
        | RuleExpr::Prec(_, inner)
        | RuleExpr::PrecLeft(_, inner)
        | RuleExpr::PrecRight(_, inner)
        | RuleExpr::Field(_, inner) => {
            check_undefined_refs(inner, grammar, _context_rule, errors);
        }
        RuleExpr::Literal(_) | RuleExpr::Blank => {}
    }
}

fn check_empty_alternatives(
    expr: &RuleExpr,
    context_rule: &SmolStr,
    errors: &mut Vec<GrammarError>,
) {
    match expr {
        RuleExpr::Choice(alts) => {
            for alt in alts {
                if matches!(alt, RuleExpr::Blank) {
                    errors.push(GrammarError::EmptyAlternative(context_rule.clone()));
                    break;
                }
            }
            for alt in alts {
                check_empty_alternatives(alt, context_rule, errors);
            }
        }
        RuleExpr::Seq(exprs) => {
            for e in exprs {
                check_empty_alternatives(e, context_rule, errors);
            }
        }
        RuleExpr::Repeat(inner)
        | RuleExpr::Repeat1(inner)
        | RuleExpr::Optional(inner)
        | RuleExpr::Token(inner)
        | RuleExpr::Prec(_, inner)
        | RuleExpr::PrecLeft(_, inner)
        | RuleExpr::PrecRight(_, inner)
        | RuleExpr::Field(_, inner) => {
            check_empty_alternatives(inner, context_rule, errors);
        }
        RuleExpr::Literal(_) | RuleExpr::RuleRef(_) | RuleExpr::Blank => {}
    }
}

/// Detect cycles using DFS with a "visiting" (gray) / "visited" (black) coloring scheme.
fn detect_cycles(grammar: &Grammar, errors: &mut Vec<GrammarError>) {
    let mut visited = FxHashSet::default();
    let mut visiting = Vec::<SmolStr>::new();
    let mut on_stack = FxHashSet::default();

    for rule_name in grammar.rules.keys() {
        if !visited.contains(rule_name) {
            dfs_cycle(
                rule_name,
                grammar,
                &mut visited,
                &mut visiting,
                &mut on_stack,
                errors,
            );
        }
    }
}

fn dfs_cycle(
    rule_name: &SmolStr,
    grammar: &Grammar,
    visited: &mut FxHashSet<SmolStr>,
    visiting: &mut Vec<SmolStr>,
    on_stack: &mut FxHashSet<SmolStr>,
    errors: &mut Vec<GrammarError>,
) {
    on_stack.insert(rule_name.clone());
    visiting.push(rule_name.clone());

    if let Some(rule) = grammar.rules.get(rule_name) {
        let refs = collect_rule_refs(&rule.expr);
        for dep in &refs {
            if on_stack.contains(dep) {
                let cycle_start = visiting.iter().position(|r| r == dep).unwrap();
                let mut cycle: Vec<SmolStr> = visiting[cycle_start..].to_vec();
                cycle.push(dep.clone());
                errors.push(GrammarError::CycleDetected(cycle));
            } else if !visited.contains(dep) && grammar.rules.contains_key(dep.as_str()) {
                dfs_cycle(dep, grammar, visited, visiting, on_stack, errors);
            }
        }
    }

    visiting.pop();
    on_stack.remove(rule_name);
    visited.insert(rule_name.clone());
}

fn collect_rule_refs(expr: &RuleExpr) -> Vec<SmolStr> {
    let mut refs = Vec::new();
    collect_refs_inner(expr, &mut refs);
    refs
}

fn collect_refs_inner(expr: &RuleExpr, refs: &mut Vec<SmolStr>) {
    match expr {
        RuleExpr::RuleRef(name) => refs.push(name.clone()),
        RuleExpr::Seq(exprs) | RuleExpr::Choice(exprs) => {
            for e in exprs {
                collect_refs_inner(e, refs);
            }
        }
        RuleExpr::Repeat(inner)
        | RuleExpr::Repeat1(inner)
        | RuleExpr::Optional(inner)
        | RuleExpr::Token(inner)
        | RuleExpr::Prec(_, inner)
        | RuleExpr::PrecLeft(_, inner)
        | RuleExpr::PrecRight(_, inner)
        | RuleExpr::Field(_, inner) => {
            collect_refs_inner(inner, refs);
        }
        RuleExpr::Literal(_) | RuleExpr::Blank => {}
    }
}

/// Find rules unreachable from the entry point (first rule in the grammar).
fn detect_unreachable(grammar: &Grammar, errors: &mut Vec<GrammarError>) {
    if grammar.rules.is_empty() {
        return;
    }

    let entry = match grammar.rules.keys().next() {
        Some(k) => k.clone(),
        None => return,
    };

    let mut reachable = FxHashSet::default();
    let mut stack = vec![entry];

    while let Some(name) = stack.pop() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        if let Some(rule) = grammar.rules.get(&name) {
            for dep in collect_rule_refs(&rule.expr) {
                if grammar.rules.contains_key(dep.as_str()) && !reachable.contains(&dep) {
                    stack.push(dep);
                }
            }
        }
    }

    for rule_name in grammar.rules.keys() {
        if !reachable.contains(rule_name) {
            errors.push(GrammarError::UnreachableRule(rule_name.clone()));
        }
    }
}

fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "Identifier" | "Integer" | "Float" | "String" | "Char" | "Boolean"
    )
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
    fn detects_undefined_rule() {
        let g = make_grammar(vec![("A", RuleExpr::RuleRef("B".into()))]);
        let errs = validate_grammar(&g);
        assert!(
            errs.iter()
                .any(|e| matches!(e, GrammarError::UndefinedRule(n) if n == "B"))
        );
    }

    #[test]
    fn allows_builtin_refs() {
        let g = make_grammar(vec![("A", RuleExpr::RuleRef("Identifier".into()))]);
        let errs = validate_grammar(&g);
        assert!(
            !errs
                .iter()
                .any(|e| matches!(e, GrammarError::UndefinedRule(_))),
            "builtins should not be flagged"
        );
    }

    #[test]
    fn detects_cycle() {
        let g = make_grammar(vec![
            ("A", RuleExpr::RuleRef("B".into())),
            ("B", RuleExpr::RuleRef("A".into())),
        ]);
        let errs = validate_grammar(&g);
        assert!(
            errs.iter()
                .any(|e| matches!(e, GrammarError::CycleDetected(_)))
        );
    }

    #[test]
    fn no_false_cycle_on_dag() {
        // A -> B -> C (no cycle)
        let g = make_grammar(vec![
            ("A", RuleExpr::RuleRef("B".into())),
            ("B", RuleExpr::RuleRef("C".into())),
            ("C", RuleExpr::Literal("x".into())),
        ]);
        let errs = validate_grammar(&g);
        assert!(
            !errs
                .iter()
                .any(|e| matches!(e, GrammarError::CycleDetected(_))),
            "DAG should not report cycles"
        );
    }

    #[test]
    fn detects_unreachable_rule() {
        let g = make_grammar(vec![
            ("A", RuleExpr::Literal("x".into())),
            ("B", RuleExpr::Literal("y".into())),
        ]);
        let errs = validate_grammar(&g);
        assert!(
            errs.iter()
                .any(|e| matches!(e, GrammarError::UnreachableRule(n) if n == "B"))
        );
    }

    #[test]
    fn reachable_rules_not_flagged() {
        let g = make_grammar(vec![
            ("A", RuleExpr::RuleRef("B".into())),
            ("B", RuleExpr::Literal("x".into())),
        ]);
        let errs = validate_grammar(&g);
        assert!(
            !errs
                .iter()
                .any(|e| matches!(e, GrammarError::UnreachableRule(_))),
            "all rules reachable from A"
        );
    }

    #[test]
    fn detects_empty_alternative() {
        let g = make_grammar(vec![(
            "A",
            RuleExpr::Choice(vec![RuleExpr::Literal("x".into()), RuleExpr::Blank]),
        )]);
        let errs = validate_grammar(&g);
        assert!(
            errs.iter()
                .any(|e| matches!(e, GrammarError::EmptyAlternative(n) if n == "A"))
        );
    }

    #[test]
    fn no_empty_alternative_without_blank() {
        let g = make_grammar(vec![(
            "A",
            RuleExpr::Choice(vec![
                RuleExpr::Literal("x".into()),
                RuleExpr::Literal("y".into()),
            ]),
        )]);
        let errs = validate_grammar(&g);
        assert!(
            !errs
                .iter()
                .any(|e| matches!(e, GrammarError::EmptyAlternative(_)))
        );
    }
}
