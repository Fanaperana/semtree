use rustc_hash::FxHashMap;
use semtree_core::SyntaxKind;
use semtree_grammar::Grammar;
use semtree_green::GreenNode;
use semtree_red::SyntaxNode;
use smol_str::SmolStr;
use text_size::{TextRange, TextSize};

use crate::glr::error_recovery::GlrErrorRecovery;
use crate::glr::gss::{Gss, GssNodeId};
use crate::glr::sppf::{Sppf, SppfNodeId};
use crate::glr::table::{Action, ParseTable, Symbol};
use crate::runtime_lexer::{RawToken, RuntimeLexer, RuntimeTokenKind};
use crate::runtime_parser::{RuntimeParseError, rule_name_to_kind};

struct ReduceResult {
    new_active: Vec<GssNodeId>,
    ambiguities: usize,
}

/// A GLR parser that handles ambiguous grammars by maintaining multiple
/// parse stacks simultaneously using a Graph-Structured Stack (GSS)
/// and building a Shared Packed Parse Forest (SPPF).
pub struct GlrParser {
    table: ParseTable,
    lexer: RuntimeLexer,
    grammar: Grammar,
}

/// Result of a GLR parse.
pub struct GlrParseResult {
    pub green_tree: GreenNode,
    pub errors: Vec<RuntimeParseError>,
    pub ambiguity_count: usize,
    pub kind_names: FxHashMap<SyntaxKind, SmolStr>,
}

impl GlrParseResult {
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green_tree.clone())
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn is_ambiguous(&self) -> bool {
        self.ambiguity_count > 0
    }
}

enum SppfEvent {
    StartNode(SmolStr),
    FinishNode,
    Token(SyntaxKind, SmolStr),
    StartError,
}

impl GlrParser {
    pub fn new(grammar: Grammar) -> Self {
        let table = ParseTable::from_grammar(&grammar);
        let lexer = RuntimeLexer::new(&grammar);
        Self {
            table,
            lexer,
            grammar,
        }
    }

    pub fn has_conflicts(&self) -> bool {
        self.table.has_conflicts
    }

    pub fn state_count(&self) -> usize {
        self.table.state_count
    }

