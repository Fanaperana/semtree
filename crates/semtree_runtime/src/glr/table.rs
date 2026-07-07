use rustc_hash::{FxHashMap, FxHashSet};
use semtree_grammar::{Grammar, RuleExpr};
use smol_str::SmolStr;
use std::collections::BTreeSet;

/// A terminal or non-terminal symbol in the grammar.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Symbol {
    Terminal(SmolStr),
    NonTerminal(SmolStr),
    /// End of input marker.
    Eof,
    /// Matches any identifier token.
    IdentTerminal,
    /// Matches any integer token.
    IntTerminal,
    /// Matches any float token.
    FloatTerminal,
    /// Matches any string token.
    StringTerminal,
    /// Epsilon (empty production).
    Epsilon,
}

/// A single production: NonTerminal → [Symbol...]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Production {
    pub lhs: SmolStr,
    pub rhs: Vec<Symbol>,
    pub id: usize,
}

/// An LR(0) item: a production with a dot position.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LRItem {
    pub prod_id: usize,
    pub dot: usize,
    pub lookahead: Symbol,
}

impl LRItem {
    pub fn is_complete(&self, prods: &[Production]) -> bool {
        self.dot >= prods[self.prod_id].rhs.len()
    }

    pub fn next_symbol<'a>(&self, prods: &'a [Production]) -> Option<&'a Symbol> {
        let rhs = &prods[self.prod_id].rhs;
        if self.dot < rhs.len() {
            Some(&rhs[self.dot])
        } else {
            None
        }
    }

    pub fn advance(&self) -> Self {
        LRItem {
            prod_id: self.prod_id,
            dot: self.dot + 1,
            lookahead: self.lookahead.clone(),
        }
    }
}

/// A set of LR(1) items representing a parser state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSet {
    pub items: BTreeSet<LRItem>,
    pub id: usize,
}

/// Parse table action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Shift(usize),
    Reduce(usize),
    Accept,
    Error,
}

/// Goto table entry.
#[derive(Debug, Clone)]
pub struct GotoEntry {
    pub state: usize,
    pub symbol: SmolStr,
    pub target: usize,
}

/// The compiled GLR parse table.
#[derive(Debug, Clone)]
pub struct ParseTable {
    pub productions: Vec<Production>,
    /// action[state][terminal] → list of actions (multiple = GLR conflict)
    pub action: Vec<FxHashMap<Symbol, Vec<Action>>>,
    /// goto[state][non_terminal] → target state
    pub goto: Vec<FxHashMap<SmolStr, usize>>,
    pub state_count: usize,
    pub start_symbol: SmolStr,
    pub has_conflicts: bool,
}

impl ParseTable {
    /// Build a GLR parse table from a Grammar IR.
    pub fn from_grammar(grammar: &Grammar) -> Self {
        let mut builder = TableBuilder::new(grammar);
        builder.build()
    }
}

struct TableBuilder<'g> {
    grammar: &'g Grammar,
    productions: Vec<Production>,
    first_sets: FxHashMap<SmolStr, FxHashSet<Symbol>>,
    follow_sets: FxHashMap<SmolStr, FxHashSet<Symbol>>,
    states: Vec<ItemSet>,
    state_map: FxHashMap<BTreeSet<LRItem>, usize>,
}

impl<'g> TableBuilder<'g> {
    fn new(grammar: &'g Grammar) -> Self {
        Self {
            grammar,
            productions: Vec::new(),
            first_sets: FxHashMap::default(),
            follow_sets: FxHashMap::default(),
            states: Vec::new(),
            state_map: FxHashMap::default(),
        }
    }

    fn build(&mut self) -> ParseTable {
        self.flatten_productions();
        self.compute_first_sets();
        self.compute_follow_sets();
        self.build_lr1_states();
        self.build_tables()
    }

