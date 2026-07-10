<div align="center">

# SemTree

### Universal Incremental Language Infrastructure

*The parsing engine that beats Tree-sitter — with a complete language toolchain built in.*

[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Crates](https://img.shields.io/badge/Crates-21-brightgreen)](ROADMAP.md)
[![Tests](https://img.shields.io/badge/Tests-297_passing-success)](https://github.com/Fanaperana/semtree)
[![GLR](https://img.shields.io/badge/Parser-RD_%2B_GLR-blueviolet)]()
[![Languages](https://img.shields.io/badge/Grammars-6_languages-ff69b4)]()
[![Neovim](https://img.shields.io/badge/Neovim-Plugin-57A143?logo=neovim&logoColor=white)]()
[![WASM](https://img.shields.io/badge/WASM-Ready-624DE5?logo=webassembly&logoColor=white)]()
[![Python](https://img.shields.io/badge/Python-Bindings-3776AB?logo=python&logoColor=white)]()

---

**Parser** · **Formatter** · **Linter** · **Refactoring** · **IDE Services** · **AI APIs** · **Plugin System**

*All from a single grammar definition.*

[Docs](docs/) · [Getting Started](#quick-start) · [Benchmarks](#benchmarks-semtree-vs-tree-sitter) · [Neovim](#neovim-integration) · [Architecture](#architecture) · [Roadmap](ROADMAP.md)

</div>

---

## Why SemTree?

| | Tree-sitter | SemTree |
|--|------------|---------|
| **What you get** | Parser only | Parser + formatter + linter + refactoring + IDE + AI APIs |
| **Parse speed** | Baseline | **1.5-3.7x faster** |
| **Incremental** | Edit + reparse | **Up to 5,419x faster** on deletions |
| **Setup** | Install per-language parser | One binary, all languages |
| **Bindings** | C only | C FFI + Python (PyO3) + WASM |
| **Grammar format** | JavaScript DSL | Clean declarative DSL |
| **Parser algorithms** | LR + GLR | Recursive Descent + GLR |
| **Language** | C | Rust |
| **Error recovery** | Good | **1.6-8.7x faster**, 100% lossless |

---

## Benchmarks: SemTree vs Tree-sitter

> All benchmarks: median of 30 iterations, `--release` build, 5 languages.

### Parse Speed

SemTree is **1.5-3.7x faster** than tree-sitter on cold parse across all languages and sizes.

| Language | 1 KB | 10 KB | 100 KB | 1 MB |
|----------|------|-------|--------|------|
| **JSON** | 2.9x faster | 3.6x faster | 3.7x faster | 3.6x faster |
| **JavaScript** | 1.7x faster | 1.8x faster | 1.9x faster | 1.9x faster |
| **Rust** | 1.7x faster | 1.8x faster | 1.8x faster | 1.8x faster |
| **CSS** | 1.6x faster | 1.7x faster | 1.8x faster | 1.8x faster |
| **Python** | 2.5x faster | 3.0x faster | 3.0x faster | 3.0x faster |

### Incremental Reparse

Even doing a **full reparse**, SemTree beats tree-sitter's optimized `edit() + reparse()`.

| Edit Type | Tree-sitter | SemTree | Speedup |
|-----------|-------------|---------|---------|
| Insert character | 1.35 ms | 190 us | **7.1x** |
| Delete line | 677 us | 125 ns | **5,419x** |
| Append block | 1.33 ms | 191 us | **7.0x** |

### Error Recovery

SemTree handles broken code **1.6-8.7x faster** while preserving 100% of source text.

| Broken Code | Tree-sitter | SemTree | Speedup |
|-------------|-------------|---------|---------|
| Missing semicolons (JS) | 18.5 us | 11.7 us | **1.6x** |
| Unclosed braces (JS) | 71.9 us | 8.2 us | **8.7x** |
| Garbage tokens (JS) | 31.5 us | 9.8 us | **3.2x** |
| Mixed valid/invalid (Rust) | 61.4 us | 13.1 us | **4.7x** |
| Invalid JSON | 21.1 us | 8.2 us | **2.6x** |

### Features Only SemTree Has

| Feature | Status |
|---------|--------|
| Semantic model (symbols, scopes, references) | Built-in |
| Code formatting | Built-in |
| Linting with semantics | Built-in |
| Refactoring (rename, extract, inline) | Built-in |
| AI APIs (JSON command interface) | Built-in |
| Plugin system | Built-in |
| Interactive tree inspector (Neovim) | Built-in |
| GLR parser for ambiguous grammars | Built-in |

<details>
<summary><b>Run Benchmarks Yourself</b></summary>

```bash
cargo run -p semtree_bench --release -- 100   # 100 iterations
cargo run -p semtree_bench --release -- 30    # quick run
```

</details>

---

## Documentation

Full docs use the [Diátaxis](https://diataxis.fr/) structure:

| Section | For |
|---------|-----|
| [Tutorials](docs/tutorials/) | First-time learning path |
| [How-to guides](docs/how-to/) | CLI, grammars, Neovim LSP, any project |
| [Reference](docs/reference/) | Complete DSL syntax + CLI flags |
| [Explanation](docs/explanation/) | Architecture, RD vs GLR, vs Tree-sitter |

Start at **[docs/README.md](docs/README.md)**.  
Neovim LSP example: [`docs/how-to/examples/todo-lsp/`](docs/how-to/examples/todo-lsp/).

## Quick Start

```bash
# Clone and build
git clone https://github.com/Fanaperana/semtree.git
cd semtree
cargo build

# Install the CLI
cargo install --path crates/semtree_cli

# Parse a Python file (grammar auto-detected)
semtree run myfile.py

# Pretty-printed tree
semtree run -f sexp-pretty myfile.py

# Indented tree with byte ranges
semtree run -f tree myfile.py

# JSON output
semtree run -f json myfile.py

# Use the GLR parser backend
semtree run --backend glr -f tree myfile.py

# Lint, format, query
semtree lint myfile.rs
semtree format myfile.rs
semtree query myfile.rs Function
semtree symbols myfile.rs
```

---

## Neovim Integration

SemTree includes a Neovim plugin with an **interactive tree inspector** — navigate the syntax tree and see source code highlighted in real time, just like tree-sitter's `:InspectTree`.

### Install

Add to your `lazy.nvim` config (`~/.config/nvim/lua/plugins/init.lua`):

```lua
{
    dir = "/path/to/semtree/editors/neovim",
    name = "semtree",
    lazy = false,
    config = function()
        require("semtree").setup({
            binary_path = nil,  -- auto-detect from PATH
        })
    end,
},
```

### Commands

| Command | Description |
|---------|-------------|
| `:SemTreeInspect` | Interactive tree inspector with real-time highlighting |
| `:SemTreeParse` | Pretty-printed syntax tree |
| `:SemTreeSymbols` | List all symbols |
| `:SemTreeLint` | Inline diagnostics |
| `:SemTreeFormat` | Format buffer |

### Inspector Keybindings

| Key | Action |
|-----|--------|
| `j`/`k` | Navigate nodes (source highlights automatically) |
| `Enter` | Jump to source location |
| `q` | Close inspector |

> See [`examples/neovim-setup/README.md`](examples/neovim-setup/README.md) for the complete setup guide.

---

## Grammar DSL

SemTree grammars are clean, declarative `.semtree` files:

```
language rust

keyword fn
keyword let
keyword struct

Function :=
    "fn" name:Identifier Parameters Block

Parameters :=
    "(" ParameterList? ")"

ParameterList :=
    Parameter ParameterTail*

ParameterTail :=
    "," Parameter
```

**6 languages included**: JSON, TOML, JavaScript, Python, Rust, CSS.

Import tree-sitter grammars: `semtree import grammar.json`

---

## Architecture

```
Source Code --> Lexer --> Tokens --> Parser --> Green Tree --> Red Tree --> Typed AST --> Semantic DB
                                      |            |
                                  RD / GLR    Arc-shared
                                              immutable
```

### 21 Crates

| Layer | Crates |
|-------|--------|
| **Core** | `semtree_core` · `semtree_lexer` · `semtree_green` · `semtree_red` |
| **Parsing** | `semtree_parser` · `semtree_grammar` · `semtree_runtime` · `semtree_ts_import` |
| **Analysis** | `semtree_query` · `semtree_ast` · `semtree_semantic` |
| **Tooling** | `semtree_format` · `semtree_lint` · `semtree_ide` · `semtree_refactor` |
| **Integration** | `semtree_ai` · `semtree_plugin` · `semtree_ffi` · `semtree_cli` |
| **Distribution** | `semtree_wasm` · `semtree_bench` |

### Parser Backends

| Backend | Algorithm | Best For |
|---------|-----------|----------|
| **RD** (default) | Recursive descent with backtracking | Most grammars, fastest for unambiguous languages |
| **GLR** | Generalized LR with Graph-Structured Stack | Ambiguous grammars, conflict resolution |

Select with `--backend glr` or let SemTree auto-detect.

### Tree Architecture

- **Green Tree** — Immutable, `Arc`-shared, no parent pointers. Enables incremental reparsing by reusing unchanged subtrees.
- **Red Tree** — On-demand wrapper with parent/sibling/ancestor navigation and absolute offsets.
- **SPPF** — Shared Packed Parse Forest for compact ambiguity representation (GLR backend).

---

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the full roadmap. Phases 1-11 are complete:

- [x] Phase 1-3: Core infrastructure, parser, typed AST, semantics
- [x] Phase 4: Performance parity with tree-sitter
- [x] Phase 5: Language ecosystem (6 grammars)
- [x] Phase 6-7: IDE services, refactoring API
- [x] Phase 8-9: AI APIs, plugin system
- [x] Phase 10: C FFI, CLI tools, Python bindings
- [x] Phase 11: GLR/RNGLR parser engine

---

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

[MIT](LICENSE)

---

<div align="center">

**Built with Rust** · **Faster than Tree-sitter** · **Complete Language Toolchain**

</div>
