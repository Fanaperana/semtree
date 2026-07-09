# todo-lsp — SemTree language server example

A minimal LSP server for a tiny `todo` language, powered by the `semtree` CLI.

Use this as a template for **any** language: swap the grammar, change the filetype, rebuild.

## Quick start

```bash
# 1. Install semtree (once)
cargo install --path ../../../../crates/semtree_cli

# 2. Build the LSP
cargo build --release

# 3. Parse a sample without LSP
semtree run -g grammars/todo.semtree -f sexp-pretty samples/demo.todo
```

## Neovim setup

```lua
vim.filetype.add({ extension = { todo = "todo" } })

vim.api.nvim_create_autocmd("FileType", {
  pattern = "todo",
  callback = function(args)
    local root = vim.fs.root(args.buf, { "grammars", "Cargo.toml" })
    vim.lsp.start({
      name = "todo-lsp",
      cmd = { root .. "/target/release/todo-lsp" },
      root_dir = root,
      cmd_env = {
        TODOLSP_GRAMMAR = root .. "/grammars/todo.semtree",
        TODOLSP_SEMTREE = vim.fn.exepath("semtree"),
      },
    })
  end,
})
```

Open `samples/demo.todo` and try hover / diagnostics / document symbols.

## Layout

```
todo-lsp/
├── Cargo.toml
├── grammars/todo.semtree
├── samples/demo.todo
├── src/main.rs          # LSP server
└── README.md
```

## Adapt to your language

1. Edit `grammars/todo.semtree` (or replace it).
2. Change `todo` filetype / `.todo` extension in Neovim config.
3. Rebuild: `cargo build --release`.
