# Tutorial: Parse your first file

**Goal:** parse a real source file and see its syntax tree.

**Time:** ~5 minutes

**You need:** SemTree installed ([previous tutorial](01-install.md))

---

## 1. Parse a demo file

From the SemTree repo root:

```bash
semtree run examples/demo.py
```

SemTree auto-detects the language from the `.py` extension and uses `grammars/python.semtree`.

## 2. Try different output formats

```bash
# Pretty S-expression (readable)
semtree run -f sexp-pretty examples/demo.py

# Indented tree with byte ranges
semtree run -f tree examples/demo.py

# JSON (for tools / scripts)
semtree run -f json examples/demo.py | head -40
```

## 3. Parse another language

```bash
semtree run grammars/tests/test.json
semtree run -f sexp-pretty examples/demo.toml
```

## 4. Point at a grammar explicitly

```bash
semtree run -g grammars/python.semtree -f tree examples/demo.py
```

Useful when the extension is unknown or you want a custom grammar.

## What you just did

You used the grammar-driven parser (`semtree run`) — the same engine Neovim and the LSP example use. Next: [Write your first grammar](03-first-grammar.md).
