use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use text_size::TextRange;

/// A scope identifier.
pub type ScopeId = usize;

/// A lexical scope containing symbol bindings.
#[derive(Debug, Clone)]
pub struct Scope {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    pub range: TextRange,
    /// Map from name to symbol table indices.
    pub bindings: FxHashMap<SmolStr, Vec<usize>>,
}

impl Scope {
    pub fn new(id: ScopeId, parent: Option<ScopeId>, range: TextRange) -> Self {
        Self {
            id,
            parent,
            range,
            bindings: FxHashMap::default(),
        }
    }

    pub fn define(&mut self, name: SmolStr, symbol_id: usize) {
        self.bindings.entry(name).or_default().push(symbol_id);
    }

    pub fn lookup_local(&self, name: &str) -> Option<usize> {
        self.bindings.get(name).and_then(|ids| ids.last().copied())
    }
}

/// A tree of lexical scopes.
#[derive(Debug, Clone, Default)]
pub struct ScopeTree {
    scopes: Vec<Scope>,
}

impl ScopeTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn root_scope(&mut self, range: TextRange) -> ScopeId {
        let id = self.scopes.len();
        self.scopes.push(Scope::new(id, None, range));
        id
    }

    pub fn child_scope(&mut self, parent: ScopeId, range: TextRange) -> ScopeId {
        let id = self.scopes.len();
        self.scopes.push(Scope::new(id, Some(parent), range));
        id
    }

    pub fn get(&self, id: ScopeId) -> Option<&Scope> {
        self.scopes.get(id)
    }

    pub fn get_mut(&mut self, id: ScopeId) -> Option<&mut Scope> {
        self.scopes.get_mut(id)
    }

    /// Look up a name starting from a scope, walking up parent scopes.
    pub fn resolve(&self, name: &str, start_scope: ScopeId) -> Option<usize> {
        let mut scope_id = Some(start_scope);
        while let Some(id) = scope_id {
            if let Some(scope) = self.get(id) {
                if let Some(symbol_id) = scope.lookup_local(name) {
                    return Some(symbol_id);
                }
                scope_id = scope.parent;
            } else {
                break;
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.scopes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.scopes.is_empty()
    }
}
