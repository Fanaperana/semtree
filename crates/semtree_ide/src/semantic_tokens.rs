use semtree_core::SyntaxKind;
use semtree_red::{SyntaxElement, SyntaxNode};
use semtree_semantic::SemanticModel;
use text_size::TextRange;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticTokenType {
    Keyword,
    Type,
    Function,
    Variable,
    Parameter,
    Property,
    Enum,
    String,
    Number,
    Comment,
    Operator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticTokenModifier {
    Declaration,
    Definition,
    Readonly,
}

#[derive(Debug, Clone)]
pub struct SemanticToken {
    pub range: TextRange,
    pub token_type: SemanticTokenType,
    pub modifiers: Vec<SemanticTokenModifier>,
}

/// Classify all tokens in the tree for semantic highlighting.
pub fn classify_tokens(root: &SyntaxNode, model: &SemanticModel) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    collect_tokens(root, model, &mut tokens);
    tokens
}

fn collect_tokens(node: &SyntaxNode, model: &SemanticModel, out: &mut Vec<SemanticToken>) {
    for elem in node.children_with_tokens() {
        match elem {
            SyntaxElement::Token(tok) => {
                if let Some(st) = classify_single_token(&tok, node, model) {
                    out.push(st);
                }
            }
            SyntaxElement::Node(child) => {
                collect_tokens(&child, model, out);
            }
        }
    }
}

fn classify_single_token(
    tok: &semtree_red::SyntaxToken,
    _parent: &SyntaxNode,
    model: &SemanticModel,
) -> Option<SemanticToken> {
    let kind = tok.kind();
    let range = tok.text_range();

    if kind.is_keyword() {
        return Some(SemanticToken {
            range,
            token_type: SemanticTokenType::Keyword,
            modifiers: Vec::new(),
        });
    }

    if kind == SyntaxKind::LINE_COMMENT || kind == SyntaxKind::BLOCK_COMMENT {
        return Some(SemanticToken {
            range,
            token_type: SemanticTokenType::Comment,
            modifiers: Vec::new(),
        });
    }

    if kind == SyntaxKind::STRING_LIT || kind == SyntaxKind::CHAR_LIT {
        return Some(SemanticToken {
            range,
            token_type: SemanticTokenType::String,
            modifiers: Vec::new(),
        });
    }

    if kind == SyntaxKind::INT_LIT || kind == SyntaxKind::FLOAT_LIT {
        return Some(SemanticToken {
            range,
            token_type: SemanticTokenType::Number,
            modifiers: Vec::new(),
        });
    }

    if is_operator(kind) {
        return Some(SemanticToken {
            range,
            token_type: SemanticTokenType::Operator,
            modifiers: Vec::new(),
        });
    }

    if kind == SyntaxKind::IDENT {
        return Some(classify_ident(tok, model));
    }

    None
}

fn classify_ident(tok: &semtree_red::SyntaxToken, model: &SemanticModel) -> SemanticToken {
    let range = tok.text_range();
    let name = tok.text();

    let symbols = model.symbols.find_by_name(name);
    if let Some(sym) = symbols.first() {
        use semtree_semantic::SymbolKind;
        let (token_type, modifiers) = match sym.kind {
            SymbolKind::Function => {
                let mut mods = Vec::new();
                if sym.range.contains_range(range) {
                    mods.push(SemanticTokenModifier::Definition);
                }
                (SemanticTokenType::Function, mods)
            }
            SymbolKind::Variable => {
                let mut mods = Vec::new();
                if !sym.is_mutable {
                    mods.push(SemanticTokenModifier::Readonly);
                }
                if sym.range.contains_range(range) {
                    mods.push(SemanticTokenModifier::Declaration);
                }
                (SemanticTokenType::Variable, mods)
            }
            SymbolKind::Parameter => (SemanticTokenType::Parameter, Vec::new()),
            SymbolKind::Struct | SymbolKind::TypeAlias => (SemanticTokenType::Type, Vec::new()),
            SymbolKind::Enum => (SemanticTokenType::Enum, Vec::new()),
            SymbolKind::Field => (SemanticTokenType::Property, Vec::new()),
            _ => (SemanticTokenType::Variable, Vec::new()),
        };
        SemanticToken {
            range,
            token_type,
            modifiers,
        }
    } else {
        SemanticToken {
            range,
            token_type: SemanticTokenType::Variable,
            modifiers: Vec::new(),
        }
    }
}

fn is_operator(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::PLUS
            | SyntaxKind::MINUS
            | SyntaxKind::STAR
            | SyntaxKind::SLASH
            | SyntaxKind::PERCENT
            | SyntaxKind::AMP
            | SyntaxKind::PIPE
            | SyntaxKind::CARET
            | SyntaxKind::TILDE
            | SyntaxKind::BANG
            | SyntaxKind::LT
            | SyntaxKind::GT
            | SyntaxKind::EQ
            | SyntaxKind::EQEQ
            | SyntaxKind::NEQ
            | SyntaxKind::LTEQ
            | SyntaxKind::GTEQ
            | SyntaxKind::AMPAMP
            | SyntaxKind::PIPEPIPE
    )
}