    /// Flatten Grammar IR rules into a list of flat productions.
    fn flatten_productions(&mut self) {
        let start = self
            .grammar
            .entry_rule
            .clone()
            .unwrap_or_else(|| "source_file".into());

        // Augmented start: S' → S $
        self.productions.push(Production {
            lhs: "__start__".into(),
            rhs: vec![Symbol::NonTerminal(start.clone()), Symbol::Eof],
            id: 0,
        });

        let rule_names: Vec<SmolStr> = self.grammar.rules.keys().cloned().collect();
        for name in &rule_names {
            let rule = &self.grammar.rules[name];
            let alternatives = self.expr_to_alternatives(&rule.expr);
            for alt in alternatives {
                let id = self.productions.len();
                self.productions.push(Production {
                    lhs: name.clone(),
                    rhs: alt,
                    id,
                });
            }
        }
    }

    /// Convert a RuleExpr into a list of alternative right-hand sides.
    fn expr_to_alternatives(&self, expr: &RuleExpr) -> Vec<Vec<Symbol>> {
        match expr {
            RuleExpr::Literal(s) => vec![vec![Symbol::Terminal(s.clone())]],
            RuleExpr::RuleRef(name) => {
                let sym = match name.as_str() {
                    "Identifier" | "identifier" | "_identifier" => Symbol::IdentTerminal,
                    "Integer" | "integer" | "number" => Symbol::IntTerminal,
                    "Float" | "float" => Symbol::FloatTerminal,
                    "String" | "string" => Symbol::StringTerminal,
                    _ => Symbol::NonTerminal(name.clone()),
                };
                vec![vec![sym]]
            }
            RuleExpr::Seq(exprs) => {
                let mut result = vec![vec![]];
                for e in exprs {
                    let sub_alts = self.expr_to_alternatives(e);
                    let mut new_result = Vec::new();
                    for existing in &result {
                        for sub in &sub_alts {
                            let mut combined = existing.clone();
                            combined.extend(sub.iter().cloned());
                            new_result.push(combined);
                        }
                    }
                    result = new_result;
                }
                result
            }
            RuleExpr::Choice(exprs) => {
                let mut all = Vec::new();
                for e in exprs {
                    all.extend(self.expr_to_alternatives(e));
                }
                all
            }
            RuleExpr::Optional(inner) => {
                let mut alts = self.expr_to_alternatives(inner);
                alts.push(vec![]); // epsilon alternative
                alts
            }
            RuleExpr::Repeat(inner) => {
                // A* → ε | A A*
                // We create a helper non-terminal for this.
                // For simplicity in table building, treat as optional(A+)
                // which gives: ε | inner+
                let inner_alts = self.expr_to_alternatives(inner);
                let mut alts = vec![vec![]]; // epsilon
                alts.extend(inner_alts);
                alts
            }
            RuleExpr::Repeat1(inner) => self.expr_to_alternatives(inner),
            RuleExpr::Field(_, inner) => self.expr_to_alternatives(inner),
            RuleExpr::Token(inner) => self.expr_to_alternatives(inner),
            RuleExpr::Prec(_, inner)
            | RuleExpr::PrecLeft(_, inner)
            | RuleExpr::PrecRight(_, inner) => self.expr_to_alternatives(inner),
            RuleExpr::Blank => vec![vec![]],
        }
    }

    fn compute_first_sets(&mut self) {
        let nt_names: Vec<SmolStr> = self.grammar.rules.keys().cloned().collect();
        for name in &nt_names {
            self.first_sets
                .entry(name.clone())
                .or_insert_with(FxHashSet::default);
        }

        let mut changed = true;
        while changed {
            changed = false;
            for prod in &self.productions {
                if prod.lhs.as_str() == "__start__" {
                    continue;
                }
                let first_of_rhs = self.first_of_sequence(&prod.rhs);
                let entry = self
                    .first_sets
                    .entry(prod.lhs.clone())
                    .or_insert_with(FxHashSet::default);
                for sym in first_of_rhs {
                    if entry.insert(sym) {
                        changed = true;
                    }
                }
            }
        }
    }

    fn first_of_sequence(&self, symbols: &[Symbol]) -> FxHashSet<Symbol> {
        let mut result = FxHashSet::default();
        if symbols.is_empty() {
            result.insert(Symbol::Epsilon);
            return result;
        }

        for sym in symbols {
            let first = self.first_of_symbol(sym);
            let has_epsilon = first.contains(&Symbol::Epsilon);
            for s in first {
                if s != Symbol::Epsilon {
                    result.insert(s);
                }
            }
            if !has_epsilon {
                return result;
            }
        }
        result.insert(Symbol::Epsilon);
        result
    }

