# How to build a Neovim LSP for your language with SemTree

**Goal:** give Neovim hover, diagnostics, and document symbols for a custom language — using SemTree as the parser.

**Approach:** a small Rust language server that shells out to (or embeds) SemTree, speaking LSP over stdio. Neovim connects via `vim.lsp.start`.

This guide uses the complete example in [`examples/todo-lsp/`](examples/todo-lsp/).

---

## What you get

| Feature | How |
|---------|-----|
| Diagnostics | Parse errors from `semtree run -f inspect` |
| Hover | Node kind + range under cursor |
| Document symbols | Top-level rule nodes |
| Works for any grammar | Point at a `.semtree` file |

---

## Architecture

```
Neovim  --LSP/stdio-->  todo-lsp (Rust)
                              |
                              v
                     semtree run -g grammar.semtree -f inspect
                              |
                              v
                     syntax tree (DEPTH|START|END|KIND|TEXT)
```

You can later replace the subprocess with an in-process `RuntimeParser` for speed. Start with the CLI — it's easier to debug.

---

## Step 1 — Create the language + samples

```bash
cd docs/how-to/examples/todo-lsp
# grammar and samples are already here; or create your own:
# semtree init --name todo --output .
```

Grammar: `grammars/todo.semtree`  
Samples: `samples/demo.todo`

Verify:

```bash
semtree run -g grammars/todo.semtree -f inspect samples/demo.todo | head
```

## Step 2 — Build the LSP server

```bash
cd docs/how-to/examples/todo-lsp
cargo build --release
# binary: target/release/todo-lsp
```

## Step 3 — Wire Neovim

Add to your Neovim config (e.g. `lua/plugins/todo-lsp.lua` or inside lazy.nvim):

```lua
vim.api.nvim_create_autocmd("FileType", {
  pattern = "todo",
  callback = function(args)
    vim.lsp.start({
      name = "todo-lsp",
      cmd = { "/ABSOLUTE/PATH/TO/todo-lsp/target/release/todo-lsp" },
      root_dir = vim.fs.root(args.buf, { "grammars", ".git" }),
      -- Pass grammar path via env if you want:
      cmd_env = {
        TODOLSP_GRAMMAR = vim.fs.joinpath(
          vim.fs.root(args.buf, { "grammars" }) or ".",
          "grammars/todo.semtree"
        ),
      },
    })
  end,
})

-- filetype for .todo files
vim.filetype.add({ extension = { todo = "todo" } })
```

Replace the absolute path with your build output.

## Step 4 — Try it

```bash
nvim docs/how-to/examples/todo-lsp/samples/demo.todo
```

Then:

- Hover a word (`K` or `:lua vim.lsp.buf.hover()`)
- Open symbols (`:lua vim.lsp.buf.document_symbol()`)
- Break a line and watch diagnostics update on save / change

## Step 5 — Adapt to YOUR language

1. Replace `grammars/todo.semtree` with your grammar.
2. Change the file extension / filetype in the Neovim autocmd.
3. Rebuild `todo-lsp` (or rename the crate).
4. Point `TODOLSP_GRAMMAR` (or the equivalent env) at your grammar.

No SemTree core changes required.

---

## Design notes (so you can extend it)

### Diagnostics

Parse `inspect` lines; any parse errors printed on stderr by `semtree run` become LSP diagnostics. For richer errors, switch to `-f json` and read the error list.

### Hover

Find the smallest node whose `[start, end)` byte range contains the cursor offset. Show `kind` + snippet.

### Symbols

Collect nodes whose kind matches a allowlist (`TodoItem`, `FunctionDef`, `ClassDef`, …) configurable per language.

### Performance

For large files, embed SemTree:

```rust
use semtree_grammar::parse_semtree_dsl;
use semtree_runtime::RuntimeParser;

let grammar = parse_semtree_dsl(include_str!("../grammars/todo.semtree")).unwrap();
let parser = RuntimeParser::new(grammar);
let result = parser.parse(&source);
```

The example starts with a subprocess so you can see every command in the terminal while developing.

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| LSP never starts | Check `:LspInfo`, path to binary, filetype autocmd |
| No diagnostics | Run the same `semtree run -g ...` in a shell; fix grammar first |
| Wrong grammar | Set `TODOLSP_GRAMMAR` to an absolute path |
| Hover empty | Cursor may be on trivia; move onto a token |

---

## Next

- Read the full example: [`examples/todo-lsp/`](examples/todo-lsp/)
- DSL details: [DSL syntax](../reference/dsl-syntax.md)
- Portable project layout: [Apply to any project](apply-to-any-project.md)
