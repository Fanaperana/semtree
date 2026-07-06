use semtree_core::SyntaxKind;

use crate::parser::Parser;

/// Parse a complete source file.
pub(crate) fn source_file(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::SOURCE_FILE);

    while !p.at_end() {
        item(p);
    }

    p.finish_node();
}

fn item(p: &mut Parser) {
    match p.current() {
        SyntaxKind::KW_FN => function(p),
        SyntaxKind::KW_STRUCT => struct_def(p),
        SyntaxKind::KW_ENUM => enum_def(p),
        SyntaxKind::KW_LET => let_stmt(p),
        SyntaxKind::KW_USE => use_decl(p),
        SyntaxKind::KW_MOD => mod_decl(p),
        SyntaxKind::KW_IMPL => impl_def(p),
        SyntaxKind::KW_TRAIT => trait_def(p),
        SyntaxKind::KW_PUB => {
            // `pub` followed by an item
            let _m = p.start_node(SyntaxKind::ERROR);
            p.bump(); // eat `pub`
            p.finish_node();
            item(p);
        }
        _ => {
            p.error_recover("expected item");
        }
    }
}

fn function(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::FUNCTION);
    p.expect(SyntaxKind::KW_FN);

    if p.at(SyntaxKind::IDENT) {
        p.bump();
    } else {
        p.error_here("expected function name");
    }

    if p.at(SyntaxKind::LPAREN) {
        param_list(p);
    }

    // Optional return type: -> Type
    if p.at(SyntaxKind::ARROW) {
        p.bump();
        type_ref(p);
    }

    if p.at(SyntaxKind::LBRACE) {
        block(p);
    } else {
        p.error_here("expected function body");
    }

    p.finish_node();
}

fn param_list(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::PARAM_LIST);
    p.expect(SyntaxKind::LPAREN);

    while !p.at(SyntaxKind::RPAREN) && !p.at_end() {
        param(p);
        if !p.eat(SyntaxKind::COMMA) {
            break;
        }
    }

    p.expect(SyntaxKind::RPAREN);
    p.finish_node();
}

fn param(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::PARAM);

    if p.at(SyntaxKind::IDENT) {
        p.bump();
    } else {
        p.error_here("expected parameter name");
    }

    if p.eat(SyntaxKind::COLON) {
        type_ref(p);
    }

    p.finish_node();
}

fn type_ref(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::TYPE_REF);

    if p.at(SyntaxKind::AMP) {
        p.bump();
        if p.eat(SyntaxKind::KW_MUT) {}
    }

    if p.at(SyntaxKind::IDENT) {
        p.bump();
        // generic args: Type<T>
        if p.at(SyntaxKind::LT) {
            p.bump();
            while !p.at(SyntaxKind::GT) && !p.at_end() {
                type_ref(p);
                if !p.eat(SyntaxKind::COMMA) {
                    break;
                }
            }
            p.expect(SyntaxKind::GT);
        }
    } else {
        p.error_here("expected type");
    }

    p.finish_node();
}

fn block(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::BLOCK);
    p.expect(SyntaxKind::LBRACE);

    while !p.at(SyntaxKind::RBRACE) && !p.at_end() {
        stmt(p);
    }

    p.expect(SyntaxKind::RBRACE);
    p.finish_node();
}

fn stmt(p: &mut Parser) {
    match p.current() {
        SyntaxKind::KW_LET => let_stmt(p),
        SyntaxKind::KW_RETURN => return_stmt(p),
        SyntaxKind::KW_IF => if_expr(p),
        SyntaxKind::KW_WHILE => while_expr(p),
        SyntaxKind::KW_FOR => for_expr(p),
        SyntaxKind::KW_LOOP => loop_expr(p),
        SyntaxKind::KW_FN => function(p),
        SyntaxKind::LBRACE => block(p),
        _ => expr_stmt(p),
    }
}

fn let_stmt(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::LET_STMT);
    p.expect(SyntaxKind::KW_LET);
    p.eat(SyntaxKind::KW_MUT);

    if p.at(SyntaxKind::IDENT) {
        p.bump();
    } else {
        p.error_here("expected variable name");
    }

    if p.eat(SyntaxKind::COLON) {
        type_ref(p);
    }

    if p.eat(SyntaxKind::EQ) {
        expr(p);
    }

    p.eat(SyntaxKind::SEMICOLON);
    p.finish_node();
}

fn return_stmt(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::RETURN_STMT);
    p.expect(SyntaxKind::KW_RETURN);

    if !p.at(SyntaxKind::SEMICOLON) && !p.at(SyntaxKind::RBRACE) && !p.at_end() {
        expr(p);
    }

    p.eat(SyntaxKind::SEMICOLON);
    p.finish_node();
}

fn expr_stmt(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::EXPR_STMT);
    expr(p);
    p.eat(SyntaxKind::SEMICOLON);
    p.finish_node();
}

