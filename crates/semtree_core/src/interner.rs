use rustc_hash::FxHashMap;
use smol_str::SmolStr;

/// A simple string interner for deduplicating identifiers and keywords.
#[derive(Debug, Default)]
pub struct Interner {
    map: FxHashMap<SmolStr, u32>,
    strings: Vec<SmolStr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InternedStr(pub u32);

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, s: &str) -> InternedStr {
        if let Some(&id) = self.map.get(s) {
            return InternedStr(id);
        }
        let id = self.strings.len() as u32;
        let smol: SmolStr = s.into();
        self.strings.push(smol.clone());
        self.map.insert(smol, id);
        InternedStr(id)
    }

    pub fn resolve(&self, id: InternedStr) -> &str {
        &self.strings[id.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}
