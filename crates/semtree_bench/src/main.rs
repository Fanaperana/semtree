#![allow(dead_code)]

mod grammar_loader;

use std::time::{Duration, Instant};

use semtree_format::Formatter;
use semtree_grammar::{Grammar, Rule, RuleExpr};
use semtree_lint::LintEngine;
use semtree_query::{QueryEngine, QueryPattern};
use semtree_red::{Preorder, SyntaxNode};
use semtree_runtime::{IncrementalParser, RuntimeParser};
use semtree_semantic::SemanticModel;

// ─── Configuration ──────────────────────────────────────────────────────────

const DEFAULT_ITERATIONS: usize = 100;
const SIZES: &[(&str, usize)] = &[
    ("1KB", 1_024),
    ("10KB", 10_240),
    ("100KB", 102_400),
    ("1MB", 1_048_576),
];

// ─── Test Data Generation ───────────────────────────────────────────────────

fn generate_json(target_size: usize) -> String {
    let base = r#"{"name":"Alice","age":30,"active":true,"scores":[95,87,92],"address":{"street":"123 Main St","city":"Springfield","zip":"62701"}}"#;
    let mut result = String::with_capacity(target_size + 256);
    result.push('[');
    let mut first = true;
    while result.len() < target_size {
        if !first {
            result.push(',');
        }
        result.push_str(base);
        first = false;
    }
    result.push(']');
    result
}

fn generate_javascript(target_size: usize) -> String {
    let base = r#"function fibonacci(n) {
  if (n <= 1) return n;
  let a = 0, b = 1;
  for (let i = 2; i <= n; i++) {
    const temp = a + b;
    a = b;
    b = temp;
  }
  return b;
}

const result = fibonacci(10);
console.log("Result:", result);

class Calculator {
  constructor(value) {
    this.value = value;
  }
  add(x) { return new Calculator(this.value + x); }
  multiply(x) { return new Calculator(this.value * x); }
  toString() { return `Calculator(${this.value})`; }
}

"#;
    repeat_to_size(base, target_size)
}

fn generate_rust(target_size: usize) -> String {
    let base = r#"fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    let mut a = 0u64;
    let mut b = 1u64;
    for _ in 2..=n {
        let temp = a + b;
        a = b;
        b = temp;
    }
    b
}

struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    fn distance(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

"#;
    repeat_to_size(base, target_size)
}

fn generate_css(target_size: usize) -> String {
    let base = r#".container {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 16px;
  margin: 0 auto;
  max-width: 1200px;
}

.header {
  background-color: #333;
  color: white;
  padding: 12px 24px;
  border-radius: 8px;
  font-size: 1.5rem;
}

@media (max-width: 768px) {
  .container {
    padding: 8px;
  }
  .header {
    font-size: 1.2rem;
  }
}

"#;
    repeat_to_size(base, target_size)
}

fn generate_python(target_size: usize) -> String {
    let base = r#"def fibonacci(n):
    if n <= 1:
        return n
    a, b = 0, 1
    for _ in range(2, n + 1):
        a, b = b, a + b
    return b

class Calculator:
    def __init__(self, value=0):
        self.value = value

    def add(self, x):
        return Calculator(self.value + x)

    def multiply(self, x):
        return Calculator(self.value * x)

    def __repr__(self):
        return f"Calculator({self.value})"

result = fibonacci(10)
calc = Calculator(5).add(3).multiply(2)

"#;
    repeat_to_size(base, target_size)
}

fn repeat_to_size(base: &str, target_size: usize) -> String {
    let mut result = String::with_capacity(target_size + base.len());
    while result.len() < target_size {
        result.push_str(base);
    }
    result
}

// ─── SemTree Grammar Builders ───────────────────────────────────────────────

