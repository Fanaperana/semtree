#![allow(dead_code)]

use std::time::{Duration, Instant};

use semtree_format::Formatter;
use semtree_grammar::{Grammar, Rule, RuleExpr};
use semtree_lint::LintEngine;
use semtree_query::{QueryEngine, QueryPattern};
use semtree_red::{Preorder, SyntaxNode};
use semtree_runtime::RuntimeParser;
use semtree_semantic::SemanticModel;

// ─── Configuration ──────────────────────────────────────────────────────────

const DEFAULT_ITERATIONS: usize = 100;
const SIZES: &[(& str, usize)] = &[
    ("1KB", 1_024),
    ("10KB", 10_240),
    ("100KB", 102_400),
    ("1MB", 1_048_576),
];

// ─── Test Data Generation ───────────────────────────────────────────────────

fn generate_json(target_size: usize) -> String {
    let base = r#"{"name":"Alice","age":30,"active":true,"scores":[95,87,92],"address":{"street":"123 Main St","city":"Springfield","zip":"62701"}}"#;
    let mut result = String::with_capacity(target_size + 256);
    result.push_str("[");
    let mut first = true;
    while result.len() < target_size {
        if !first {
            result.push(',');
        }
        result.push_str(base);
        first = false;
    }
    result.push_str("]");
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

    g.add_rule("source_file", Rule {
        name: "source_file".into(),
        expr: RuleExpr::RuleRef("value".into()),
        fields: vec![],
    });

    g.add_rule("value", Rule {
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
    });

    g.add_rule("object", Rule {
        name: "object".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::Literal("{".into()),
            RuleExpr::Optional(Box::new(RuleExpr::RuleRef("pair_list".into()))),
            RuleExpr::Literal("}".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("pair_list", Rule {
        name: "pair_list".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::RuleRef("pair".into()),
            RuleExpr::Repeat(Box::new(RuleExpr::Seq(vec![
                RuleExpr::Literal(",".into()),
                RuleExpr::RuleRef("pair".into()),
            ]))),
        ]),
        fields: vec![],
    });

    g.add_rule("pair", Rule {
        name: "pair".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::RuleRef("String".into()),
            RuleExpr::Literal(":".into()),
            RuleExpr::RuleRef("value".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("array", Rule {
        name: "array".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::Literal("[".into()),
            RuleExpr::Optional(Box::new(RuleExpr::RuleRef("value_list".into()))),
            RuleExpr::Literal("]".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("value_list", Rule {
        name: "value_list".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::RuleRef("value".into()),
            RuleExpr::Repeat(Box::new(RuleExpr::Seq(vec![
                RuleExpr::Literal(",".into()),
                RuleExpr::RuleRef("value".into()),
            ]))),
        ]),
        fields: vec![],
    });

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

    g.add_rule("source_file", Rule {
        name: "source_file".into(),
        expr: RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("statement".into()))),
        fields: vec![],
    });

    g.add_rule("statement", Rule {
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
    });

    g.add_rule("function_decl", Rule {
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
    });

    g.add_rule("class_decl", Rule {
        name: "class_decl".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::Literal("class".into()),
            RuleExpr::RuleRef("Identifier".into()),
            RuleExpr::Literal("{".into()),
            RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("method_def".into()))),
            RuleExpr::Literal("}".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("method_def", Rule {
        name: "method_def".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::RuleRef("Identifier".into()),
            RuleExpr::Literal("(".into()),
            RuleExpr::Optional(Box::new(RuleExpr::RuleRef("param_list".into()))),
            RuleExpr::Literal(")".into()),
            RuleExpr::RuleRef("block".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("variable_decl", Rule {
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
    });

    g.add_rule("return_stmt", Rule {
        name: "return_stmt".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::Literal("return".into()),
            RuleExpr::Optional(Box::new(RuleExpr::RuleRef("expression".into()))),
            RuleExpr::Literal(";".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("if_stmt", Rule {
        name: "if_stmt".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::Literal("if".into()),
            RuleExpr::Literal("(".into()),
            RuleExpr::RuleRef("expression".into()),
            RuleExpr::Literal(")".into()),
            RuleExpr::RuleRef("block_or_stmt".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("for_stmt", Rule {
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
    });

    g.add_rule("expression_stmt", Rule {
        name: "expression_stmt".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::RuleRef("expression".into()),
            RuleExpr::Literal(";".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("expression", Rule {
        name: "expression".into(),
        expr: RuleExpr::Choice(vec![
            RuleExpr::RuleRef("String".into()),
            RuleExpr::RuleRef("Integer".into()),
            RuleExpr::RuleRef("Float".into()),
            RuleExpr::RuleRef("Identifier".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("block_or_stmt", Rule {
        name: "block_or_stmt".into(),
        expr: RuleExpr::Choice(vec![
            RuleExpr::RuleRef("block".into()),
            RuleExpr::RuleRef("statement".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("block", Rule {
        name: "block".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::Literal("{".into()),
            RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("statement".into()))),
            RuleExpr::Literal("}".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("param_list", Rule {
        name: "param_list".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::RuleRef("Identifier".into()),
            RuleExpr::Repeat(Box::new(RuleExpr::Seq(vec![
                RuleExpr::Literal(",".into()),
                RuleExpr::RuleRef("Identifier".into()),
            ]))),
        ]),
        fields: vec![],
    });

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

    g.add_rule("source_file", Rule {
        name: "source_file".into(),
        expr: RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("item".into()))),
        fields: vec![],
    });

    g.add_rule("item", Rule {
        name: "item".into(),
        expr: RuleExpr::Choice(vec![
            RuleExpr::RuleRef("function".into()),
            RuleExpr::RuleRef("struct_def".into()),
            RuleExpr::RuleRef("impl_block".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("function", Rule {
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
    });

    g.add_rule("struct_def", Rule {
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
    });

    g.add_rule("impl_block", Rule {
        name: "impl_block".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::Literal("impl".into()),
            RuleExpr::RuleRef("Identifier".into()),
            RuleExpr::Literal("{".into()),
            RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("function".into()))),
            RuleExpr::Literal("}".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("block", Rule {
        name: "block".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::Literal("{".into()),
            RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("statement".into()))),
            RuleExpr::Literal("}".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("statement", Rule {
        name: "statement".into(),
        expr: RuleExpr::Choice(vec![
            RuleExpr::RuleRef("let_stmt".into()),
            RuleExpr::RuleRef("return_stmt".into()),
            RuleExpr::RuleRef("expr_stmt".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("let_stmt", Rule {
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
    });

    g.add_rule("return_stmt", Rule {
        name: "return_stmt".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::Literal("return".into()),
            RuleExpr::Optional(Box::new(RuleExpr::RuleRef("expression".into()))),
            RuleExpr::Literal(";".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("expr_stmt", Rule {
        name: "expr_stmt".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::RuleRef("expression".into()),
            RuleExpr::Literal(";".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("expression", Rule {
        name: "expression".into(),
        expr: RuleExpr::Choice(vec![
            RuleExpr::RuleRef("String".into()),
            RuleExpr::RuleRef("Integer".into()),
            RuleExpr::RuleRef("Float".into()),
            RuleExpr::RuleRef("Identifier".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("param_list", Rule {
        name: "param_list".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::RuleRef("param".into()),
            RuleExpr::Repeat(Box::new(RuleExpr::Seq(vec![
                RuleExpr::Literal(",".into()),
                RuleExpr::RuleRef("param".into()),
            ]))),
        ]),
        fields: vec![],
    });

    g.add_rule("param", Rule {
        name: "param".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::RuleRef("Identifier".into()),
            RuleExpr::Literal(":".into()),
            RuleExpr::RuleRef("Identifier".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("field_list", Rule {
        name: "field_list".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::RuleRef("field".into()),
            RuleExpr::Repeat(Box::new(RuleExpr::Seq(vec![
                RuleExpr::Literal(",".into()),
                RuleExpr::RuleRef("field".into()),
            ]))),
        ]),
        fields: vec![],
    });

    g.add_rule("field", Rule {
        name: "field".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::Optional(Box::new(RuleExpr::Literal("pub".into()))),
            RuleExpr::RuleRef("Identifier".into()),
            RuleExpr::Literal(":".into()),
            RuleExpr::RuleRef("Identifier".into()),
        ]),
        fields: vec![],
    });

    g
}

fn build_css_grammar() -> Grammar {
    let mut g = Grammar::new("css");

    g.add_rule("source_file", Rule {
        name: "source_file".into(),
        expr: RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("rule_set".into()))),
        fields: vec![],
    });

    g.add_rule("rule_set", Rule {
        name: "rule_set".into(),
        expr: RuleExpr::Choice(vec![
            RuleExpr::RuleRef("at_rule".into()),
            RuleExpr::RuleRef("style_rule".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("style_rule", Rule {
        name: "style_rule".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::RuleRef("selector".into()),
            RuleExpr::Literal("{".into()),
            RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("declaration".into()))),
            RuleExpr::Literal("}".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("at_rule", Rule {
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
    });

    g.add_rule("selector", Rule {
        name: "selector".into(),
        expr: RuleExpr::RuleRef("Identifier".into()),
        fields: vec![],
    });

    g.add_rule("declaration", Rule {
        name: "declaration".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::RuleRef("Identifier".into()),
            RuleExpr::Literal(":".into()),
            RuleExpr::RuleRef("Identifier".into()),
            RuleExpr::Literal(";".into()),
        ]),
        fields: vec![],
    });

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

    g.add_rule("source_file", Rule {
        name: "source_file".into(),
        expr: RuleExpr::Repeat(Box::new(RuleExpr::RuleRef("statement".into()))),
        fields: vec![],
    });

    g.add_rule("statement", Rule {
        name: "statement".into(),
        expr: RuleExpr::Choice(vec![
            RuleExpr::RuleRef("function_def".into()),
            RuleExpr::RuleRef("class_def".into()),
            RuleExpr::RuleRef("assignment".into()),
            RuleExpr::RuleRef("return_stmt".into()),
            RuleExpr::RuleRef("expr_stmt".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("function_def", Rule {
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
    });

    g.add_rule("class_def", Rule {
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
    });

    g.add_rule("assignment", Rule {
        name: "assignment".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::RuleRef("Identifier".into()),
            RuleExpr::Literal("=".into()),
            RuleExpr::RuleRef("expression".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("return_stmt", Rule {
        name: "return_stmt".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::Literal("return".into()),
            RuleExpr::Optional(Box::new(RuleExpr::RuleRef("expression".into()))),
        ]),
        fields: vec![],
    });

    g.add_rule("expr_stmt", Rule {
        name: "expr_stmt".into(),
        expr: RuleExpr::RuleRef("expression".into()),
        fields: vec![],
    });

    g.add_rule("expression", Rule {
        name: "expression".into(),
        expr: RuleExpr::Choice(vec![
            RuleExpr::RuleRef("String".into()),
            RuleExpr::RuleRef("Integer".into()),
            RuleExpr::RuleRef("Float".into()),
            RuleExpr::RuleRef("Identifier".into()),
        ]),
        fields: vec![],
    });

    g.add_rule("param_list", Rule {
        name: "param_list".into(),
        expr: RuleExpr::Seq(vec![
            RuleExpr::RuleRef("Identifier".into()),
            RuleExpr::Repeat(Box::new(RuleExpr::Seq(vec![
                RuleExpr::Literal(",".into()),
                RuleExpr::RuleRef("Identifier".into()),
            ]))),
        ]),
        fields: vec![],
    });

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

    BenchResult { min, max, avg, median, iterations }
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
    let col1_w = rows.iter().map(|r| r.test_name.len()).max().unwrap_or(15).max(15);
    let col2_w = rows.iter().map(|r| r.ts_result.len()).max().unwrap_or(12).max(12);
    let col3_w = rows.iter().map(|r| r.st_result.len()).max().unwrap_or(12).max(12);
    let col4_w = rows.iter().map(|r| r.ratio.len()).max().unwrap_or(15).max(15);

    let total_w = col1_w + col2_w + col3_w + col4_w + 7;

    println!();
    println!("╔{}╗", "═".repeat(total_w));
    let title_pad = (total_w.saturating_sub(title.len())) / 2;
    println!("║{}{}{}║",
        " ".repeat(title_pad),
        title,
        " ".repeat(total_w - title_pad - title.len())
    );
    println!("╠{}╤{}╤{}╤{}╣",
        "═".repeat(col1_w + 1),
        "═".repeat(col2_w + 2),
        "═".repeat(col3_w + 2),
        "═".repeat(col4_w + 2)
    );
    println!("║ {:<col1_w$}│ {:<col2_w$} │ {:<col3_w$} │ {:<col4_w$} ║",
        "Test", "Tree-sitter", "SemTree", "Ratio (ST/TS)");
    println!("╠{}╪{}╪{}╪{}╣",
        "═".repeat(col1_w + 1),
        "═".repeat(col2_w + 2),
        "═".repeat(col3_w + 2),
        "═".repeat(col4_w + 2)
    );

    for row in rows {
        println!("║ {:<col1_w$}│ {:<col2_w$} │ {:<col3_w$} │ {:<col4_w$} ║",
            row.test_name, row.ts_result, row.st_result, row.ratio);
    }

    println!("╚{}╧{}╧{}╧{}╝",
        "═".repeat(col1_w + 1),
        "═".repeat(col2_w + 2),
        "═".repeat(col3_w + 2),
        "═".repeat(col4_w + 2)
    );
}

fn print_single_table(title: &str, rows: &[(String, String)]) {
    let col1_w = rows.iter().map(|(n, _)| n.len()).max().unwrap_or(20).max(20);
    let col2_w = rows.iter().map(|(_, v)| v.len()).max().unwrap_or(20).max(20);

    let total_w = col1_w + col2_w + 3;

    println!();
    println!("╔{}╗", "═".repeat(total_w));
    let title_pad = (total_w.saturating_sub(title.len())) / 2;
    println!("║{}{}{}║",
        " ".repeat(title_pad),
        title,
        " ".repeat(total_w - title_pad - title.len())
    );
    println!("╠{}╤{}╣",
        "═".repeat(col1_w + 1),
        "═".repeat(col2_w + 2)
    );

    for (name, value) in rows {
        println!("║ {:<col1_w$}│ {:<col2_w$} ║", name, value);
    }

    println!("╚{}╧{}╝",
        "═".repeat(col1_w + 1),
        "═".repeat(col2_w + 2)
    );
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
                ts_result: format!("{} ({:.0} MB/s)",
                    format_duration(ts_result.median),
                    ts_result.throughput_mbs(actual_size)),
                st_result: format!("{} ({:.0} MB/s)",
                    format_duration(st_result.median),
                    st_result.throughput_mbs(actual_size)),
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
                start_position: tree_sitter::Point { row: 0, column: insert_pos },
                old_end_position: tree_sitter::Point { row: 0, column: insert_pos },
                new_end_position: tree_sitter::Point { row: 0, column: insert_pos + 1 },
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

        // SemTree full reparse (no incremental yet—this measures the baseline)
        let parser = RuntimeParser::new(lang.grammar.clone());
        let st_result = bench(iterations, || {
            let mut edited = source.clone();
            edited.insert(insert_pos, ' ');
            let _ = parser.parse(&edited);
        });

        rows.push(TableRow {
            test_name: format!("{} 10KB incr", lang.name),
            ts_result: format!("{} (edit+reparse)", format_duration(ts_result.median)),
            st_result: format!("{} (full reparse)", format_duration(st_result.median)),
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
            ts_result: format!("{} ({} nodes)", format_duration(ts_result.median), ts_node_count),
            st_result: format!("{} ({} nodes)", format_duration(st_result.median), st_node_count),
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
                format!("{:.1}x more", st_estimated_bytes as f64 / ts_estimated_bytes as f64)
            } else {
                format!("{:.1}x less", ts_estimated_bytes as f64 / st_estimated_bytes as f64)
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
            grammar: build_json_grammar(),
            ts_parse: ts_parse_json,
        },
        LangBench {
            name: "JavaScript",
            generate: generate_javascript,
            grammar: build_javascript_grammar(),
            ts_parse: ts_parse_javascript,
        },
        LangBench {
            name: "Rust",
            generate: generate_rust,
            grammar: build_rust_grammar(),
            ts_parse: ts_parse_rust,
        },
        LangBench {
            name: "CSS",
            generate: generate_css,
            grammar: build_css_grammar(),
            ts_parse: ts_parse_css,
        },
        LangBench {
            name: "Python",
            generate: generate_python,
            grammar: build_python_grammar(),
            ts_parse: ts_parse_python,
        },
    ];

    // 1. Cold parse benchmarks
    print!("Running cold parse benchmarks...");
    let cold_rows = run_cold_parse_benchmarks(&langs, iterations);
    println!(" done!");
    print_table("Cold Parse: SemTree vs Tree-sitter", &cold_rows);

    // 2. Incremental reparse benchmarks
    print!("Running incremental reparse benchmarks...");
    let incr_rows = run_incremental_benchmarks(&langs, iterations);
    println!(" done!");
    print_table("Incremental Reparse: SemTree vs Tree-sitter", &incr_rows);

    // 3. Tree traversal benchmarks
    print!("Running tree traversal benchmarks...");
    let trav_rows = run_traversal_benchmarks(&langs, iterations);
    println!(" done!");
    print_table("Tree Traversal: SemTree vs Tree-sitter", &trav_rows);

    // 4. Memory benchmarks
    let mem_rows = run_memory_benchmarks(&langs);
    print_table("Memory Usage (estimated, 10KB input)", &mem_rows);

    // 5. SemTree-only extras
    print!("Running SemTree-only benchmarks...");
    let extras = run_semtree_extras(&langs);
    println!(" done!");
    print_single_table("SemTree Bonus Features (tree-sitter can't do these)", &extras);

    println!();
    println!("Notes:");
    println!("  - Tree-sitter uses optimized C parsers compiled from grammar specifications");
    println!("  - SemTree uses a runtime-interpreted grammar (no codegen needed)");
    println!("  - SemTree provides semantic analysis, formatting, linting, and queries");
    println!("  - All times are median of {iterations} iterations");
    println!("  - Ratio > 1.0x means SemTree is slower; < 1.0x means SemTree is faster");
}
