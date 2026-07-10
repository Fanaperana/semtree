use smol_str::SmolStr;
use text_size::{TextRange, TextSize};

use crate::glr::gss::{Gss, GssNodeId};
use crate::glr::sppf::Sppf;
use crate::glr::table::{Action, ParseTable, Symbol};
use crate::runtime_lexer::{RawToken, RuntimeTokenKind};
use crate::runtime_parser::RuntimeParseError;
use semtree_core::SyntaxKind;

/// Recovery tokens that typically start new statements.
const RECOVERY_TOKENS: &[&str] = &[
    ";", "}", ")", "]", "fn", "let", "if", "while", "for", "return", "struct", "enum", "impl",
    "trait", "use", "mod", "pub", "def", "class", "elif", "else", "import", "from", "try",
    "except", "finally", "raise", "with", "pass", "break", "continue",
];

pub struct GlrErrorRecovery;

impl GlrErrorRecovery {
    /// Try to recover from a parse error by:
    /// 1. Skipping tokens until a recovery point is found.
    /// 2. Trying to insert expected tokens (forward repair).
    /// 3. Creating a new GSS node in state 0 as last resort.
    pub fn recover(
        tokens: &[RawToken],
        token_idx: usize,
        sppf: &mut Sppf,
        gss: &mut Gss,
        table: &ParseTable,
        source: &str,
    ) -> Option<(Vec<GssNodeId>, usize, RuntimeParseError)> {
        let start_idx = token_idx;

        // Strategy 1: Skip tokens until we find one the initial state can handle.
        let mut skip_idx = token_idx;
        let mut skipped_sppf_children = Vec::new();

        while skip_idx < tokens.len() && tokens[skip_idx].kind != RuntimeTokenKind::Eof {
            let tok = &tokens[skip_idx];

            // Skip trivia.
            if tok.kind.is_trivia() {
                skip_idx += 1;
                continue;
            }

            // Check if this is a recovery token.
            let is_recovery = RECOVERY_TOKENS.contains(&tok.text(source));

            // Check if state 0 can handle this token.
            let terminal = token_to_symbol(tok, source);
            let can_shift = table
                .action
                .first()
                .and_then(|a| a.get(&terminal))
                .map(|actions| actions.iter().any(|a| matches!(a, Action::Shift(_))))
                .unwrap_or(false);

            if (is_recovery || can_shift) && skip_idx > start_idx {
                break;
            }

            // Create SPPF node for skipped token.
            let syntax_kind = match tok.kind {
                RuntimeTokenKind::Ident | RuntimeTokenKind::Keyword(_) => SyntaxKind::IDENT,
                RuntimeTokenKind::Integer => SyntaxKind::INT_LIT,
                RuntimeTokenKind::Float => SyntaxKind::FLOAT_LIT,
                RuntimeTokenKind::StringLit => SyntaxKind::STRING_LIT,
                _ => SyntaxKind::IDENT,
            };
            let sppf_node = sppf.create_terminal(
                terminal,
                SmolStr::from(tok.text(source)),
                tok.range,
                syntax_kind,
            );
            skipped_sppf_children.push(sppf_node);
            skip_idx += 1;

            if skipped_sppf_children.len() > 50 {
                break;
            }
        }

        if skip_idx <= start_idx || skip_idx >= tokens.len() {
            // Couldn't skip anything useful. Skip one token and restart at state 0.
            let new_idx = (token_idx + 1).min(tokens.len());
            let new_node = gss.create_node(0);
            let epsilon = sppf.create_epsilon();
            gss.add_link(new_node, new_node, epsilon);

            let range = if token_idx < tokens.len() {
                tokens[token_idx].range
            } else {
                let end = tokens
                    .last()
                    .map(|t| t.range.end())
                    .unwrap_or(TextSize::new(0));
                TextRange::new(end, end)
            };

            return Some((
                vec![new_node],
                new_idx,
                RuntimeParseError {
                    message: "unexpected token".to_string(),
                    range,
                },
            ));
        }

        // Create error SPPF node wrapping skipped tokens.
        let range_start = tokens[start_idx].range.start();
        let range_end = tokens[skip_idx.saturating_sub(1)].range.end();
        let error_range = if range_start <= range_end {
            TextRange::new(range_start, range_end)
        } else {
            TextRange::new(range_end, range_start)
        };
        let _error_node = sppf.create_error(
            skipped_sppf_children,
            error_range,
            "unexpected tokens".to_string(),
        );

        // Restart from state 0 at the recovery point.
        let new_node = gss.create_node(0);
        let epsilon = sppf.create_epsilon();
        gss.add_link(new_node, new_node, epsilon);

        Some((
            vec![new_node],
            skip_idx,
            RuntimeParseError {
                message: format!("skipped {} unexpected token(s)", skip_idx - start_idx),
                range: error_range,
            },
        ))
    }
}

fn token_to_symbol(tok: &RawToken, source: &str) -> Symbol {
    match tok.kind {
        RuntimeTokenKind::Keyword(_) | RuntimeTokenKind::Literal(_) => {
            Symbol::Terminal(SmolStr::from(tok.text(source)))
        }
        RuntimeTokenKind::Ident => Symbol::IdentTerminal,
        RuntimeTokenKind::Integer => Symbol::IntTerminal,
        RuntimeTokenKind::Float => Symbol::FloatTerminal,
        RuntimeTokenKind::StringLit => Symbol::StringTerminal,
        RuntimeTokenKind::Eof => Symbol::Eof,
        _ => Symbol::Terminal(SmolStr::from(tok.text(source))),
    }
}