    fn first_of_symbol(&self, sym: &Symbol) -> FxHashSet<Symbol> {
        let mut result = FxHashSet::default();
        match sym {
            Symbol::NonTerminal(name) => {
                if let Some(set) = self.first_sets.get(name) {
                    result.extend(set.iter().cloned());
                }
            }
            Symbol::Epsilon => {
                result.insert(Symbol::Epsilon);
            }
            other => {
                result.insert(other.clone());
            }
        }
        result
    }

    fn compute_follow_sets(&mut self) {
        let nt_names: Vec<SmolStr> = self.grammar.rules.keys().cloned().collect();
        for name in &nt_names {
            self.follow_sets
                .entry(name.clone())
                .or_insert_with(FxHashSet::default);
        }

        // FOLLOW(start) includes $
        let start = self
            .grammar
            .entry_rule
            .clone()
            .unwrap_or_else(|| "source_file".into());
        self.follow_sets
            .entry(start)
            .or_insert_with(FxHashSet::default)
            .insert(Symbol::Eof);

        let mut changed = true;
        while changed {
            changed = false;
            let prods: Vec<Production> = self.productions.clone();
            for prod in &prods {
                for (i, sym) in prod.rhs.iter().enumerate() {
                    if let Symbol::NonTerminal(name) = sym {
                        let rest = &prod.rhs[i + 1..];
                        let first_rest = self.first_of_sequence(rest);

                        let entry = self
                            .follow_sets
                            .entry(name.clone())
                            .or_insert_with(FxHashSet::default);

                        for s in &first_rest {
                            if *s != Symbol::Epsilon && entry.insert(s.clone()) {
                                changed = true;
                            }
                        }

                        if first_rest.contains(&Symbol::Epsilon) {
                            let follow_lhs: Vec<Symbol> = self
                                .follow_sets
                                .get(&prod.lhs)
                                .cloned()
                                .unwrap_or_default()
                                .into_iter()
                                .collect();
                            let entry = self
                                .follow_sets
                                .entry(name.clone())
                                .or_insert_with(FxHashSet::default);
                            for s in follow_lhs {
                                if entry.insert(s) {
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn build_lr1_states(&mut self) {
        let initial_item = LRItem {
            prod_id: 0,
            dot: 0,
            lookahead: Symbol::Eof,
        };
        let mut initial_items = BTreeSet::new();
        initial_items.insert(initial_item);
        let initial_closure = self.closure(&initial_items);

        self.states.push(ItemSet {
            items: initial_closure.clone(),
            id: 0,
        });
        self.state_map.insert(initial_closure, 0);

        let mut worklist = vec![0usize];
        let mut iterations = 0u32;

        while let Some(state_id) = worklist.pop() {
            iterations += 1;
            if iterations > 50_000 {
                break;
            }

            let items = self.states[state_id].items.clone();
            let symbols = self.symbols_after_dot(&items);

            for sym in symbols {
                let goto_items = self.goto_set(&items, &sym);
                if goto_items.is_empty() {
                    continue;
                }

                let goto_closure = self.closure(&goto_items);
                if goto_closure.is_empty() {
                    continue;
                }

                if !self.state_map.contains_key(&goto_closure) {
                    let new_id = self.states.len();
                    self.state_map.insert(goto_closure.clone(), new_id);
                    self.states.push(ItemSet {
                        items: goto_closure,
                        id: new_id,
                    });
                    worklist.push(new_id);
                }
            }
        }
    }

    fn closure(&self, items: &BTreeSet<LRItem>) -> BTreeSet<LRItem> {
        let mut result = items.clone();
        let mut worklist: Vec<LRItem> = items.iter().cloned().collect();
        let mut iterations = 0u32;

        while let Some(item) = worklist.pop() {
            iterations += 1;
            if iterations > 100_000 {
                break;
            }

            if let Some(sym) = item.next_symbol(&self.productions) {
                if let Symbol::NonTerminal(name) = sym {
                    // Find all productions for this non-terminal.
                    let rest = &self.productions[item.prod_id].rhs[item.dot + 1..];
                    let mut lookaheads = self.first_of_sequence(rest);
                    if lookaheads.contains(&Symbol::Epsilon) {
                        lookaheads.remove(&Symbol::Epsilon);
                        lookaheads.insert(item.lookahead.clone());
                    }

                    for prod in &self.productions {
                        if prod.lhs == *name {
                            for la in &lookaheads {
                                let new_item = LRItem {
                                    prod_id: prod.id,
                                    dot: 0,
                                    lookahead: la.clone(),
                                };
                                if result.insert(new_item.clone()) {
                                    worklist.push(new_item);
                                }
                            }
                        }
                    }
                }
            }
        }
        result
    }

    fn goto_set(&self, items: &BTreeSet<LRItem>, symbol: &Symbol) -> BTreeSet<LRItem> {
        let mut result = BTreeSet::new();
        for item in items {
            if let Some(next) = item.next_symbol(&self.productions) {
                if next == symbol {
                    result.insert(item.advance());
                }
            }
        }
        result
    }

    fn symbols_after_dot(&self, items: &BTreeSet<LRItem>) -> Vec<Symbol> {
        let mut seen = FxHashSet::default();
        let mut result = Vec::new();
        for item in items {
            if let Some(sym) = item.next_symbol(&self.productions) {
                if seen.insert(sym.clone()) {
                    result.push(sym.clone());
                }
            }
        }
        result
    }

    fn build_tables(&self) -> ParseTable {
        let num_states = self.states.len();
        let mut action: Vec<FxHashMap<Symbol, Vec<Action>>> =
            vec![FxHashMap::default(); num_states];
        let mut goto: Vec<FxHashMap<SmolStr, usize>> = vec![FxHashMap::default(); num_states];
        let mut has_conflicts = false;

        for state in &self.states {
            let sid = state.id;

            for item in &state.items {
                if item.is_complete(&self.productions) {
                    let prod = &self.productions[item.prod_id];
                    if prod.lhs.as_str() == "__start__" {
                        action
                            .get_mut(sid)
                            .unwrap()
                            .entry(Symbol::Eof)
                            .or_default()
                            .push(Action::Accept);
                    } else {
                        let la = &item.lookahead;
                        let actions = action
                            .get_mut(sid)
                            .unwrap()
                            .entry(la.clone())
                            .or_default();
                        let new_action = Action::Reduce(item.prod_id);
                        if !actions.contains(&new_action) {
                            if !actions.is_empty() {
                                has_conflicts = true;
                            }
                            actions.push(new_action);
                        }
                    }
                } else if let Some(sym) = item.next_symbol(&self.productions) {
                    match sym {
                        Symbol::NonTerminal(name) => {
                            let goto_items = self.goto_set(&state.items, sym);
                            if !goto_items.is_empty() {
                                let closure = self.closure(&goto_items);
                                if let Some(&target) = self.state_map.get(&closure) {
                                    goto.get_mut(sid).unwrap().insert(name.clone(), target);
                                }
                            }
                        }
                        terminal => {
                            let goto_items = self.goto_set(&state.items, terminal);
                            if !goto_items.is_empty() {
                                let closure = self.closure(&goto_items);
                                if let Some(&target) = self.state_map.get(&closure) {
                                    let actions = action
                                        .get_mut(sid)
                                        .unwrap()
                                        .entry(terminal.clone())
                                        .or_default();
                                    let new_action = Action::Shift(target);
                                    if !actions.contains(&new_action) {
                                        if !actions.is_empty() {
                                            has_conflicts = true;
                                        }
                                        actions.push(new_action);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let start = self
            .grammar
            .entry_rule
            .clone()
            .unwrap_or_else(|| "source_file".into());

        ParseTable {
            productions: self.productions.clone(),
            action,
            goto,
            state_count: num_states,
            start_symbol: start,
            has_conflicts,
        }
    }
}
