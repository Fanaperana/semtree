use semtree_red::SyntaxNode;
use smol_str::SmolStr;
use text_size::TextRange;

/// A single match from a query execution.
#[derive(Debug, Clone)]
pub struct QueryMatch {
    /// The node that matched the top-level pattern.
    pub node: SyntaxNode,
    /// Named captures within the match.
    pub captures: Vec<QueryCapture>,
}

impl QueryMatch {
    pub fn get_capture(&self, name: &str) -> Option<&QueryCapture> {
        self.captures.iter().find(|c| c.name.as_str() == name)
    }

    pub fn capture_text(&self, name: &str) -> Option<String> {
        self.get_capture(name).map(|c| c.node.text())
    }
}

/// A named capture within a query match.
#[derive(Debug, Clone)]
pub struct QueryCapture {
    pub name: SmolStr,
    pub node: SyntaxNode,
}

impl QueryCapture {
    pub fn text(&self) -> String {
        self.node.text()
    }

    pub fn range(&self) -> TextRange {
        self.node.text_range()
    }
}
