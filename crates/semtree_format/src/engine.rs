use semtree_core::SyntaxKind;
use semtree_red::{SyntaxNode, SyntaxToken, SyntaxElement};

use crate::config::FormatConfig;

/// A syntax-tree-driven code formatter.
///
/// The formatter walks the syntax tree and emits properly indented,
/// spaced, and line-broken output based on node kinds and config.
pub struct Formatter {
    config: FormatConfig,
}

impl Formatter {
    pub fn new(config: FormatConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(FormatConfig::default())
    }

    /// Format a syntax tree to a string.
    pub fn format(&self, root: &SyntaxNode) -> String {
        let mut ctx = FormatContext {
            config: &self.config,
            output: String::new(),
            indent: 0,
            at_line_start: true,
            last_kind: None,
        };

        ctx.format_node(root);

        if self.config.trailing_newline && !ctx.output.ends_with('\n') {
            ctx.output.push('\n');
        }

        ctx.output
    }
}

struct FormatContext<'a> {
    config: &'a FormatConfig,
    output: String,
    indent: usize,
    at_line_start: bool,
    last_kind: Option<SyntaxKind>,
}

impl<'a> FormatContext<'a> {
    fn format_node(&mut self, node: &SyntaxNode) {
        match node.kind() {
            SyntaxKind::SOURCE_FILE => self.format_source_file(node),
            SyntaxKind::FUNCTION => self.format_function(node),
            SyntaxKind::STRUCT_DEF => self.format_struct(node),
            SyntaxKind::ENUM_DEF => self.format_enum(node),
            SyntaxKind::BLOCK => self.format_block(node),
            SyntaxKind::LET_STMT => self.format_simple_statement(node),
            SyntaxKind::RETURN_STMT => self.format_simple_statement(node),
            SyntaxKind::EXPR_STMT => self.format_simple_statement(node),
            SyntaxKind::IF_EXPR => self.format_if(node),
            SyntaxKind::WHILE_EXPR => self.format_while(node),
            SyntaxKind::PARAM_LIST => self.format_param_list(node),
            SyntaxKind::IMPL_DEF => self.format_impl(node),
            SyntaxKind::TRAIT_DEF => self.format_trait(node),
            _ => self.format_children_inline(node),
        }
    }

    fn format_source_file(&mut self, node: &SyntaxNode) {
        let items = node.children();
        for (i, child) in items.iter().enumerate() {
            if i > 0 && self.config.blank_line_between_items && self.is_top_level_item(child) {
                self.newline();
            }
            self.format_node(child);
            self.newline();
        }
    }

    fn format_function(&mut self, node: &SyntaxNode) {
        self.write_indent();
        for elem in node.children_with_tokens() {
            match elem {
                SyntaxElement::Token(ref t) if t.kind().is_trivia() => {
                    if t.kind() == SyntaxKind::NEWLINE {
                        continue;
                    }
                    if t.kind() == SyntaxKind::WHITESPACE {
                        continue;
                    }
                }
                SyntaxElement::Token(ref t) => {
                    self.ensure_space_before_token(t);
                    self.output.push_str(t.text());
                    self.at_line_start = false;
                    self.last_kind = Some(t.kind());
                }
                SyntaxElement::Node(ref n) => {
                    if n.kind() == SyntaxKind::BLOCK {
                        if self.config.space_before_brace {
                            self.space();
                        }
                        self.format_block(n);
                    } else if n.kind() == SyntaxKind::PARAM_LIST {
                        self.format_param_list(n);
                    } else {
                        self.format_node(n);
                    }
                }
            }
        }
    }

    fn format_param_list(&mut self, node: &SyntaxNode) {
        let tokens: Vec<_> = node.children_with_tokens().into_iter()
            .filter(|e| !e.kind().is_trivia())
            .collect();

        for elem in &tokens {
            match elem {
                SyntaxElement::Token(t) => {
                    if t.kind() == SyntaxKind::COMMA {
                        self.output.push(',');
                        if self.config.space_after_comma {
                            self.space();
                        }
                    } else if t.kind() == SyntaxKind::COLON {
                        self.output.push(':');
                        if self.config.space_after_colon {
                            self.space();
                        }
                    } else {
                        self.output.push_str(t.text());
                    }
                    self.at_line_start = false;
                    self.last_kind = Some(t.kind());
                }
                SyntaxElement::Node(n) => {
                    self.format_node(n);
                }
            }
        }
    }

    fn format_block(&mut self, node: &SyntaxNode) {
        self.output.push('{');
        self.at_line_start = false;

        let children = node.children();
        if children.is_empty() {
            self.output.push('}');
            return;
        }

        self.newline();
        self.indent += 1;

        for child in &children {
            self.format_node(child);
            self.newline();
        }

        self.indent -= 1;
        self.write_indent();
        self.output.push('}');
        self.at_line_start = false;
        self.last_kind = Some(SyntaxKind::RBRACE);
    }

    fn format_simple_statement(&mut self, node: &SyntaxNode) {
        self.write_indent();
        let elements: Vec<_> = node.children_with_tokens().into_iter()
            .filter(|e| !e.kind().is_trivia())
            .collect();

        for elem in &elements {
            match elem {
                SyntaxElement::Token(t) => {
                    self.ensure_space_before_token(t);
                    self.output.push_str(t.text());
                    self.at_line_start = false;
                    self.last_kind = Some(t.kind());
                }
                SyntaxElement::Node(n) => {
                    if n.kind() == SyntaxKind::BLOCK {
                        if self.config.space_before_brace {
                            self.space();
                        }
                        self.format_block(n);
                    } else {
                        self.format_children_inline(n);
                    }
                }
            }
        }
    }

