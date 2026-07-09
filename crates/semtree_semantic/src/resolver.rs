use semtree_core::SyntaxKind;
use semtree_red::{SyntaxElement, SyntaxNode};
use smol_str::SmolStr;
use text_size::TextRange;

use crate::diagnostics::Diagnostic;
use crate::scope::{ScopeId, ScopeTree};
use crate::symbols::{Symbol, SymbolKind, SymbolTable};

/// The semantic model: builds symbols, scopes, and diagnostics from a syntax tree.
pub struct SemanticModel {
    pub symbols: SymbolTable,
    pub scopes: ScopeTree,
    pub diagnostics: Vec<Diagnostic>,
    pub references: Vec<Reference>,
}

/// A reference from one location to a symbol.
#[derive(Debug, Clone)]
pub struct Reference {
    pub range: TextRange,
    pub target_symbol: usize,
}

impl SemanticModel {
    /// Analyze a syntax tree and build the full semantic model.
    pub fn analyze(root: &SyntaxNode) -> Self {
        let mut model = Self {
            symbols: SymbolTable::new(),
            scopes: ScopeTree::new(),
            diagnostics: Vec::new(),
            references: Vec::new(),
        };

        let root_scope = model.scopes.root_scope(root.text_range());
        model.visit_node(root, root_scope);
        model
    }

    /// Find all references to a symbol by name.
    pub fn find_references(&self, name: &str) -> Vec<&Reference> {
        let symbol_ids: Vec<usize> = self
            .symbols
            .all()
            .iter()
            .enumerate()
            .filter(|(_, s)| s.name == name)
            .map(|(i, _)| i)
            .collect();

        self.references
            .iter()
            .filter(|r| symbol_ids.contains(&r.target_symbol))
            .collect()
    }

    /// Get all symbols visible at a given scope.
    pub fn visible_symbols(&self, scope_id: ScopeId) -> Vec<&Symbol> {
        let mut result = Vec::new();
        let mut current = Some(scope_id);

        while let Some(id) = current {
            if let Some(scope) = self.scopes.get(id) {
                for ids in scope.bindings.values() {
                    for &sym_id in ids {
                        if let Some(sym) = self.symbols.get(sym_id) {
                            result.push(sym);
                        }
                    }
                }
                current = scope.parent;
            } else {
                break;
            }
        }

        result
    }

    /// Get all public symbols.
    pub fn public_api(&self) -> Vec<&Symbol> {
        self.symbols.all().iter().filter(|s| s.is_public).collect()
    }

    fn visit_node(&mut self, node: &SyntaxNode, scope: ScopeId) {
        match node.kind() {
            SyntaxKind::FUNCTION => self.visit_function(node, scope),
            SyntaxKind::STRUCT_DEF => self.visit_struct(node, scope),
            SyntaxKind::ENUM_DEF => self.visit_enum(node, scope),
            SyntaxKind::TRAIT_DEF => self.visit_trait(node, scope),
            SyntaxKind::IMPL_DEF => self.visit_impl(node, scope),
            SyntaxKind::BLOCK => self.visit_block(node, scope),
            SyntaxKind::LET_STMT => self.visit_let(node, scope),
            SyntaxKind::RETURN_STMT => self.visit_children(node, scope),
            SyntaxKind::EXPR_STMT => self.visit_children(node, scope),
            SyntaxKind::IF_EXPR => self.visit_children(node, scope),
            _ => self.visit_children(node, scope),
        }
    }

    fn visit_children(&mut self, node: &SyntaxNode, scope: ScopeId) {
        for child in node.children() {
            self.visit_node(&child, scope);
        }

        // Track identifier references.
        for elem in node.children_with_tokens() {
            if let SyntaxElement::Token(tok) = elem
                && tok.kind() == SyntaxKind::IDENT
            {
                let name = tok.text();
                if let Some(sym_id) = self.scopes.resolve(name, scope) {
                    self.references.push(Reference {
                        range: tok.text_range(),
                        target_symbol: sym_id,
                    });
                }
            }
        }
    }

    fn visit_function(&mut self, node: &SyntaxNode, scope: ScopeId) {
        let is_public = self.has_keyword(node, SyntaxKind::KW_PUB);

        if let Some(name_tok) = node.child_token(SyntaxKind::IDENT) {
            let name: SmolStr = name_tok.text().into();
            let sym_id = self.symbols.add(Symbol {
                name: name.clone(),
                kind: SymbolKind::Function,
                range: node.text_range(),
                scope,
                is_public,
                is_mutable: false,
            });

            if let Some(scope_data) = self.scopes.get_mut(scope) {
                scope_data.define(name, sym_id);
            }
        }

        // Create a new scope for the function body.
        let fn_scope = self.scopes.child_scope(scope, node.text_range());

        // Register parameters.
        if let Some(param_list) = node.child_node(SyntaxKind::PARAM_LIST) {
            for param in param_list.children() {
                if param.kind() == SyntaxKind::PARAM
                    && let Some(name_tok) = param.child_token(SyntaxKind::IDENT)
                {
                    let name: SmolStr = name_tok.text().into();
                    let sym_id = self.symbols.add(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Parameter,
                        range: param.text_range(),
                        scope: fn_scope,
                        is_public: false,
                        is_mutable: false,
                    });
                    if let Some(scope_data) = self.scopes.get_mut(fn_scope) {
                        scope_data.define(name, sym_id);
                    }
                }
            }
        }

