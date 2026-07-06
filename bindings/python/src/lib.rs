use pyo3::prelude::*;
use semtree_core::SyntaxKind;
use semtree_grammar::parse_semtree_dsl;
use semtree_green::GreenNode;
use semtree_red::{SyntaxElement, SyntaxNode};
use semtree_runtime::RuntimeParser;

fn syntax_kind_name(kind: SyntaxKind) -> String {
    match kind {
        SyntaxKind::EOF => "EOF".into(),
        SyntaxKind::WHITESPACE => "WHITESPACE".into(),
        SyntaxKind::NEWLINE => "NEWLINE".into(),
        SyntaxKind::LINE_COMMENT => "LINE_COMMENT".into(),
        SyntaxKind::BLOCK_COMMENT => "BLOCK_COMMENT".into(),
        SyntaxKind::IDENT => "IDENT".into(),
        SyntaxKind::INT_LIT => "INT_LIT".into(),
        SyntaxKind::FLOAT_LIT => "FLOAT_LIT".into(),
        SyntaxKind::STRING_LIT => "STRING_LIT".into(),
        SyntaxKind::CHAR_LIT => "CHAR_LIT".into(),
        SyntaxKind::BOOL_LIT => "BOOL_LIT".into(),
        SyntaxKind::LPAREN => "LPAREN".into(),
        SyntaxKind::RPAREN => "RPAREN".into(),
        SyntaxKind::LBRACE => "LBRACE".into(),
        SyntaxKind::RBRACE => "RBRACE".into(),
        SyntaxKind::LBRACKET => "LBRACKET".into(),
        SyntaxKind::RBRACKET => "RBRACKET".into(),
        SyntaxKind::SEMICOLON => "SEMICOLON".into(),
        SyntaxKind::COLON => "COLON".into(),
        SyntaxKind::COMMA => "COMMA".into(),
        SyntaxKind::DOT => "DOT".into(),
        SyntaxKind::SOURCE_FILE => "SOURCE_FILE".into(),
        SyntaxKind::FUNCTION => "FUNCTION".into(),
        SyntaxKind::PARAM_LIST => "PARAM_LIST".into(),
        SyntaxKind::PARAM => "PARAM".into(),
        SyntaxKind::BLOCK => "BLOCK".into(),
        SyntaxKind::LET_STMT => "LET_STMT".into(),
        SyntaxKind::EXPR_STMT => "EXPR_STMT".into(),
        SyntaxKind::RETURN_STMT => "RETURN_STMT".into(),
        SyntaxKind::IF_EXPR => "IF_EXPR".into(),
        SyntaxKind::BINARY_EXPR => "BINARY_EXPR".into(),
        SyntaxKind::CALL_EXPR => "CALL_EXPR".into(),
        SyntaxKind::LITERAL => "LITERAL".into(),
        SyntaxKind::STRUCT_DEF => "STRUCT_DEF".into(),
        SyntaxKind::ENUM_DEF => "ENUM_DEF".into(),
        SyntaxKind::ARRAY_EXPR => "ARRAY_EXPR".into(),
        SyntaxKind::ERROR => "ERROR".into(),
        other => format!("KIND_{}", other.0),
    }
}

fn node_to_sexp(node: &SyntaxNode) -> String {
    let mut s = format!("({}", syntax_kind_name(node.kind()));
    for child in node.children_with_tokens() {
        s.push(' ');
        match child {
            SyntaxElement::Node(n) => s.push_str(&node_to_sexp(&n)),
            SyntaxElement::Token(t) => {
                s.push_str(&format!("({} {:?})", syntax_kind_name(t.kind()), t.text()));
            }
        }
    }
    s.push(')');
    s
}

