use semtree_core::SyntaxKind;
use semtree_red::{SyntaxNode, SyntaxToken};

use crate::typed::{AstChildren, AstNode};

/// Macro to define a typed AST node with accessors.
macro_rules! ast_node {
    (
        $(#[$meta:meta])*
        $name:ident => $kind:expr
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone)]
        pub struct $name {
            syntax: SyntaxNode,
        }

        impl AstNode for $name {
            fn kind() -> SyntaxKind { $kind }

            fn cast(node: SyntaxNode) -> Option<Self> {
                if node.kind() == $kind {
                    Some(Self { syntax: node })
                } else {
                    None
                }
            }

            fn syntax(&self) -> &SyntaxNode {
                &self.syntax
            }
        }
    };
}

// ── Source File ──────────────────────────────────────────────

ast_node! {
    /// The root node of a parsed file.
    SourceFile => SyntaxKind::SOURCE_FILE
}

impl SourceFile {
    pub fn functions(&self) -> AstChildren<Function> {
        AstChildren::new(&self.syntax)
    }

    pub fn structs(&self) -> AstChildren<StructDef> {
        AstChildren::new(&self.syntax)
    }

    pub fn enums(&self) -> AstChildren<EnumDef> {
        AstChildren::new(&self.syntax)
    }

    pub fn impl_blocks(&self) -> AstChildren<ImplDef> {
        AstChildren::new(&self.syntax)
    }

    pub fn trait_defs(&self) -> AstChildren<TraitDef> {
        AstChildren::new(&self.syntax)
    }
}

// ── Function ────────────────────────────────────────────────

ast_node! {
    /// A function definition: `fn name(params) { body }`
    Function => SyntaxKind::FUNCTION
}

impl Function {
    pub fn name(&self) -> Option<SyntaxToken> {
        self.syntax.child_token(SyntaxKind::IDENT)
    }

    pub fn name_text(&self) -> Option<String> {
        self.name().map(|t| t.text().to_string())
    }

    pub fn param_list(&self) -> Option<ParamList> {
        self.syntax
            .child_node(SyntaxKind::PARAM_LIST)
            .and_then(ParamList::cast)
    }

    pub fn body(&self) -> Option<Block> {
        self.syntax
            .child_node(SyntaxKind::BLOCK)
            .and_then(Block::cast)
    }

    pub fn return_type(&self) -> Option<TypeRef> {
        self.syntax
            .child_node(SyntaxKind::TYPE_REF)
            .and_then(TypeRef::cast)
    }
}

// ── ParamList ───────────────────────────────────────────────

ast_node! {
    /// A parameter list: `(x: i32, y: i32)`
    ParamList => SyntaxKind::PARAM_LIST
}

impl ParamList {
    pub fn params(&self) -> AstChildren<Param> {
        AstChildren::new(&self.syntax)
    }
}

// ── Param ───────────────────────────────────────────────────

ast_node! {
    /// A single function parameter.
    Param => SyntaxKind::PARAM
}

impl Param {
    pub fn name(&self) -> Option<SyntaxToken> {
        self.syntax.child_token(SyntaxKind::IDENT)
    }

    pub fn ty(&self) -> Option<TypeRef> {
        self.syntax
            .child_node(SyntaxKind::TYPE_REF)
            .and_then(TypeRef::cast)
    }
}

// ── Block ───────────────────────────────────────────────────

ast_node! {
    /// A block: `{ statements... }`
    Block => SyntaxKind::BLOCK
}

impl Block {
    pub fn statements(&self) -> Vec<Statement> {
        self.syntax
            .children()
            .into_iter()
            .filter_map(Statement::cast)
            .collect()
    }
}

// ── Statement ───────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Statement {
    Let(LetStmt),
    Expr(ExprStmt),
    Return(ReturnStmt),
}

impl Statement {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::LET_STMT => LetStmt::cast(node).map(Statement::Let),
            SyntaxKind::EXPR_STMT => ExprStmt::cast(node).map(Statement::Expr),
            SyntaxKind::RETURN_STMT => ReturnStmt::cast(node).map(Statement::Return),
            _ => None,
        }
    }

    pub fn syntax(&self) -> &SyntaxNode {
        match self {
            Statement::Let(s) => s.syntax(),
            Statement::Expr(s) => s.syntax(),
            Statement::Return(s) => s.syntax(),
        }
    }
}

// ── LetStmt ─────────────────────────────────────────────────

ast_node! {
    /// A let statement: `let x = expr;`
    LetStmt => SyntaxKind::LET_STMT
}

impl LetStmt {
    pub fn name(&self) -> Option<SyntaxToken> {
        self.syntax.child_token(SyntaxKind::IDENT)
    }