    pub fn parse(&self, source: &str) -> GlrParseResult {
        let tokens = self.lexer.tokenize(source);
        let mut gss = Gss::new();
        let mut sppf = Sppf::new();
        let mut errors = Vec::new();
        let mut ambiguity_count = 0;

        // Initialize with start state 0.
        let start_node = gss.create_node(0);
        let mut active: Vec<GssNodeId> = vec![start_node];

        let mut token_idx = 0;

        // Cap active GSS nodes to prevent exponential blowup on
        // highly ambiguous grammars.  When the cap is hit we prune
        // to the lowest-state (closest to start) nodes.
        const MAX_ACTIVE: usize = 64;

        // Total reduction budget across the entire parse.  If exhausted
        // we fall through to tree building with whatever we have.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);

        loop {
            if active.len() > MAX_ACTIVE {
                active.sort_by_key(|n| gss.state_of(*n));
                active.truncate(MAX_ACTIVE);
            }

            // Bail out if parsing takes too long (ambiguity explosion).
            if std::time::Instant::now() >= deadline {
                errors.push(RuntimeParseError {
                    message:
                        "GLR parse timed out (grammar too ambiguous); consider using --backend rd"
                            .to_string(),
                    range: if token_idx < tokens.len() {
                        tokens[token_idx].range
                    } else {
                        TextRange::new(TextSize::new(0), TextSize::new(0))
                    },
                });
                break;
            }

            if active.is_empty() {
                // All stacks died — try error recovery.
                if token_idx < tokens.len() && tokens[token_idx].kind != RuntimeTokenKind::Eof {
                    let recovery = GlrErrorRecovery::recover(
                        &tokens,
                        token_idx,
                        &mut sppf,
                        &mut gss,
                        &self.table,
                        source,
                    );
                    if let Some((new_active, new_idx, error)) = recovery {
                        active = new_active;
                        token_idx = new_idx;
                        errors.push(error);
                        continue;
                    }
                }
                break;
            }

            // Skip trivia tokens — add them to SPPF but don't shift through parse table.
            while token_idx < tokens.len() && tokens[token_idx].kind.is_trivia() {
                token_idx += 1;
            }

            if token_idx >= tokens.len() {
                break;
            }

            let tok = &tokens[token_idx];
            if tok.kind == RuntimeTokenKind::Eof {
                // Try to accept.
                let mut accepted = false;
                for &gss_node in &active {
                    let state = gss.state_of(gss_node);
                    if let Some(actions) = self.table.action.get(state)
                        && let Some(action_list) = actions.get(&Symbol::Eof)
                    {
                        for action in action_list {
                            if *action == Action::Accept {
                                accepted = true;
                            }
                        }
                    }
                }

                // Do any pending reductions before accepting.
                let reduce_result =
                    self.perform_reductions(&active, &Symbol::Eof, &mut gss, &mut sppf, deadline);
                active = reduce_result.new_active;
                ambiguity_count += reduce_result.ambiguities;

                if accepted || self.check_accept(&active, &gss) {
                    break;
                }

                // Not accepted yet: keep reducing.
                if active.is_empty() {
                    break;
                }
                break;
            }

            // Determine the current terminal symbol for the table lookup.
            let terminal = self.token_to_symbol(tok, source);

            // Phase 1: Perform all reductions.
            let reduce_result =
                self.perform_reductions(&active, &terminal, &mut gss, &mut sppf, deadline);
            active = reduce_result.new_active;
            ambiguity_count += reduce_result.ambiguities;

            // Phase 2: Perform shifts.
            let shift_result = self.perform_shifts(
                &active, &terminal, tok, token_idx, &mut gss, &mut sppf, source,
            );

            if shift_result.is_empty() {
                // No shifts possible — error.
                if tok.kind != RuntimeTokenKind::Eof {
                    errors.push(RuntimeParseError {
                        message: format!("unexpected token '{}'", tok.text(source)),
                        range: tok.range,
                    });

                    let recovery = GlrErrorRecovery::recover(
                        &tokens,
                        token_idx,
                        &mut sppf,
                        &mut gss,
                        &self.table,
                        source,
                    );
                    if let Some((new_active, new_idx, error)) = recovery {
                        active = new_active;
                        token_idx = new_idx;
                        if error.message != errors.last().map(|e| e.message.as_str()).unwrap_or("")
                        {
                            errors.push(error);
                        }
                        continue;
                    }
                }
                break;
            }

            active = shift_result;
            token_idx += 1;
        }

        // Build the green tree from SPPF.
        // If there are SPPF nodes with actual terminals, use the tree builder.
        // Otherwise, fall back to emitting all tokens directly.
        let has_sppf_terminals = (0..sppf.node_count()).any(|i| {
            matches!(
                sppf.get(SppfNodeId(i as u32)),
                crate::glr::sppf::SppfNodeKind::Terminal { .. }
            )
        });
        let green_tree = if has_sppf_terminals {
            self.build_final_tree(source, &tokens, &sppf, &gss, &active)
        } else {
            self.build_trivial_tree(source, &tokens)
        };

        let kind_names = self.build_kind_names();

        GlrParseResult {
            green_tree,
            errors,
            ambiguity_count,
            kind_names,
        }
    }

    fn token_to_symbol(&self, tok: &RawToken, source: &str) -> Symbol {
        match tok.kind {
            RuntimeTokenKind::Keyword(_) | RuntimeTokenKind::Literal(_) => {
                Symbol::Terminal(SmolStr::from(tok.text(source)))
            }
            RuntimeTokenKind::Ident => Symbol::IdentTerminal,
            RuntimeTokenKind::Integer => Symbol::IntTerminal,
            RuntimeTokenKind::Float => Symbol::FloatTerminal,
            RuntimeTokenKind::StringLit => Symbol::StringTerminal,
            RuntimeTokenKind::Eof => Symbol::Eof,
            RuntimeTokenKind::Indent => Symbol::Terminal("INDENT".into()),
            RuntimeTokenKind::Dedent => Symbol::Terminal("DEDENT".into()),
            RuntimeTokenKind::Custom(_) => Symbol::Terminal(SmolStr::from(tok.text(source))),
            _ => Symbol::Terminal(SmolStr::from(tok.text(source))),
        }
    }

