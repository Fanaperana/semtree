# SemTree

**Universal Incremental Language Infrastructure**

SemTree is a next-generation language infrastructure platform written in Rust. It provides a complete language ecosystem: incremental parser, lossless syntax trees, typed AST generation, semantic model, query engine, and more.

## Architecture

```
Source Code → Lexer → Token Stream → Parser → Green Tree → Red Tree → Typed AST → Semantic DB
```

### Crates

| Crate | Description |
|-------|-------------|
| `semtree_core` | Core types: `SyntaxKind`, `Token`, `Trivia`, `TextSpan`, `Interner` |
| `semtree_lexer` | Unicode-aware lexer with trivia preservation and incremental support |
| `semtree_green` | Immutable, compact green tree with structural sharing via `Arc` |
| `semtree_red` | Navigable red tree wrapper with parent/sibling/ancestor traversal |
| `semtree_parser` | Event-based parser with Pratt expression parsing and error recovery |
| `semtree_grammar` | Grammar IR, SemTree DSL parser, and grammar validation |
| `semtree_ts_import` | Tree-sitter `grammar.json` → SemTree Grammar IR converter |
| `semtree_cli` | CLI tool: `semtree parse`, `check`, `init`, `query`, `benchmark`, etc. |

## Quick Start

```bash
# Build
cargo build

# Run tests
cargo test

# Parse a file
cargo run --bin semtree -- parse example.rs

# Parse as JSON
cargo run --bin semtree -- parse example.rs --format json

# Parse as S-expression
cargo run --bin semtree -- parse example.rs --format sexp

# Initialize a new language project
cargo run --bin semtree -- init --name my_lang

# Check a grammar
cargo run --bin semtree -- check my_lang/grammar.semtree

# Query for all functions
cargo run --bin semtree -- query example.rs Function

# Benchmark parsing
cargo run --bin semtree -- benchmark example.rs --iterations 1000

# Import a Tree-sitter grammar
cargo run --bin semtree -- import grammar.json --output grammar.semtree.json

# System diagnostics
cargo run --bin semtree -- doctor
```

## Grammar DSL

SemTree grammars are defined in `.semtree` files:

```
language rust

keyword fn
keyword let
keyword struct

Function :=
    "fn"
    name: Identifier
    Parameters
    Block

Parameters :=
    "(" ")"

indent Block
linebreak Function
space around "+"
```

## Tree Architecture

**Green Tree** — Immutable, structurally shared, no parent pointers. Enables incremental reparsing by reusing unchanged subtrees across edits.

**Red Tree** — On-demand wrapper providing parent pointers, sibling navigation, ancestor traversal, and absolute text offsets. Cheap to create from any green tree root.

## Requirements

- Rust 1.85+ (edition 2024)