    pub fn name_text(&self) -> Option<String> {
        self.name().map(|t| t.text().to_string())
    }

    pub fn initializer(&self) -> Option<Expr> {
        self.syntax
            .children()
            .into_iter()
            .filter_map(Expr::cast)
            .next()
    }

    pub fn ty(&self) -> Option<TypeRef> {
        self.syntax
            .child_node(SyntaxKind::TYPE_REF)
            .and_then(TypeRef::cast)
    }
}

// ── ExprStmt ────────────────────────────────────────────────

ast_node! {
    /// An expression statement: `expr;`
    ExprStmt => SyntaxKind::EXPR_STMT
}

impl ExprStmt {
    pub fn expr(&self) -> Option<Expr> {
        self.syntax
            .children()
            .into_iter()
            .filter_map(Expr::cast)
            .next()
    }
}

// ── ReturnStmt ──────────────────────────────────────────────

ast_node! {
    /// A return statement: `return expr;`
    ReturnStmt => SyntaxKind::RETURN_STMT
}

impl ReturnStmt {
    pub fn expr(&self) -> Option<Expr> {
        self.syntax
            .children()
            .into_iter()
            .filter_map(Expr::cast)
            .next()
    }
}

// ── Expr ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Expr {
    Binary(BinaryExpr),
    Call(CallExpr),
    If(IfExpr),
    Literal(LiteralExpr),
    Path(PathExpr),
    Paren(ParenExpr),
    Unary(UnaryExpr),
    Block(Block),
}

impl Expr {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::BINARY_EXPR => BinaryExpr::cast(node).map(Expr::Binary),
            SyntaxKind::CALL_EXPR => CallExpr::cast(node).map(Expr::Call),
            SyntaxKind::IF_EXPR => IfExpr::cast(node).map(Expr::If),
            SyntaxKind::LITERAL => LiteralExpr::cast(node).map(Expr::Literal),
            SyntaxKind::PATH_EXPR => PathExpr::cast(node).map(Expr::Path),
            SyntaxKind::PAREN_EXPR => ParenExpr::cast(node).map(Expr::Paren),
            SyntaxKind::UNARY_EXPR => UnaryExpr::cast(node).map(Expr::Unary),
            SyntaxKind::BLOCK => Block::cast(node).map(Expr::Block),
            _ => None,
        }
    }

    pub fn syntax(&self) -> &SyntaxNode {
        match self {
            Expr::Binary(e) => e.syntax(),
            Expr::Call(e) => e.syntax(),
            Expr::If(e) => e.syntax(),
            Expr::Literal(e) => e.syntax(),
            Expr::Path(e) => e.syntax(),
            Expr::Paren(e) => e.syntax(),
            Expr::Unary(e) => e.syntax(),
            Expr::Block(e) => e.syntax(),
        }
    }
}

// ── BinaryExpr ──────────────────────────────────────────────

ast_node! {
    /// A binary expression: `a + b`
    BinaryExpr => SyntaxKind::BINARY_EXPR
}

impl BinaryExpr {
    pub fn lhs(&self) -> Option<Expr> {
        self.syntax
            .children()
            .into_iter()
            .filter_map(Expr::cast)
            .next()
    }

    pub fn rhs(&self) -> Option<Expr> {
        self.syntax
            .children()
            .into_iter()
            .filter_map(Expr::cast)
            .nth(1)
    }

    pub fn op(&self) -> Option<SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .into_iter()
            .find_map(|e| {
                if let semtree_red::SyntaxElement::Token(t) = e
                    && !t.kind().is_trivia()
                    && t.kind() != SyntaxKind::IDENT
                    && t.kind().0 >= 50
                    && t.kind().0 < 80
                {
                    return Some(t);
                }
                None
            })
    }

    pub fn op_text(&self) -> Option<String> {
        self.op().map(|t| t.text().to_string())
    }
}

// ── CallExpr ────────────────────────────────────────────────

ast_node! {
    /// A function call: `foo(args)`
    CallExpr => SyntaxKind::CALL_EXPR
}

impl CallExpr {
    pub fn callee(&self) -> Option<Expr> {
        self.syntax
            .children()
            .into_iter()
            .filter_map(Expr::cast)
            .next()
    }

    pub fn arg_list(&self) -> Option<ArgList> {
        self.syntax
            .child_node(SyntaxKind::ARG_LIST)
            .and_then(ArgList::cast)
    }
}

// ── ArgList ─────────────────────────────────────────────────

ast_node! {
    /// An argument list: `(a, b, c)`
    ArgList => SyntaxKind::ARG_LIST
}

// ── IfExpr ──────────────────────────────────────────────────

ast_node! {
    /// An if expression: `if cond { ... } else { ... }`
    IfExpr => SyntaxKind::IF_EXPR
}

