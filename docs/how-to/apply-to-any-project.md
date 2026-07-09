# How to apply SemTree to any project

This is the portable recipe: take **any** language (or DSL) and wire SemTree into your repo so teammates can parse, inspect, and editor-integrate it.

---

## Pattern overview

```
your-project/
├── grammars/
│   └── mylang.semtree      # language definition
├── samples/                # golden test inputs
├── tools/                  # optional: LSP, scripts
└── README.md               # how to use SemTree here
```

You do **not** need to vendor the whole SemTree source. Install the binary once; keep only the grammar in your project.

---

## Step 1 — Install SemTree once

```bash
cargo install --git https://github.com/Fanaperana/semtree.git --package semtree_cli
# or from a local clone:
# cargo install --path /path/to/semtree/crates/semtree_cli
```

## Step 2 — Add a grammar to your repo

```bash
mkdir -p grammars samples
semtree init --name mylang --output /tmp
cp /tmp/mylang/grammar.semtree grammars/mylang.semtree
```

Edit `grammars/mylang.semtree` for your syntax. Keep the **first rule** as the document root.

## Step 3 — Add sample files

```bash
# samples/hello.mylang
echo 'let x = 1;' > samples/hello.mylang
```

## Step 4 — Add a Makefile / script

```makefile
# Makefile
.PHONY: parse check format

parse:
	semtree run -g grammars/mylang.semtree -f sexp-pretty samples/hello.mylang

check:
	semtree check grammars/mylang.semtree

format:
	semtree format grammars/mylang.semtree
```

Teammates run `make parse` without learning SemTree internals.

## Step 5 — CI check (optional)

```yaml
# .github/workflows/grammar.yml
name: Grammar
on: [push, pull_request]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install --git https://github.com/Fanaperana/semtree.git --package semtree_cli
      - run: semtree check grammars/mylang.semtree
      - run: semtree run -g grammars/mylang.semtree -f tree samples/hello.mylang
```

## Step 6 — Editor integration

### Neovim (plugin)

Point the SemTree Neovim plugin at your binary, then:

```
:SemTreeInspect
```

For unknown extensions, always pass `-g`:

```bash
semtree run -g grammars/mylang.semtree samples/hello.mylang
```

### Neovim LSP

Follow [Build a Neovim LSP](neovim-lsp.md) and point it at `grammars/mylang.semtree`.

## Step 7 — Use from your own code

### CLI subprocess (any language)

```bash
semtree run -g grammars/mylang.semtree -f json samples/hello.mylang
```

Parse the JSON in Python/Node/etc.

### Rust crate

```rust
use semtree_grammar::parse_semtree_dsl;
use semtree_runtime::RuntimeParser;

let grammar = parse_semtree_dsl(include_str!("../grammars/mylang.semtree")).unwrap();
let parser = RuntimeParser::new(grammar);
let result = parser.parse("let x = 1;");
let root = result.syntax();
println!("{}", root.text());
```

### Python bindings / WASM

See the `bindings/` and WASM crates in the SemTree repo for embedding options.

---

## Checklist for a new language

- [ ] `grammars/<lang>.semtree` with clear entry rule
- [ ] `samples/` with at least 3 files (happy path + edge + broken)
- [ ] `semtree check` passes
- [ ] `semtree run -g ... -f tree` produces a sensible tree
- [ ] Document the one command teammates should run
- [ ] (Optional) Neovim plugin or LSP wired up

---

## Common project shapes

| Project type | What to keep in-repo | How people use it |
|--------------|----------------------|-------------------|
| Internal DSL | `grammars/*.semtree` + samples | CLI + CI |
| Open-source language | grammar + Neovim plugin config | docs + LSP |
| Config format | grammar only | `semtree run` in scripts |
| IDE product | grammar + Rust embedding | `RuntimeParser` / FFI |

You can start with CLI-only and add LSP later without changing the grammar.
