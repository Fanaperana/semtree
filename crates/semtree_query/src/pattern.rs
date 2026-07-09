use semtree_core::SyntaxKind;
use smol_str::SmolStr;

/// A compiled query pattern for matching syntax tree nodes.
///
/// Patterns use an S-expression-like syntax similar to Tree-sitter queries:
///
/// ```text
/// (Function name: (Identifier) @name)
/// (LetStatement (Identifier) @var "=" (Expression) @value)
/// (BinaryExpr left: (_) @left "+" right: (_) @right)
/// ```
#[derive(Debug, Clone)]
pub struct QueryPattern {
    pub nodes: Vec<PatternNode>,
}

#[derive(Debug, Clone)]
pub struct PatternNode {
    /// The SyntaxKind to match, or None for wildcard `(_)`.
    pub kind: Option<SyntaxKind>,
    /// The kind name as a string (for runtime matching by name).
    pub kind_name: Option<SmolStr>,
    /// If set, capture this node with the given name (e.g. `@name`).
    pub capture: Option<SmolStr>,
    /// Required child patterns.
    pub children: Vec<PatternNode>,
    /// Match a specific text value.
    pub text_match: Option<SmolStr>,
    /// Field name this child is expected under.
    pub field_name: Option<SmolStr>,
    /// Predicates to filter matches.
    pub predicates: Vec<PatternPredicate>,
}

impl PatternNode {
    pub fn wildcard() -> Self {
        Self {
            kind: None,
            kind_name: None,
            capture: None,
            children: Vec::new(),
            text_match: None,
            field_name: None,
            predicates: Vec::new(),
        }
    }

    pub fn with_kind(kind: SyntaxKind) -> Self {
        Self {
            kind: Some(kind),
            kind_name: None,
            capture: None,
            children: Vec::new(),
            text_match: None,
            field_name: None,
            predicates: Vec::new(),
        }
    }

    pub fn with_kind_name(name: impl Into<SmolStr>) -> Self {
        Self {
            kind: None,
            kind_name: Some(name.into()),
            capture: None,
            children: Vec::new(),
            text_match: None,
            field_name: None,
            predicates: Vec::new(),
        }
    }

    pub fn capture(mut self, name: impl Into<SmolStr>) -> Self {
        self.capture = Some(name.into());
        self
    }

    pub fn child(mut self, child: PatternNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn text(mut self, text: impl Into<SmolStr>) -> Self {
        self.text_match = Some(text.into());
        self
    }
}

/// Predicates for filtering query matches.
#[derive(Debug, Clone)]
pub enum PatternPredicate {
    /// `#eq? @capture "value"` — captured text equals value.
    Eq(SmolStr, SmolStr),
    /// `#match? @capture "regex"` — captured text matches regex.
    Match(SmolStr, SmolStr),
    /// `#not-eq? @capture "value"`
    NotEq(SmolStr, SmolStr),
}

/// Parse a query from S-expression syntax.
///
/// Syntax:
///   `(KindName child1 child2 @capture)`
///   `(_)` — wildcard
///   `"literal"` — text match
///   `@name` — capture
pub fn parse_query(input: &str) -> Result<QueryPattern, String> {
    let mut parser = QueryParser::new(input);
    let nodes = parser.parse_top_level()?;
    Ok(QueryPattern { nodes })
}

struct QueryParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> QueryParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn parse_top_level(&mut self) -> Result<Vec<PatternNode>, String> {
        let mut nodes = Vec::new();
        self.skip_ws();
        while self.pos < self.input.len() {
            if self.peek() == Some('(') {
                nodes.push(self.parse_node()?);
            } else {
                break;
            }
            self.skip_ws();
        }
        Ok(nodes)
    }

    fn parse_node(&mut self) -> Result<PatternNode, String> {
        self.expect('(')?;
        self.skip_ws();

        // Kind name or wildcard
        let mut node = if self.peek() == Some('_') {
            self.advance();
            PatternNode::wildcard()
        } else {
            let name = self.read_ident()?;
            PatternNode::with_kind_name(name)
        };

        self.skip_ws();

        // Parse children, captures, text matches
        while self.pos < self.input.len() && self.peek() != Some(')') {
            self.skip_ws();
            match self.peek() {
                Some('(') => {
                    let child = self.parse_node()?;
                    node.children.push(child);
                }
                Some('@') => {
                    self.advance(); // skip @
                    let name = self.read_ident()?;
                    node.capture = Some(name.into());
                }
                Some('"') => {
                    let text = self.read_string()?;
                    // Text match applies as a child pattern
                    let text_node = PatternNode::wildcard().text(text);
                    node.children.push(text_node);
                }
                Some(')') => break,
                Some(_) => {
                    // Could be a field name like `name:` followed by a pattern
                    let ident = self.read_ident()?;
                    self.skip_ws();
                    if self.peek() == Some(':') {
                        self.advance(); // skip :
                        self.skip_ws();
                        if self.peek() == Some('(') {
                            let mut child = self.parse_node()?;
                            child.field_name = Some(ident.into());
                            node.children.push(child);
                        }
                    } else if self.peek() == Some('@') {
                        // It was actually a capture on the previous node
                        // Back up — this is a kind name for a simple child
                        let child = PatternNode::with_kind_name(ident);
                        node.children.push(child);
                    } else {
                        let child = PatternNode::with_kind_name(ident);
                        node.children.push(child);
                    }
                }
                None => break,
            }
        }

        self.expect(')')?;
        Ok(node)
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += c.len_utf8();
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        self.skip_ws();
        match self.peek() {
            Some(c) if c == expected => {
                self.advance();
                Ok(())
            }
            Some(c) => Err(format!(
                "expected '{expected}', found '{c}' at position {}",
                self.pos
            )),
            None => Err(format!("expected '{expected}', found end of input")),
        }
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_ident(&mut self) -> Result<String, String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' {
                self.advance();
            } else {
                break;
            }
        }
        if self.pos == start {
            Err(format!("expected identifier at position {}", self.pos))
        } else {
            Ok(self.input[start..self.pos].to_string())
        }
    }

    fn read_string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == '"' {
                let s = self.input[start..self.pos].to_string();
                self.advance(); // consume closing "
                return Ok(s);
            }
            if c == '\\' {
                self.advance();
            }
            self.advance();
        }
        Err("unterminated string".to_string())
    }
}