impl IfExpr {
    pub fn condition(&self) -> Option<Expr> {
        self.syntax
            .children()
            .into_iter()
            .filter_map(Expr::cast)
            .next()
    }

    pub fn then_branch(&self) -> Option<Block> {
        self.syntax
            .children()
            .into_iter()
            .filter_map(Block::cast)
            .next()
    }

    pub fn else_branch(&self) -> Option<Block> {
        self.syntax
            .children()
            .into_iter()
            .filter_map(Block::cast)
            .nth(1)
    }
}

// ── LiteralExpr ─────────────────────────────────────────────

ast_node! {
    /// A literal: `42`, `"hello"`, `true`
    LiteralExpr => SyntaxKind::LITERAL
}

impl LiteralExpr {
    pub fn token(&self) -> Option<SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .into_iter()
            .find_map(|e| {
                if let semtree_red::SyntaxElement::Token(t) = e
                    && t.kind().is_literal()
                {
                    return Some(t);
                }
                None
            })
    }

    pub fn text(&self) -> String {
        self.token()
            .map(|t| t.text().to_string())
            .unwrap_or_default()
    }
}

// ── PathExpr ────────────────────────────────────────────────

ast_node! {
    /// A path expression: `x`, `std::io`
    PathExpr => SyntaxKind::PATH_EXPR
}

// ── ParenExpr ───────────────────────────────────────────────

ast_node! {
    /// A parenthesized expression: `(expr)`
    ParenExpr => SyntaxKind::PAREN_EXPR
}

impl ParenExpr {
    pub fn inner(&self) -> Option<Expr> {
        self.syntax
            .children()
            .into_iter()
            .filter_map(Expr::cast)
            .next()
    }
}

// ── UnaryExpr ───────────────────────────────────────────────

ast_node! {
    /// A unary expression: `-x`, `!flag`
    UnaryExpr => SyntaxKind::UNARY_EXPR
}

// ── StructDef ───────────────────────────────────────────────

ast_node! {
    /// A struct definition.
    StructDef => SyntaxKind::STRUCT_DEF
}

impl StructDef {
    pub fn name(&self) -> Option<SyntaxToken> {
        self.syntax.child_token(SyntaxKind::IDENT)
    }

    pub fn name_text(&self) -> Option<String> {
        self.name().map(|t| t.text().to_string())
    }

    pub fn fields(&self) -> AstChildren<FieldDef> {
        AstChildren::new(&self.syntax)
    }
}

// ── FieldDef ────────────────────────────────────────────────

ast_node! {
    /// A struct field definition.
    FieldDef => SyntaxKind::FIELD_DEF
}

impl FieldDef {
    pub fn name(&self) -> Option<SyntaxToken> {
        self.syntax.child_token(SyntaxKind::IDENT)
    }

    pub fn ty(&self) -> Option<TypeRef> {
        self.syntax
            .child_node(SyntaxKind::TYPE_REF)
            .and_then(TypeRef::cast)
    }
}

// ── EnumDef ─────────────────────────────────────────────────

ast_node! {
    /// An enum definition.
    EnumDef => SyntaxKind::ENUM_DEF
}

impl EnumDef {
    pub fn name(&self) -> Option<SyntaxToken> {
        self.syntax.child_token(SyntaxKind::IDENT)
    }

    pub fn name_text(&self) -> Option<String> {
        self.name().map(|t| t.text().to_string())
    }

    pub fn variants(&self) -> AstChildren<VariantDef> {
        AstChildren::new(&self.syntax)
    }
}

// ── VariantDef ──────────────────────────────────────────────

ast_node! {
    /// An enum variant.
    VariantDef => SyntaxKind::VARIANT_DEF
}

impl VariantDef {
    pub fn name(&self) -> Option<SyntaxToken> {
        self.syntax.child_token(SyntaxKind::IDENT)
    }
}

// ── TypeRef ─────────────────────────────────────────────────

ast_node! {
    /// A type reference.
    TypeRef => SyntaxKind::TYPE_REF
}

// ── ImplDef ─────────────────────────────────────────────────

ast_node! {
    /// An impl block.
    ImplDef => SyntaxKind::IMPL_DEF
}

impl ImplDef {
    pub fn functions(&self) -> AstChildren<Function> {
        AstChildren::new(&self.syntax)
    }
}

// ── TraitDef ────────────────────────────────────────────────

ast_node! {
    /// A trait definition.
    TraitDef => SyntaxKind::TRAIT_DEF
}

impl TraitDef {
    pub fn name(&self) -> Option<SyntaxToken> {
        self.syntax.child_token(SyntaxKind::IDENT)
    }

    pub fn name_text(&self) -> Option<String> {
        self.name().map(|t| t.text().to_string())
    }
}
