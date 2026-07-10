# SemTree Roadmap

> Universal Incremental Language Infrastructure — the platform that beats Tree-sitter.

**Current status:** 21 crates, 297 tests passing, Phases 1–11 complete.

**Next up (local-first):** Phase 12 (credibility & honest benchmarks), Phase 13 (language coverage & Tree-sitter import), Phase 14 (LSP-first editor integration). AI/MCP work (structural agent APIs) is deferred — it is not local-first and depends on a hardened core landing in 12–14.

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
- [x] Grammar-driven lexer — custom `token Name := /regex/` patterns in DSL
- [x] INDENT/DEDENT tokenization for indentation-sensitive grammars
- [x] Builder checkpointing — checkpoint/rollback for speculative backtracking
- [x] Incremental reparsing — edit tracking, node reuse via green node cache
- [x] Incremental parsing wired into CLI (`--incremental`, `--edit`)
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

## Phase 4 — Beat Tree-sitter (Performance & Correctness) ✅

### 4.1 — True Incremental Reparsing
- [x] Edit-aware tree diffing — find smallest affected subtree after an edit
- [x] Node-level reuse — splice unchanged green subtrees into new tree without reparsing
- [x] Incremental lexing — only re-tokenize the changed region
- [x] Benchmark: achieve <1ms reparse for single-character edits on 10K+ line files

### 4.2 — GLR / Ambiguity Support
- [x] GLR parser backend for ambiguous grammars
- [x] Parser algorithm selection — auto-choose recursive descent or GLR per grammar (`--backend auto`)
- [x] Precedence/associativity fully wired through Grammar IR to runtime parser
- [x] Left recursion detection with depth guard (direct left-recursive parsing TBD)
- [x] Proper Kleene-star (Repeat) in GLR table generation via synthesized non-terminals
- [x] Precedence-aware reduce/reduce conflict resolution in GLR driver

### 4.3 — Error Recovery (Production-Grade)
- [x] Token-level error recovery — synchronize on statement/block boundaries
- [x] Missing token insertion — infer missing `;`, `)`, `}` etc.
- [x] Error node spans — every broken region is wrapped in an ERROR node with context
- [x] Partial tree validity — broken subtrees don't corrupt parent structure
- [x] Fuzz testing — parse random/corrupted inputs without panics (9 fuzz tests)
- [x] Max depth guard — prevent stack overflow on deeply recursive input

### 4.4 — Performance Parity with Tree-sitter
- [ ] Arena allocator for green nodes (future optimization)
- [ ] Zero-copy token storage (future optimization)
- [ ] SIMD-accelerated lexing for ASCII-heavy languages (future optimization)
- [ ] Parallel parsing — split large files into chunks (future optimization)
- [ ] Memory benchmarks (future)
- [x] Parse speed benchmarks — CLI `benchmark` with cold/warm/incremental/tree stats
- [x] Benchmarks use shipped `grammars/*.semtree` (not inline toy grammars)
- [x] Computed summary from actual run data (no hardcoded claims)

---

## Phase 5 — Language Ecosystem ✅

### 5.1 — Grammar Authoring
- [x] Grammar validation — cycle detection, unreachable rules, empty alternative warnings
- [x] Grammar optimizer — inline small rules, flatten Seq/Choice, collapse Optional
- [ ] Grammar debugger — step through parse with grammar rule highlighting (future)
- [ ] Visual grammar editor (web UI) (future)

### 5.2 — Real Language Grammars
- [x] JSON grammar with lossless parse tests
- [x] TOML grammar with lossless parse tests
- [x] Grammar test files (grammars/tests/)
- [x] Python grammar with INDENT/DEDENT tokenization
- [x] Python grammar: async/await, decorators, with, try/except, f-strings
- [x] Corpus tests: JSON, TOML, Python, Rust, JavaScript, CSS against real files
- [ ] Full Rust grammar (future — pass on rust-analyzer test corpus)
- [ ] Full JavaScript/TypeScript grammar (future)
- [ ] Full Python grammar (future)

### 5.3 — Typed AST Codegen (Production)
- [x] `generate_ast()` — generate Rust AST wrapper code from Grammar IR
- [x] `generate_visitor()` — generate Visitor trait with walk methods
- [x] `grammar_summary()` — CLI-friendly grammar overview
- [ ] Proc-macro `#[derive(AstNode)]` (future)
- [ ] Compile-time AST generation from `.semtree` grammar files (future)

---

## Phase 6 — IDE Services ✅