// ── Expression parsing (Pratt parser) ────────────────────────

fn expr(p: &mut Parser) {
    expr_bp(p, 0);
}

fn expr_bp(p: &mut Parser, min_bp: u8) {
    // Parse the LHS atom; `lhs` is the event position of its StartNode.
    let mut lhs = match p.current() {
        SyntaxKind::INT_LIT | SyntaxKind::FLOAT_LIT | SyntaxKind::STRING_LIT
        | SyntaxKind::CHAR_LIT | SyntaxKind::KW_TRUE | SyntaxKind::KW_FALSE => {
            let m = p.start_node(SyntaxKind::LITERAL);
            p.bump();
            p.finish_node();
            m
        }
        SyntaxKind::IDENT => {
            let m = p.start_node(SyntaxKind::PATH_EXPR);
            p.bump();
            p.finish_node();
            m
        }
        SyntaxKind::LPAREN => {
            let m = p.start_node(SyntaxKind::PAREN_EXPR);
            p.bump();
            expr(p);
            p.expect(SyntaxKind::RPAREN);
            p.finish_node();
            m
        }
        SyntaxKind::LBRACKET => {
            let m = p.start_node(SyntaxKind::ARRAY_EXPR);
            p.bump();
            while !p.at(SyntaxKind::RBRACKET) && !p.at_end() {
                expr(p);
                if !p.eat(SyntaxKind::COMMA) {
                    break;
                }
            }
            p.expect(SyntaxKind::RBRACKET);
            p.finish_node();
            m
        }
        SyntaxKind::BANG | SyntaxKind::MINUS => {
            let m = p.start_node(SyntaxKind::UNARY_EXPR);
            p.bump();
            expr_bp(p, 255);
            p.finish_node();
            m
        }
        SyntaxKind::KW_IF => {
            if_expr(p);
            return;
        }
        _ => {
            p.error_recover("expected expression");
            return;
        }
    };

    // Postfix and infix
    loop {
        let op = p.current();

        // Postfix: function call
        if op == SyntaxKind::LPAREN {
            p.start_node_before(lhs, SyntaxKind::CALL_EXPR);
            let _arg = p.start_node(SyntaxKind::ARG_LIST);
            p.bump();
            while !p.at(SyntaxKind::RPAREN) && !p.at_end() {
                expr(p);
                if !p.eat(SyntaxKind::COMMA) {
                    break;
                }
            }
            p.expect(SyntaxKind::RPAREN);
            p.finish_node(); // ARG_LIST
            p.finish_node(); // CALL_EXPR
            continue;
        }

        // Postfix: field access
        if op == SyntaxKind::DOT {
            p.start_node_before(lhs, SyntaxKind::FIELD_EXPR);
            p.bump();
            if p.at(SyntaxKind::IDENT) {
                p.bump();
            } else {
                p.error_here("expected field name");
            }
            p.finish_node();
            continue;
        }

        // Postfix: index
        if op == SyntaxKind::LBRACKET {
            p.start_node_before(lhs, SyntaxKind::INDEX_EXPR);
            p.bump();
            expr(p);
            p.expect(SyntaxKind::RBRACKET);
            p.finish_node();
            continue;
        }

        // Assignment
        if op == SyntaxKind::EQ {
            if min_bp > 1 {
                break;
            }
            p.start_node_before(lhs, SyntaxKind::ASSIGN_EXPR);
            p.bump();
            expr_bp(p, 1);
            p.finish_node();
            break;
        }

        // Binary operators
        if let Some((l_bp, r_bp)) = infix_binding_power(op) {
            if l_bp < min_bp {
                break;
            }
            p.start_node_before(lhs, SyntaxKind::BINARY_EXPR);
            p.bump();
            expr_bp(p, r_bp);
            p.finish_node();
            continue;
        }

        break;
    }
}

fn infix_binding_power(op: SyntaxKind) -> Option<(u8, u8)> {
    Some(match op {
        SyntaxKind::PIPEPIPE => (3, 4),
        SyntaxKind::AMPAMP => (5, 6),
        SyntaxKind::EQEQ | SyntaxKind::NEQ => (7, 8),
        SyntaxKind::LT | SyntaxKind::GT | SyntaxKind::LTEQ | SyntaxKind::GTEQ => (9, 10),
        SyntaxKind::PIPE => (11, 12),
        SyntaxKind::CARET => (13, 14),
        SyntaxKind::AMP => (15, 16),
        SyntaxKind::SHL | SyntaxKind::SHR => (17, 18),
        SyntaxKind::PLUS | SyntaxKind::MINUS => (19, 20),
        SyntaxKind::STAR | SyntaxKind::SLASH | SyntaxKind::PERCENT => (21, 22),
        _ => return None,
    })
}

// ── Control flow expressions ─────────────────────────────────