        // Visit body.
        for child in node.children() {
            if child.kind() == SyntaxKind::BLOCK {
                self.visit_block(&child, fn_scope);
            }
        }
    }

    fn visit_block(&mut self, node: &SyntaxNode, parent_scope: ScopeId) {
        let block_scope = self.scopes.child_scope(parent_scope, node.text_range());
        for child in node.children() {
            self.visit_node(&child, block_scope);
        }
    }

    fn visit_let(&mut self, node: &SyntaxNode, scope: ScopeId) {
        let is_mutable = self.has_keyword(node, SyntaxKind::KW_MUT);

        if let Some(name_tok) = node.child_token(SyntaxKind::IDENT) {
            let name: SmolStr = name_tok.text().into();
            let sym_id = self.symbols.add(Symbol {
                name: name.clone(),
                kind: SymbolKind::Variable,
                range: node.text_range(),
                scope,
                is_public: false,
                is_mutable,
            });
            if let Some(scope_data) = self.scopes.get_mut(scope) {
                scope_data.define(name, sym_id);
            }
        }

        self.visit_children(node, scope);
    }

    fn visit_struct(&mut self, node: &SyntaxNode, scope: ScopeId) {
        let is_public = self.has_keyword(node, SyntaxKind::KW_PUB);

        if let Some(name_tok) = node.child_token(SyntaxKind::IDENT) {
            let name: SmolStr = name_tok.text().into();
            let sym_id = self.symbols.add(Symbol {
                name: name.clone(),
                kind: SymbolKind::Struct,
                range: node.text_range(),
                scope,
                is_public,
                is_mutable: false,
            });
            if let Some(scope_data) = self.scopes.get_mut(scope) {
                scope_data.define(name, sym_id);
            }
        }

        // Register fields.
        for child in node.children() {
            if child.kind() == SyntaxKind::FIELD_DEF
                && let Some(name_tok) = child.child_token(SyntaxKind::IDENT)
            {
                let name: SmolStr = name_tok.text().into();
                self.symbols.add(Symbol {
                    name,
                    kind: SymbolKind::Field,
                    range: child.text_range(),
                    scope,
                    is_public: false,
                    is_mutable: false,
                });
            }
        }
    }

    fn visit_enum(&mut self, node: &SyntaxNode, scope: ScopeId) {
        let is_public = self.has_keyword(node, SyntaxKind::KW_PUB);

        if let Some(name_tok) = node.child_token(SyntaxKind::IDENT) {
            let name: SmolStr = name_tok.text().into();
            let sym_id = self.symbols.add(Symbol {
                name: name.clone(),
                kind: SymbolKind::Enum,
                range: node.text_range(),
                scope,
                is_public,
                is_mutable: false,
            });
            if let Some(scope_data) = self.scopes.get_mut(scope) {
                scope_data.define(name, sym_id);
            }
        }

        for child in node.children() {
            if child.kind() == SyntaxKind::VARIANT_DEF
                && let Some(name_tok) = child.child_token(SyntaxKind::IDENT)
            {
                let name: SmolStr = name_tok.text().into();
                self.symbols.add(Symbol {
                    name,
                    kind: SymbolKind::Variant,
                    range: child.text_range(),
                    scope,
                    is_public: false,
                    is_mutable: false,
                });
            }
        }
    }

    fn visit_trait(&mut self, node: &SyntaxNode, scope: ScopeId) {
        let is_public = self.has_keyword(node, SyntaxKind::KW_PUB);

        if let Some(name_tok) = node.child_token(SyntaxKind::IDENT) {
            let name: SmolStr = name_tok.text().into();
            let sym_id = self.symbols.add(Symbol {
                name: name.clone(),
                kind: SymbolKind::Trait,
                range: node.text_range(),
                scope,
                is_public,
                is_mutable: false,
            });
            if let Some(scope_data) = self.scopes.get_mut(scope) {
                scope_data.define(name, sym_id);
            }
        }

        self.visit_children(node, scope);
    }

    fn visit_impl(&mut self, node: &SyntaxNode, scope: ScopeId) {
        let impl_scope = self.scopes.child_scope(scope, node.text_range());
        for child in node.children() {
            self.visit_node(&child, impl_scope);
        }
    }

    fn has_keyword(&self, node: &SyntaxNode, kw: SyntaxKind) -> bool {
        node.children_with_tokens()
            .into_iter()
            .any(|e| e.kind() == kw)
    }
}