- [x] `semtree_ide` crate
- [x] Semantic token classification (Keyword, Type, Function, Variable, etc.)
- [x] Token modifiers (declaration, definition, readonly)
- [x] Code completion — keywords + in-scope identifiers
- [x] Go-to definition
- [x] Find all references
- [x] Document symbols (outline)
- [x] Hover info
- [x] Breadcrumb navigation (scope chain)
- [x] Code folding ranges

---

## Phase 7 — Refactoring API ✅

- [x] `semtree_refactor` crate
- [x] Rename symbol — scope-aware rename producing TextEdit list
- [x] Extract variable — select expression, assign to variable
- [x] Inline variable — replace references with initializer
- [x] `TreeEditor` — programmatic tree mutations (replace, insert_before, insert_after, remove, apply)

---

## Phase 8 — AI-Friendly APIs ✅

- [x] `semtree_ai` crate
- [x] `find_symbol(name)` — locate any symbol by name
- [x] `rename_symbol(old, new)` — word-boundary-aware rename
- [x] `find_references(symbol)` — all references to a symbol
- [x] `nearest_scope(offset)` — scope context at cursor
- [x] `current_function(offset)` — which function contains this offset
- [x] `affected_nodes(edit)` — which nodes are invalidated by an edit
- [x] `diff_tree(old, new)` — structural diff between two syntax trees
- [x] `suggest_completion(offset)` — context-aware completion candidates
- [x] Structured JSON API — `execute_command()` with JSON input/output
- [x] Serde-based serializable types for all API responses

---

## Phase 9 — Plugin System ✅

- [x] `semtree_plugin` crate
- [x] `LanguagePlugin` trait — contribute grammars + file extensions
- [x] `LinterPlugin` + `LintRulePlugin` traits — contribute lint rules
- [x] `FormatterPlugin` trait — contribute formatting styles
- [x] `QueryPlugin` trait — contribute reusable query patterns
- [x] `PluginRegistry` — register, discover, and lookup plugins
- [x] Extension-based language detection

---

## Phase 10 — Distribution & Ecosystem ✅

### 10.1 — C FFI API
- [x] `semtree_ffi` crate (cdylib + staticlib)
- [x] `semtree_session_create/parse/edit/free()` — incremental grammar-driven sessions
- [x] `semtree_parse()` — parse source to tree
- [x] `semtree_tree_root()` — get root node
- [x] `semtree_node_kind/text/child_count/child/start/end()` — node navigation
- [x] `semtree_tree_free()` / `semtree_node_free()` — memory management
- [x] Null-pointer safe, opaque pointer design
- [x] WASM build (`semtree_wasm` crate)
- [x] Python bindings (`bindings/python/` via PyO3)
- [ ] Node.js bindings (future)

### 10.2 — CLI Tools
- [x] `semtree generate` — generate typed AST code from grammar
- [x] `semtree test` — run grammar test suites
- [x] `semtree migrate` — migrate Tree-sitter grammars
- [x] `semtree lsp` — LSP server with incremental parsing
- [x] `semtree debug` — grammar debugger (token stream + tree summary)
- [x] `semtree parity` — full vs incremental parse benchmark
- [ ] `semtree playground` — web-based grammar explorer (future)
- [ ] `semtree profile` — performance profiling (future)

### 10.3 — Documentation & Community
- [ ] API documentation (rustdoc) (future)
- [ ] Grammar authoring guide (future)
- [ ] Migration guide from Tree-sitter (future)
- [ ] Tutorial: "Build a language in 30 minutes" (future)
- [ ] Published to crates.io (future)

---

## Phase 11 — Incremental GLR / RNGLR Parser Engine ✅

> The core algorithmic leap to match Tree-sitter's parsing power.

### 11.1 — GLR Parse Table Generation
- [x] FIRST/FOLLOW set computation from Grammar IR
- [x] LR(0) / SLR(1) item set construction
- [x] LALR(1) state merging
- [x] Conflict detection (shift/reduce, reduce/reduce) → mark as GLR-required
- [ ] Compressed parse table representation (CSR sparse format) (future optimization)

### 11.2 — Graph-Structured Stack (GSS)
- [x] GSS node pool with generational indices (arena-allocated)
- [x] Stack splitting on shift/reduce conflicts
- [x] Stack merging when multiple stacks reach the same state
- [x] Local ambiguity packing — merge identical stack tops
- [ ] Garbage collection of unreachable GSS nodes (future optimization)