    fn format_if(&mut self, node: &SyntaxNode) {
        self.write_indent();
        for elem in node.children_with_tokens() {
            match elem {
                SyntaxElement::Token(ref t) if t.kind().is_trivia() => continue,
                SyntaxElement::Token(ref t) => {
                    self.ensure_space_before_token(t);
                    self.output.push_str(t.text());
                    self.at_line_start = false;
                    self.last_kind = Some(t.kind());
                }
                SyntaxElement::Node(ref n) => {
                    if n.kind() == SyntaxKind::BLOCK {
                        if self.config.space_before_brace {
                            self.space();
                        }
                        self.format_block(n);
                    } else {
                        self.space();
                        self.format_children_inline(n);
                    }
                }
            }
        }
    }

    fn format_while(&mut self, node: &SyntaxNode) {
        self.format_if(node);
    }

    fn format_struct(&mut self, node: &SyntaxNode) {
        self.write_indent();
        // Emit keywords and name
        for elem in node.children_with_tokens() {
            match elem {
                SyntaxElement::Token(ref t) if t.kind().is_trivia() => continue,
                SyntaxElement::Token(ref t) if t.kind() == SyntaxKind::LBRACE => {
                    if self.config.space_before_brace {
                        self.space();
                    }
                    self.output.push('{');
                    self.at_line_start = false;
                    self.newline();
                    self.indent += 1;
                }
                SyntaxElement::Token(ref t) if t.kind() == SyntaxKind::RBRACE => {
                    self.indent -= 1;
                    self.write_indent();
                    self.output.push('}');
                    self.at_line_start = false;
                }
                SyntaxElement::Token(ref t) if t.kind() == SyntaxKind::COMMA => {
                    self.output.push(',');
                    self.newline();
                }
                SyntaxElement::Token(ref t) => {
                    self.ensure_space_before_token(t);
                    self.output.push_str(t.text());
                    self.at_line_start = false;
                    self.last_kind = Some(t.kind());
                }
                SyntaxElement::Node(ref n) => {
                    if n.kind() == SyntaxKind::FIELD_DEF {
                        self.write_indent();
                        self.format_children_inline(n);
                    } else {
                        self.format_node(n);
                    }
                }
            }
        }
    }

    fn format_enum(&mut self, node: &SyntaxNode) {
        self.format_struct(node);
    }

    fn format_impl(&mut self, node: &SyntaxNode) {
        self.format_function(node);
    }

    fn format_trait(&mut self, node: &SyntaxNode) {
        self.format_function(node);
    }

    fn format_children_inline(&mut self, node: &SyntaxNode) {
        for elem in node.children_with_tokens() {
            match elem {
                SyntaxElement::Token(ref t) if t.kind().is_trivia() => {
                    if t.kind() == SyntaxKind::WHITESPACE && !self.at_line_start {
                        self.space();
                    }
                }
                SyntaxElement::Token(ref t) => {
                    if self.config.space_around_operators && self.is_operator(t.kind()) {
                        self.space();
                        self.output.push_str(t.text());
                        self.space();
                    } else {
                        self.output.push_str(t.text());
                    }
                    self.at_line_start = false;
                    self.last_kind = Some(t.kind());
                }
                SyntaxElement::Node(ref n) => {
                    self.format_node(n);
                }
            }
        }
    }

    fn ensure_space_before_token(&mut self, token: &SyntaxToken) {
        if self.at_line_start {
            return;
        }
        let kind = token.kind();
        // Space before most tokens except punctuation openers.
        if kind != SyntaxKind::LPAREN
            && kind != SyntaxKind::SEMICOLON
            && kind != SyntaxKind::COMMA
            && kind != SyntaxKind::RPAREN
            && kind != SyntaxKind::RBRACE
            && kind != SyntaxKind::RBRACKET
            && kind != SyntaxKind::DOT
            && kind != SyntaxKind::COLON
        {
            if let Some(last) = self.last_kind {
                if last != SyntaxKind::LPAREN
                    && last != SyntaxKind::LBRACE
                    && last != SyntaxKind::LBRACKET
                    && last != SyntaxKind::DOT
                {
                    self.space();
                }
            }
        }
    }

    fn is_operator(&self, kind: SyntaxKind) -> bool {
        kind.0 >= 50 && kind.0 < 80
    }

    fn is_top_level_item(&self, node: &SyntaxNode) -> bool {
        matches!(
            node.kind(),
            SyntaxKind::FUNCTION
                | SyntaxKind::STRUCT_DEF
                | SyntaxKind::ENUM_DEF
                | SyntaxKind::IMPL_DEF
                | SyntaxKind::TRAIT_DEF
                | SyntaxKind::USE_DECL
                | SyntaxKind::MOD_DECL
        )
    }

    fn space(&mut self) {
        if !self.output.ends_with(' ') && !self.output.ends_with('\n') {
            self.output.push(' ');
        }
    }

    fn newline(&mut self) {
        if !self.output.ends_with('\n') {
            self.output.push('\n');
        }
        self.at_line_start = true;
        self.last_kind = None;
    }

    fn write_indent(&mut self) {
        if self.at_line_start {
            let indent_str = self.config.indent_str();
            for _ in 0..self.indent {
                self.output.push_str(&indent_str);
            }
            self.at_line_start = false;
        }
    }
}
