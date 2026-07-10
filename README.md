<div align="center">

# SemTree

### Universal Incremental Language Infrastructure

*A complete language toolchain from one grammar — parser, formatter, linter, refactoring, IDE, and AI APIs.*

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
| **Cold parse** | Baseline | ~2.3x faster on JSON, ~1.5–2.9x on CSS; slower on JS/Rust/Python |
| **Incremental** | Mature, fast (µs) | Correct & 100% lossless, but not yet optimized (see below) |
| **Memory** | Compact | Higher — builds more granular trees |
| **Error recovery** | Good | 100% lossless text; faster on most broken inputs |
| **Setup** | Install per-language parser | One binary, all languages |
| **Bindings** | C only | C FFI + Python (PyO3) + WASM |
| **Grammar format** | JavaScript DSL | Clean declarative DSL |
| **Parser algorithms** | LR + GLR | Recursive Descent + GLR |
| **Language** | C | Rust |

---

## Benchmarks: SemTree vs Tree-sitter

> Median of **100 iterations**, `--release`, 5 languages, measured against the real `tree-sitter`
> C parsers using the shipped [`grammars/*.semtree`](grammars/). Full raw output is committed at
> [`crates/semtree_bench/BENCHMARKS.txt`](crates/semtree_bench/BENCHMARKS.txt).
> Reproduce with `cargo run -p semtree_bench --release -- 100`.
>
> **Honest framing:** SemTree is a young Rust engine; tree-sitter is a mature, heavily optimized C
> parser. SemTree wins on some languages and on lossless error recovery, and ships an entire
> toolchain tree-sitter doesn't — but it loses on other languages, on memory, and on incremental
> reparsing today. We publish the losses too. Numbers are from an Apple Silicon Mac; your ratios
> will differ.

### Parse Speed (cold)

Ratio is SemTree median ÷ tree-sitter median (**faster** = SemTree quicker).

| Language | 1 KB | 10 KB | 100 KB | 1 MB |
|----------|------|-------|--------|------|
| **JSON** | 2.4x faster | 2.4x faster | 2.4x faster | 2.3x faster |
| **CSS** | 2.5x faster | 2.8x faster | 1.5x faster | 4.2x slower |
| **Python** | 1.2x slower | 1.1x slower | 1.1x slower | 1.1x slower |
| **JavaScript** | 1.3x slower | 1.3x slower | 1.7x slower | 6.1x slower |
| **Rust** | 2.0x slower | 2.0x slower | 2.2x slower | 5.1x slower |

SemTree's recursive-descent runtime is faster on simpler grammars (JSON, CSS) but slower on the
richer ones, and scaling degrades on very large (1 MB) inputs. Faster parsing on complex grammars
and large files is an open optimization target (arena allocation, zero-copy tokens — ROADMAP 4.4).

### Incremental Reparse

Both sides measure **only** the reparse step (the initial parse is excluded): tree-sitter uses
`edit() + parse(old)`, SemTree uses `IncrementalParser::update()`. Every SemTree result is verified
to reproduce the edited source losslessly.

| Edit (10 KB) | Tree-sitter | SemTree | SemTree reuse |
|--------------|-------------|---------|---------------|
| Insert char (JSON) | 15 µs | 351 µs | miss — 0% reused |
| Insert char (JS) | 42 µs | 1.40 ms | miss — 0% reused |
| Append block (JSON) | 17 µs | 214 µs | sibling splice — 100% |

SemTree's incremental path is **correct but not yet fast**: a mid-file single-character insert
currently falls back to a full reparse (`SpliceMiss`, 0% reused), and even a successful splice
rebuilds the green tree in O(n). Real subtree reuse is tracked in ROADMAP 4.1 / 11.5 — **no
incremental speed claims are made until that lands.**

### Error Recovery (100% lossless)

SemTree preserves **100% of the source text** on every broken input, and is faster on most:

| Broken Code | Tree-sitter | SemTree | Speed |
|-------------|-------------|---------|-------|
| Unclosed braces (JS) | 77.2 µs | 16.3 µs | **4.7x faster** |
| Missing colons (CSS) | 34.5 µs | 8.8 µs | **3.9x faster** |
| Invalid JSON | 20.8 µs | 6.8 µs | **3.1x faster** |
| Garbage tokens (JS) | 30.6 µs | 14.8 µs | **2.1x faster** |
| Mixed valid/invalid (Rust) | 62.5 µs | 30.6 µs | **2.0x faster** |
| Missing semicolons (JS) | 20.8 µs | 27.1 µs | 1.3x slower |
| Indentation errors (Python) | 19.5 µs | 40.7 µs | 2.1x slower |

All SemTree trees retain every byte of source; tree-sitter is occasionally more precise about the
number of error regions (e.g. Rust, Python).

### Memory

SemTree currently uses **more** memory than tree-sitter — it builds more granular trees. Node
elision (single-child precedence-chain collapse) has cut the gap substantially (JS/Rust/Python
node counts down ~30–40%), but there's more to do (compact/interned node storage). Reducing node
count and bytes-per-node is tracked in ROADMAP 15.A/15.B.

| Language (10 KB) | Tree-sitter nodes | SemTree nodes |
|------------------|-------------------|---------------|
| JSON | 6,876 | 10,120 |
| CSS | 3,597 | 10,062 |
| Python | 4,423 | 16,944 |
| JavaScript | 4,577 | 19,144 |
| Rust | 4,222 | 20,080 |

### Features Only SemTree Has

Where SemTree unambiguously leads: it's an entire toolchain from one grammar, not just a parser.

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
cargo run -p semtree_bench --release -- 100   # 100 iterations (matches the tables above)
cargo run -p semtree_bench --release -- 30    # quick run
```

The run prints a per-test breakdown and an explicit "Where SemTree is SLOWER" list, and writes
nothing hidden — the committed [`BENCHMARKS.txt`](crates/semtree_bench/BENCHMARKS.txt) is the exact
output of the 100-iteration run.

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

**Built with Rust** · **One Grammar, a Whole Toolchain** · **Parser · Formatter · Linter · IDE · AI**

</div>
