# SemTree

**Universal Incremental Language Infrastructure**

SemTree is a next-generation language infrastructure platform written in Rust. Unlike Tree-sitter which only provides parsing, SemTree delivers a complete language ecosystem: incremental parser, lossless syntax trees, typed AST, semantic model, formatter, linter, refactoring API, AI APIs, and a plugin system — all from a single grammar definition.

## Benchmarks: SemTree vs Tree-sitter

Measured on real-world inputs across 5 languages. Median of 50 iterations, `--release` build.

### Cold Parse Throughput

| Language | Size | Tree-sitter | SemTree | Result |
|----------|------|-------------|---------|--------|
| JSON | 1 KB | 16 MB/s | 44 MB/s | **2.8x faster** |
| JSON | 10 KB | 15 MB/s | 53 MB/s | **3.4x faster** |
| JSON | 100 KB | 15 MB/s | 53 MB/s | **3.6x faster** |
| JSON | 1 MB | 14 MB/s | 43 MB/s | **3.4x faster** |
| JavaScript | 1 KB | 13 MB/s | 22 MB/s | **1.6x faster** |
| JavaScript | 10 KB | 13 MB/s | 23 MB/s | **1.7x faster** |
| JavaScript | 100 KB | 12 MB/s | 23 MB/s | **1.9x faster** |
| JavaScript | 1 MB | 11 MB/s | 22 MB/s | **1.8x faster** |
| Rust | 1 KB | 14 MB/s | 24 MB/s | **1.7x faster** |
| Rust | 10 KB | 14 MB/s | 25 MB/s | **1.8x faster** |
| Rust | 100 KB | 12 MB/s | 22 MB/s | **1.9x faster** |
| Rust | 1 MB | 13 MB/s | 23 MB/s | **1.7x faster** |
| CSS | 1 KB | 19 MB/s | 28 MB/s | **1.5x faster** |
| CSS | 10 KB | 19 MB/s | 32 MB/s | **1.7x faster** |
| CSS | 100 KB | 18 MB/s | 32 MB/s | **1.8x faster** |
| CSS | 1 MB | 17 MB/s | 30 MB/s | **1.7x faster** |
| Python | 1 KB | 10 MB/s | 28 MB/s | **2.7x faster** |
| Python | 10 KB | 10 MB/s | 31 MB/s | **3.1x faster** |
| Python | 100 KB | 9 MB/s | 31 MB/s | **3.3x faster** |
| Python | 1 MB | 9 MB/s | 27 MB/s | **3.1x faster** |

### Incremental Reparse (10 KB files)

| Language | Tree-sitter (edit + reparse) | SemTree (full reparse) | Result |
|----------|------------------------------|------------------------|--------|
| JSON | 660.9 µs | 194.2 µs | **3.4x faster** |
| JavaScript | 828.7 µs | 440.6 µs | **1.9x faster** |
| Rust | 758.5 µs | 411.4 µs | **1.8x faster** |
| CSS | 590.9 µs | 332.1 µs | **1.8x faster** |
| Python | 1.10 ms | 345.1 µs | **3.2x faster** |

### Tree Traversal (10 KB files, full DFS)

| Language | Tree-sitter | SemTree | Notes |
|----------|-------------|---------|-------|
| JSON | 249.2 µs (6,876 nodes) | 113.8 µs (6,814 nodes) | **2.2x faster** |
| JavaScript | 217.7 µs (4,577 nodes) | 228.9 µs (12,588 nodes) | 1.05x slower (3x more nodes) |
| Rust | 213.1 µs (4,222 nodes) | 219.7 µs (12,770 nodes) | 1.03x slower (3x more nodes) |
| CSS | 127.6 µs (3,597 nodes) | 187.3 µs (10,966 nodes) | 1.5x slower (3x more nodes) |
| Python | 225.9 µs (4,423 nodes) | 196.1 µs (10,832 nodes) | **1.2x faster** (2.4x more nodes) |

### Features Tree-sitter Can't Do

| Feature | SemTree Time | Tree-sitter |
|---------|-------------|-------------|
| Semantic model (symbols, scopes, refs) | 165 µs | N/A |
| Find all identifiers (query) | 126 µs | N/A |
| Code formatting | 82 µs | N/A |
| Linting with semantics | 165 µs | N/A |
| Refactoring (rename, extract, inline) | Built-in | N/A |
| AI APIs (JSON command interface) | Built-in | N/A |
| Plugin system | Built-in | N/A |
| C FFI API | Built-in | Built-in |

