/// A language-agnostic enumeration of syntax node/token kinds.
///
/// The raw `u16` value is split into ranges:
///   0..256       — built-in tokens (punctuation, keywords, literals)
///   256..4096    — built-in composite nodes
///   4096..       — language-specific extensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SyntaxKind(pub u16);

impl SyntaxKind {
    // ── Sentinel ──────────────────────────────────────────────
    pub const TOMBSTONE: Self = Self(u16::MAX);
    pub const EOF: Self = Self(0);

    // ── Trivia ────────────────────────────────────────────────
    pub const WHITESPACE: Self = Self(1);
    pub const NEWLINE: Self = Self(2);
    pub const LINE_COMMENT: Self = Self(3);
    pub const BLOCK_COMMENT: Self = Self(4);

    // ── Literals ──────────────────────────────────────────────
    pub const IDENT: Self = Self(10);
    pub const INT_LIT: Self = Self(11);
    pub const FLOAT_LIT: Self = Self(12);
    pub const STRING_LIT: Self = Self(13);
    pub const CHAR_LIT: Self = Self(14);
    pub const BOOL_LIT: Self = Self(15);

    // ── Punctuation ───────────────────────────────────────────
    pub const LPAREN: Self = Self(30);
    pub const RPAREN: Self = Self(31);
    pub const LBRACE: Self = Self(32);
    pub const RBRACE: Self = Self(33);
    pub const LBRACKET: Self = Self(34);
    pub const RBRACKET: Self = Self(35);
    pub const SEMICOLON: Self = Self(36);
    pub const COLON: Self = Self(37);
    pub const COMMA: Self = Self(38);
    pub const DOT: Self = Self(39);
    pub const DOTDOT: Self = Self(40);
    pub const ARROW: Self = Self(41);
    pub const FAT_ARROW: Self = Self(42);
    pub const COLONCOLON: Self = Self(43);
    pub const HASH: Self = Self(44);
    pub const AT: Self = Self(45);
    pub const QUESTION: Self = Self(46);

    // ── Operators ─────────────────────────────────────────────
    pub const PLUS: Self = Self(50);
    pub const MINUS: Self = Self(51);
    pub const STAR: Self = Self(52);
    pub const SLASH: Self = Self(53);
    pub const PERCENT: Self = Self(54);
    pub const AMP: Self = Self(55);
    pub const PIPE: Self = Self(56);
    pub const CARET: Self = Self(57);
    pub const TILDE: Self = Self(58);
    pub const BANG: Self = Self(59);
    pub const LT: Self = Self(60);
    pub const GT: Self = Self(61);
    pub const EQ: Self = Self(62);
    pub const EQEQ: Self = Self(63);
    pub const NEQ: Self = Self(64);
    pub const LTEQ: Self = Self(65);
    pub const GTEQ: Self = Self(66);
    pub const AMPAMP: Self = Self(67);
    pub const PIPEPIPE: Self = Self(68);
    pub const PLUSEQ: Self = Self(69);
    pub const MINUSEQ: Self = Self(70);
    pub const STAREQ: Self = Self(71);
    pub const SLASHEQ: Self = Self(72);
    pub const SHL: Self = Self(73);
    pub const SHR: Self = Self(74);

    // ── Keywords ──────────────────────────────────────────────
    pub const KW_FN: Self = Self(100);
    pub const KW_LET: Self = Self(101);
    pub const KW_MUT: Self = Self(102);
    pub const KW_IF: Self = Self(103);
    pub const KW_ELSE: Self = Self(104);
    pub const KW_WHILE: Self = Self(105);
    pub const KW_FOR: Self = Self(106);
    pub const KW_RETURN: Self = Self(107);
    pub const KW_STRUCT: Self = Self(108);
    pub const KW_ENUM: Self = Self(109);
    pub const KW_IMPL: Self = Self(110);
    pub const KW_TRAIT: Self = Self(111);
    pub const KW_PUB: Self = Self(112);
    pub const KW_USE: Self = Self(113);
    pub const KW_MOD: Self = Self(114);
    pub const KW_MATCH: Self = Self(115);
    pub const KW_TRUE: Self = Self(116);
    pub const KW_FALSE: Self = Self(117);
    pub const KW_SELF: Self = Self(118);
    pub const KW_SUPER: Self = Self(119);
    pub const KW_AS: Self = Self(120);
    pub const KW_IN: Self = Self(121);
    pub const KW_CONST: Self = Self(122);
    pub const KW_STATIC: Self = Self(123);
    pub const KW_TYPE: Self = Self(124);
    pub const KW_WHERE: Self = Self(125);
    pub const KW_LOOP: Self = Self(126);
    pub const KW_BREAK: Self = Self(127);
    pub const KW_CONTINUE: Self = Self(128);

    // ── Composite Nodes ───────────────────────────────────────
    pub const SOURCE_FILE: Self = Self(256);
    pub const FUNCTION: Self = Self(257);
    pub const PARAM_LIST: Self = Self(258);
    pub const PARAM: Self = Self(259);
    pub const BLOCK: Self = Self(260);
    pub const LET_STMT: Self = Self(261);
    pub const EXPR_STMT: Self = Self(262);
    pub const RETURN_STMT: Self = Self(263);
    pub const IF_EXPR: Self = Self(264);
    pub const WHILE_EXPR: Self = Self(265);
    pub const FOR_EXPR: Self = Self(266);
    pub const LOOP_EXPR: Self = Self(267);
    pub const MATCH_EXPR: Self = Self(268);
    pub const MATCH_ARM: Self = Self(269);
    pub const BINARY_EXPR: Self = Self(270);
    pub const UNARY_EXPR: Self = Self(271);
    pub const CALL_EXPR: Self = Self(272);
    pub const INDEX_EXPR: Self = Self(273);
    pub const FIELD_EXPR: Self = Self(274);
    pub const PATH_EXPR: Self = Self(275);
    pub const LITERAL: Self = Self(276);
    pub const PAREN_EXPR: Self = Self(277);
    pub const STRUCT_DEF: Self = Self(278);
    pub const ENUM_DEF: Self = Self(279);
    pub const FIELD_DEF: Self = Self(280);
    pub const VARIANT_DEF: Self = Self(281);
    pub const TYPE_REF: Self = Self(282);
    pub const ARG_LIST: Self = Self(283);
    pub const IMPL_DEF: Self = Self(284);
    pub const TRAIT_DEF: Self = Self(285);
    pub const USE_DECL: Self = Self(286);
    pub const MOD_DECL: Self = Self(287);
    pub const ARRAY_EXPR: Self = Self(288);
    pub const TUPLE_EXPR: Self = Self(289);
    pub const CLOSURE_EXPR: Self = Self(290);
    pub const ASSIGN_EXPR: Self = Self(291);

    // ── Error ─────────────────────────────────────────────────
    pub const ERROR: Self = Self(999);

    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            Self::WHITESPACE | Self::NEWLINE | Self::LINE_COMMENT | Self::BLOCK_COMMENT
        )
    }

    pub fn is_keyword(self) -> bool {
        self.0 >= 100 && self.0 < 200
    }

    pub fn is_literal(self) -> bool {
        self.0 >= 10 && self.0 < 20
    }

    pub fn is_composite(self) -> bool {
        self.0 >= 256
    }
}

impl std::fmt::Display for SyntaxKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SyntaxKind({})", self.0)
    }
}