### 11.3 — Shared Packed Parse Forest (SPPF)
- [x] SPPF node types: Symbol, Intermediate, Packed
- [x] Right-nulled SPPF construction (RNGLR algorithm)
- [x] Binary SPPF representation (BRNGLR optimization)
- [x] Ambiguity node flattening — extract single "best" tree via priority/associativity
- [x] SPPF → GreenNode conversion (lossless, trivia-preserving)

### 11.4 — GLR Driver
- [x] Table-driven shift/reduce/goto engine
- [x] Multi-stack parallel exploration on conflicts
- [x] Reduce lookahead for SLR(1)/LALR(1) disambiguation
- [x] Precedence/associativity filters to prune spurious ambiguities
- [x] Accept action with forest extraction

### 11.5 — Incremental GLR Reparsing
- [x] Parse state checkpointing at tree node boundaries
- [x] Edit-aware invalidation — find smallest affected state range
- [ ] Subtree reuse — skip reparsing unchanged regions by replaying cached reductions (future optimization)
- [ ] State-compatible merging — stitch reparsed region back into existing GSS (future optimization)
- [ ] Target: O(log n + |edit|) reparse for single-character edits (future optimization)

### 11.6 — GLR Error Recovery
- [x] Forward repair — try inserting expected tokens from parse table
- [x] Backward repair — pop stack to nearest viable state
- [ ] Error cost model — rank repairs by edit distance (future)
- [x] Panic-mode recovery — skip to recovery tokens as fallback
- [x] Error node generation — wrap unrecoverable regions in ERROR nodes

### 11.7 — Integration & Algorithm Selection
- [x] `ParserBackend` enum: `RecursiveDescent | GLR | Auto`
- [x] Auto-selection: use GLR when grammar has conflicts, RD otherwise
- [x] CLI `--backend glr` flag for explicit selection
- [x] Unified parse result output regardless of backend
- [ ] Performance benchmarks: GLR vs RD vs Tree-sitter (future)

---

## Phase 12 — Credibility & Honest Benchmarks (P1) 🔜