    fn token_to_syntax_kind(&self, tok: &RawToken) -> SyntaxKind {
        match tok.kind {
            RuntimeTokenKind::Keyword(_) => SyntaxKind::IDENT,
            RuntimeTokenKind::Literal(_) => SyntaxKind::IDENT,
            RuntimeTokenKind::Ident => SyntaxKind::IDENT,
            RuntimeTokenKind::Integer => SyntaxKind::INT_LIT,
            RuntimeTokenKind::Float => SyntaxKind::FLOAT_LIT,
            RuntimeTokenKind::StringLit => SyntaxKind::STRING_LIT,
            RuntimeTokenKind::Whitespace => SyntaxKind::WHITESPACE,
            RuntimeTokenKind::Newline => SyntaxKind::NEWLINE,
            RuntimeTokenKind::LineComment => SyntaxKind::LINE_COMMENT,
            RuntimeTokenKind::BlockComment => SyntaxKind::BLOCK_COMMENT,
            RuntimeTokenKind::Error => SyntaxKind::ERROR,
            RuntimeTokenKind::Eof => SyntaxKind::EOF,
            RuntimeTokenKind::Indent | RuntimeTokenKind::Dedent | RuntimeTokenKind::Custom(_) => {
                SyntaxKind::IDENT
            }
        }
    }

    fn check_accept(&self, active: &[GssNodeId], gss: &Gss) -> bool {
        for &node in active {
            let state = gss.state_of(node);
            if let Some(actions) = self.table.action.get(state)
                && let Some(action_list) = actions.get(&Symbol::Eof)
                && action_list.contains(&Action::Accept)
            {
                return true;
            }
        }
        false
    }

