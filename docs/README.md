# SemTree Documentation

Documentation follows the [Diátaxis](https://diataxis.fr/) framework — four kinds of docs for four kinds of needs.

| You want to… | Go here |
|--------------|---------|
| **Learn by doing** (first time) | [Tutorials](tutorials/) |
| **Solve a concrete task** | [How-to guides](how-to/) |
| **Look up exact syntax / flags** | [Reference](reference/) |
| **Understand why SemTree works this way** | [Explanation](explanation/) |

---

## Start here

1. [Install SemTree](tutorials/01-install.md) — 5 minutes
2. [Parse your first file](tutorials/02-parse-first-file.md) — 5 minutes
3. [Write your first grammar](tutorials/03-first-grammar.md) — 15 minutes
4. [Use SemTree in Neovim](tutorials/04-neovim.md) — 10 minutes

Then pick a how-to for your project:

- [Apply SemTree to any project](how-to/apply-to-any-project.md)
- [Build a Neovim LSP for your language](how-to/neovim-lsp.md)
- [Use the `semtree` CLI](how-to/use-the-cli.md)

---

## Map of the docs

```
docs/
├── tutorials/          # Learning-oriented (start here)
│   ├── 01-install.md
│   ├── 02-parse-first-file.md
│   ├── 03-first-grammar.md
│   └── 04-neovim.md
├── how-to/             # Task-oriented
│   ├── use-the-cli.md
│   ├── write-a-grammar.md
│   ├── apply-to-any-project.md
│   ├── neovim-lsp.md
│   ├── import-tree-sitter.md
│   └── examples/
│       └── todo-lsp/   # Complete working LSP example
├── reference/          # Information-oriented
│   ├── dsl-syntax.md   # Full .semtree language
│   ├── cli.md          # Every command and flag
│   ├── builtins.md     # Built-in token types
│   └── output-formats.md
└── explanation/        # Understanding-oriented
    ├── architecture.md
    ├── green-red-trees.md
    ├── parsers-rd-vs-glr.md
    └── vs-tree-sitter.md
```

---

## Conventions

- Commands assume `semtree` is on your `PATH`.
- Paths like `grammars/json.semtree` are relative to the SemTree repo root unless noted.
- Copy-paste blocks are meant to work as written.