### Run Benchmarks Yourself

```bash
cargo run -p semtree_bench --release -- 100
```

---

## Architecture

```
Source Code → Lexer → Token Stream → Parser → Green Tree → Red Tree → Typed AST → Semantic DB
```

### Crates (19)

| Crate | Description |
|-------|-------------|
| `semtree_core` | Foundation types: `SyntaxKind`, `Token`, `Trivia`, `TextSpan`, `Interner` |
| `semtree_lexer` | Unicode-aware lexer with trivia preservation |
| `semtree_green` | Immutable green tree with Arc-based structural sharing |
| `semtree_red` | Navigable red tree with parent/sibling/ancestor traversal |
| `semtree_parser` | Event-based parser with Pratt expression parsing |
| `semtree_grammar` | Grammar IR, DSL parser, validator, optimizer |
| `semtree_ts_import` | Tree-sitter `grammar.json` importer |
| `semtree_runtime` | Grammar-driven runtime parser (Grammar IR → working parser) |
| `semtree_query` | S-expression tree query engine with captures |
| `semtree_ast` | Typed AST wrappers + codegen + visitor generation |
| `semtree_semantic` | Symbol table, scope tree, references, diagnostics |
| `semtree_format` | Syntax-tree-driven code formatter |
| `semtree_lint` | Rule-based linter with built-in rules |
| `semtree_ide` | IDE services: semantic tokens, completion, navigation, folding |
| `semtree_refactor` | Refactoring API: rename, extract, inline, tree edit |
| `semtree_ai` | AI-friendly JSON APIs for agent integration |
| `semtree_plugin` | Plugin system with trait-based extensibility |
| `semtree_ffi` | C FFI API (cdylib + staticlib) |
| `semtree_cli` | CLI: parse, check, query, format, lint, benchmark, generate, test |

## Quick Start

```bash
# Build
cargo build

# Run all 176 tests
cargo test

# Parse a file
cargo run --bin semtree -- parse example.rs

# Parse as JSON
cargo run --bin semtree -- parse example.rs --format json

# Parse as S-expression
cargo run --bin semtree -- parse example.rs --format sexp

# Parse with a grammar
cargo run --bin semtree -- run source.js --grammar grammars/javascript.semtree

# Query for all functions
cargo run --bin semtree -- query example.rs Function

# Format code
cargo run --bin semtree -- format example.rs

# Lint code
cargo run --bin semtree -- lint example.rs

# Show symbols
cargo run --bin semtree -- symbols example.rs

# Benchmark parsing
cargo run --bin semtree -- benchmark example.rs --iterations 1000

# Import a Tree-sitter grammar
cargo run --bin semtree -- import grammar.json --output grammar.semtree.json

# Generate typed AST code
cargo run --bin semtree -- generate grammar.semtree

# Initialize a new language project
cargo run --bin semtree -- init --name my_lang

# System diagnostics
cargo run --bin semtree -- doctor
```

## Language Grammars

SemTree includes grammars for 6 languages:

| Language | Grammar File | Status |
|----------|-------------|--------|
| JSON | `grammars/json.semtree` | Full coverage |
| TOML | `grammars/toml.semtree` | Full coverage |
| JavaScript | `grammars/javascript.semtree` | Comprehensive (statements, expressions, classes, modules) |
| Python | `grammars/python.semtree` | Comprehensive (decorators, comprehensions, type hints) |
| Rust | `grammars/rust.semtree` | Comprehensive (items, patterns, types, lifetimes) |
| CSS | `grammars/css.semtree` | Comprehensive (selectors, at-rules, values, functions) |

### Grammar DSL

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
    "(" ParameterList? ")"

ParameterList :=
    Parameter ParameterTail*

ParameterTail :=
    "," Parameter

indent Block
linebreak Function
space around "+"
```

## Tree Architecture

**Green Tree** — Immutable, structurally shared, no parent pointers. Enables incremental reparsing by reusing unchanged subtrees across edits.

**Red Tree** — On-demand wrapper providing parent pointers, sibling navigation, ancestor traversal, and absolute text offsets. Cheap to create from any green tree root.

## Requirements

- Rust 1.85+ (edition 2024)

## License

MIT OR Apache-2.0