    fn perform_reductions(
        &self,
        active: &[GssNodeId],
        lookahead: &Symbol,
        gss: &mut Gss,
        sppf: &mut Sppf,
        deadline: std::time::Instant,
    ) -> ReduceResult {
        let mut current = active.to_vec();
        let mut ambiguities = 0;
        let mut changed = true;
        let mut iterations = 0u32;

        while changed && iterations < 200 {
            if std::time::Instant::now() >= deadline {
                break;
            }
            changed = false;
            iterations += 1;
            let snapshot: Vec<GssNodeId> = current.clone();

            for &gss_node in &snapshot {
                if std::time::Instant::now() >= deadline {
                    changed = false;
                    break;
                }
                let state = gss.state_of(gss_node);

                let actions = match self.table.action.get(state) {
                    Some(a) => a,
                    None => continue,
                };

                let mut reduce_actions: Vec<usize> = actions
                    .get(lookahead)
                    .into_iter()
                    .flatten()
                    .filter_map(|a| match a {
                        Action::Reduce(prod_id) => Some(*prod_id),
                        _ => None,
                    })
                    .collect();

                if reduce_actions.len() > 1 {
                    let max_prec = reduce_actions
                        .iter()
                        .map(|id| self.table.productions[*id].prec)
                        .max()
                        .unwrap_or(0);
                    let before = reduce_actions.len();
                    reduce_actions.retain(|id| self.table.productions[*id].prec == max_prec);
                    if reduce_actions.len() < before {
                        ambiguities += before - reduce_actions.len();
                    } else {
                        ambiguities += reduce_actions.len() - 1;
                    }
                }

                for prod_id in reduce_actions {
                    let prod = &self.table.productions[prod_id];
                    let lhs = prod.lhs.clone();
                    let rhs_len = prod.rhs.len();

                    // Walk back `rhs_len` links in the GSS to find predecessor nodes.
                    let paths = gss.paths(gss_node, rhs_len);

                    for path in paths {
                        if path.is_empty() {
                            continue;
                        }

                        let base_node = path[0].0;
                        let base_state = gss.state_of(base_node);

                        let target_state =
                            match self.table.goto.get(base_state).and_then(|g| g.get(&lhs)) {
                                Some(&s) => s,
                                None => continue,
                            };

                        let sppf_children: Vec<SppfNodeId> = path
                            .iter()
                            .skip(1)
                            .map(|(_, sppf_id)| *sppf_id)
                            .filter(|id| id.0 != u32::MAX)
                            .collect();

                        let range = if sppf_children.is_empty() {
                            TextRange::new(TextSize::new(0), TextSize::new(0))
                        } else {
                            let mut start = sppf.range_of(sppf_children[0]).start();
                            let mut end = sppf.range_of(sppf_children[0]).end();
                            for id in &sppf_children[1..] {
                                let r = sppf.range_of(*id);
                                if r.start() < start {
                                    start = r.start();
                                }
                                if r.end() > end {
                                    end = r.end();
                                }
                            }
                            TextRange::new(start, end)
                        };

                        let sppf_node = sppf.create_symbol(lhs.clone(), sppf_children, range);

                        if let Some(existing) = gss.find_node_with_state(target_state, &current) {
                            if gss.add_link(existing, base_node, sppf_node) {
                                changed = true;
                            }
                        } else {
                            let new_node = gss.create_node(target_state);
                            gss.add_link(new_node, base_node, sppf_node);
                            current.push(new_node);
                            changed = true;
                        }
                    }
                }
            }
        }

        ReduceResult {
            new_active: current,
            ambiguities,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn perform_shifts(
        &self,
        active: &[GssNodeId],
        terminal: &Symbol,
        tok: &RawToken,
        _tok_idx: usize,
        gss: &mut Gss,
        sppf: &mut Sppf,
        source: &str,
    ) -> Vec<GssNodeId> {
        let mut new_active = Vec::new();
        let syntax_kind = self.token_to_syntax_kind(tok);

        let sppf_terminal = sppf.create_terminal(
            terminal.clone(),
            SmolStr::from(tok.text(source)),
            tok.range,
            syntax_kind,
        );

        for &gss_node in active {
            let state = gss.state_of(gss_node);

            if let Some(actions) = self.table.action.get(state) {
                // Check both the specific terminal and builtin terminal types.
                let terminals_to_check = self.matching_terminals(terminal, tok, source);

                for check_terminal in &terminals_to_check {
                    if let Some(action_list) = actions.get(check_terminal) {
                        for action in action_list {
                            if let Action::Shift(target) = action {
                                if let Some(existing) =
                                    gss.find_node_with_state(*target, &new_active)
                                {
                                    gss.add_link(existing, gss_node, sppf_terminal);
                                } else {
                                    let new_node = gss.create_node(*target);
                                    gss.add_link(new_node, gss_node, sppf_terminal);
                                    new_active.push(new_node);
                                }
                            }
                        }
                    }
                }
            }
        }

        new_active
    }

    /// For a given token, return all Symbol variants to check in the action table.
    /// A keyword token "def" should match both Terminal("def") and IdentTerminal.
    fn matching_terminals(&self, primary: &Symbol, tok: &RawToken, source: &str) -> Vec<Symbol> {
        let mut result = vec![primary.clone()];
        match tok.kind {
            RuntimeTokenKind::Keyword(_) => {
                result.push(Symbol::IdentTerminal);
            }
            RuntimeTokenKind::Ident => {
                result.push(Symbol::Terminal(SmolStr::from(tok.text(source))));
            }
            _ => {}
        }
        result
    }

    fn build_final_tree(
        &self,
        source: &str,
        tokens: &[RawToken],
        sppf: &Sppf,
        _gss: &Gss,
        _active: &[GssNodeId],
    ) -> GreenNode {
        // Collect all SPPF terminal byte offsets so we can interleave trivia.
        let mut sppf_events = Vec::new();
        if sppf.node_count() > 0 {
            let root = SppfNodeId((sppf.node_count().saturating_sub(1)) as u32);
            self.collect_sppf_events(sppf, root, &mut sppf_events);
        }

        // Build the tree by walking tokens in order, emitting trivia tokens
        // inline with the SPPF content.
        let mut builder = semtree_green::GreenNodeBuilder::new();
        builder.start_node(SyntaxKind::SOURCE_FILE);

        let mut tok_idx = 0;
        let mut evt_idx = 0;

        while tok_idx < tokens.len() || evt_idx < sppf_events.len() {
            // Emit leading trivia.
            while tok_idx < tokens.len() && tokens[tok_idx].kind.is_trivia() {
                let tok = &tokens[tok_idx];
                let kind = match tok.kind {
                    RuntimeTokenKind::Whitespace => SyntaxKind::WHITESPACE,
                    RuntimeTokenKind::Newline => SyntaxKind::NEWLINE,
                    RuntimeTokenKind::LineComment => SyntaxKind::LINE_COMMENT,
                    RuntimeTokenKind::BlockComment => SyntaxKind::BLOCK_COMMENT,
                    _ => SyntaxKind::WHITESPACE,
                };
                builder.token(kind, tok.text(source));
                tok_idx += 1;
            }

            if evt_idx < sppf_events.len() {
                match &sppf_events[evt_idx] {
                    SppfEvent::StartNode(name) => {
                        let kind = rule_name_to_kind(name);
                        builder.start_node(kind);
                    }
                    SppfEvent::FinishNode => {
                        builder.finish_node();
                    }
                    SppfEvent::Token(syntax_kind, text) => {
                        builder.token(*syntax_kind, text.as_str());
                        // Advance past the matching non-trivia token.
                        if tok_idx < tokens.len()
                            && !tokens[tok_idx].kind.is_trivia()
                            && tokens[tok_idx].kind != RuntimeTokenKind::Eof
                        {
                            tok_idx += 1;
                        }
                    }
                    SppfEvent::StartError => {
                        builder.start_node(SyntaxKind::ERROR);
                    }
                }
                evt_idx += 1;
            } else {
                // No more SPPF events, just emit remaining non-trivia tokens.
                if tok_idx < tokens.len() && tokens[tok_idx].kind == RuntimeTokenKind::Eof {
                    break;
                }
                if tok_idx < tokens.len() {
                    tok_idx += 1;
                } else {
                    break;
                }
            }
        }

        builder.finish_node();
        builder.finish()
    }

    fn collect_sppf_events(&self, sppf: &Sppf, id: SppfNodeId, events: &mut Vec<SppfEvent>) {
        use crate::glr::sppf::SppfNodeKind;
        match sppf.get(id) {
            SppfNodeKind::Terminal {
                text, syntax_kind, ..
            } => {
                events.push(SppfEvent::Token(*syntax_kind, text.clone()));
            }
            SppfNodeKind::Symbol { name, children, .. } => {
                events.push(SppfEvent::StartNode(name.clone()));
                for &child in children {
                    self.collect_sppf_events(sppf, child, events);
                }
                events.push(SppfEvent::FinishNode);
            }
            SppfNodeKind::Packed { children, .. } => {
                for &child in children {
                    self.collect_sppf_events(sppf, child, events);
                }
            }
            SppfNodeKind::Error { children, .. } => {
                events.push(SppfEvent::StartError);
                for &child in children {
                    self.collect_sppf_events(sppf, child, events);
                }
                events.push(SppfEvent::FinishNode);
            }
            SppfNodeKind::Epsilon => {}
        }
    }

    #[allow(dead_code)]
    fn emit_trivia_up_to(
        &self,
        tokens: &[RawToken],
        idx: &mut usize,
        end_offset: usize,
        builder: &mut semtree_green::GreenNodeBuilder,
        source: &str,
    ) {
        while *idx < tokens.len() {
            let tok = &tokens[*idx];
            if u32::from(tok.range.start()) as usize >= end_offset {
                break;
            }
            if tok.kind.is_trivia() {
                let kind = match tok.kind {
                    RuntimeTokenKind::Whitespace => SyntaxKind::WHITESPACE,
                    RuntimeTokenKind::Newline => SyntaxKind::NEWLINE,
                    RuntimeTokenKind::LineComment => SyntaxKind::LINE_COMMENT,
                    RuntimeTokenKind::BlockComment => SyntaxKind::BLOCK_COMMENT,
                    _ => SyntaxKind::WHITESPACE,
                };
                builder.token(kind, tok.text(source));
                *idx += 1;
            } else {
                break;
            }
        }
    }

    fn build_trivial_tree(&self, source: &str, tokens: &[RawToken]) -> GreenNode {
        let mut builder = semtree_green::GreenNodeBuilder::new();
        builder.start_node(SyntaxKind::SOURCE_FILE);
        for tok in tokens {
            if tok.kind == RuntimeTokenKind::Eof {
                break;
            }
            let kind = self.token_to_syntax_kind(tok);
            builder.token(kind, tok.text(source));
        }
        builder.finish_node();
        builder.finish()
    }

    fn build_kind_names(&self) -> FxHashMap<SyntaxKind, SmolStr> {
        let mut map = FxHashMap::default();
        map.insert(SyntaxKind::SOURCE_FILE, "source_file".into());
        map.insert(SyntaxKind::ERROR, "ERROR".into());
        map.insert(SyntaxKind::WHITESPACE, "whitespace".into());
        map.insert(SyntaxKind::NEWLINE, "newline".into());
        map.insert(SyntaxKind::LINE_COMMENT, "comment".into());
        map.insert(SyntaxKind::BLOCK_COMMENT, "comment".into());
        map.insert(SyntaxKind::IDENT, "identifier".into());
        map.insert(SyntaxKind::INT_LIT, "integer".into());
        map.insert(SyntaxKind::FLOAT_LIT, "float".into());
        map.insert(SyntaxKind::STRING_LIT, "string".into());

        for name in self.grammar.rules.keys() {
            let kind = rule_name_to_kind(name);
            map.insert(kind, name.clone());
        }
        map
    }
}
