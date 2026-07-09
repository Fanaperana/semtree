# Architecture

SemTree is a Cargo workspace of focused crates. The important idea: **one grammar IR**, many consumers.

```
.semtree DSL ──► Grammar IR ──► RuntimeParser (RD)
                     │              │
                     │              ├── Green tree ──► Red tree ──► tools
                     │              │
                     └──────────► GlrParser (GLR / GSS / SPPF)
```

## Layers

| Layer | Crates | Role |
|-------|--------|------|
| Core | `semtree_core`, `lexer`, `green`, `red` | Tokens + trees |
| Grammar | `semtree_grammar`, `semtree_ts_import` | DSL / import → IR |
| Parse | `semtree_parser`, `semtree_runtime` | Event parser + grammar-driven RD/GLR |
| Analyze | `query`, `ast`, `semantic` | Queries, typed AST, symbols |
| Tools | `format`, `lint`, `ide`, `refactor` | Product features |
| Integrate | `ai`, `plugin`, `ffi`, `cli` | Embeddings + CLI |

## Why this shape

- **Grammar IR** lets DSL, Tree-sitter import, and future frontends share one parser engine.
- **Green trees** make incremental reuse cheap (immutable, shared).
- **Red trees** give IDE-friendly navigation without storing parents in the green layer.
- **CLI** is a thin façade — the same APIs power Neovim and the LSP example.

## Where to look in the code

| Concern | Path |
|---------|------|
| DSL parser | `crates/semtree_grammar/src/dsl.rs` |
| DSL formatter | `crates/semtree_grammar/src/format_dsl.rs` |
| RD runtime | `crates/semtree_runtime/src/runtime_parser.rs` |
| GLR engine | `crates/semtree_runtime/src/glr/` |
| CLI entry | `crates/semtree_cli/src/main.rs` |
| Neovim plugin | `editors/neovim/` |
