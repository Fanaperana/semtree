use semtree_core::SyntaxKind;
use semtree_grammar::parse_semtree_dsl;
use semtree_green::GreenNode;
use semtree_red::{SyntaxElement, SyntaxNode};
use semtree_runtime::RuntimeParser;
use wasm_bindgen::prelude::*;

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

#[wasm_bindgen]
pub struct SemTreeParser {
    parser: RuntimeParser,
}

#[wasm_bindgen]
impl SemTreeParser {
    #[wasm_bindgen(constructor)]
    pub fn new(grammar_source: &str) -> Result<SemTreeParser, JsValue> {
        let grammar = parse_semtree_dsl(grammar_source)
            .map_err(|e| JsValue::from_str(&format!("Grammar parse error: {e}")))?;
        Ok(SemTreeParser {
            parser: RuntimeParser::new(grammar),
        })
    }

    pub fn parse(&self, source: &str) -> SemTreeTree {
        let result = self.parser.parse(source);
        let root = result.syntax();
        let error_count = result.errors.len();
        SemTreeTree {
            green: result.green_tree,
            root,
            source: source.to_string(),
            error_count,
        }
    }
}

#[wasm_bindgen]
pub struct SemTreeTree {
    #[allow(dead_code)]
    green: GreenNode,
    root: SyntaxNode,
    source: String,
    error_count: usize,
}

#[wasm_bindgen]
impl SemTreeTree {
    pub fn root_node(&self) -> SemTreeNode {
        SemTreeNode {
            node: self.root.clone(),
        }
    }

    pub fn to_sexp(&self) -> String {
        node_to_sexp(&self.root)
    }

    pub fn to_json(&self) -> JsValue {
        let json = node_to_json(&self.root);
        serde_wasm_bindgen::to_value(&json).unwrap_or(JsValue::NULL)
    }

    pub fn text(&self) -> String {
        self.source.clone()
    }

    pub fn error_count(&self) -> usize {
        self.error_count
    }
}

#[wasm_bindgen]
pub struct SemTreeNode {
    node: SyntaxNode,
}

#[wasm_bindgen]
impl SemTreeNode {
    pub fn kind(&self) -> u16 {
        self.node.kind().0
    }

    pub fn kind_name(&self) -> String {
        syntax_kind_name(self.node.kind())
    }

    pub fn text(&self) -> String {
        self.node.text()
    }

    pub fn start_byte(&self) -> u32 {
        u32::from(self.node.text_range().start())
    }

    pub fn end_byte(&self) -> u32 {
        u32::from(self.node.text_range().end())
    }

    pub fn child_count(&self) -> usize {
        self.node.children().len()
    }

    pub fn child(&self, index: usize) -> Option<SemTreeNode> {
        self.node
            .children()
            .into_iter()
            .nth(index)
            .map(|n| SemTreeNode { node: n })
    }

    pub fn children(&self) -> Vec<SemTreeNode> {
        self.node
            .children()
            .into_iter()
            .map(|n| SemTreeNode { node: n })
            .collect()
    }

    pub fn to_sexp(&self) -> String {
        node_to_sexp(&self.node)
    }
}

#[wasm_bindgen]
pub fn parse_json(source: &str) -> Result<SemTreeTree, JsValue> {
    let grammar_src = include_str!("../../../grammars/json.semtree");
    let grammar = parse_semtree_dsl(grammar_src)
        .map_err(|e| JsValue::from_str(&format!("JSON grammar error: {e}")))?;
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse(source);
    let root = result.syntax();
    let error_count = result.errors.len();
    Ok(SemTreeTree {
        green: result.green_tree,
        root,
        source: source.to_string(),
        error_count,
    })
}

#[wasm_bindgen]
pub fn parse_with_grammar(grammar_dsl: &str, source: &str) -> Result<SemTreeTree, JsValue> {
    let parser = SemTreeParser::new(grammar_dsl)?;
    Ok(parser.parse(source))
}
