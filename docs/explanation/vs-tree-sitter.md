# SemTree vs Tree-sitter

## Different product shapes

| | Tree-sitter | SemTree |
|--|------------|---------|
| Core product | Incremental parser library | Parser **plus** formatter, linter, refactor, IDE, AI APIs |
| Grammar authoring | `grammar.js` | `.semtree` DSL (+ JSON import) |
| Editor story | First-class in Neovim/Helix/etc. | Plugin + DIY LSP (documented) |
| Language | C core | Rust |

## What SemTree optimizes for

1. **One grammar → many tools** (not just highlighting)
2. **Rust-native embedding** and a batteries-included CLI
3. **Portable project recipe**: keep a `.semtree` file in *your* repo; install one binary

## What Tree-sitter still wins at today

- Ecosystem size (hundreds of grammars)
- Deep editor integrations already shipped
- Years of production hardening on GLR incremental parsing

## Practical advice

- Use SemTree when you control a DSL / language and want parse + tooling quickly.
- Import Tree-sitter grammars when you need a head start, then simplify into `.semtree`.
- For Neovim highlighting of mainstream languages, tree-sitter remains the default; SemTree shines for **your** languages and for full toolchain demos.