fn node_to_json(node: &SyntaxNode) -> serde_json::Value {
    let children: Vec<serde_json::Value> = node
        .children_with_tokens()
        .into_iter()
        .map(|child| match child {
            SyntaxElement::Node(n) => node_to_json(&n),
            SyntaxElement::Token(t) => serde_json::json!({
                "kind": syntax_kind_name(t.kind()),
                "text": t.text(),
                "range": [u32::from(t.text_range().start()), u32::from(t.text_range().end())]
            }),
        })
        .collect();

    serde_json::json!({
        "kind": syntax_kind_name(node.kind()),
        "range": [u32::from(node.text_range().start()), u32::from(node.text_range().end())],
        "children": children
    })
}

#[pyclass]
struct Parser {
    inner: RuntimeParser,
}

#[pymethods]
impl Parser {
    #[new]
    fn new(grammar_source: &str) -> PyResult<Self> {
        let grammar = parse_semtree_dsl(grammar_source)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Grammar parse error: {e}")))?;
        Ok(Parser {
            inner: RuntimeParser::new(grammar),
        })
    }

    fn parse(&self, source: &str) -> PyResult<Tree> {
        let result = self.inner.parse(source);
        let root = result.syntax();
        let error_count = result.errors.len();
        Ok(Tree {
            green: result.green_tree,
            root,
            source: source.to_string(),
            error_count,
        })
    }
}

#[pyclass]
struct Tree {
    #[allow(dead_code)]
    green: GreenNode,
    root: SyntaxNode,
    source: String,
    error_count: usize,
}

#[pymethods]
impl Tree {
    fn root_node(&self) -> Node {
        Node {
            node: self.root.clone(),
        }
    }

    fn text(&self) -> String {
        self.source.clone()
    }

    fn to_sexp(&self) -> String {
        node_to_sexp(&self.root)
    }

    fn to_json(&self) -> PyResult<String> {
        let json = node_to_json(&self.root);
        serde_json::to_string_pretty(&json)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("JSON error: {e}")))
    }

    fn error_count(&self) -> usize {
        self.error_count
    }
}

#[pyclass]
struct Node {
    node: SyntaxNode,
}

#[pymethods]
impl Node {
    fn kind(&self) -> u16 {
        self.node.kind().0
    }

    fn kind_name(&self) -> String {
        syntax_kind_name(self.node.kind())
    }

    fn text(&self) -> String {
        self.node.text()
    }

    fn start_byte(&self) -> u32 {
        u32::from(self.node.text_range().start())
    }

    fn end_byte(&self) -> u32 {
        u32::from(self.node.text_range().end())
    }

    fn child_count(&self) -> usize {
        self.node.children().len()
    }

    fn child(&self, index: usize) -> Option<Node> {
        self.node
            .children()
            .into_iter()
            .nth(index)
            .map(|n| Node { node: n })
    }

    fn children(&self) -> Vec<Node> {
        self.node
            .children()
            .into_iter()
            .map(|n| Node { node: n })
            .collect()
    }

    fn to_sexp(&self) -> String {
        node_to_sexp(&self.node)
    }

    fn __repr__(&self) -> String {
        format!(
            "<Node kind={} range={}..{}>",
            syntax_kind_name(self.node.kind()),
            u32::from(self.node.text_range().start()),
            u32::from(self.node.text_range().end()),
        )
    }

    fn __str__(&self) -> String {
        self.node.text()
    }
}

#[pyfunction]
fn parse_json(source: &str) -> PyResult<Tree> {
    let grammar_src = include_str!("../../../grammars/json.semtree");
    let grammar = parse_semtree_dsl(grammar_src)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("JSON grammar error: {e}")))?;
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse(source);
    let root = result.syntax();
    let error_count = result.errors.len();
    Ok(Tree {
        green: result.green_tree,
        root,
        source: source.to_string(),
        error_count,
    })
}

#[pymodule]
fn semtree(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Parser>()?;
    m.add_class::<Tree>()?;
    m.add_class::<Node>()?;
    m.add_function(wrap_pyfunction!(parse_json, m)?)?;
    Ok(())
}
