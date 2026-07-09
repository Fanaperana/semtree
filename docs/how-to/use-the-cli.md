# How to use the `semtree` binary

## Parse with auto-detected grammar

```bash
semtree run path/to/file.py
semtree run path/to/file.js
semtree run path/to/file.rs
```

Supported extensions: `.py`, `.js`/`.jsx`/`.mjs`, `.rs`, `.css`, `.json`, `.toml`.

## Parse with an explicit grammar

```bash
semtree run -g grammars/python.semtree myfile.py
semtree run -g ./my-lang/grammar.semtree sample.mylang
```

## Choose an output format

```bash
semtree run -f tree file.py          # indented tree + ranges
semtree run -f sexp-pretty file.py   # pretty S-expression
semtree run -f sexp file.py          # compact one-line sexp
semtree run -f json file.py          # JSON with ranges
semtree run -f inspect file.py       # machine format for editors
```

## Choose a parser backend

```bash
semtree run --backend rd file.py     # recursive descent (default)
semtree run --backend glr file.py    # GLR (ambiguous grammars)
```

## Format files

```bash
# Source files (Rust-oriented formatter today)
semtree format example.rs

# Grammar files (dedicated DSL formatter)
semtree format grammars/python.semtree > /tmp/out.semtree
```

## Validate a grammar

```bash
semtree check grammars/json.semtree
```

## Query / symbols / lint

```bash
semtree query example.rs Function
semtree symbols example.rs
semtree lint example.rs
```

## Scaffold a new language project

```bash
semtree init --name mylang --output .
```

## Import Tree-sitter

```bash
semtree import path/to/grammar.json -o mylang.semtree.json
semtree migrate path/to/grammar.json -o mylang.semtree.json
```

## Generate typed AST Rust code

```bash
semtree generate grammars/json.semtree -o /tmp/json_ast.rs
```

## Benchmark

```bash
semtree benchmark examples/demo.py -i 100
```

## Health check

```bash
semtree doctor
```

Full flag list: [CLI reference](../reference/cli.md).
