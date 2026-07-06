use smol_str::SmolStr;
use text_size::TextRange;

use crate::scope::ScopeId;

/// A symbol in the program: a variable, function, type, field, etc.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: SmolStr,
    pub kind: SymbolKind,
    pub range: TextRange,
    pub scope: ScopeId,
    pub is_public: bool,
    pub is_mutable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Variable,
    Function,
    Parameter,
    Struct,
    Enum,
    Variant,
    Field,
    Trait,
    Impl,
    Module,
    TypeAlias,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolKind::Variable => write!(f, "variable"),
            SymbolKind::Function => write!(f, "function"),
            SymbolKind::Parameter => write!(f, "parameter"),
            SymbolKind::Struct => write!(f, "struct"),
            SymbolKind::Enum => write!(f, "enum"),
            SymbolKind::Variant => write!(f, "variant"),
            SymbolKind::Field => write!(f, "field"),
            SymbolKind::Trait => write!(f, "trait"),
            SymbolKind::Impl => write!(f, "impl"),
            SymbolKind::Module => write!(f, "module"),
            SymbolKind::TypeAlias => write!(f, "type alias"),
        }
    }
}

/// A flat table of all symbols found during semantic analysis.
#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    symbols: Vec<Symbol>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, symbol: Symbol) -> usize {
        let id = self.symbols.len();
        self.symbols.push(symbol);
        id
    }

    pub fn get(&self, id: usize) -> Option<&Symbol> {
        self.symbols.get(id)
    }

    pub fn all(&self) -> &[Symbol] {
        &self.symbols
    }

    pub fn find_by_name(&self, name: &str) -> Vec<&Symbol> {
        self.symbols.iter().filter(|s| s.name == name).collect()
    }

    pub fn find_by_kind(&self, kind: SymbolKind) -> Vec<&Symbol> {
        self.symbols.iter().filter(|s| s.kind == kind).collect()
    }

    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }
}