fn if_expr(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::IF_EXPR);
    p.expect(SyntaxKind::KW_IF);
    expr(p);
    if p.at(SyntaxKind::LBRACE) {
        block(p);
    } else {
        p.error_here("expected block after if condition");
    }
    if p.eat(SyntaxKind::KW_ELSE) {
        if p.at(SyntaxKind::KW_IF) {
            if_expr(p);
        } else if p.at(SyntaxKind::LBRACE) {
            block(p);
        }
    }
    p.finish_node();
}

fn while_expr(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::WHILE_EXPR);
    p.expect(SyntaxKind::KW_WHILE);
    expr(p);
    if p.at(SyntaxKind::LBRACE) {
        block(p);
    }
    p.finish_node();
}

fn for_expr(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::FOR_EXPR);
    p.expect(SyntaxKind::KW_FOR);
    if p.at(SyntaxKind::IDENT) {
        p.bump();
    }
    p.expect(SyntaxKind::KW_IN);
    expr(p);
    if p.at(SyntaxKind::LBRACE) {
        block(p);
    }
    p.finish_node();
}

fn loop_expr(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::LOOP_EXPR);
    p.expect(SyntaxKind::KW_LOOP);
    if p.at(SyntaxKind::LBRACE) {
        block(p);
    }
    p.finish_node();
}

// ── Struct / Enum definitions ────────────────────────────────

fn struct_def(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::STRUCT_DEF);
    p.expect(SyntaxKind::KW_STRUCT);

    if p.at(SyntaxKind::IDENT) {
        p.bump();
    } else {
        p.error_here("expected struct name");
    }

    if p.at(SyntaxKind::LBRACE) {
        p.bump();
        while !p.at(SyntaxKind::RBRACE) && !p.at_end() {
            field_def(p);
            if !p.eat(SyntaxKind::COMMA) {
                break;
            }
        }
        p.expect(SyntaxKind::RBRACE);
    }

    p.finish_node();
}

fn field_def(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::FIELD_DEF);

    if p.at(SyntaxKind::IDENT) {
        p.bump();
    }
    if p.eat(SyntaxKind::COLON) {
        type_ref(p);
    }

    p.finish_node();
}

fn enum_def(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::ENUM_DEF);
    p.expect(SyntaxKind::KW_ENUM);

    if p.at(SyntaxKind::IDENT) {
        p.bump();
    }

    if p.at(SyntaxKind::LBRACE) {
        p.bump();
        while !p.at(SyntaxKind::RBRACE) && !p.at_end() {
            let _v = p.start_node(SyntaxKind::VARIANT_DEF);
            if p.at(SyntaxKind::IDENT) {
                p.bump();
            }
            // Optional tuple/struct variant fields
            if p.at(SyntaxKind::LPAREN) {
                p.bump();
                while !p.at(SyntaxKind::RPAREN) && !p.at_end() {
                    type_ref(p);
                    if !p.eat(SyntaxKind::COMMA) {
                        break;
                    }
                }
                p.expect(SyntaxKind::RPAREN);
            }
            p.finish_node();
            if !p.eat(SyntaxKind::COMMA) {
                break;
            }
        }
        p.expect(SyntaxKind::RBRACE);
    }

    p.finish_node();
}

fn use_decl(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::USE_DECL);
    p.expect(SyntaxKind::KW_USE);

    // Simple path: use foo::bar::baz;
    while p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::KW_SELF) || p.at(SyntaxKind::KW_SUPER) {
        p.bump();
        if !p.eat(SyntaxKind::COLONCOLON) {
            break;
        }
    }

    p.eat(SyntaxKind::SEMICOLON);
    p.finish_node();
}

fn mod_decl(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::MOD_DECL);
    p.expect(SyntaxKind::KW_MOD);

    if p.at(SyntaxKind::IDENT) {
        p.bump();
    }

    if p.at(SyntaxKind::LBRACE) {
        block(p);
    } else {
        p.eat(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

fn impl_def(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::IMPL_DEF);
    p.expect(SyntaxKind::KW_IMPL);

    type_ref(p);

    if p.at(SyntaxKind::LBRACE) {
        p.bump();
        while !p.at(SyntaxKind::RBRACE) && !p.at_end() {
            item(p);
        }
        p.expect(SyntaxKind::RBRACE);
    }

    p.finish_node();
}

fn trait_def(p: &mut Parser) {
    let _m = p.start_node(SyntaxKind::TRAIT_DEF);
    p.expect(SyntaxKind::KW_TRAIT);

    if p.at(SyntaxKind::IDENT) {
        p.bump();
    }

    if p.at(SyntaxKind::LBRACE) {
        p.bump();
        while !p.at(SyntaxKind::RBRACE) && !p.at_end() {
            item(p);
        }
        p.expect(SyntaxKind::RBRACE);
    }

    p.finish_node();
}
