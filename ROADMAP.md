# SemTree Roadmap

> Universal Incremental Language Infrastructure — the platform that beats Tree-sitter.

**Current status:** 14 crates, 89 tests passing, Phases 1–3 complete.

---

## Phase 1 — Core Infrastructure ✅

- [x] `semtree_core` — SyntaxKind, TextSpan, Token, Trivia, Interner
- [x] `semtree_lexer` — Unicode-aware, zero-copy lexer with trivia preservation
- [x] `semtree_green` — Immutable green tree with Arc-based structural sharing
- [x] `semtree_red` — Navigable red tree with parent/sibling/ancestor pointers
- [x] `semtree_parser` — Event-based parser with Pratt expression parsing
- [x] `semtree_grammar` — Grammar IR + SemTree DSL parser + validator
- [x] `semtree_ts_import` — Tree-sitter `grammar.json` importer
- [x] `semtree_cli` — CLI skeleton (init, parse, check, import, doctor, benchmark)

## Phase 2 — Parser Generator & Queries ✅

- [x] `semtree_runtime` — Grammar-driven runtime parser (Grammar IR → working parser)
- [x] Runtime lexer — tokenize from grammar keywords/literals automatically
- [x] Builder checkpointing — checkpoint/rollback for speculative backtracking
- [x] Incremental reparsing — edit tracking, node reuse via green node cache
- [x] `semtree_query` — S-expression tree query engine with captures
- [x] CLI `run` command — parse any file with any grammar
- [x] CLI `query` command — query trees with S-expressions or kind names

## Phase 3 — Typed AST, Semantics, Formatter, Linter ✅

- [x] `semtree_ast` — Typed AST wrappers (AstNode trait, built-in types, codegen)
- [x] `semtree_semantic` — Symbol table, scope tree, references, semantic model
- [x] `semtree_format` — Syntax-tree-driven formatter with configurable style
- [x] `semtree_lint` — Rule-based linter with 4 built-in rules + custom rule API
- [x] CLI `format`, `lint`, `symbols` commands

---

## Phase 4 — Beat Tree-sitter (Performance & Correctness)

### 4.1 — True Incremental Reparsing
- [ ] Edit-aware tree diffing — find smallest affected subtree after an edit
- [ ] Node-level reuse — splice unchanged green subtrees into new tree without reparsing
- [ ] Incremental lexing — only re-tokenize the changed region
- [ ] Benchmark: achieve <1ms reparse for single-character edits on 10K+ line files

### 4.2 — GLR / Ambiguity Support
- [ ] GLR parser backend for ambiguous grammars
- [ ] Parser algorithm selection — auto-choose recursive descent, Pratt, or GLR per rule
- [ ] Precedence/associativity fully wired through Grammar IR to runtime parser
- [ ] Left recursion elimination or direct left-recursive parsing support

### 4.3 — Error Recovery (Production-Grade)
- [ ] Token-level error recovery — synchronize on statement/block boundaries
- [ ] Missing token insertion — infer missing `;`, `)`, `}` etc.
- [ ] Error node spans — every broken region is wrapped in an ERROR node with context
- [ ] Partial tree validity — broken subtrees don't corrupt parent structure
- [ ] Fuzz testing — parse 10K+ random/corrupted inputs without panics

### 4.4 — Performance Parity with Tree-sitter
- [ ] Arena allocator for green nodes (avoid per-node Arc overhead)
- [ ] Zero-copy token storage — reference source text instead of SmolStr copies
- [ ] SIMD-accelerated lexing for ASCII-heavy languages
- [ ] Parallel parsing — split large files into chunks
- [ ] Memory benchmarks — target ≤ Tree-sitter memory per node
- [ ] Parse speed benchmarks — target ≤ 10% slower than Tree-sitter cold parse
- [ ] Incremental speed benchmarks — target faster than Tree-sitter on common edits

---

## Phase 5 — Language Ecosystem

### 5.1 — Grammar Authoring
- [ ] Grammar validation — cycle detection, unreachable rules, ambiguity warnings
- [ ] Grammar optimizer — inline small rules, flatten unnecessary sequences
- [ ] Grammar debugger — step through parse with grammar rule highlighting
- [ ] Visual grammar editor (web UI)
- [ ] Automatic grammar migration from Tree-sitter `grammar.js`

### 5.2 — Real Language Grammars
- [ ] Full Rust grammar (pass on rust-analyzer test corpus)
- [ ] Full JavaScript/TypeScript grammar
- [ ] Full Python grammar
- [ ] Full Go grammar
- [ ] Full C/C++ grammar
- [ ] JSON / TOML / YAML / Markdown grammars
- [ ] Grammar test suite — golden file tests for each language

### 5.3 — Typed AST Codegen (Production)
- [ ] Proc-macro for `#[derive(AstNode)]` from grammar
- [ ] Compile-time AST generation from `.semtree` grammar files
- [ ] Enum dispatch for heterogeneous node types
- [ ] Visitor pattern generation
- [ ] Walker/rewriter pattern generation

