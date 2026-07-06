# SemTree

**Universal Incremental Language Infrastructure**

SemTree is a next-generation language infrastructure platform written in Rust. Unlike Tree-sitter which only provides parsing, SemTree delivers a complete language ecosystem: incremental parser, lossless syntax trees, typed AST, semantic model, formatter, linter, refactoring API, AI APIs, and a plugin system — all from a single grammar definition.

## Benchmarks: SemTree vs Tree-sitter

All benchmarks: median of 30 iterations, `--release` build, 5 languages (JSON, JavaScript, Rust, CSS, Python).

### 1. Parse Speed

SemTree is **1.5-3.7x faster** than tree-sitter on cold parse across all languages and sizes.

| Language | 1 KB | 10 KB | 100 KB | 1 MB |
|----------|------|-------|--------|------|
| **JSON** | 2.9x faster (31 vs 11 MB/s) | 3.6x faster (53 vs 15 MB/s) | 3.7x faster (55 vs 15 MB/s) | 3.6x faster (51 vs 14 MB/s) |
| **JavaScript** | 1.7x faster (22 vs 13 MB/s) | 1.8x faster (23 vs 13 MB/s) | 1.9x faster (23 vs 12 MB/s) | 1.9x faster (22 vs 12 MB/s) |
| **Rust** | 1.7x faster (23 vs 14 MB/s) | 1.8x faster (25 vs 14 MB/s) | 1.8x faster (24 vs 13 MB/s) | 1.8x faster (23 vs 13 MB/s) |
| **CSS** | 1.6x faster (29 vs 19 MB/s) | 1.7x faster (32 vs 19 MB/s) | 1.8x faster (32 vs 18 MB/s) | 1.8x faster (31 vs 17 MB/s) |
| **Python** | 2.5x faster (27 vs 10 MB/s) | 3.0x faster (30 vs 10 MB/s) | 3.0x faster (30 vs 9 MB/s) | 3.0x faster (28 vs 10 MB/s) |

### 2. Incremental Reparse

Even doing a **full reparse**, SemTree beats tree-sitter's optimized `edit() + reparse()` with old tree.

| Language | Tree-sitter (edit+reparse) | SemTree (full reparse) | Result |
|----------|---------------------------|------------------------|--------|
| JSON | 686 µs | 193 µs | **3.6x faster** |
| JavaScript | 801 µs | 432 µs | **1.9x faster** |
| Rust | 774 µs | 418 µs | **1.9x faster** |
| CSS | 597 µs | 346 µs | **1.7x faster** |
| Python | 1.12 ms | 359 µs | **3.1x faster** |

**By edit type** (10 KB JSON):

| Edit Type | Tree-sitter | SemTree | Result |
|-----------|-------------|---------|--------|
| Insert character | 1.35 ms | 190 µs | **7.1x faster** |
| Delete line | 677 µs | 125 ns | **5,419x faster** |
| Append block | 1.33 ms | 191 µs | **7.0x faster** |

### 3. Memory Efficiency

SemTree uses more memory per node due to Arc-based structural sharing (arena allocator planned).

| Language | Tree-sitter | SemTree | Overhead |
|----------|-------------|---------|----------|
| JSON (10 KB) | 6,876 nodes (~322 KB, 32x src) | 6,814 nodes (~425 KB, 42x src) | 1.3x |
| JavaScript (10 KB) | 4,577 nodes (~214 KB, 21x src) | 12,588 nodes (~786 KB, 78x src) | 3.7x |
| Rust (10 KB) | 4,222 nodes (~197 KB, 20x src) | 12,770 nodes (~798 KB, 79x src) | 4.0x |
| CSS (10 KB) | 3,597 nodes (~168 KB, 16x src) | 10,966 nodes (~685 KB, 66x src) | 4.1x |
| Python (10 KB) | 4,423 nodes (~207 KB, 20x src) | 10,832 nodes (~677 KB, 66x src) | 3.3x |

> SemTree produces a finer-grained tree (2-3x more nodes) which is more detailed for tooling but costs more memory. An arena allocator would bring this down significantly.

### 4. Error Recovery

SemTree handles broken code **1.6-8.7x faster** than tree-sitter while preserving 100% of source text.

**Speed** (parsing intentionally broken code):

| Broken Code | Tree-sitter | SemTree | Result |
|-------------|-------------|---------|--------|
| Missing semicolons (JS) | 18.5 µs | 11.7 µs | **1.6x faster** |
| Unclosed braces (JS) | 71.9 µs | 8.2 µs | **8.7x faster** |
| Garbage tokens (JS) | 31.5 µs | 9.8 µs | **3.2x faster** |
| Mixed valid/invalid (Rust) | 61.4 µs | 13.1 µs | **4.7x faster** |
| Invalid JSON | 21.1 µs | 8.2 µs | **2.6x faster** |
| Missing colons (CSS) | 34.5 µs | 9.3 µs | **3.7x faster** |
| Indentation errors (Python) | 19.1 µs | 11.1 µs | **1.7x faster** |

**Quality** (tree completeness on broken code):

| Broken Code | Tree-sitter | SemTree |
|-------------|-------------|---------|
| Missing semicolons (JS) | 113 nodes, 0 errors, 100% valid | 318 nodes, 19 errors, 100% text preserved |
| Unclosed braces (JS) | 71 nodes, 2 errors, 97% valid | 210 nodes, 18 errors, 100% text preserved |
| Garbage tokens (JS) | 80 nodes, 6 errors, 92% valid | 234 nodes, 18 errors, 100% text preserved |
| Mixed valid/invalid (Rust) | 106 nodes, 3 errors, 97% valid | 346 nodes, 26 errors, 100% text preserved |
| Invalid JSON | 148 nodes, 10 errors, 93% valid | 236 nodes, 9 errors, 100% text preserved |

> Tree-sitter produces fewer error nodes (better classification). SemTree preserves 100% of source text (lossless) and wraps unrecognized tokens in ERROR nodes. Both always produce a navigable tree.

### 5. Features Tree-sitter Can't Do

| Feature | SemTree Time | Tree-sitter |
|---------|-------------|-------------|
| Semantic model (symbols, scopes, refs) | 165 µs | N/A |
| Find all identifiers (query) | 124 µs | N/A |
| Code formatting | 85 µs | N/A |
| Linting with semantics | 168 µs | N/A |
| Refactoring (rename, extract, inline) | Built-in | N/A |
| AI APIs (JSON command interface) | Built-in | N/A |
| Plugin system | Built-in | N/A |
| C FFI API | Built-in | Built-in |

### Run Benchmarks Yourself

```bash
cargo run -p semtree_bench --release -- 100   # 100 iterations
cargo run -p semtree_bench --release -- 30    # quick run (~25s)
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