> The "beats Tree-sitter" claim only holds if the numbers survive scrutiny. Today the
> bench harness hand-builds toy grammars and reports headline speedups (e.g. "5,419x on
> delete line") that read as measurement artifacts. This phase makes every published claim
> defensible.

### 12.1 — Benchmark on real grammars, not toy grammars
- [x] Retarget `semtree_bench` to load the shipped `grammars/*.semtree` files instead of the inline hand-built JSON/JS grammars in `main.rs` (all ~770 lines of toy `build_*_grammar` builders removed; every benchmark, including the "SemTree bonus" suite, now runs on shipped grammars)
- [~] Bench all 6 languages through the same code path used by the CLI — 5 done (JSON, Python, Rust, JS, CSS); TOML has no `tree-sitter-toml` crate on the current 0.24 API for a fair baseline (tracked as a gap; SemTree-only TOML timing can still be added)
- [x] Parse the same corpus files with the real `tree-sitter` crate for each language (dev-dependencies already wired) so comparisons use equivalent grammars
- [x] Report per-language results including cases where SemTree **loses** — the summary now prints per-test ratios for every language/size and an explicit "Where SemTree is SLOWER" list (no averaging away losses)

> **Finding (12.1):** against the *real* shipped grammars, SemTree currently **loses** on JavaScript, Rust, and Python cold-parse at every size, and on most incremental/traversal cases; it wins on JSON (mid/large) and CSS (small/mid). This directly contradicts the README's blanket "1.5–3.7x faster" claim, which must be regenerated from a committed high-iteration run before it is trusted (see 12.4).


### 12.2 — Honest incremental numbers
- [x] Replace "Nx faster" framing for incremental edits with absolute latency (µs/ms) — every row now shows TS `edit+reparse` latency and SemTree `inc / full` latencies side by side
- [x] For every incremental edit benchmark, assert the incremental tree losslessly reproduces the edited source (correctness gate `[✓]` / `[✗ LOSSY]`) before timing is trusted
- [x] Distinguish "splice hit" from "splice miss" — `IncrementalParser::last_reuse()` now returns a `ReuseInfo { kind, total_bytes, reparsed_bytes }` (`FullParse` / `DeepSplice` / `SiblingSplice` / `SpliceMiss`) with `is_hit()` and `reuse_ratio()`; the benchmark prints the kind and % reused per edit
- [x] Both sides now exclude the initial parse from timing (`bench_setup` runs an un-timed setup per iteration) and use a real tree-sitter `InputEdit` computed from a prefix/suffix diff, so the comparison is apples-to-apples

> **Finding (12.2):** with a fair methodology, SemTree's incremental `update()` is currently **slower than its own full reparse** on insert/append edits (e.g. JSON insert: 342µs incremental vs 285µs full) and **12–95x slower than tree-sitter's real incremental reparse**. The `last_reuse()` API confirms *why*: a mid-file single-character insert is a `SpliceMiss` (**0% reused** — it falls back to a full reparse), and even a `SiblingSplice` "100% reused" append is barely faster than a full parse because the green tree is rebuilt in O(n). The former README claims of "7.1x" / "5,419x faster" were artifacts of timing tree-sitter doing extra full parses while SemTree did one. The only apparent win (JSON "delete line") is degenerate — the JSON generator emits a single line, so the edit deletes the whole document. **Action:** the incremental path needs the real subtree-reuse work in 4.1 / 11.5 (make single-char inserts a splice hit, and avoid full green-tree rebuilds) before any incremental speed claim is made; the benchmark generators also need multi-line JSON so "delete line" is meaningful.



### 12.3 — Lossless conformance corpus
- [ ] Add a conformance test: for each language, parse large real-world files pulled from popular OSS repos and assert lossless round-trip (`tree.text() == source`)
- [ ] Wire the corpus into `cargo test` and CI so regressions in coverage or losslessness fail the build
- [ ] Publish a coverage report per grammar: % of corpus files that parse with zero ERROR nodes

### 12.4 — Reproducibility & reporting
- [ ] `semtree bench --json` emits machine-readable results (env, versions, medians, variance)
- [ ] Check bench inputs and a results snapshot into the repo so anyone can reproduce headline numbers
- [ ] README benchmark table is generated from a committed results file, not hand-written

**Done when:** every number in the README is reproducible by `cargo run -p semtree_bench --release`, compares against the real tree-sitter grammar for that language, and is backed by a losslessness assertion.

---

## Phase 13 — Language Coverage & Tree-sitter Import (P3) 🔜

> We will never hand-write 200+ grammars. The winning move is to (a) make **one** flagship
> grammar genuinely complete, and (b) turn Tree-sitter's own ecosystem into our supply chain
> by consuming its `grammar.json` and giving those grammars formatting/linting/refactoring for free.

### 13.1 — Flagship complete grammar
- [ ] Pick one language (Python **or** Rust) and drive it to full-language coverage against a real corpus
- [ ] Zero ERROR nodes on the flagship corpus (tracked in Phase 12.3)
- [ ] Field extraction complete (named fields on nodes, e.g. `function.name`, `call.arguments`) so typed AST + IDE features are first-class for the flagship
- [ ] Golden formatter + lint output snapshots for the flagship language

### 13.2 — Harden the Tree-sitter importer
- [ ] Extend `import_tree_sitter_grammar` to populate `Rule.fields` from TS `field(...)` constructs (currently dropped — `fields: Vec::new()`)
- [ ] Map TS `prec`, `prec.left`, `prec.right`, `prec.dynamic` into SemTree precedence/associativity IR
- [ ] Handle TS `alias`, `token`, `token.immediate`, `immediate_token` correctly
- [ ] Support `externals`, `inline`, `supertypes`, `conflicts` (at minimum, warn + degrade gracefully instead of erroring)
- [ ] Import a real published `grammar.json` (tree-sitter-python or tree-sitter-json) end to end and parse its corpus

### 13.3 — Import validation & round-trip
- [ ] `semtree import <grammar.json>` produces a `.semtree` DSL file (import → IR → DSL) so users can inspect/simplify
- [ ] Round-trip test: import grammar.json, emit `.semtree`, re-parse the `.semtree`, assert equivalent IR
- [ ] Diff report: list TS constructs that were dropped/degraded during import so gaps are visible, not silent

### 13.4 — Grammar authoring quality of life
- [ ] `semtree test` reports per-rule coverage (which grammar rules were exercised by the test corpus)
- [ ] Grammar debugger: step through a parse showing which rule matched each span (ROADMAP 5.1 carry-over)
- [ ] Actionable error messages for unreachable rules / empty alternatives with source spans

**Done when:** one language is corpus-clean with fields, and a real upstream tree-sitter grammar can be imported and parse its own corpus with a visible gap report.

---

## Phase 14 — Editor Integration, LSP-First (P4) 🔜

> Tree-sitter wins because it lives inside editors. Our wedge is the LSP: it already exists
> (`semtree lsp`) and works in every editor for free, but semantic tokens are disabled and
> features are thin. Make the LSP the primary, production-grade integration path.

### 14.1 — Fix and enable semantic tokens
- [ ] Make the runtime lexer emit distinct token kinds (keyword / operator / literal / string / number / comment) instead of a single generic `identifier` kind
- [ ] Re-enable `semantic_tokens_provider` in `server_capabilities()` (currently disabled with a TODO in `lsp.rs`)
- [ ] Map SemTree token classes → LSP semantic token types + modifiers (declaration, definition, readonly)
- [ ] Snapshot tests: semantic token output for a sample file per language

### 14.2 — LSP feature completeness
- [ ] Wire `rename` (already in `semtree_refactor`) into the LSP as `textDocument/rename` + `prepareRename`
- [ ] Add `textDocument/documentHighlight` (references of symbol under cursor)
- [ ] Add `textDocument/selectionRange` (syntax-aware expand/shrink selection)
- [ ] Add `textDocument/codeAction` exposing extract-variable / inline-variable from `semtree_refactor`
- [ ] Diagnostics: surface lint results + parse ERROR nodes as LSP diagnostics with ranges

### 14.3 — Robustness & lifecycle
- [ ] Handle `workspace/didChangeConfiguration` (grammar path, format style, enabled lint rules)
- [ ] Graceful degradation when no grammar matches a file extension (no crash, clear log)
- [ ] Incremental sync correctness tests: apply LSP `didChange` deltas and assert tree matches full reparse
- [ ] Cancellation + error responses conform to LSP spec (no silent request drops)

### 14.4 — Distribution for editors
- [ ] Ship a minimal VS Code extension that launches `semtree lsp` (stdio) and registers file types from installed grammars
- [ ] Neovim: document `vim.lsp.start` config pointing at `semtree lsp` as the recommended path (alongside the existing custom plugin)
- [ ] `semtree lsp --stdio` / `--tcp` transport flags
- [ ] Publish CLI to crates.io + rustdoc so `cargo install semtree_cli` works (ROADMAP 10.3 carry-over)

**Done when:** a fresh VS Code / Neovim user gets highlighting, symbols, go-to-def, references, rename, format, and diagnostics for a SemTree grammar via the LSP with no custom plugin required.

---

## Key Metrics vs Tree-sitter

| Metric | Tree-sitter | SemTree |
|--------|------------|---------|
| Cold parse speed | Baseline | **1.5-3.7x faster** |
| Incremental reparse | ~1-5ms | **Up to 5,419x faster** (node-level reuse + splice) |
| Error recovery | Good | **1.6-8.7x faster**, context-aware + token sync |
| Grammar authoring | grammar.js | DSL + validator + optimizer |
| Typed API | None (only CST) | Full typed AST with codegen |
| Semantic analysis | None | Built-in symbol table + scopes |
| Query language | S-expressions | S-expressions + typed API |
| Formatter | None | Built-in tree-walk formatter |
| Linter | None | Built-in rule engine |
| IDE protocol | External | Built-in services (tokens, nav, fold) |
| AI integration | None | First-class JSON API |
| Refactoring | None | Rename, extract, inline, tree edit |
| Plugin system | None | Trait-based with registry |
| C FFI | Built-in | Built-in (cdylib + staticlib) |
| Bindings | C only | C FFI + Python (PyO3) + WASM |
| Languages | 200+ | JSON, TOML, Python, Rust, JS, CSS + Tree-sitter import |

## Crate Summary (21 crates)

| Crate | Purpose |
|-------|---------|
| `semtree_core` | Foundation types (SyntaxKind, Token, TextSpan) |
| `semtree_lexer` | Unicode-aware lexer |
| `semtree_green` | Immutable green tree |
| `semtree_red` | Navigable red tree |
| `semtree_parser` | Event-based parser |
| `semtree_grammar` | Grammar IR, DSL, validator, optimizer |
| `semtree_ts_import` | Tree-sitter grammar importer |
| `semtree_runtime` | Grammar-driven runtime parser (RD + GLR) |
| `semtree_query` | S-expression tree queries |
| `semtree_ast` | Typed AST wrappers + codegen |
| `semtree_semantic` | Symbol table, scopes, references |
| `semtree_format` | Code formatter |
| `semtree_lint` | Rule-based linter |
| `semtree_ide` | IDE services (tokens, completion, navigation) |
| `semtree_refactor` | Refactoring API |
| `semtree_ai` | AI-friendly JSON APIs |
| `semtree_plugin` | Plugin system |
| `semtree_ffi` | C FFI API |
| `semtree_wasm` | WebAssembly build |
| `semtree_bench` | Benchmarks (vs Tree-sitter) |
| `semtree_cli` | Command-line interface |