---

## Phase 6 — IDE Services

### 6.1 — LSP Protocol
- [ ] `semtree_lsp` crate — Language Server Protocol implementation
- [ ] `textDocument/didOpen`, `didChange`, `didClose`
- [ ] `textDocument/completion` — keyword and identifier completion
- [ ] `textDocument/hover` — show type/symbol info
- [ ] `textDocument/definition` — goto definition
- [ ] `textDocument/references` — find all references
- [ ] `textDocument/rename` — rename symbol across scopes
- [ ] `textDocument/documentSymbol` — outline view
- [ ] `textDocument/formatting` — code formatting
- [ ] `textDocument/diagnostics` — lint + parse errors
- [ ] `textDocument/semanticTokens` — semantic syntax highlighting
- [ ] `textDocument/codeFolding` — fold regions from syntax tree
- [ ] `textDocument/codeAction` — quick fixes from lint rules

### 6.2 — Semantic Tokens
- [ ] Token classification — keyword, type, function, variable, parameter, etc.
- [ ] Modifier support — declaration, definition, readonly, static, deprecated
- [ ] Semantic highlighting driven by grammar + semantic model

### 6.3 — Code Navigation
- [ ] Breadcrumb navigation (scope-aware)
- [ ] Symbol search across files
- [ ] Call hierarchy
- [ ] Type hierarchy

---

## Phase 7 — Refactoring API

- [ ] `semtree_refactor` crate
- [ ] Extract function — select code, extract into new function with params
- [ ] Extract variable — select expression, assign to variable
- [ ] Inline variable — replace variable with its definition
- [ ] Rename symbol — scope-aware rename across files
- [ ] Move item — move function/struct between modules
- [ ] Change signature — add/remove/reorder function parameters
- [ ] Tree edit API — programmatic syntax tree mutations that preserve formatting

---

## Phase 8 — AI-Friendly APIs

- [ ] `semtree_ai` crate
- [ ] `find_symbol(name)` — locate any symbol by name
- [ ] `rename_symbol(old, new)` — safe rename across scopes
- [ ] `extract_function(range)` — extract selection into function
- [ ] `find_references(symbol)` — all references to a symbol
- [ ] `nearest_scope(offset)` — scope context at cursor
- [ ] `current_function(offset)` — which function contains this offset
- [ ] `affected_nodes(edit)` — which nodes are invalidated by an edit
- [ ] `diff_tree(old, new)` — structural diff between two syntax trees
- [ ] `suggest_completion(offset)` — context-aware completion candidates
- [ ] Structured JSON API for all operations
- [ ] WASM bindings for browser-based AI agents

---

## Phase 9 — Plugin System

- [ ] `semtree_plugin` crate — plugin trait and loader
- [ ] Plugin registry — discover and load plugins
- [ ] Language plugins — contribute grammars + semantic rules
- [ ] Linter plugins — contribute custom lint rules
- [ ] Formatter plugins — contribute formatting styles
- [ ] Query plugins — contribute reusable query patterns
- [ ] Code generator plugins — contribute code generation from AST
- [ ] Hot-reload support — reload plugins without restarting

---

## Phase 10 — Distribution & Ecosystem

### 10.1 — Bindings
- [ ] C API (`libsemtree`) — stable C ABI for FFI
- [ ] WASM build — run SemTree in the browser
- [ ] Python bindings (`py-semtree`)
- [ ] Node.js bindings (`@semtree/core`)
- [ ] Go bindings

### 10.2 — Tooling
- [ ] `semtree playground` — web-based grammar + parse explorer
- [ ] `semtree test` — run grammar test suites
- [ ] `semtree generate` — generate typed AST code from grammar
- [ ] `semtree migrate` — migrate Tree-sitter grammars to SemTree
- [ ] `semtree profile` — performance profiling for grammars

### 10.3 — Documentation & Community
- [ ] API documentation (rustdoc)
- [ ] Grammar authoring guide
- [ ] Migration guide from Tree-sitter
- [ ] Tutorial: "Build a language in 30 minutes"
- [ ] Benchmark suite with public results
- [ ] CI/CD pipeline with regression tests
- [ ] Published to crates.io

---

## Key Metrics to Beat Tree-sitter

| Metric | Tree-sitter | SemTree Target |
|--------|------------|----------------|
| Cold parse speed | Baseline | Within 10% or faster |
| Incremental reparse | ~1-5ms | <1ms for single edits |
| Memory per node | ~32 bytes | ≤28 bytes (arena) |
| Error recovery | Good | Better (context-aware) |
| Grammar authoring | grammar.js | DSL + visual editor |
| Typed API | None (only CST) | Full typed AST |
| Semantic analysis | None | Built-in symbol table |
| Query language | S-expressions | S-expressions + typed |
| Formatter | None | Built-in |
| Linter | None | Built-in |
| IDE protocol | External | Built-in LSP |
| AI integration | None | First-class API |
| Languages supported | 200+ | Start with 6 core |