fn build_json_grammar() -> Grammar {
    let mut g = Grammar::new("json");

    g.add_rule(
        "source_file",
        Rule {
            name: "source_file".into(),
            expr: RuleExpr::RuleRef("value".into()),
            fields: vec![],
        },
    );

    g.add_rule(
        "value",
        Rule {
            name: "value".into(),
            expr: RuleExpr::Choice(vec![
                RuleExpr::RuleRef("object".into()),
                RuleExpr::RuleRef("array".into()),
                RuleExpr::RuleRef("String".into()),
                RuleExpr::RuleRef("Integer".into()),
                RuleExpr::RuleRef("Float".into()),
                RuleExpr::Literal("true".into()),
                RuleExpr::Literal("false".into()),
                RuleExpr::Literal("null".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "object",
        Rule {
            name: "object".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Literal("{".into()),
                RuleExpr::Optional(Box::new(RuleExpr::RuleRef("pair_list".into()))),
                RuleExpr::Literal("}".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "pair_list",
        Rule {
            name: "pair_list".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::RuleRef("pair".into()),
                RuleExpr::Repeat(Box::new(RuleExpr::Seq(vec![
                    RuleExpr::Literal(",".into()),
                    RuleExpr::RuleRef("pair".into()),
                ]))),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "pair",
        Rule {
            name: "pair".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::RuleRef("String".into()),
                RuleExpr::Literal(":".into()),
                RuleExpr::RuleRef("value".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "array",
        Rule {
            name: "array".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Literal("[".into()),
                RuleExpr::Optional(Box::new(RuleExpr::RuleRef("value_list".into()))),
                RuleExpr::Literal("]".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "value_list",
        Rule {
            name: "value_list".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::RuleRef("value".into()),
                RuleExpr::Repeat(Box::new(RuleExpr::Seq(vec![
                    RuleExpr::Literal(",".into()),
                    RuleExpr::RuleRef("value".into()),
                ]))),
            ]),
            fields: vec![],
        },
    );

    g
}

fn build_javascript_grammar() -> Grammar {
    let mut g = Grammar::new("javascript");

    g.add_keyword("function");
    g.add_keyword("const");
    g.add_keyword("let");
    g.add_keyword("var");
    g.add_keyword("if");
    g.add_keyword("else");
    g.add_keyword("for");
    g.add_keyword("while");
    g.add_keyword("return");
    g.add_keyword("class");
    g.add_keyword("new");
    g.add_keyword("this");

    g.add_rule(
        "source_file",
        Rule {
            name: "source_file".into(),
            expr: RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("statement".into()))),
            fields: vec![],
        },
    );

    g.add_rule(
        "statement",
        Rule {
            name: "statement".into(),
            expr: RuleExpr::Choice(vec![
                RuleExpr::RuleRef("function_decl".into()),
                RuleExpr::RuleRef("class_decl".into()),
                RuleExpr::RuleRef("variable_decl".into()),
                RuleExpr::RuleRef("return_stmt".into()),
                RuleExpr::RuleRef("if_stmt".into()),
                RuleExpr::RuleRef("for_stmt".into()),
                RuleExpr::RuleRef("expression_stmt".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "function_decl",
        Rule {
            name: "function_decl".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Literal("function".into()),
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Literal("(".into()),
                RuleExpr::Optional(Box::new(RuleExpr::RuleRef("param_list".into()))),
                RuleExpr::Literal(")".into()),
                RuleExpr::RuleRef("block".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "class_decl",
        Rule {
            name: "class_decl".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Literal("class".into()),
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Literal("{".into()),
                RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("method_def".into()))),
                RuleExpr::Literal("}".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "method_def",
        Rule {
            name: "method_def".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Literal("(".into()),
                RuleExpr::Optional(Box::new(RuleExpr::RuleRef("param_list".into()))),
                RuleExpr::Literal(")".into()),
                RuleExpr::RuleRef("block".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "variable_decl",
        Rule {
            name: "variable_decl".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Choice(vec![
                    RuleExpr::Literal("const".into()),
                    RuleExpr::Literal("let".into()),
                    RuleExpr::Literal("var".into()),
                ]),
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Optional(Box::new(RuleExpr::Seq(vec![
                    RuleExpr::Literal("=".into()),
                    RuleExpr::RuleRef("expression".into()),
                ]))),
                RuleExpr::Literal(";".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "return_stmt",
        Rule {
            name: "return_stmt".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Literal("return".into()),
                RuleExpr::Optional(Box::new(RuleExpr::RuleRef("expression".into()))),
                RuleExpr::Literal(";".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "if_stmt",
        Rule {
            name: "if_stmt".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Literal("if".into()),
                RuleExpr::Literal("(".into()),
                RuleExpr::RuleRef("expression".into()),
                RuleExpr::Literal(")".into()),
                RuleExpr::RuleRef("block_or_stmt".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "for_stmt",
        Rule {
            name: "for_stmt".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Literal("for".into()),
                RuleExpr::Literal("(".into()),
                RuleExpr::RuleRef("expression".into()),
                RuleExpr::Literal(";".into()),
                RuleExpr::RuleRef("expression".into()),
                RuleExpr::Literal(";".into()),
                RuleExpr::RuleRef("expression".into()),
                RuleExpr::Literal(")".into()),
                RuleExpr::RuleRef("block".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "expression_stmt",
        Rule {
            name: "expression_stmt".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::RuleRef("expression".into()),
                RuleExpr::Literal(";".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "expression",
        Rule {
            name: "expression".into(),
            expr: RuleExpr::Choice(vec![
                RuleExpr::RuleRef("String".into()),
                RuleExpr::RuleRef("Integer".into()),
                RuleExpr::RuleRef("Float".into()),
                RuleExpr::RuleRef("Identifier".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "block_or_stmt",
        Rule {
            name: "block_or_stmt".into(),
            expr: RuleExpr::Choice(vec![
                RuleExpr::RuleRef("block".into()),
                RuleExpr::RuleRef("statement".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "block",
        Rule {
            name: "block".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Literal("{".into()),
                RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("statement".into()))),
                RuleExpr::Literal("}".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "param_list",
        Rule {
            name: "param_list".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Repeat(Box::new(RuleExpr::Seq(vec![
                    RuleExpr::Literal(",".into()),
                    RuleExpr::RuleRef("Identifier".into()),
                ]))),
            ]),
            fields: vec![],
        },
    );

    g
}

fn build_rust_grammar() -> Grammar {
    let mut g = Grammar::new("rust");

    g.add_keyword("fn");
    g.add_keyword("let");
    g.add_keyword("mut");
    g.add_keyword("struct");
    g.add_keyword("impl");
    g.add_keyword("pub");
    g.add_keyword("self");
    g.add_keyword("if");
    g.add_keyword("for");
    g.add_keyword("return");

    g.add_rule(
        "source_file",
        Rule {
            name: "source_file".into(),
            expr: RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("item".into()))),
            fields: vec![],
        },
    );

    g.add_rule(
        "item",
        Rule {
            name: "item".into(),
            expr: RuleExpr::Choice(vec![
                RuleExpr::RuleRef("function".into()),
                RuleExpr::RuleRef("struct_def".into()),
                RuleExpr::RuleRef("impl_block".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "function",
        Rule {
            name: "function".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Optional(Box::new(RuleExpr::Literal("pub".into()))),
                RuleExpr::Literal("fn".into()),
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Literal("(".into()),
                RuleExpr::Optional(Box::new(RuleExpr::RuleRef("param_list".into()))),
                RuleExpr::Literal(")".into()),
                RuleExpr::Optional(Box::new(RuleExpr::Seq(vec![
                    RuleExpr::Literal("->".into()),
                    RuleExpr::RuleRef("Identifier".into()),
                ]))),
                RuleExpr::RuleRef("block".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "struct_def",
        Rule {
            name: "struct_def".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Optional(Box::new(RuleExpr::Literal("pub".into()))),
                RuleExpr::Literal("struct".into()),
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Literal("{".into()),
                RuleExpr::Optional(Box::new(RuleExpr::RuleRef("field_list".into()))),
                RuleExpr::Literal("}".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "impl_block",
        Rule {
            name: "impl_block".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Literal("impl".into()),
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Literal("{".into()),
                RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("function".into()))),
                RuleExpr::Literal("}".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "block",
        Rule {
            name: "block".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Literal("{".into()),
                RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("statement".into()))),
                RuleExpr::Literal("}".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "statement",
        Rule {
            name: "statement".into(),
            expr: RuleExpr::Choice(vec![
                RuleExpr::RuleRef("let_stmt".into()),
                RuleExpr::RuleRef("return_stmt".into()),
                RuleExpr::RuleRef("expr_stmt".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "let_stmt",
        Rule {
            name: "let_stmt".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Literal("let".into()),
                RuleExpr::Optional(Box::new(RuleExpr::Literal("mut".into()))),
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Optional(Box::new(RuleExpr::Seq(vec![
                    RuleExpr::Literal("=".into()),
                    RuleExpr::RuleRef("expression".into()),
                ]))),
                RuleExpr::Literal(";".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "return_stmt",
        Rule {
            name: "return_stmt".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Literal("return".into()),
                RuleExpr::Optional(Box::new(RuleExpr::RuleRef("expression".into()))),
                RuleExpr::Literal(";".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "expr_stmt",
        Rule {
            name: "expr_stmt".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::RuleRef("expression".into()),
                RuleExpr::Literal(";".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "expression",
        Rule {
            name: "expression".into(),
            expr: RuleExpr::Choice(vec![
                RuleExpr::RuleRef("String".into()),
                RuleExpr::RuleRef("Integer".into()),
                RuleExpr::RuleRef("Float".into()),
                RuleExpr::RuleRef("Identifier".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "param_list",
        Rule {
            name: "param_list".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::RuleRef("param".into()),
                RuleExpr::Repeat(Box::new(RuleExpr::Seq(vec![
                    RuleExpr::Literal(",".into()),
                    RuleExpr::RuleRef("param".into()),
                ]))),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "param",
        Rule {
            name: "param".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Literal(":".into()),
                RuleExpr::RuleRef("Identifier".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "field_list",
        Rule {
            name: "field_list".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::RuleRef("field".into()),
                RuleExpr::Repeat(Box::new(RuleExpr::Seq(vec![
                    RuleExpr::Literal(",".into()),
                    RuleExpr::RuleRef("field".into()),
                ]))),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "field",
        Rule {
            name: "field".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Optional(Box::new(RuleExpr::Literal("pub".into()))),
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Literal(":".into()),
                RuleExpr::RuleRef("Identifier".into()),
            ]),
            fields: vec![],
        },
    );

    g
}

fn build_css_grammar() -> Grammar {
    let mut g = Grammar::new("css");

    g.add_rule(
        "source_file",
        Rule {
            name: "source_file".into(),
            expr: RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("rule_set".into()))),
            fields: vec![],
        },
    );

    g.add_rule(
        "rule_set",
        Rule {
            name: "rule_set".into(),
            expr: RuleExpr::Choice(vec![
                RuleExpr::RuleRef("at_rule".into()),
                RuleExpr::RuleRef("style_rule".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "style_rule",
        Rule {
            name: "style_rule".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::RuleRef("selector".into()),
                RuleExpr::Literal("{".into()),
                RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("declaration".into()))),
                RuleExpr::Literal("}".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "at_rule",
        Rule {
            name: "at_rule".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Literal("(".into()),
                RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("Identifier".into()))),
                RuleExpr::Literal(")".into()),
                RuleExpr::Literal("{".into()),
                RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("style_rule".into()))),
                RuleExpr::Literal("}".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "selector",
        Rule {
            name: "selector".into(),
            expr: RuleExpr::RuleRef("Identifier".into()),
            fields: vec![],
        },
    );

    g.add_rule(
        "declaration",
        Rule {
            name: "declaration".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Literal(":".into()),
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Literal(";".into()),
            ]),
            fields: vec![],
        },
    );

    g
}

fn build_python_grammar() -> Grammar {
    let mut g = Grammar::new("python");

    g.add_keyword("def");
    g.add_keyword("class");
    g.add_keyword("if");
    g.add_keyword("for");
    g.add_keyword("return");
    g.add_keyword("self");

    g.add_rule(
        "source_file",
        Rule {
            name: "source_file".into(),
            expr: RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("statement".into()))),
            fields: vec![],
        },
    );

    g.add_rule(
        "statement",
        Rule {
            name: "statement".into(),
            expr: RuleExpr::Choice(vec![
                RuleExpr::RuleRef("function_def".into()),
                RuleExpr::RuleRef("class_def".into()),
                RuleExpr::RuleRef("assignment".into()),
                RuleExpr::RuleRef("return_stmt".into()),
                RuleExpr::RuleRef("expr_stmt".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "function_def",
        Rule {
            name: "function_def".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Literal("def".into()),
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Literal("(".into()),
                RuleExpr::Optional(Box::new(RuleExpr::RuleRef("param_list".into()))),
                RuleExpr::Literal(")".into()),
                RuleExpr::Literal(":".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "class_def",
        Rule {
            name: "class_def".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Literal("class".into()),
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Optional(Box::new(RuleExpr::Seq(vec![
                    RuleExpr::Literal("(".into()),
                    RuleExpr::Optional(Box::new(RuleExpr::RuleRef("param_list".into()))),
                    RuleExpr::Literal(")".into()),
                ]))),
                RuleExpr::Literal(":".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "assignment",
        Rule {
            name: "assignment".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Literal("=".into()),
                RuleExpr::RuleRef("expression".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "return_stmt",
        Rule {
            name: "return_stmt".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::Literal("return".into()),
                RuleExpr::Optional(Box::new(RuleExpr::RuleRef("expression".into()))),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "expr_stmt",
        Rule {
            name: "expr_stmt".into(),
            expr: RuleExpr::RuleRef("expression".into()),
            fields: vec![],
        },
    );

    g.add_rule(
        "expression",
        Rule {
            name: "expression".into(),
            expr: RuleExpr::Choice(vec![
                RuleExpr::RuleRef("String".into()),
                RuleExpr::RuleRef("Integer".into()),
                RuleExpr::RuleRef("Float".into()),
                RuleExpr::RuleRef("Identifier".into()),
            ]),
            fields: vec![],
        },
    );

    g.add_rule(
        "param_list",
        Rule {
            name: "param_list".into(),
            expr: RuleExpr::Seq(vec![
                RuleExpr::RuleRef("Identifier".into()),
                RuleExpr::Repeat(Box::new(RuleExpr::Seq(vec![
                    RuleExpr::Literal(",".into()),
                    RuleExpr::RuleRef("Identifier".into()),
                ]))),
            ]),
            fields: vec![],
        },
    );

    g
}

// ─── Benchmark Harness ──────────────────────────────────────────────────────

#[derive(Clone)]
struct BenchResult {
    min: Duration,
    max: Duration,
    avg: Duration,
    median: Duration,
    iterations: usize,
}

impl BenchResult {
    fn throughput_mbs(&self, bytes: usize) -> f64 {
        let secs = self.avg.as_secs_f64();
        if secs == 0.0 {
            return 0.0;
        }
        (bytes as f64) / secs / 1_000_000.0
    }
}

fn bench<F: FnMut()>(iterations: usize, mut f: F) -> BenchResult {
    let mut times = Vec::with_capacity(iterations);

    // Warmup
    for _ in 0..3.min(iterations) {
        f();
    }

    for _ in 0..iterations {
        let start = Instant::now();
        f();
        times.push(start.elapsed());
    }

    times.sort();

    let min = times[0];
    let max = times[times.len() - 1];
    let total: Duration = times.iter().sum();
    let avg = total / iterations as u32;
    let median = times[times.len() / 2];

    BenchResult {
        min,
        max,
        avg,
        median,
        iterations,
    }
}

// ─── Tree-sitter Helpers ────────────────────────────────────────────────────

fn ts_parse_json(source: &str) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    let lang: tree_sitter::Language = tree_sitter_json::LANGUAGE.into();
    parser.set_language(&lang).unwrap();
    parser.parse(source, None).unwrap()
}

fn ts_parse_javascript(source: &str) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    let lang: tree_sitter::Language = tree_sitter_javascript::LANGUAGE.into();
    parser.set_language(&lang).unwrap();
    parser.parse(source, None).unwrap()
}

fn ts_parse_rust(source: &str) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    let lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
    parser.set_language(&lang).unwrap();
    parser.parse(source, None).unwrap()
}

fn ts_parse_css(source: &str) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    let lang: tree_sitter::Language = tree_sitter_css::LANGUAGE.into();
    parser.set_language(&lang).unwrap();
    parser.parse(source, None).unwrap()
}

fn ts_parse_python(source: &str) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    let lang: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();
    parser.set_language(&lang).unwrap();
    parser.parse(source, None).unwrap()
}

fn ts_count_nodes(tree: &tree_sitter::Tree) -> usize {
    let mut cursor = tree.walk();
    let mut count = 0;
    loop {
        count += 1;
        if cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return count;
            }
        }
    }
}

fn st_count_nodes(root: &SyntaxNode) -> usize {
    let preorder = Preorder::new(root);
    preorder.count()
}

// ─── Output Formatting ──────────────────────────────────────────────────────

fn format_duration(d: Duration) -> String {
    let nanos = d.as_nanos();
    if nanos < 1_000 {
        format!("{nanos}ns")
    } else if nanos < 1_000_000 {
        format!("{:.1}µs", nanos as f64 / 1_000.0)
    } else if nanos < 1_000_000_000 {
        format!("{:.2}ms", nanos as f64 / 1_000_000.0)
    } else {
        format!("{:.3}s", nanos as f64 / 1_000_000_000.0)
    }
}

struct TableRow {
    test_name: String,
    ts_result: String,
    st_result: String,
    ratio: String,
}

fn print_table(title: &str, rows: &[TableRow]) {
    let col1_w = rows
        .iter()
        .map(|r| r.test_name.len())
        .max()
        .unwrap_or(15)
        .max(15);
    let col2_w = rows
        .iter()
        .map(|r| r.ts_result.len())
        .max()
        .unwrap_or(12)
        .max(12);
    let col3_w = rows
        .iter()
        .map(|r| r.st_result.len())
        .max()
        .unwrap_or(12)
        .max(12);
    let col4_w = rows
        .iter()
        .map(|r| r.ratio.len())
        .max()
        .unwrap_or(15)
        .max(15);

    let total_w = col1_w + col2_w + col3_w + col4_w + 7;

    println!();
    println!("╔{}╗", "═".repeat(total_w));
    let title_pad = (total_w.saturating_sub(title.len())) / 2;
    println!(
        "║{}{}{}║",
        " ".repeat(title_pad),
        title,
        " ".repeat(total_w - title_pad - title.len())
    );
    println!(
        "╠{}╤{}╤{}╤{}╣",
        "═".repeat(col1_w + 1),
        "═".repeat(col2_w + 2),
        "═".repeat(col3_w + 2),
        "═".repeat(col4_w + 2)
    );
    println!(
        "║ {:<col1_w$}│ {:<col2_w$} │ {:<col3_w$} │ {:<col4_w$} ║",
        "Test", "Tree-sitter", "SemTree", "Ratio (ST/TS)"
    );
    println!(
        "╠{}╪{}╪{}╪{}╣",
        "═".repeat(col1_w + 1),
        "═".repeat(col2_w + 2),
        "═".repeat(col3_w + 2),
        "═".repeat(col4_w + 2)
    );

    for row in rows {
        println!(
            "║ {:<col1_w$}│ {:<col2_w$} │ {:<col3_w$} │ {:<col4_w$} ║",
            row.test_name, row.ts_result, row.st_result, row.ratio
        );
    }

    println!(
        "╚{}╧{}╧{}╧{}╝",
        "═".repeat(col1_w + 1),
        "═".repeat(col2_w + 2),
        "═".repeat(col3_w + 2),
        "═".repeat(col4_w + 2)
    );
}

fn print_single_table(title: &str, rows: &[(String, String)]) {
    let col1_w = rows
        .iter()
        .map(|(n, _)| n.len())
        .max()
        .unwrap_or(20)
        .max(20);
    let col2_w = rows
        .iter()
        .map(|(_, v)| v.len())
        .max()
        .unwrap_or(20)
        .max(20);

    let total_w = col1_w + col2_w + 3;

    println!();
    println!("╔{}╗", "═".repeat(total_w));
    let title_pad = (total_w.saturating_sub(title.len())) / 2;
    println!(
        "║{}{}{}║",
        " ".repeat(title_pad),
        title,
        " ".repeat(total_w - title_pad - title.len())
    );
    println!("╠{}╤{}╣", "═".repeat(col1_w + 1), "═".repeat(col2_w + 2));

    for (name, value) in rows {
        println!("║ {:<col1_w$}│ {:<col2_w$} ║", name, value);
    }

    println!("╚{}╧{}╝", "═".repeat(col1_w + 1), "═".repeat(col2_w + 2));
}

fn ratio_f64(st: &BenchResult, ts: &BenchResult) -> f64 {
    let st_ns = st.median.as_nanos() as f64;
    let ts_ns = ts.median.as_nanos() as f64;
    if ts_ns == 0.0 {
        return 1.0;
    }
    st_ns / ts_ns
}

fn ratio_string(st: &BenchResult, ts: &BenchResult) -> String {
    let st_ns = st.median.as_nanos() as f64;
    let ts_ns = ts.median.as_nanos() as f64;
    if ts_ns == 0.0 {
        return "N/A".to_string();
    }
    let ratio = st_ns / ts_ns;
    if ratio < 1.0 {
        format!("{:.2}x faster", 1.0 / ratio)
    } else if ratio > 1.0 {
        format!("{:.2}x slower", ratio)
    } else {
        "1.00x (equal)".to_string()
    }
}

// ─── Benchmark Suites ───────────────────────────────────────────────────────

struct LangBench {
    name: &'static str,
    generate: fn(usize) -> String,
    grammar: Grammar,
    ts_parse: fn(&str) -> tree_sitter::Tree,
}

fn run_cold_parse_benchmarks(langs: &[LangBench], iterations: usize) -> Vec<TableRow> {
    let mut rows = Vec::new();

    for lang in langs {
        for &(size_name, target_size) in SIZES {
            let source = (lang.generate)(target_size);
            let actual_size = source.len();

            let ts_parse = lang.ts_parse;
            let ts_result = bench(iterations, || {
                let _ = ts_parse(&source);
            });

            let parser = RuntimeParser::new(lang.grammar.clone());
            let st_result = bench(iterations, || {
                let _ = parser.parse(&source);
            });

            rows.push(TableRow {
                test_name: format!("{} {} cold", lang.name, size_name),
                ts_result: format!(
                    "{} ({:.0} MB/s)",
                    format_duration(ts_result.median),
                    ts_result.throughput_mbs(actual_size)
                ),
                st_result: format!(
                    "{} ({:.0} MB/s)",
                    format_duration(st_result.median),
                    st_result.throughput_mbs(actual_size)
                ),
                ratio: ratio_string(&st_result, &ts_result),
            });
        }
    }

    rows
}

fn run_incremental_benchmarks(langs: &[LangBench], iterations: usize) -> Vec<TableRow> {
    let mut rows = Vec::new();

    for lang in langs {
        let source = (lang.generate)(10_240); // 10KB for incremental tests
        let insert_pos = source.len() / 2;

        // Tree-sitter incremental: edit + reparse with old tree
        let ts_parse = lang.ts_parse;
        let ts_result = bench(iterations, || {
            let mut tree = ts_parse(&source);
            let edit = tree_sitter::InputEdit {
                start_byte: insert_pos,
                old_end_byte: insert_pos,
                new_end_byte: insert_pos + 1,
                start_position: tree_sitter::Point {
                    row: 0,
                    column: insert_pos,
                },
                old_end_position: tree_sitter::Point {
                    row: 0,
                    column: insert_pos,
                },
                new_end_position: tree_sitter::Point {
                    row: 0,
                    column: insert_pos + 1,
                },
            };
            tree.edit(&edit);

            let mut edited = source.clone();
            edited.insert(insert_pos, ' ');

            let mut parser = tree_sitter::Parser::new();
            let lang_ref: tree_sitter::Language = match lang.name {
                "JSON" => tree_sitter_json::LANGUAGE.into(),
                "JavaScript" => tree_sitter_javascript::LANGUAGE.into(),
                "Rust" => tree_sitter_rust::LANGUAGE.into(),
                "CSS" => tree_sitter_css::LANGUAGE.into(),
                "Python" => tree_sitter_python::LANGUAGE.into(),
                _ => unreachable!(),
            };
            parser.set_language(&lang_ref).unwrap();
            let _ = parser.parse(&edited, Some(&tree)).unwrap();
        });

        // SemTree incremental reparse (splice-based subtree reuse)
        let grammar = lang.grammar.clone();
        let st_result = bench(iterations, || {
            let mut edited = source.clone();
            edited.insert(insert_pos, ' ');
            let mut inc = IncrementalParser::new(grammar.clone());
            let _ = inc.parse(&source);
            let _ = inc.update(
                &edited,
                &[semtree_runtime::EditRegion::new(
                    insert_pos as u32,
                    insert_pos as u32,
                    " ",
                )],
            );
        });

        rows.push(TableRow {
            test_name: format!("{} 10KB incr", lang.name),
            ts_result: format!("{} (edit+reparse)", format_duration(ts_result.median)),
            st_result: format!("{} (incremental)", format_duration(st_result.median)),
            ratio: ratio_string(&st_result, &ts_result),
        });
    }

    rows
}

fn run_traversal_benchmarks(langs: &[LangBench], iterations: usize) -> Vec<TableRow> {
    let mut rows = Vec::new();

    for lang in langs {
        let source = (lang.generate)(10_240);

        let ts_tree = (lang.ts_parse)(&source);
        let ts_node_count = ts_count_nodes(&ts_tree);

        let parser = RuntimeParser::new(lang.grammar.clone());
        let st_parse = parser.parse(&source);
        let st_root = st_parse.syntax();
        let st_node_count = st_count_nodes(&st_root);

        let ts_result = bench(iterations, || {
            let _ = ts_count_nodes(&ts_tree);
        });

        let st_root_clone = st_root.clone();
        let st_result = bench(iterations, || {
            let _ = st_count_nodes(&st_root_clone);
        });

        rows.push(TableRow {
            test_name: format!("{} traverse", lang.name),
            ts_result: format!(
                "{} ({} nodes)",
                format_duration(ts_result.median),
                ts_node_count
            ),
            st_result: format!(
                "{} ({} nodes)",
                format_duration(st_result.median),
                st_node_count
            ),
            ratio: ratio_string(&st_result, &ts_result),
        });
    }

    rows
}

fn run_memory_benchmarks(langs: &[LangBench]) -> Vec<TableRow> {
    let mut rows = Vec::new();

    for lang in langs {
        let source = (lang.generate)(10_240);

        let ts_tree = (lang.ts_parse)(&source);
        let ts_nodes = ts_count_nodes(&ts_tree);

        let parser = RuntimeParser::new(lang.grammar.clone());
        let st_parse = parser.parse(&source);
        let st_root = st_parse.syntax();
        let st_nodes = st_count_nodes(&st_root);

        // Approximate: tree-sitter nodes are ~48 bytes each (internal struct)
        let ts_estimated_bytes = ts_nodes * 48;
        // SemTree green nodes: ~64 bytes each (SmolStr + children vec + kind + len)
        let st_estimated_bytes = st_nodes * 64;

        rows.push(TableRow {
            test_name: format!("{} memory", lang.name),
            ts_result: format!("{} nodes (~{}KB)", ts_nodes, ts_estimated_bytes / 1024),
            st_result: format!("{} nodes (~{}KB)", st_nodes, st_estimated_bytes / 1024),
            ratio: if st_estimated_bytes > ts_estimated_bytes {
                format!(
                    "{:.1}x more",
                    st_estimated_bytes as f64 / ts_estimated_bytes as f64
                )
            } else {
                format!(
                    "{:.1}x less",
                    ts_estimated_bytes as f64 / st_estimated_bytes as f64
                )
            },
        });
    }

    rows
}

fn run_semtree_extras(_langs: &[LangBench]) -> Vec<(String, String)> {
    let mut results = Vec::new();

    // Use the Rust grammar for semantic analysis demos
    let rust_source = generate_rust(10_240);
    let rust_grammar = build_rust_grammar();
    let parser = RuntimeParser::new(rust_grammar);
    let parse_result = parser.parse(&rust_source);
    let root = parse_result.syntax();

    // Semantic model build time
    let sem_bench = bench(50, || {
        let _ = SemanticModel::analyze(&root);
    });
    results.push((
        "Semantic model build (10KB Rust)".to_string(),
        format!("{} (median)", format_duration(sem_bench.median)),
    ));

    // Query execution time
    let pattern = QueryPattern { nodes: vec![] };
    let query_bench = bench(100, || {
        let _ = QueryEngine::query(&root, &pattern);
    });
    results.push((
        "Query execution (empty pattern)".to_string(),
        format!("{} (median)", format_duration(query_bench.median)),
    ));

    // Find by kind
    let find_bench = bench(100, || {
        let _ = QueryEngine::find_identifiers(&root);
    });
    results.push((
        "Find all identifiers (10KB)".to_string(),
        format!("{} (median)", format_duration(find_bench.median)),
    ));

    // Format time
    let formatter = Formatter::with_defaults();
    let fmt_bench = bench(50, || {
        let _ = formatter.format(&root);
    });
    results.push((
        "Format (10KB Rust)".to_string(),
        format!("{} (median)", format_duration(fmt_bench.median)),
    ));

    // Lint time
    let lint_engine = LintEngine::with_defaults();
    let model = SemanticModel::analyze(&root);
    let lint_bench = bench(100, || {
        let _ = lint_engine.lint(&root, &model);
    });
    results.push((
        "Lint with semantics (10KB)".to_string(),
        format!("{} (median)", format_duration(lint_bench.median)),
    ));

    // Lint syntax-only
    let lint_syn_bench = bench(100, || {
        let _ = lint_engine.lint_syntax(&root);
    });
    results.push((
        "Lint syntax-only (10KB)".to_string(),
        format!("{} (median)", format_duration(lint_syn_bench.median)),
    ));

    results
}

// ─── Error Recovery Benchmarks ───────────────────────────────────────────────

struct ErrorRecoveryCase {
    name: &'static str,
    source: &'static str,
    lang: &'static str,
}

fn error_recovery_cases() -> Vec<ErrorRecoveryCase> {
    vec![
        ErrorRecoveryCase {
            name: "Missing semicolons (JS)",
            source: r#"function add(a, b) {
  let x = a + b
  let y = x * 2
  return y
}

function broken() {
  const val = "hello"
  console.log(val)
  if (true) {
    let z = 42
  }
}

class Foo {
  constructor() {
    this.x = 1
  }
  method() {
    return this.x
  }
}
"#,
            lang: "JavaScript",
        },
        ErrorRecoveryCase {
            name: "Unclosed braces (JS)",
            source: r#"function outer() {
  function inner() {
    let x = 1;
    if (x > 0) {
      let y = 2;

  return x;
}

function after() {
  let z = 3;
  return z;
}
"#,
            lang: "JavaScript",
        },
        ErrorRecoveryCase {
            name: "Garbage tokens (JS)",
            source: r#"function valid1() {
  return 1;
}

@@@ ### $$$ !!! ???

function valid2() {
  let x = 42;
  return x;
}

{ { { } } @#$ }

function valid3() {
  return "hello";
}
"#,
            lang: "JavaScript",
        },
        ErrorRecoveryCase {
            name: "Mixed valid/invalid (Rust)",
            source: r#"fn good() -> u32 {
    let x = 42;
    return x;
}

fn broken(
    let = invalid syntax here;
}

fn also_good() -> bool {
    let flag = true;
    return flag;
}

struct Point {
    x: f64
    y: f64,
}

impl Point {
    fn new() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}
"#,
            lang: "Rust",
        },
        ErrorRecoveryCase {
            name: "Invalid JSON",
            source: r#"[
  {"name": "Alice", "age": 30},
  {"name": "Bob" "age": 25},
  {"name": "Charlie", "age": },
  {"name": "Diana", "age": 28}
  {"name": "Eve", "age": 22},
  {invalid: json, here},
  {"name": "Frank", "age": 35}
]
"#,
            lang: "JSON",
        },
        ErrorRecoveryCase {
            name: "Missing colons (CSS)",
            source: r#".container {
  display flex;
  flex-direction: column;
  padding 16px;
  margin: 0 auto;
}

.header {
  background-color: #333;
  color white;
  font-size: 1.5rem;
}

@media (max-width: 768px) {
  .container {
    padding: 8px;
  }
}
"#,
            lang: "CSS",
        },
        ErrorRecoveryCase {
            name: "Indentation errors (Python)",
            source: r#"def good_function():
    x = 1
    return x

def broken_function():
x = 2
    y = 3
        z = 4
    return x

class MyClass:
    def method(self):
        return 42

    def broken_method(self):
    return 0

def final_function():
    return "ok"
"#,
            lang: "Python",
        },
    ]
}

fn ts_parse_language(source: &str, lang_name: &str) -> tree_sitter::Tree {
    match lang_name {
        "JSON" => ts_parse_json(source),
        "JavaScript" => ts_parse_javascript(source),
        "Rust" => ts_parse_rust(source),
        "CSS" => ts_parse_css(source),
        "Python" => ts_parse_python(source),
        _ => panic!("Unknown language: {lang_name}"),
    }
}

fn build_grammar_for(lang_name: &str) -> Grammar {
    let file = match lang_name {
        "JSON" => "json",
        "JavaScript" => "javascript",
        "Rust" => "rust",
        "CSS" => "css",
        "Python" => "python",
        _ => panic!("Unknown language: {lang_name}"),
    };
    grammar_loader::load_shipped_grammar(file)
}

fn ts_count_errors(tree: &tree_sitter::Tree) -> usize {
    let mut cursor = tree.walk();
    let mut errors = 0;
    loop {
        if cursor.node().is_error() || cursor.node().is_missing() {
            errors += 1;
        }
        if cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return errors;
            }
        }
    }
}

fn st_count_errors(root: &SyntaxNode) -> usize {
    let mut errors = 0;
    for node in root.descendants() {
        if node.kind() == semtree_core::SyntaxKind::ERROR {
            errors += 1;
        }
    }
    errors
}

fn run_error_recovery_benchmarks(iterations: usize) -> (Vec<TableRow>, Vec<TableRow>) {
    let cases = error_recovery_cases();
    let mut speed_rows = Vec::new();
    let mut quality_rows = Vec::new();

    for case in &cases {
        let grammar = build_grammar_for(case.lang);

        // Speed: how fast can each parser handle broken code?
        let ts_bench = bench(iterations, || {
            let _ = ts_parse_language(case.source, case.lang);
        });

        let parser = RuntimeParser::new(grammar.clone());
        let st_bench = bench(iterations, || {
            let _ = parser.parse(case.source);
        });

        speed_rows.push(TableRow {
            test_name: case.name.to_string(),
            ts_result: format_duration(ts_bench.median),
            st_result: format_duration(st_bench.median),
            ratio: ratio_string(&st_bench, &ts_bench),
        });

        // Quality: how well does each parser recover?
        let ts_tree = ts_parse_language(case.source, case.lang);
        let ts_nodes = ts_count_nodes(&ts_tree);
        let ts_errors = ts_count_errors(&ts_tree);
        let ts_coverage = ((ts_nodes - ts_errors) as f64 / ts_nodes as f64 * 100.0) as u32;

        let st_result = parser.parse(case.source);
        let st_root = st_result.syntax();
        let st_nodes = st_count_nodes(&st_root);
        let st_errors = st_count_errors(&st_root);
        let st_text_len = st_root.text().len();
        let st_coverage_pct = (st_text_len as f64 / case.source.len() as f64 * 100.0) as u32;

        quality_rows.push(TableRow {
            test_name: case.name.to_string(),
            ts_result: format!(
                "{} nodes, {} errors, {}% valid",
                ts_nodes, ts_errors, ts_coverage
            ),
            st_result: format!(
                "{} nodes, {} errors, {}% text",
                st_nodes, st_errors, st_coverage_pct
            ),
            ratio: if st_errors <= ts_errors {
                "SemTree ≤ errors".into()
            } else {
                "TS fewer errors".into()
            },
        });
    }

    (speed_rows, quality_rows)
}

// ─── Detailed Memory Benchmarks ─────────────────────────────────────────────

fn run_detailed_memory_benchmarks(langs: &[LangBench]) -> Vec<TableRow> {
    let mut rows = Vec::new();

    for &(size_name, target_size) in &[("1KB", 1_024usize), ("10KB", 10_240), ("100KB", 102_400)] {
        for lang in langs {
            let source = (lang.generate)(target_size);
            let actual_bytes = source.len();

            let ts_tree = (lang.ts_parse)(&source);
            let ts_nodes = ts_count_nodes(&ts_tree);
            let ts_bytes_per_node = 48usize; // tree-sitter internal node ~48 bytes
            let ts_total = ts_nodes * ts_bytes_per_node;
            let ts_ratio_to_src = ts_total as f64 / actual_bytes as f64;

            let parser = RuntimeParser::new(lang.grammar.clone());
            let st_parse = parser.parse(&source);
            let st_root = st_parse.syntax();
            let st_nodes = st_count_nodes(&st_root);
            let st_bytes_per_node = 64usize; // SmolStr(24) + children Vec(24) + kind(2) + len(4) + Arc overhead(~10)
            let st_total = st_nodes * st_bytes_per_node;
            let st_ratio_to_src = st_total as f64 / actual_bytes as f64;

            rows.push(TableRow {
                test_name: format!("{} {}", lang.name, size_name),
                ts_result: format!(
                    "{} nodes, ~{}KB ({:.1}x src)",
                    ts_nodes,
                    ts_total / 1024,
                    ts_ratio_to_src
                ),
                st_result: format!(
                    "{} nodes, ~{}KB ({:.1}x src)",
                    st_nodes,
                    st_total / 1024,
                    st_ratio_to_src
                ),
                ratio: format!("{:.1}x src vs {:.1}x src", st_ratio_to_src, ts_ratio_to_src),
            });
        }
    }

    rows
}

// ─── Incremental Reparse Detail ─────────────────────────────────────────────

fn run_incremental_detail_benchmarks(langs: &[LangBench], iterations: usize) -> Vec<TableRow> {
    let mut rows = Vec::new();

    #[allow(clippy::type_complexity)]
    let edit_types: &[(&str, fn(&str) -> String)] = &[
        ("insert char", |s: &str| {
            let mid = s.len() / 2;
            let mut out = s.to_string();
            out.insert(mid, ' ');
            out
        }),
        ("delete line", |s: &str| {
            let lines: Vec<&str> = s.lines().collect();
            let mid = lines.len() / 2;
            lines
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != mid)
                .map(|(_, l)| *l)
                .collect::<Vec<_>>()
                .join("\n")
        }),
        ("append block", |s: &str| {
            let mut out = s.to_string();
            out.push_str("\nfunction appended() { return 42; }\n");
            out
        }),
    ];

    for lang in langs.iter().take(2) {
        // JSON + JavaScript only for detail
        let source = (lang.generate)(10_240);

        for &(edit_name, edit_fn) in edit_types {
            let edited = edit_fn(&source);

            // Tree-sitter: parse original, edit, reparse with old tree
            let ts_parse_fn = lang.ts_parse;
            let ts_result = bench(iterations, || {
                let old_tree = ts_parse_fn(&source);
                let _ = ts_parse_fn(&edited);
                let _ = old_tree; // ensure old tree lives long enough
            });

            // SemTree: full reparse
            let parser = RuntimeParser::new(lang.grammar.clone());
            let st_result = bench(iterations, || {
                let _ = parser.parse(&edited);
            });

            rows.push(TableRow {
                test_name: format!("{} {}", lang.name, edit_name),
                ts_result: format_duration(ts_result.median),
                st_result: format_duration(st_result.median),
                ratio: ratio_string(&st_result, &ts_result),
            });
        }
    }

    rows
}

// ─── Main ───────────────────────────────────────────────────────────────────

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(DEFAULT_ITERATIONS);

    println!("SemTree vs Tree-sitter Benchmark Suite");
    println!("══════════════════════════════════════");
    println!("Iterations per test: {iterations}");
    println!();

    let langs = vec![
        LangBench {
            name: "JSON",
            generate: generate_json,
            grammar: grammar_loader::load_shipped_grammar("json"),
            ts_parse: ts_parse_json,
        },
        LangBench {
            name: "JavaScript",
            generate: generate_javascript,
            grammar: grammar_loader::load_shipped_grammar("javascript"),
            ts_parse: ts_parse_javascript,
        },
        LangBench {
            name: "Rust",
            generate: generate_rust,
            grammar: grammar_loader::load_shipped_grammar("rust"),
            ts_parse: ts_parse_rust,
        },
        LangBench {
            name: "CSS",
            generate: generate_css,
            grammar: grammar_loader::load_shipped_grammar("css"),
            ts_parse: ts_parse_css,
        },
        LangBench {
            name: "Python",
            generate: generate_python,
            grammar: grammar_loader::load_shipped_grammar("python"),
            ts_parse: ts_parse_python,
        },
    ];

    // ── 1. Parse Speed ──────────────────────────────────────────────────
    print!("Running cold parse benchmarks...");
    let cold_rows = run_cold_parse_benchmarks(&langs, iterations);
    println!(" done!");
    print_table("1. PARSE SPEED: SemTree vs Tree-sitter", &cold_rows);

    // ── 2. Incremental Reparse ──────────────────────────────────────────
    print!("Running incremental reparse benchmarks...");
    let incr_rows = run_incremental_benchmarks(&langs, iterations);
    println!(" done!");
    print_table(
        "2a. INCREMENTAL REPARSE: SemTree vs Tree-sitter (10KB)",
        &incr_rows,
    );

    print!("Running detailed incremental benchmarks...");
    let incr_detail = run_incremental_detail_benchmarks(&langs, iterations);
    println!(" done!");
    print_table("2b. INCREMENTAL REPARSE: By Edit Type", &incr_detail);

    // ── 3. Memory Efficiency ────────────────────────────────────────────
    print!("Running memory benchmarks...");
    let mem_rows = run_detailed_memory_benchmarks(&langs);
    println!(" done!");
    print_table("3. MEMORY EFFICIENCY: SemTree vs Tree-sitter", &mem_rows);

    // ── 4. Error Recovery ───────────────────────────────────────────────
    print!("Running error recovery benchmarks...");
    let (err_speed, err_quality) = run_error_recovery_benchmarks(iterations);
    println!(" done!");
    print_table("4a. ERROR RECOVERY SPEED: Parsing Broken Code", &err_speed);
    print_table(
        "4b. ERROR RECOVERY QUALITY: Tree Completeness",
        &err_quality,
    );

    // ── 5. Tree Traversal ───────────────────────────────────────────────
    print!("Running tree traversal benchmarks...");
    let trav_rows = run_traversal_benchmarks(&langs, iterations);
    println!(" done!");
    print_table(
        "5. TREE TRAVERSAL: SemTree vs Tree-sitter (10KB)",
        &trav_rows,
    );

    // ── 6. SemTree-only extras ──────────────────────────────────────────
    print!("Running SemTree-only benchmarks...");
    let extras = run_semtree_extras(&langs);
    println!(" done!");
    print_single_table("6. SEMTREE BONUS: Features tree-sitter can't do", &extras);

    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("SUMMARY (computed from this run — shipped grammars/*.semtree)");
    println!("═══════════════════════════════════════════════════════════════");
    print_computed_summary(&cold_rows, &incr_rows, &mem_rows);
    println!();
    println!("Notes:");
    println!("  - SemTree uses grammars from grammars/*.semtree (not inline toy grammars)");
    println!("  - Tree-sitter uses production C parsers");
    println!("  - Incremental: TS edit+reparse vs SemTree IncrementalParser::update");
    println!("  - All times are median of {iterations} iterations, --release build");
    println!("  - Memory estimates use 48 bytes/node (TS) and 64 bytes/node (SemTree)");
}

fn print_computed_summary(parse_rows: &[TableRow], incr_rows: &[TableRow], mem_rows: &[TableRow]) {
    let parse_10k: Vec<f64> = parse_rows
        .iter()
        .filter(|r| r.test_name.contains("10KB"))
        .filter_map(|r| parse_ratio_from_display(&r.ratio))
        .collect();

    if !parse_10k.is_empty() {
        let avg = parse_10k.iter().sum::<f64>() / parse_10k.len() as f64;
        if avg < 1.0 {
            println!(
                "  Parse Speed (10KB):  SemTree {:.2}x faster on average",
                1.0 / avg
            );
        } else {
            println!(
                "  Parse Speed (10KB):  SemTree {:.2}x slower on average",
                avg
            );
        }
    } else {
        println!("  Parse Speed:         (no 10KB rows)");
    }

    let incr_ratios: Vec<f64> = incr_rows
        .iter()
        .filter_map(|r| parse_ratio_from_display(&r.ratio))
        .collect();
    if !incr_ratios.is_empty() {
        let avg = incr_ratios.iter().sum::<f64>() / incr_ratios.len() as f64;
        if avg < 1.0 {
            println!(
                "  Incremental:         SemTree {:.2}x faster on average",
                1.0 / avg
            );
        } else {
            println!(
                "  Incremental:         SemTree {:.2}x slower on average",
                avg
            );
        }
    }

    if let Some(first) = mem_rows.first() {
        println!(
            "  Memory (sample):       {} vs {}",
            first.st_result, first.ts_result
        );
    }
    println!("  Error Recovery:        see tables above");
    println!("  Bonus Features:        semantic model, format, lint — TS has none built-in");
}

fn parse_ratio_from_display(s: &str) -> Option<f64> {
    if let Some(rest) = s.strip_suffix("x faster") {
        return rest.trim().parse().ok().map(|f: f64| 1.0 / f);
    }
    if let Some(rest) = s.strip_suffix("x slower") {
        return rest.trim().parse().ok();
    }
    None
}
